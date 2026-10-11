mod bosses;
mod capture;
mod applog;
mod core;
mod events;
mod parser;
mod recorder;
mod stream;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use dps_core::capture::captured_payload::CapturedPayload;
use dps_core::combat::ping_tracker::PingTracker;
use dps_core::engine::DpsEngine;
use dps_core::entity::details_context::TargetDetailsResponse;
use dps_core::entity::dps_data::DpsData;
use dps_core::entity::fight_record::{FightRecord, FightSummary};
use dps_core::history::fight_history::FightHistoryManager;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, State, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt as _};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::capture::{now_ms, CaptureError};
use crate::core::{AlarmConfig, Core, Snapshot, Status};
use crate::recorder::Recorder;

struct AppState {
    terms_ok: Arc<AtomicBool>,
    /// The overlay lets mouse clicks through to the game.
    locked: Arc<AtomicBool>,
    core: Arc<Mutex<Core>>,
    dps: Arc<Mutex<DpsEngine>>,
    history: Arc<FightHistoryManager>,
    ping: Arc<PingTracker>,
    recorder: Arc<Mutex<Recorder>>,
    capturing: Arc<AtomicBool>,
}

fn lock<T>(core: &Mutex<T>) -> MutexGuard<'_, T> {
    core.lock().unwrap_or_else(|e| e.into_inner())
}

fn emit_snapshot(app: &AppHandle, core: &Mutex<Core>) {
    let snap = lock(core).snapshot();
    let _ = app.emit("snapshot", snap);
}

#[tauri::command]
fn get_snapshot(state: State<AppState>) -> Snapshot {
    lock(&state.core).snapshot()
}

#[tauri::command]
fn get_dps(state: State<AppState>) -> DpsData {
    lock(&state.dps).snapshot()
}

#[tauri::command]
fn set_target_mode(state: State<AppState>, mode: String) {
    const MODES: [&str; 4] = ["bossTargets", "lastHitByMe", "allTargets", "trainTargets"];
    if MODES.contains(&mode.as_str()) {
        lock(&state.dps).set_target_mode(&mode);
    }
}

#[tauri::command]
fn set_every_fight(state: State<AppState>, on: bool) {
    lock(&state.dps).set_every_fight(on);
}

#[tauri::command]
fn get_skills(state: State<AppState>, target_id: i32, actor_id: i32) -> TargetDetailsResponse {
    lock(&state.dps).details(target_id, actor_id)
}

#[tauri::command]
fn get_all_skills(state: State<AppState>, target_id: i32) -> TargetDetailsResponse {
    lock(&state.dps).details_all(target_id)
}

