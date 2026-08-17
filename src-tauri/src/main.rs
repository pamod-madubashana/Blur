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
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

#[derive(Default)]
struct AppState {
    running: Arc<AtomicBool>,
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
