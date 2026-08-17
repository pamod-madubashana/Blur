#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod network;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Position, State, WebviewWindow, WindowEvent};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri_plugin_dialog::DialogExt;

#[derive(Default)]
struct AppState {
    running: Arc<AtomicBool>,
    tray_id: std::sync::Mutex<Option<String>>,
}

#[derive(Serialize, Deserialize, Default)]
struct Config {
    game_path: Option<String>,
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve app data dir: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("blur-lan-launcher.config.json"))
}

fn load_config(app: &AppHandle) -> Config {
    if let Ok(path) = config_path(app) {
        if let Ok(text) = fs::read_to_string(&path) {
            if let Ok(cfg) = serde_json::from_str::<Config>(&text) {
                return cfg;
            }
        }
    }
    Config::default()
}

fn save_config(app: &AppHandle, cfg: &Config) -> Result<(), String> {
    let path = config_path(app)?;
    let text = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| e.to_string())
}

const WINDOW_MARGIN_X_PX: i32 = 16;
const WINDOW_MARGIN_Y_PX: i32 = 50;

fn position_window_bottom_right(window: &WebviewWindow) -> Result<(), String> {
    let monitor = window
        .current_monitor()
        .map_err(|e| e.to_string())?
        .or_else(|| window.primary_monitor().ok().flatten())
        .ok_or("no monitor available".to_string())?;
    let monitor_pos = monitor.position();
    let monitor_size = monitor.size();
    let window_size = window.outer_size().map_err(|e| e.to_string())?;

    let x = monitor_pos.x + monitor_size.width as i32 - window_size.width as i32 - WINDOW_MARGIN_X_PX;
    let y = monitor_pos.y + monitor_size.height as i32 - window_size.height as i32 - WINDOW_MARGIN_Y_PX;

    window.set_position(Position::Physical(PhysicalPosition::new(x, y))).map_err(|e| e.to_string())
}

fn emit_log(app: &AppHandle, msg: impl Into<String>) {
    let _ = app.emit("log", msg.into());
}

fn emit_status(app: &AppHandle, status: &str) {
    let _ = app.emit("status", status.to_string());
}

#[derive(Serialize, Clone)]
struct AdapterProgressPayload {
    name: String,
    phase: String,
}

fn emit_adapter_progress(app: &AppHandle, name: &str, phase: &str) {
    let _ = app.emit("adapter_progress", AdapterProgressPayload {
        name: name.to_string(),
        phase: phase.to_string(),
    });
}

fn emit_adapters_list(app: &AppHandle, adapters: &[String]) {
    let _ = app.emit("adapters", adapters.to_vec());
}

#[derive(Serialize, Clone)]
struct FileCheckPayload {
    file: String,
    status: String,
}

fn emit_file_check(app: &AppHandle, file: &str, status: &str) {
    let _ = app.emit("file_check", FileCheckPayload {
        file: file.to_string(),
        status: status.to_string(),
    });
}

fn emit_file_check_done(app: &AppHandle, all_ok: bool) {
    let _ = app.emit("file_check_done", all_ok);
}

#[derive(Clone, serde::Serialize)]
struct FirewallCheckPayload {
    rule: String,
    status: String,
}

fn emit_firewall_check(app: &AppHandle, rule: &str, status: &str) {
    let _ = app.emit("firewall_check", FirewallCheckPayload {
        rule: rule.to_string(),
        status: status.to_string(),
    });
}

fn emit_firewall_check_done(app: &AppHandle) {
    let _ = app.emit("firewall_check_done", ());
}