/// Where the overlay was last left (logical pixels), kept in the app data dir.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy)]
struct OverlayGeometry {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

const DEFAULT_OVERLAY: (f64, f64) = (420.0, 200.0);
/// Gap kept from the screen edge for the first placement, logical pixels.
const EDGE_MARGIN: f64 = 24.0;

fn overlay_file(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.path().app_data_dir().ok().map(|d| d.join("overlay.json"))
}

/// The saved geometry, if it is still on a connected screen (a monitor that was
/// unplugged would otherwise leave the overlay out of reach).
fn saved_geometry(app: &AppHandle) -> Option<OverlayGeometry> {
    let g: OverlayGeometry = serde_json::from_str(&std::fs::read_to_string(overlay_file(app)?).ok()?).ok()?;
    let visible = app.available_monitors().ok()?.iter().any(|m| {
        let sf = m.scale_factor();
        let (mx, my) = (m.position().x as f64 / sf, m.position().y as f64 / sf);
        let (mw, mh) = (m.size().width as f64 / sf, m.size().height as f64 / sf);
        g.x + 40.0 >= mx && g.y + 20.0 >= my && g.x + 40.0 <= mx + mw && g.y + 20.0 <= my + mh
    });
    visible.then_some(g)
}

/// Called by the overlay page when it is moved or resized.
#[tauri::command]
fn save_overlay_geometry(app: AppHandle, x: f64, y: f64, w: f64, h: f64) {
    if !(x.is_finite() && y.is_finite() && w.is_finite() && h.is_finite()) {
        return;
    }
    let g = OverlayGeometry { x, y, w: w.clamp(220.0, 1600.0), h: h.clamp(60.0, 1600.0) };
    if let (Some(path), Ok(json)) = (overlay_file(&app), serde_json::to_string(&g)) {
        let _ = std::fs::write(path, json);
    }
}

/// Create the always-on-top, see-through DPS overlay unless it already exists.
/// Window creation must not run on the main thread from a sync command (it
/// deadlocks on Windows), so callers are async.
fn open_overlay(app: &AppHandle) -> Result<(), String> {
    if app.get_webview_window("overlay").is_some() {
        return Ok(());
    }
    let geometry = saved_geometry(app);
    let (w, h) = geometry.map_or(DEFAULT_OVERLAY, |g| (g.w, g.h));
    let mut builder = WebviewWindowBuilder::new(app, "overlay", WebviewUrl::App("overlay.html".into()))
        .title("DPS overlay")
        .inner_size(w, h)
        .min_inner_size(300.0, 70.0)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true);
    match geometry {
        Some(g) => builder = builder.position(g.x, g.y),
        // First run: the top-left corner of the main screen.
        None => {
            if let Ok(Some(m)) = app.primary_monitor() {
                let sf = m.scale_factor();
                let x = m.position().x as f64 / sf + EDGE_MARGIN;
                let y = m.position().y as f64 / sf + EDGE_MARGIN;
                builder = builder.position(x, y);
            }
        }
    }
    builder.build().map(|_| ()).map_err(|e| e.to_string())
}

/// Let clicks go through the overlay to the game (or take them again).
fn apply_lock(app: &AppHandle, locked: bool) {
    if let Some(w) = app.get_webview_window("overlay") {
        let _ = w.set_ignore_cursor_events(locked);
    }
    if let Some(state) = app.try_state::<AppState>() {
        state.locked.store(locked, Ordering::SeqCst);
    }
    let _ = app.emit("overlay-locked", locked);
}

fn toggle_lock(app: &AppHandle) -> bool {
    let now = !app.state::<AppState>().locked.load(Ordering::SeqCst);
    apply_lock(app, now);
    now
}

#[tauri::command]
fn toggle_overlay_lock(app: AppHandle) -> bool {
    toggle_lock(&app)
}

#[tauri::command]
fn get_overlay_locked(state: State<AppState>) -> bool {
    state.locked.load(Ordering::SeqCst)
}

/// Show or hide the overlay window without closing it (hotkey, auto-hide).
fn set_overlay_visible_inner(app: &AppHandle, visible: bool) {
    if let Some(w) = app.get_webview_window("overlay") {
        if visible {
            let _ = w.show();
        } else {
            let _ = w.hide();
        }
    } else if visible {
        let _ = open_overlay(app);
    }
}

#[tauri::command]
fn set_overlay_visible(app: AppHandle, visible: bool) {
    set_overlay_visible_inner(&app, visible);
}

fn toggle_overlay_visible_inner(app: &AppHandle) {
    let visible = app.get_webview_window("overlay").and_then(|w| w.is_visible().ok()).unwrap_or(false);
    set_overlay_visible_inner(app, !visible);
}

/// Put the overlay back at the top-left of the main screen at its default size.
#[tauri::command]
fn reset_overlay_position(app: AppHandle) {
    if let Some(path) = overlay_file(&app) {
        let _ = std::fs::remove_file(path);
    }
    if let Some(w) = app.get_webview_window("overlay") {
        let (mut x, mut y) = (EDGE_MARGIN, EDGE_MARGIN);
        if let Ok(Some(m)) = app.primary_monitor() {
            let sf = m.scale_factor();
            x += m.position().x as f64 / sf;
            y += m.position().y as f64 / sf;
        }
        let _ = w.set_size(LogicalSize::new(DEFAULT_OVERLAY.0, DEFAULT_OVERLAY.1));
        let _ = w.set_position(LogicalPosition::new(x, y));
        let _ = w.show();
    }
}

