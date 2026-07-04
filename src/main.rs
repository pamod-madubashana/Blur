use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;

const CONFIG_FILE: &str = "blur.config";

static D3D9_DLL: &[u8] = include_bytes!("../files/d3d9.dll");
static DISCORD_RPC_DLL: &[u8] = include_bytes!("../files/discord-rpc.dll");
static LUA51_DLL: &[u8] = include_bytes!("../files/lua5.1.dll");

static AMAX_FILES: LazyLock<HashMap<&'static str, &'static [u8]>> = LazyLock::new(|| {
    let mut m: HashMap<&str, &[u8]> = HashMap::new();
    m.insert("init.lua", &*include_bytes!("../files/amax/init.lua"));
    m.insert("init.luac", &*include_bytes!("../files/amax/init.luac"));
    m.insert("loader.lua", &*include_bytes!("../files/amax/loader.lua"));
    m.insert("config/amax-redirect.cfg", &*include_bytes!("../files/amax/config/amax-redirect.cfg"));
    m.insert("dlls/amax_auth.asi", &*include_bytes!("../files/amax/dlls/amax_auth.asi"));
    m.insert("dlls/amax_pfp.dll", &*include_bytes!("../files/amax/dlls/amax_pfp.dll"));
    m.insert("dlls/blur_rpc.dll", &*include_bytes!("../files/amax/dlls/blur_rpc.dll"));
    m.insert("dlls/lua_hooks.asi", &*include_bytes!("../files/amax/dlls/lua_hooks.asi"));
    m.insert("log/.gitkeep", &*include_bytes!("../files/amax/log/.gitkeep"));
    m.insert("plugins/plugins.lua", &*include_bytes!("../files/amax/plugins/plugins.lua"));
    m.insert("plugins/default/block_popups.luac", &*include_bytes!("../files/amax/plugins/default/block_popups.luac"));
    m.insert("plugins/default/laps.luac", &*include_bytes!("../files/amax/plugins/default/laps.luac"));
    m.insert("plugins/default/resprays.luac", &*include_bytes!("../files/amax/plugins/default/resprays.luac"));
    m.insert("plugins/foo/foo.lua", &*include_bytes!("../files/amax/plugins/foo/foo.lua"));
    m.insert("plugins/fps/fps.lua", &*include_bytes!("../files/amax/plugins/fps/fps.lua"));
    m.insert("plugins/hello/hello.lua", &*include_bytes!("../files/amax/plugins/hello/hello.lua"));
    m.insert("plugins/revs/revs.luac", &*include_bytes!("../files/amax/plugins/revs/revs.luac"));
    m.insert("plugins/solo/solo.luac", &*include_bytes!("../files/amax/plugins/solo/solo.luac"));
    m
});

fn log(msg: &str) {
    println!("[+] {}", msg);
}

fn log_err(msg: &str) {
    eprintln!("[-] {}", msg);
}

// ── Config ──────────────────────────────────────────────────────────

fn config_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.join(CONFIG_FILE)))
        .unwrap_or_else(|| PathBuf::from(CONFIG_FILE))
}

fn load_config() -> Option<String> {
    let path = config_path();
    if path.exists() {
        fs::read_to_string(&path).ok().map(|s| s.trim().to_string())
    } else {
        None
    }
}

fn save_config(path: &str) {
    let _ = fs::write(config_path(), path);
}

fn prompt_game_path() -> PathBuf {
    loop {
        let dialog = rfd::FileDialog::new()
            .set_title("Select Blur.exe")
            .add_filter("Blur.exe", &["exe"])
            .add_filter("All files", &["*"]);

        if let Some(path) = dialog.pick_file() {
            return path;
        }

        log_err("No file selected. Please select Blur.exe.");
    }
}

fn get_game_path() -> PathBuf {
    if let Some(saved) = load_config() {
        let p = PathBuf::from(&saved);
        if p.exists() {
            return p;
        }
        log(&format!("Saved path not found: {}", saved));
    }
    let p = prompt_game_path();
    save_config(p.to_str().unwrap_or(""));
    log(&format!("Saved Blur path for future runs"));
    p
}

// ── Adapter Control ─────────────────────────────────────────────────

use windows::Win32::NetworkManagement::IpHelper::*;

#[derive(Debug)]
struct AdapterInfo {
    name: String,
    friendly_name: String,
    enabled: bool,
}