fn check_and_copy_files(app: &AppHandle, game_dir: &str) -> Result<(), String> {
    // In dev mode, files/ is relative to CWD; in bundled mode, it's in the resource dir
    let dev_path = PathBuf::from("files");
    let bundled_path = app.path().resource_dir().ok()
        .and_then(|r| Some(r.join("files")));

    let files_dir = if dev_path.exists() {
        dev_path
    } else if let Some(ref p) = bundled_path {
        if p.exists() { p.clone() } else { return Ok(()); }
    } else {
        return Ok(());
    };

    emit_log(app, "Checking online fix files...");
    let game_path = PathBuf::from(game_dir);
    let prefix = files_dir.to_string_lossy().to_string();

    fn walk_and_check(app: &AppHandle, src: &PathBuf, dest: &PathBuf, prefix: &str) -> Result<(), String> {
        if src.is_dir() {
            for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                let path = entry.path();
                let relative = path.strip_prefix(prefix).unwrap_or(&path);
                let target = dest.join(relative);

                if path.is_dir() {
                    if !target.exists() {
                        fs::create_dir_all(&target).map_err(|e| e.to_string())?;
                    }
                    walk_and_check(app, &path, dest, prefix)?;
                } else {
                    let file_name = relative.to_string_lossy().to_string();
                    if target.exists() {
                        emit_log(app, format!("  OK: {file_name}"));
                        emit_file_check(app, &file_name, "ok");
                    } else {
                        emit_log(app, format!("  COPY: {file_name}"));
                        emit_file_check(app, &file_name, "copying");
                        if let Some(parent) = target.parent() {
                            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                        }
                        fs::copy(&path, &target).map_err(|e| e.to_string())?;
                        emit_file_check(app, &file_name, "copied");
                    }
                }
            }
        }
        Ok(())
    }

    walk_and_check(app, &files_dir, &game_path, &prefix)?;

    emit_log(app, "File check complete.");
    emit_file_check_done(app, true);
    Ok(())
}