#[tauri::command]
fn set_muted(app: AppHandle, state: State<AppState>, muted: bool) {
    lock(&state.core).set_muted(muted);
    applog::line(if muted { "alarms muted" } else { "alarms unmuted" });
    emit_snapshot(&app, &state.core);
}

fn toggle_mute(app: &AppHandle) {
    let state = app.state::<AppState>();
    let muted = !lock(&state.core).muted();
    lock(&state.core).set_muted(muted);
    applog::line(if muted { "alarms muted" } else { "alarms unmuted" });
    emit_snapshot(app, &state.core);
}

#[tauri::command]
fn set_overlay_at_startup(app: AppHandle, state: State<AppState>, on: bool) {
    lock(&state.core).set_overlay_at_startup(on);
    emit_snapshot(&app, &state.core);
}

#[tauri::command]
fn get_autostart(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
fn set_autostart(app: AppHandle, on: bool) -> Result<(), String> {
    let launch = app.autolaunch();
    let r = if on { launch.enable() } else { launch.disable() };
    r.map_err(|e| e.to_string())
}

// Global shortcuts. They differ from the A2Tools ones (Ctrl+Alt+L, Ctrl+Alt+Up), so both can run together.
fn shortcut_lock() -> Shortcut {
    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyK)
}
fn shortcut_visible() -> Shortcut {
    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyJ)
}
fn shortcut_mute() -> Shortcut {
    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyM)
}

fn on_shortcut(app: &AppHandle, shortcut: &Shortcut) {
    if *shortcut == shortcut_lock() {
        toggle_lock(app);
    } else if *shortcut == shortcut_visible() {
        toggle_overlay_visible_inner(app);
    } else if *shortcut == shortcut_mute() {
        toggle_mute(app);
    }
}

/// Open the overlay, or close it when it is already open. True when open now.
#[tauri::command]
async fn toggle_overlay(app: AppHandle) -> Result<bool, String> {
    if let Some(w) = app.get_webview_window("overlay") {
        w.close().map_err(|e| e.to_string())?;
        return Ok(false);
    }
    open_overlay(&app)?;
    Ok(true)
}

/// Bring the full window (alarms, history, settings) forward.
#[tauri::command]
fn show_settings(app: AppHandle) {
    show_main(&app);
}

/// Round trip to the game server in ms, once a ping reply has been seen.
#[tauri::command]
fn get_ping(state: State<AppState>) -> Option<i32> {
    state.ping.current_ping_ms()
}

#[tauri::command]
fn get_character_name(state: State<AppState>) -> Option<String> {
    lock(&state.dps).character_name()
}

#[tauri::command]
fn set_character_name(state: State<AppState>, name: String) {
    lock(&state.dps).set_character_name(&name);
}

#[tauri::command]
fn reset_dps(state: State<AppState>) {
    lock(&state.dps).reset();
}

#[tauri::command]
fn list_fights(state: State<AppState>) -> Vec<FightSummary> {
    state.history.list_fights()
}

#[tauri::command]
fn load_fight(state: State<AppState>, id: String) -> Result<FightRecord, String> {
    state.history.load_fight(&id)
}

#[tauri::command]
fn delete_fight(state: State<AppState>, id: String) -> Result<(), String> {
    state.history.delete_fight(&id)
}

/// Save boss fights that finished, every 30 s. The engine lock is held only to
/// collect them; the disk work happens after it is released.
fn start_history_loop(dps: Arc<Mutex<DpsEngine>>, history: Arc<FightHistoryManager>) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(30));
        let records = lock(&dps).finished_fights();
        for record in &records {
            let _ = history.save_fight(record);
        }
    });
}

#[tauri::command]
fn start_recording(app: AppHandle, state: State<AppState>) -> Result<String, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("recordings");
    let path = lock(&state.recorder).start(&dir).map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

