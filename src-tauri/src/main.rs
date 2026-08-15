// Prevents an additional console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod network;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
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
    #[serde(default)]
    files_synced: bool,
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

/// Ensures that bundled files from `files/` exist in the game directory.
/// Copies any missing files or directories. Skips .gitkeep files.
fn sync_files(app: &AppHandle, game_dir: &Path) -> Result<(), String> {
    // Try bundled resource dir first; fall back to CWD for dev mode
    let src_files = app
        .path()
        .resource_dir()
        .ok()
        .map(|d| d.join("files"))
        .filter(|d| d.exists())
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|d| d.join("files"))
                .filter(|d| d.exists())
        })
        .ok_or("Could not locate bundled files/ directory")?;

    emit_log(app, "Checking required game files...");

    fn walk_and_copy(
        app: &AppHandle,
        src: &Path,
        game_dir: &Path,
        base: &Path,
    ) -> Result<u32, String> {
        let mut copied = 0u32;
        if src.is_dir() {
            for entry in fs::read_dir(src).map_err(|e| format!("Failed to read {}: {e}", src.display()))? {
                let entry = entry.map_err(|e| e.to_string())?;
                let name = entry.file_name();
                let path = entry.path();

                // Skip .gitkeep
                if name.to_string_lossy() == ".gitkeep" {
                    continue;
                }

                if path.is_dir() {
                    copied += walk_and_copy(app, &path, game_dir, base)?;
                } else {
                    let rel = path.strip_prefix(base).unwrap_or(&path);
                    let dest = game_dir.join(rel);
                    if let Some(parent) = dest.parent() {
                        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                    }
                    fs::copy(&path, &dest).map_err(|e| {
                        format!("Failed to copy {}: {e}", path.display())
                    })?;
                    emit_log(app, format!("  Copied: {}", rel.display()));
                    copied += 1;
                }
            }
        }
        Ok(copied)
    }

    let count = walk_and_copy(app, &src_files, game_dir, &src_files)?;
    if count == 0 {
        emit_log(app, "Online fix files are fine.");
    } else {
        emit_log(app, format!("Copied {count} missing file(s) to game directory."));
    }

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
        let _ = save_config(&app, &Config { game_path: Some(path_str.clone()), files_synced: false });
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

    let mut cfg = load_config(app);
    if cfg.files_synced {
        emit_log(app, "Online fix files are fine.");
    } else {
        if let Err(e) = sync_files(app, work_dir) {
            emit_log(app, format!("File sync warning: {e}"));
        }
        cfg.files_synced = true;
        let _ = save_config(app, &cfg);
    }

    emit_status(app, "disabling");
    emit_log(app, "Scanning for non-WiFi adapters to disable...");
    let disabled_by_us = network::list_non_wifi_up()?;
    if disabled_by_us.is_empty() {
        emit_log(app, "No non-WiFi adapters were up.");
    } else {
        emit_log(app, format!("Found {} adapter(s) to disable.", disabled_by_us.len()));
    }
    let mut successfully_disabled: Vec<String> = Vec::new();
    for name in &disabled_by_us {
        emit_log(app, format!("Disabling: {name}"));
        match network::disable_adapter(name) {
            Ok(()) => {
                emit_log(app, format!("  -> OK"));
                successfully_disabled.push(name.clone());
            }
            Err(e) => emit_log(app, format!("  -> failed: {e}")),
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

    // Brief pause to let the system settle after game closes
    emit_log(app, "Waiting 2 seconds before restoring adapters...");
    thread::sleep(Duration::from_secs(2));

    // Always restore adapters we disabled, even if game failed to launch
    restore_adapters(app, &successfully_disabled);

    emit_log(app, "=== BLUR LAN MODE END ===");
    Ok(())
}

/// Re-enables only the adapters that were disabled by this app.
fn restore_adapters(app: &AppHandle, adapters: &[String]) {
    emit_status(app, "restoring");
    if adapters.is_empty() {
        emit_log(app, "No adapters to restore.");
        return;
    }
    emit_log(app, format!("Restoring {} adapter(s)...", adapters.len()));
    for name in adapters {
        emit_log(app, format!("Enabling: {name}"));
        match network::enable_adapter(name) {
            Ok(()) => emit_log(app, format!("  -> OK")),
            Err(e) => emit_log(app, format!("  -> failed: {e}")),
        }
    }
    emit_log(app, "All adapters restored.");
}

#[tauri::command]
fn list_adapters() -> Result<Vec<network::AdapterInfo>, String> {
    network::list_all_adapters()
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
            list_adapters
        ])
        .run(tauri::generate_context!())
        .expect("error while running Blur LAN Launcher");
}
