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

fn disable_non_wifi_adapters() {
    log("Disabling non-WiFi adapters...");

    let ps_script = r#"
Get-NetAdapter |
  Where-Object { $_.Name -ne 'Wi-Fi' -and $_.Status -eq 'Up' } |
  ForEach-Object {
    Write-Host "  Disabling: $($_.Name)"
    Disable-NetAdapter -Name $_.Name -Confirm:$false
  }
"#;

    match Command::new("powershell")
        .args(["-NoProfile", "-Command", ps_script])
        .output()
    {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            for line in stdout.lines() {
                if !line.trim().is_empty() {
                    println!("{}", line);
                }
            }
            if !o.status.success() {
                let stderr = String::from_utf8_lossy(&o.stderr);
                if !stderr.trim().is_empty() {
                    log_err(&format!("Adapter disable error: {}", stderr.trim()));
                }
            }
        }
        Err(e) => log_err(&format!("Failed to run powershell: {}", e)),
    }
}

fn enable_all_adapters() {
    log("Re-enabling adapters...");

    let ps_script = r#"
Get-NetAdapter |
  Where-Object { $_.Status -eq 'Disabled' } |
  ForEach-Object {
    Write-Host "  Enabling: $($_.Name)"
    Enable-NetAdapter -Name $_.Name -Confirm:$false
  }
"#;

    match Command::new("powershell")
        .args(["-NoProfile", "-Command", ps_script])
        .output()
    {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            for line in stdout.lines() {
                if !line.trim().is_empty() {
                    println!("{}", line);
                }
            }
            if !o.status.success() {
                let stderr = String::from_utf8_lossy(&o.stderr);
                if !stderr.trim().is_empty() {
                    log_err(&format!("Adapter enable error: {}", stderr.trim()));
                }
            }
        }
        Err(e) => log_err(&format!("Failed to run powershell: {}", e)),
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

// ── Firewall Rules ──────────────────────────────────────────────────

fn enable_firewall_rules(game_exe: &Path) {
    log("Configuring firewall rules...");

    let rule_name_in = "Blur LAN Launcher - Inbound";
    let rule_name_out = "Blur LAN Launcher - Outbound";
    let exe_str = game_exe.to_str().unwrap_or("");

    // Write a temp .ps1 script, then run it elevated
    let ps_script = format!(
        "$ErrorActionPreference = 'SilentlyContinue'\n\
         netsh advfirewall firewall delete rule name=\"{rule_in}\"\n\
         netsh advfirewall firewall delete rule name=\"{rule_out}\"\n\
         netsh advfirewall firewall add rule name=\"{rule_in}\" dir=in action=allow program=\"{exe}\" enable=yes profile=any\n\
         netsh advfirewall firewall add rule name=\"{rule_out}\" dir=out action=allow program=\"{exe}\" enable=yes profile=any\n\
         Write-Host \"Firewall rules configured\"\n",
        rule_in = rule_name_in,
        rule_out = rule_name_out,
        exe = exe_str,
    );

    let temp_dir = std::env::temp_dir();
    let ps1_path = temp_dir.join("blur_firewall.ps1");
    let _ = fs::write(&ps1_path, &ps_script);

    let launcher = format!(
        "Start-Process powershell -Verb RunAs -ArgumentList '-NoProfile -ExecutionPolicy Bypass -File \"{}\"' -Wait",
        ps1_path.to_str().unwrap_or("")
    );

    match Command::new("powershell")
        .args(["-NoProfile", "-Command", &launcher])
        .output()
    {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            if stdout.contains("Firewall rules configured") {
                log("Firewall rules configured (elevated)");
            } else {
                let stderr = String::from_utf8_lossy(&o.stderr);
                if !stderr.trim().is_empty() {
                    log_err(&format!("Firewall: {}", stderr.trim()));
                }
            }
        }
        Err(e) => log_err(&format!("Failed to run powershell: {}", e)),
    }

    let _ = fs::remove_file(&ps1_path);
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

    // 4. Enable firewall rules
    enable_firewall_rules(&game_path);

    // 5. Launch game
    if let Some(pid) = launch_game(&game_path) {
        // 6. Wait for game to exit
        wait_for_process(pid);
    }

    // 7. Re-enable adapters
    enable_all_adapters();

    println!("\n=== BLUR LAN MODE END ===");
}
