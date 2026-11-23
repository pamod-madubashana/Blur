use crate::network;
use serde::Serialize;
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};

const CREATE_NO_WINDOW: u32 = 0x08000000;
const LAN_PORT: u16 = 50001;

#[derive(Serialize, Clone)]
pub struct BridgeStatus {
    pub adapter: String,
    pub address: String,
    pub mask: String,
    pub port_owner: Option<String>,
}

pub struct BridgeGuard {
    config_path: PathBuf,
    anydesk_service_stopped: bool,
}

#[derive(Clone)]
struct Candidate {
    name: String,
    address: u32,
    mask: u32,
    metric: u32,
    virtual_adapter: bool,
}

fn log(app: &AppHandle, msg: impl Into<String>) {
    let _ = app.emit("log", msg.into());
}

fn run_powershell(script: &str) -> Result<String, String> {
    let output = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("failed to run powershell: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "powershell failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn parse_ipv4(text: &str) -> Option<u32> {
    let parts: Vec<u8> = text.split('.').filter_map(|p| p.parse().ok()).collect();
    if parts.len() != 4 {
        return None;
    }
    Some(u32::from_be_bytes([parts[0], parts[1], parts[2], parts[3]]))
}

fn format_ipv4(value: u32) -> String {
    let [a, b, c, d] = value.to_be_bytes();
    format!("{a}.{b}.{c}.{d}")
}

fn prefix_to_mask(prefix: u32) -> u32 {
    if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    }
}

fn list_candidates() -> Result<Vec<Candidate>, String> {
    const SCRIPT: &str = r#"
Get-NetIPAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue |
  Where-Object { $_.IPAddress -ne '127.0.0.1' } |
  ForEach-Object {
    $iface = Get-NetIPInterface -InterfaceIndex $_.InterfaceIndex -AddressFamily IPv4 -ErrorAction SilentlyContinue
    $metric = if ($iface) { $iface.InterfaceMetric } else { 9999 }
    '{0}|{1}|{2}|{3}' -f $_.InterfaceAlias, $_.IPAddress, $_.PrefixLength, $metric
  }
"#;

    let stdout = run_powershell(SCRIPT)?;
    let mut result = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('|').collect();
        if fields.len() != 4 {
            continue;
        }
        let (Some(address), Ok(prefix), Ok(metric)) = (
            parse_ipv4(fields[1]),
            fields[2].parse::<u32>(),
            fields[3].parse::<u32>(),
        ) else {
            continue;
        };
        result.push(Candidate {
            name: fields[0].to_string(),
            address,
            mask: prefix_to_mask(prefix),
            metric,
            virtual_adapter: network::is_virtual_name(fields[0]),
        });
    }
    Ok(result)
}

fn is_usable(candidate: &Candidate) -> bool {
    let [a, b, _, _] = candidate.address.to_be_bytes();
    a != 127 && a != 0 && !(a == 169 && b == 254) && candidate.mask != 0
}

fn select_target(candidates: &[Candidate]) -> Option<Candidate> {
    let mut usable: Vec<&Candidate> = candidates.iter().filter(|c| is_usable(c)).collect();
    usable.sort_by_key(|c| (c.virtual_adapter, c.metric, c.name.clone()));
    usable.first().map(|c| (*c).clone())
}

fn bundled_shim(app: &AppHandle) -> Option<PathBuf> {
    let dev = PathBuf::from("files/amax/dlls/blur_lan_bridge.asi");
    if dev.exists() {
        return Some(dev);
    }
    app.path()
        .resource_dir()
        .ok()
        .map(|dir| dir.join("files/amax/dlls/blur_lan_bridge.asi"))
        .filter(|path| path.exists())
}

fn write_config(path: &Path, target: &Candidate) -> Result<(), String> {
    let contents = format!(
        "address={}\nmask={}\n",
        format_ipv4(target.address),
        format_ipv4(target.mask)
    );
    fs::write(path, contents).map_err(|e| format!("failed to write bridge config: {e}"))
}

fn port_owner() -> Option<(u32, String)> {
    const SCRIPT: &str = r#"
$p = Get-NetUDPEndpoint -LocalPort 50001 -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty OwningProcess
if ($p) { $n = (Get-Process -Id $p -ErrorAction SilentlyContinue).ProcessName; "$p|$n" }
"#;
    let stdout = run_powershell(SCRIPT).ok()?;
    let (pid_part, name) = stdout.split_once('|')?;
    let pid = pid_part.trim().parse().ok()?;
    Some((pid, name.trim().to_string()))
}