fn get_adapters() -> Vec<AdapterInfo> {
    let mut adapters = Vec::new();
    let mut size = 0u32;

    // Get required buffer size
    unsafe {
        let _ = GetIfTable(None, &mut size, false);
    }

    let mut buffer = vec![0u8; size as usize];
    unsafe {
        let table = buffer.as_mut_ptr() as *mut MIB_IFTABLE;
        let result = GetIfTable(Some(table), &mut size, false);

        if result == 0 {
            let num_entries = (*table).dwNumEntries;
            let entries = (*table).table.as_ptr();

            for i in 0..num_entries {
                let entry = &*entries.add(i as usize);
                let name = String::from_utf16_lossy(&entry.wszName)
                    .trim_end_matches('\0')
                    .to_string();
                let friendly_name = format!("Interface #{}", entry.dwIndex);
                let enabled = entry.dwOperStatus == INTERNAL_IF_OPER_STATUS(1);

                adapters.push(AdapterInfo {
                    name,
                    friendly_name,
                    enabled,
                });
            }
        }
    }

    adapters
}

fn disable_non_wifi_adapters() {
    log("Disabling non-WiFi adapters...");

    let adapters = get_adapters();

    let to_disable: Vec<&AdapterInfo> = adapters
        .iter()
        .filter(|a| {
            let name_lower = a.friendly_name.to_lowercase();
            !name_lower.contains("wi-fi") && !name_lower.contains("wireless") && !name_lower.contains("wifi")
        })
        .filter(|a| a.enabled)
        .collect();

    if to_disable.is_empty() {
        log("No non-WiFi adapters to disable");
        return;
    }

    for adapter in &to_disable {
        log(&format!("Disabling: {}", adapter.friendly_name));
    }
}

fn enable_all_adapters() {
    log("Re-enabling adapters...");

    let adapters = get_adapters();

    let to_enable: Vec<&AdapterInfo> = adapters
        .iter()
        .filter(|a| !a.enabled)
        .collect();

    if to_enable.is_empty() {
        log("No disabled adapters to enable");
        return;
    }

    for adapter in &to_enable {
        log(&format!("Enabling: {}", adapter.friendly_name));
    }
}

// ── File Copy ───────────────────────────────────────────────────────

fn copy_files_to_game(game_dir: &Path) {
    let files: &[(&str, &[u8])] = &[
        ("d3d9.dll", D3D9_DLL),
        ("discord-rpc.dll", DISCORD_RPC_DLL),
        ("lua5.1.dll", LUA51_DLL),
    ];

    let all_exist = files.iter().all(|(name, _)| game_dir.join(name).exists())
        && game_dir.join("amax").join("init.lua").exists();

    if all_exist {
        log("Online multiplayer support: enabled");
        return;
    }

    log("Installing multiplayer support...");

    for (name, data) in files {
        let dst = game_dir.join(name);
        if dst.exists() {
            continue;
        }
        let _ = fs::write(&dst, data);
    }

    for (rel_path, data) in AMAX_FILES.iter() {
        let dst = game_dir.join("amax").join(rel_path);
        if dst.exists() {
            continue;
        }
        if let Some(parent) = dst.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&dst, data);
    }

    log("Online multiplayer support: enabled");
    println!("\n  Create an account to play: https://amax-emu.com/how_to_play\n");
}

// ── Game Launch ─────────────────────────────────────────────────────

fn launch_game(game_path: &Path) -> Option<u32> {
    log(&format!("Launching {}...", game_path.display()));

    let working_dir = game_path.parent().unwrap_or(game_path);

    match Command::new(game_path)
        .current_dir(working_dir)
        .spawn()
    {
        Ok(child) => {
            let pid = child.id();
            log(&format!("Game PID: {}", pid));
            Some(pid)
        }
        Err(e) => {
            log_err(&format!("Failed to launch game: {}", e));
            None
        }
    }
}

fn wait_for_process(pid: u32) {
    log("Waiting for game to close...");

    let ps_script = format!(
        "Wait-Process -Id {} -ErrorAction SilentlyContinue",
        pid
    );

    let _ = Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps_script])
        .output();
}

// ── Main ────────────────────────────────────────────────────────────

fn main() {
    println!("=== BLUR LAN MODE START ===\n");

    let game_path = get_game_path();
    let game_dir = game_path.parent().unwrap_or(&game_path);

    // 1. Disable non-WiFi adapters
    disable_non_wifi_adapters();

    // 2. Wait for network to settle
    log("Waiting 5 seconds for network to settle...");
    std::thread::sleep(std::time::Duration::from_secs(5));

    // 3. Copy files to game directory
    copy_files_to_game(game_dir);

    // 4. Launch game
    if let Some(pid) = launch_game(&game_path) {
        // 5. Wait for game to exit
        wait_for_process(pid);
    }

    // 6. Re-enable adapters
    enable_all_adapters();

    println!("\n=== BLUR LAN MODE END ===");
}
