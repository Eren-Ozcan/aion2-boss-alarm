mod bosses;
mod capture;
mod core;
mod parser;
mod stream;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};

use crate::capture::{now_ms, CaptureError};
use crate::core::{AlarmConfig, Core, Snapshot, Status};

struct AppState {
    core: Arc<Mutex<Core>>,
    capturing: Arc<AtomicBool>,
}

fn lock(core: &Mutex<Core>) -> MutexGuard<'_, Core> {
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
    start_capture(app, state.core.clone(), state.capturing.clone());
}

/// Start the capture and decode loop on a background thread, unless one runs.
fn start_capture(app: AppHandle, core: Arc<Mutex<Core>>, capturing: Arc<AtomicBool>) {
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
            let Some(decoded) = tracker.feed(seg) else { continue };
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
        .invoke_handler(tauri::generate_handler![get_snapshot, set_alarm, set_all_enabled, retry_capture])
        .on_window_event(|window, event| {
            // Closing the window keeps the alarms running from the tray.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let core = Arc::new(Mutex::new(Core::load(dir.join("state.json"))));
            let capturing = Arc::new(AtomicBool::new(false));
            app.manage(AppState { core: core.clone(), capturing: capturing.clone() });

            let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            let mut tray = TrayIconBuilder::new()
                .tooltip("AION2 Boss Alarm")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_main(app),
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

            start_capture(app.handle().clone(), core.clone(), capturing);
            start_alarm_loop(app.handle().clone(), core);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running AION2 Boss Alarm");
}
