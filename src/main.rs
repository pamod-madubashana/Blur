use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

const CONFIG_FILE: &str = "blur.config";

static D3D9_DLL: &[u8] = include_bytes!("../d3d9.dll");
static DISCORD_RPC_DLL: &[u8] = include_bytes!("../discord-rpc.dll");
static LUA51_DLL: &[u8] = include_bytes!("../lua5.1.dll");
static AMAX_DIR: include_dir::Dir = include_dir::include_dir!("../amax");

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

    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", ps_script])
        .output();

    match output {
        Ok(o) => {
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

    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", ps_script])
        .output();

    match output {
        Ok(o) => {
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
    log("Extracting embedded files...");

    let files: &[(&str, &[u8])] = &[
        ("d3d9.dll", D3D9_DLL),
        ("discord-rpc.dll", DISCORD_RPC_DLL),
        ("lua5.1.dll", LUA51_DLL),
    ];

    for (name, data) in files {
        let dst = game_dir.join(name);

        if dst.exists() {
            log(&format!("Already exists, skipping: {}", name));
            continue;
        }

        match fs::write(&dst, data) {
            Ok(_) => log(&format!("Extracted: {}", name)),
            Err(e) => log_err(&format!("Failed to write {}: {}", name, e)),
        }
    }

    let dst = game_dir.join("amax");
    if dst.exists() {
        log("Already exists, skipping: amax/");
    } else {
        match extract_dir(&AMAX_DIR, &dst) {
            Ok(_) => log("Extracted: amax/"),
            Err(e) => log_err(&format!("Failed to extract amax: {}", e)),
        }
    }
}

fn extract_dir(dir: &include_dir::Dir, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for file in dir.files() {
        let file_path = dst.join(file.path());
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&file_path, file.contents())?;
    }
    for sub in dir.dirs() {
        let sub_dst = dst.join(sub.path());
        extract_dir(sub, &sub_dst)?;
    }
    Ok(())
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