#[tauri::command]
fn stop_recording(state: State<AppState>) -> String {
    let snap = lock(&state.dps).snapshot();
    let mut notes = format!("target: {}
battle_ms: {}
", snap.target_name, snap.battle_time);
    let mut rows: Vec<_> = snap.map.values().collect();
    rows.sort_by(|a, b| b.amount.total_cmp(&a.amount));
    for p in rows {
        notes.push_str(&format!("{}	{}	damage={:.0}	dps={:.1}
", p.nickname, p.job, p.amount, p.dps));
    }
    match lock(&state.recorder).stop(&notes) {
        Some((path, bytes)) => format!("{} ({} KB)", path.display(), bytes / 1024),
        None => String::new(),
    }
}


/// The first-run notice is accepted once; until then nothing is captured.
fn terms_file(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.path().app_data_dir().ok().map(|d| d.join("terms-accepted"))
}

#[tauri::command]
fn terms_accepted(state: State<AppState>) -> bool {
    state.terms_ok.load(Ordering::SeqCst)
}

#[tauri::command]
fn accept_terms(app: AppHandle, state: State<AppState>) {
    if let Some(path) = terms_file(&app) {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, env!("CARGO_PKG_VERSION"));
    }
    state.terms_ok.store(true, Ordering::SeqCst);
    applog::line("first-run notice accepted");
    start_capture(app, state.core.clone(), state.dps.clone(), state.ping.clone(), state.recorder.clone(), state.capturing.clone());
}

/// Open a web page in the default browser. Only the Npcap site, which is the
/// one link the app shows; anything else is refused.
#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    const ALLOWED: [&str; 4] = [
        "https://npcap.com",
        "https://npcap.com/",
        "https://erenozcan.dev",
        "https://github.com/Eren-Ozcan",
    ];
    if !ALLOWED.contains(&url.as_str()) {
        return Err("link not allowed".into());
    }
    std::process::Command::new("explorer.exe").arg(&url).spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// Show the folder with the app's log file.
#[tauri::command]
fn open_log_folder(app: AppHandle) -> Result<String, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let log = dir.join(applog::FILE_NAME);
    if log.exists() {
        std::process::Command::new("explorer.exe").arg(format!("/select,{}", log.display())).spawn()
    } else {
        std::process::Command::new("explorer.exe").arg(&dir).spawn()
    }
    .map_err(|e| e.to_string())?;
    Ok(log.display().to_string())
}

/// Save a fight export (JSON or CSV text made by the page) under the app's
/// `exports` folder and show it. The name is reduced to a safe file name.
#[tauri::command]
fn save_export(app: AppHandle, name: String, content: String) -> Result<String, String> {
    let safe: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') { c } else { '_' })
        .take(80)
        .collect();
    if safe.is_empty() || safe.starts_with('.') || !(safe.ends_with(".json") || safe.ends_with(".csv")) {
        return Err("bad file name".into());
    }
    if content.len() > 20 * 1024 * 1024 {
        return Err("export too large".into());
    }
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("exports");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(&safe);
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    let _ = std::process::Command::new("explorer.exe").arg(format!("/select,{}", path.display())).spawn();
    applog::line(&format!("exported {safe}"));
    Ok(path.display().to_string())
}

#[tauri::command]
fn get_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// An update was found: a Windows notification, and the tray tooltip says so for as long
/// as the app runs, since the window is usually closed to the tray.
#[tauri::command]
fn announce_update(app: AppHandle, title: String, body: String, tooltip: String) {
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(tooltip));
    }
    if let Err(e) = app.notification().builder().title(title).body(body).show() {
        applog::line(&format!("update notification failed: {e}"));
    }
}

#[tauri::command]
fn set_alarm(app: AppHandle, state: State<AppState>, id: u32, enabled: bool, minutes: Vec<u32>) {
    let minutes = minutes.into_iter().filter(|&m| m <= 24 * 60).collect();
    lock(&state.core).set_alarm(id, AlarmConfig { enabled, minutes });
    emit_snapshot(&app, &state.core);
}