#[cfg(target_os = "windows")]
fn check_firewall_rules(app: &AppHandle) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    emit_log(app, "Checking firewall rules...");

    // Check if Blur inbound rule exists
    let output = Command::new("netsh")
        .args(["advfirewall", "firewall", "show", "rule", "name=Blur LAN Launcher - Inbound"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed to run netsh: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.contains("Rule Name") {
        emit_log(app, "  Inbound rule: OK");
        emit_firewall_check(app, "Blur LAN Launcher - Inbound", "ok");
    } else {
        emit_log(app, "  Inbound rule: MISSING - creating...");
        emit_firewall_check(app, "Blur LAN Launcher - Inbound", "creating");
        let _ = Command::new("netsh")
            .args([
                "advfirewall", "firewall", "add", "rule",
                "name=Blur LAN Launcher - Inbound",
                "dir=in", "action=allow", "enable=yes",
                "program=Blur.exe", "protocol=any",
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        emit_log(app, "  Inbound rule: CREATED");
        emit_firewall_check(app, "Blur LAN Launcher - Inbound", "created");
    }

    // Check if Blur outbound rule exists
    let output = Command::new("netsh")
        .args(["advfirewall", "firewall", "show", "rule", "name=Blur LAN Launcher - Outbound"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed to run netsh: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.contains("Rule Name") {
        emit_log(app, "  Outbound rule: OK");
        emit_firewall_check(app, "Blur LAN Launcher - Outbound", "ok");
    } else {
        emit_log(app, "  Outbound rule: MISSING - creating...");
        emit_firewall_check(app, "Blur LAN Launcher - Outbound", "creating");
        let _ = Command::new("netsh")
            .args([
                "advfirewall", "firewall", "add", "rule",
                "name=Blur LAN Launcher - Outbound",
                "dir=out", "action=allow", "enable=yes",
                "program=Blur.exe", "protocol=any",
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        emit_log(app, "  Outbound rule: CREATED");
        emit_firewall_check(app, "Blur LAN Launcher - Outbound", "created");
    }

    // Enable all ICMPv4 rules
    emit_log(app, "Enabling ICMPv4 rules...");
    emit_firewall_check(app, "ICMPv4 Rules", "checking");
    let icmp_names = [
        "Core Networking Diagnostics - ICMP Echo Request (ICMPv4-In)",
        "Core Networking Diagnostics - ICMP Echo Request (ICMPv4-Out)",
        "File and Printer Sharing (Echo Request - ICMPv4-In)",
        "File and Printer Sharing (Echo Request - ICMPv4-Out)",
        "File and Printer Sharing (Echo Request - ICMPv4-In)*@{Domain,Private,Public}",
        "File and Printer Sharing (Echo Request - ICMPv4-Out)*@{Domain,Private,Public}",
        "File and Printer Sharing (Restrictive) (Echo Request - ICMPv4-In)",
        "File and Printer Sharing (Restrictive) (Echo Request - ICMPv4-Out)",
        "Core Networking - Destination Unreachable Fragmentation Needed (ICMPv4-In)",
    ];

    for name in &icmp_names {
        let output = Command::new("netsh")
            .args([
                "advfirewall", "firewall", "set", "rule",
                &format!("name={name}"),
                "new", "enable=yes",
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        if let Ok(out) = output {
            let msg = String::from_utf8_lossy(&out.stdout);
            if msg.contains("Ok") {
                emit_log(app, format!("  ICMPv4: {name} -> enabled"));
            }
        }
    }
    emit_firewall_check(app, "ICMPv4 Rules", "ok");

    emit_log(app, "Firewall check complete.");
    emit_firewall_check_done(app);
    Ok(())
}

#[tauri::command]
fn get_saved_path(app: AppHandle) -> Option<String> {
    let cfg = load_config(&app);
    match cfg.game_path {
        Some(p) if PathBuf::from(&p).exists() => Some(p),
        _ => None,
    }
}

#[tauri::command]
async fn pick_game_path(app: AppHandle) -> Option<String> {
    let file = app
        .dialog()
        .file()
        .add_filter("Blur.exe", &["exe"])
        .set_title("Select Blur.exe")
        .blocking_pick_file();

    if let Some(path) = file {
        let path_str = path.to_string();
        let _ = save_config(&app, &Config { game_path: Some(path_str.clone()) });
        Some(path_str)
    } else {
        None
    }
}

#[tauri::command]
fn is_running(state: State<AppState>) -> bool {
    state.running.load(Ordering::SeqCst)
}

#[tauri::command]
fn start_lan_mode(app: AppHandle, state: State<AppState>, game_path: String) -> Result<(), String> {
    if state.running.swap(true, Ordering::SeqCst) {
        return Err("Already running.".into());
    }

    let running_flag = state.running.clone();
    let app_handle = app.clone();

    thread::spawn(move || {
        let result = run_sequence(&app_handle, &game_path);
        if let Err(e) = result {
            emit_log(&app_handle, format!("ERROR: {e}"));
        }
        running_flag.store(false, Ordering::SeqCst);
        emit_status(&app_handle, "idle");
        let _ = app_handle.emit("finished", ());
    });

    Ok(())
}

fn run_sequence(app: &AppHandle, game_path: &str) -> Result<(), String> {
    emit_log(app, "=== BLUR LAN MODE START ===");

    let path_buf = PathBuf::from(game_path);
    let default_dir = PathBuf::from(".");
    let work_dir = path_buf.parent().unwrap_or(&default_dir);

    // Check and copy online fix files before disabling adapters
    emit_status(app, "checking");
    check_and_copy_files(app, &work_dir.to_string_lossy())?;

    // Check and enable firewall rules
    emit_status(app, "firewall");
    check_firewall_rules(app)?;

    // Disable virtual adapters (VMware, VirtualBox, etc.)
    emit_log(app, "Scanning for virtual adapters to disable...");
    let all_virtual = network::list_virtual_adapters()?;
    if all_virtual.is_empty() {
        emit_log(app, "No virtual adapters found.");
    } else {
        emit_log(app, format!("Found {} virtual adapter(s) to disable.", all_virtual.len()));
    }
    emit_adapters_list(app, &all_virtual);
    emit_status(app, "disabling");
    for name in &all_virtual {
        emit_adapter_progress(app, name, "processing");
        emit_log(app, format!("Disabling: {name}"));
        match network::disable_adapter(name) {
            Ok(()) => {
                emit_log(app, "  -> OK".to_string());
                emit_adapter_progress(app, name, "done");
            }
            Err(e) => {
                emit_log(app, format!("  -> failed: {e}"));
                emit_adapter_progress(app, name, "failed");
            }
        }
    }

    emit_status(app, "waiting");
    emit_log(app, "Waiting 5 seconds for network to settle...");
    thread::sleep(Duration::from_secs(5));

    emit_status(app, "launching");
    emit_log(app, "Launching Blur...");

    let launch_result = Command::new(&path_buf)
        .current_dir(work_dir)
        .spawn();

    match launch_result {
        Ok(mut child) => {
            emit_log(app, format!("Game PID: {}", child.id()));
            emit_status(app, "racing");
            emit_log(app, "Waiting for game to close...");
            let _ = child.wait();
            emit_log(app, "Game process exited.");
        }
        Err(e) => {
            emit_log(app, format!("ERROR: Failed to launch game: {e}"));
        }
    }

    emit_log(app, "Waiting 2 seconds before restoring adapters...");
    thread::sleep(Duration::from_secs(2));

    // Restore all virtual adapters
    restore_adapters(app, &all_virtual);

    emit_log(app, "=== BLUR LAN MODE END ===");
    Ok(())
}

fn restore_adapters(app: &AppHandle, adapters: &[String]) {
    emit_adapters_list(app, adapters);
    emit_status(app, "restoring");
    if adapters.is_empty() {
        emit_log(app, "No adapters to restore.");
        return;
    }
    emit_log(app, format!("Restoring {} adapter(s)...", adapters.len()));
    for name in adapters {
        emit_adapter_progress(app, name, "processing");
        emit_log(app, format!("Enabling: {name}"));
        match network::enable_adapter(name) {
            Ok(()) => {
                emit_log(app, "  -> OK".to_string());
                emit_adapter_progress(app, name, "done");
            }
            Err(e) => {
                emit_log(app, format!("  -> failed: {e}"));
                emit_adapter_progress(app, name, "failed");
            }
        }
    }
    emit_log(app, "All adapters restored.");
}

#[tauri::command]
fn list_adapters(app: AppHandle) -> Result<Vec<network::AdapterInfo>, String> {
    let adapters = network::list_all_adapters()?;
    emit_log(&app, format!("list_adapters: found {} adapter(s)", adapters.len()));
    for a in &adapters {
        emit_log(&app, format!("  - {} [{}] ({})", a.name, a.status, a.adapter_type));
    }
    Ok(adapters)
}

#[tauri::command]
fn disable_adapter(name: String) -> Result<(), String> {
    eprintln!("[ipc] disable_adapter called: '{name}'");
    let result = network::disable_adapter(&name);
    match &result {
        Ok(()) => eprintln!("[ipc] disable_adapter: '{name}' OK"),
        Err(e) => eprintln!("[ipc] disable_adapter: '{name}' FAILED - {e}"),
    }
    result
}

#[tauri::command]
fn enable_adapter(name: String) -> Result<(), String> {
    eprintln!("[ipc] enable_adapter called: '{name}'");
    let result = network::enable_adapter(&name);
    match &result {
        Ok(()) => eprintln!("[ipc] enable_adapter: '{name}' OK"),
        Err(e) => eprintln!("[ipc] enable_adapter: '{name}' FAILED - {e}"),
    }
    result
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::default())
        .setup(|app| {
            let show_item = MenuItemBuilder::with_id("show", "Open").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "Exit").build(app)?;
            let menu = MenuBuilder::new(app).items(&[&show_item, &quit_item]).build()?;

            let tray_id = "main-tray".to_string();
            let _tray = TrayIconBuilder::with_id(&tray_id)
                .icon(app.default_window_icon().cloned().unwrap())
                .menu(&menu)
                .tooltip("Blur LAN Launcher")
                .on_menu_event(move |app, event| {
                    match event.id().as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = position_window_bottom_right(&window);
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                            if let Some(id) = app.state::<AppState>().tray_id.lock().unwrap().as_ref() {
                                if let Some(tray) = app.tray_by_id(id) {
                                    let _ = tray.set_visible(false);
                                }
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    match event {
                        tauri::tray::TrayIconEvent::Click { button, .. } => {
                            if button == tauri::tray::MouseButton::Left {
                                let app = tray.app_handle();
                                if let Some(window) = app.get_webview_window("main") {
                                    let _ = position_window_bottom_right(&window);
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                                if let Some(id) = app.state::<AppState>().tray_id.lock().unwrap().as_ref() {
                                    if let Some(t) = app.tray_by_id(id) {
                                        let _ = t.set_visible(false);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            // Store tray ID and setup window behavior
            {
                let state = app.state::<AppState>();
                *state.tray_id.lock().unwrap() = Some(tray_id.clone());
            }
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
                let _ = position_window_bottom_right(&window);
                let w = window.clone();
                let handle = app.handle().clone();
                window.on_window_event(move |event| {
                    match event {
                        WindowEvent::CloseRequested { api, .. } => {
                            api.prevent_close();
                            let _ = w.hide();
                            if let Some(id) = handle.state::<AppState>().tray_id.lock().unwrap().as_ref() {
                                if let Some(tray) = handle.tray_by_id(id) {
                                    let _ = tray.set_visible(true);
                                }
                            }
                        }
                        WindowEvent::Focused(false) => {
                            let _ = w.hide();
                            if let Some(id) = handle.state::<AppState>().tray_id.lock().unwrap().as_ref() {
                                if let Some(tray) = handle.tray_by_id(id) {
                                    let _ = tray.set_visible(true);
                                }
                            }
                        }
                        _ => {}
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_saved_path,
            pick_game_path,
            start_lan_mode,
            is_running,
            list_adapters,
            disable_adapter,
            enable_adapter
        ])
        .run(tauri::generate_context!())
        .expect("error while running Blur LAN Launcher");
}