fn stop_anydesk() -> Result<bool, String> {
    let check = run_powershell("if (Get-Service -Name AnyDesk -ErrorAction SilentlyContinue) { 'yes' }").unwrap_or_default();
    if !check.contains("yes") {
        return Ok(false);
    }
    run_powershell("Stop-Service -Name AnyDesk -Force -ErrorAction SilentlyContinue; Stop-Process -Name AnyDesk -Force -ErrorAction SilentlyContinue")?;
    Ok(true)
}

fn start_anydesk() {
    let _ = run_powershell("Start-Service -Name AnyDesk -ErrorAction SilentlyContinue");
}

pub fn detect_address() -> Result<String, String> {
    let candidates = list_candidates()?;
    let target = select_target(&candidates)
        .ok_or_else(|| "no usable IPv4 LAN adapter found".to_string())?;
    Ok(format_ipv4(target.address))
}

pub fn prepare(app: &AppHandle, work_dir: &Path) -> Result<(BridgeGuard, BridgeStatus), String> {
    log(app, "LAN bridge: enumerating local adapters...");
    let candidates = list_candidates()?;
    let target = select_target(&candidates)
        .ok_or_else(|| "no usable IPv4 LAN adapter found (all addresses are loopback/APIPA)".to_string())?;

    log(app, format!(
        "LAN bridge: selected '{}' ({} / {})",
        target.name,
        format_ipv4(target.address),
        format_ipv4(target.mask)
    ));

    let amax_dir = work_dir.join("amax");
    let dll_dir = amax_dir.join("dlls");
    if !dll_dir.exists() {
        return Err(format!(
            "AMAX dll directory not found at {} - install the AMAX client files first",
            dll_dir.display()
        ));
    }

    let shim_source = bundled_shim(app)
        .ok_or_else(|| "blur_lan_bridge.asi is missing from the launcher resources".to_string())?;
    let shim_target = dll_dir.join("blur_lan_bridge.asi");
    let same = fs::metadata(&shim_source).ok()
        .and_then(|s| fs::metadata(&shim_target).ok().map(|d| s.len() == d.len()))
        .unwrap_or(false);
    if !same {
        fs::copy(&shim_source, &shim_target)
            .map_err(|e| format!("failed to install LAN bridge shim: {e}"))?;
    }
    log(app, "LAN bridge: Winsock shim installed in amax/dlls");

    let config_path = amax_dir.join("blur_lan_bridge.cfg");
    write_config(&config_path, &target)?;

    let mut port_owner_name = None;
    let mut anydesk_service_stopped = false;
    if let Some((pid, name)) = port_owner() {
        port_owner_name = Some(format!("{name} (PID {pid})"));
        log(app, format!("LAN bridge: UDP {LAN_PORT} is held by {name} (PID {pid})"));
        if name.to_lowercase().contains("anydesk") {
            match stop_anydesk() {
                Ok(stopped) => {
                    anydesk_service_stopped = stopped;
                    log(app, "LAN bridge: stopped AnyDesk to free UDP 50001 (will restart after game)");
                }
                Err(e) => log(app, format!("LAN bridge: could not stop AnyDesk: {e}")),
            }
        } else {
            log(app, "LAN bridge: leaving that process alone - LAN discovery may be blocked");
        }
    } else {
        log(app, format!("LAN bridge: UDP {LAN_PORT} is free"));
    }

    let status = BridgeStatus {
        adapter: target.name.clone(),
        address: format_ipv4(target.address),
        mask: format_ipv4(target.mask),
        port_owner: port_owner_name,
    };

    Ok((
        BridgeGuard { config_path, anydesk_service_stopped },
        status,
    ))
}

impl BridgeGuard {
    pub fn cleanup(&self, app: &AppHandle) {
        if self.config_path.exists() {
            if let Err(e) = fs::remove_file(&self.config_path) {
                log(app, format!("LAN bridge: failed to remove config: {e}"));
            } else {
                log(app, "LAN bridge: config removed (shim is now inert)");
            }
        }
        if self.anydesk_service_stopped {
            start_anydesk();
            log(app, "LAN bridge: AnyDesk service restarted");
        }
    }
}