/// The server clock's offset from UTC (hours), which the event schedules are in.
#[tauri::command]
fn set_server_offset(app: AppHandle, state: State<AppState>, hours: i32) {
    lock(&state.core).set_server_offset(hours);
    emit_snapshot(&app, &state.core);
}

#[tauri::command]
fn set_all_enabled(app: AppHandle, state: State<AppState>, enabled: bool) {
    lock(&state.core).set_all_enabled(enabled);
    emit_snapshot(&app, &state.core);
}

#[tauri::command]
fn retry_capture(app: AppHandle, state: State<AppState>) {
    start_capture(app, state.core.clone(), state.dps.clone(), state.ping.clone(), state.recorder.clone(), state.capturing.clone());
}

/// Start the capture and decode loop on a background thread, unless one runs.
fn start_capture(
    app: AppHandle,
    core: Arc<Mutex<Core>>,
    dps: Arc<Mutex<DpsEngine>>,
    ping: Arc<PingTracker>,
    recorder: Arc<Mutex<Recorder>>,
    capturing: Arc<AtomicBool>,
) {
    if capturing.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || {
        let running = Arc::new(AtomicBool::new(true));
        let (tx, rx) = mpsc::channel();
        match capture::start(tx, running.clone()) {
            Ok(devices) => {
                applog::line(&format!("capture started on {devices} device(s)"));
                lock(&core).set_status(Status::WaitingForGame { devices });
                emit_snapshot(&app, &core);
            }
            Err(e) => {
                let status = match e {
                    CaptureError::LibraryMissing(m) => Status::NpcapMissing(m),
                    CaptureError::NoDevices => Status::CaptureFailed("no network device with an IPv4 address".into()),
                    CaptureError::Other(m) => Status::CaptureFailed(m),
                };
                applog::line(&format!("capture failed: {}", match &status { Status::NpcapMissing(m) | Status::CaptureFailed(m) => m.as_str(), _ => "" }));
                lock(&core).set_status(status);
                emit_snapshot(&app, &core);
                capturing.store(false, Ordering::SeqCst);
                return;
            }
        }

        let mut tracker = stream::Tracker::default();
        let mut connected = false;
        loop {
            let seg = match rx.recv_timeout(Duration::from_secs(1)) {
                Ok(s) => s,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => break,
            };
            note_ping(&ping, &seg);
            let Some(decoded) = tracker.feed(seg) else { continue };
            lock(&recorder).write(decoded.flow, &decoded.raw);
            lock(&dps).feed(decoded.flow, &decoded.raw);
            let local = now_ms();
            let mut changed = false;
            let mut c = lock(&core);
            if !connected {
                connected = true;
                applog::line("game connection seen");
                c.set_status(Status::Connected);
                changed = true;
            }
            for body in &decoded.bodies {
                if let Some(server_ms) = parser::server_time_ms(body) {
                    c.note_server_time(server_ms, local);
                } else if body.starts_with(&[0x01, 0x91]) {
                    let server_now = (local as i64 + c.offset_ms()).max(0) as u64;
                    if let Some(list) = parser::parse_boss_list(body, server_now) {
                        c.apply_list(&list, local);
                        changed = true;
                    }
                }
            }
            drop(c);
            if changed {
                emit_snapshot(&app, &core);
            }
        }
        running.store(false, Ordering::SeqCst);
        capturing.store(false, Ordering::SeqCst);
    });
}

/// Both directions of the game connection go to the ping tracker, which pairs
/// each ping request the client sends with the server's reply.
fn note_ping(ping: &PingTracker, seg: &capture::Segment) {
    const GAME_PORT: u16 = 13328;
    if seg.payload.is_empty() || (seg.src_port != GAME_PORT && seg.dst_port != GAME_PORT) {
        return;
    }
    ping.on_packet(
        &CapturedPayload {
            src_port: seg.src_port,
            dst_port: seg.dst_port,
            data: seg.payload.clone(),
            device_name: None,
            captured_at_ms: now_ms() as i64,
            src_ip: None,
            dst_ip: None,
            tcp_seq: seg.seq,
            tcp_ack: 0,
        },
        GAME_PORT,
    );
}

