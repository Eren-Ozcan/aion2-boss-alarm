mod bosses;
mod capture;
mod core;
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
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder, WindowEvent};

use crate::capture::{now_ms, CaptureError};
use crate::core::{AlarmConfig, Core, Snapshot, Status};
use crate::recorder::Recorder;

struct AppState {
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

#[tauri::command]
fn set_alarm(app: AppHandle, state: State<AppState>, id: u32, enabled: bool, minutes: Vec<u32>) {
    let minutes = minutes.into_iter().filter(|&m| m <= 24 * 60).collect();
    lock(&state.core).set_alarm(id, AlarmConfig { enabled, minutes });
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
                lock(&core).set_status(Status::WaitingForGame { devices });
                emit_snapshot(&app, &core);
            }
            Err(e) => {
                let status = match e {
                    CaptureError::LibraryMissing(m) => Status::NpcapMissing(m),
                    CaptureError::NoDevices => Status::CaptureFailed("no network device with an IPv4 address".into()),
                    CaptureError::Other(m) => Status::CaptureFailed(m),
                };
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
        .invoke_handler(tauri::generate_handler![get_snapshot, get_dps, set_every_fight, get_all_skills, toggle_overlay, save_overlay_geometry, show_settings, get_character_name, set_character_name, get_ping, reset_dps, list_fights, load_fight, delete_fight, set_target_mode, get_skills, start_recording, stop_recording, set_alarm, set_all_enabled, retry_capture])
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
            let core = Arc::new(Mutex::new(Core::load(dir.join("state.json"))));
            let capturing = Arc::new(AtomicBool::new(false));
            let engine = DpsEngine::new(dir.clone());
            let history = engine.history();
            let ping = engine.ping_tracker();
            let dps = Arc::new(Mutex::new(engine));
            let recorder = Arc::new(Mutex::new(Recorder::default()));
            app.manage(AppState { core: core.clone(), dps: dps.clone(), history: history.clone(), ping: ping.clone(), recorder: recorder.clone(), capturing: capturing.clone() });

            let show = MenuItem::with_id(app, "show", "Settings & alarms", true, None::<&str>)?;
            let overlay_item = MenuItem::with_id(app, "overlay", "Overlay", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&overlay_item, &show, &quit])?;
            let mut tray = TrayIconBuilder::new()
                .tooltip("AION2 Boss Alarm")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_main(app),
                    "overlay" => {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move { let _ = toggle_overlay(app).await; });
                    }
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

            if let Err(e) = open_overlay(app.handle()) {
                eprintln!("overlay failed to open: {e}");
                show_main(app.handle());
            }
            start_history_loop(dps.clone(), history);
            start_capture(app.handle().clone(), core.clone(), dps, ping, recorder, capturing);
            start_alarm_loop(app.handle().clone(), core);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running AION2 Boss Alarm");
}