fn start_alarm_loop(app: AppHandle, core: Arc<Mutex<Core>>) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(500));
        let due = lock(&core).due_alarms(now_ms());
        for a in due {
            let _ = app.emit("alarm", a);
        }
    });
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        on_shortcut(app, shortcut);
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![toggle_overlay_lock, get_overlay_locked, set_overlay_visible, reset_overlay_position, set_muted, set_overlay_at_startup, get_autostart, set_autostart, get_snapshot, get_dps, set_every_fight, get_all_skills, toggle_overlay, save_overlay_geometry, show_settings, get_character_name, set_character_name, get_ping, reset_dps, list_fights, load_fight, delete_fight, set_target_mode, get_skills, start_recording, stop_recording, set_alarm, set_all_enabled, set_server_offset, retry_capture, terms_accepted, accept_terms, open_url, open_log_folder, save_export, get_version, announce_update])
        .on_window_event(|window, event| {
            // Closing the main window keeps the alarms running from the tray;
            // the overlay just closes.
            if window.label() != "main" {
                return;
            }
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            applog::init(&dir);
            applog::line(&format!("started, version {}", env!("CARGO_PKG_VERSION")));
            let core = Arc::new(Mutex::new(Core::load(dir.join("state.json"))));
            let capturing = Arc::new(AtomicBool::new(false));
            let engine = DpsEngine::new(dir.clone());
            let history = engine.history();
            let ping = engine.ping_tracker();
            let dps = Arc::new(Mutex::new(engine));
            let recorder = Arc::new(Mutex::new(Recorder::default()));
            let terms_ok = Arc::new(AtomicBool::new(dir.join("terms-accepted").exists()));
            app.manage(AppState { locked: Arc::new(AtomicBool::new(false)), terms_ok: terms_ok.clone(), core: core.clone(), dps: dps.clone(), history: history.clone(), ping: ping.clone(), recorder: recorder.clone(), capturing: capturing.clone() });

            let show = MenuItem::with_id(app, "show", "Settings & alarms", true, None::<&str>)?;
            let overlay_item = MenuItem::with_id(app, "overlay", "Show / hide overlay", true, None::<&str>)?;
            let lock_item = MenuItem::with_id(app, "lock", "Lock / unlock overlay (clicks pass through)", true, None::<&str>)?;
            let mute_item = MenuItem::with_id(app, "mute", "Mute / unmute alarms", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&overlay_item, &lock_item, &mute_item, &show, &quit])?;
            let mut tray = TrayIconBuilder::with_id("main")
                .tooltip("BellMeter")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_main(app),
                    "overlay" => toggle_overlay_visible_inner(app),
                    "lock" => {
                        toggle_lock(app);
                    }
                    "mute" => toggle_mute(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        show_main(tray.app_handle());
                    }
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;

            for (name, sc) in [("lock", shortcut_lock()), ("show/hide", shortcut_visible()), ("mute", shortcut_mute())] {
                if let Err(e) = app.global_shortcut().register(sc) {
                    applog::line(&format!("shortcut {name} not registered: {e}"));
                }
            }
            if !lock(&core).overlay_at_startup() {
                // The overlay stays closed until the tray or the window opens it.
            } else if let Err(e) = open_overlay(app.handle()) {
                applog::line(&format!("overlay failed to open: {e}"));
                show_main(app.handle());
            }
            // The first-run notice is shown in the main window before anything is read.
            if !terms_ok.load(Ordering::SeqCst) {
                show_main(app.handle());
            }
            start_history_loop(dps.clone(), history);
            if terms_ok.load(Ordering::SeqCst) {
                start_capture(app.handle().clone(), core.clone(), dps, ping, recorder, capturing);
            }
            start_alarm_loop(app.handle().clone(), core);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running BellMeter");
}
