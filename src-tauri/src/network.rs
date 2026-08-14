use serde::Serialize;
use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Serialize, Clone)]
pub struct AdapterInfo {
    pub name: String,
    pub status: String,
    pub adapter_type: String,
}

/// Runs a PowerShell command and returns trimmed stdout, hiding the console window.
fn run_ps(script: &str) -> Result<String, String> {
    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script]);

    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = cmd.output().map_err(|e| format!("Failed to run PowerShell: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if !stderr.is_empty() {
            return Err(stderr);
        }
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Returns the names of "up" adapters other than Wi-Fi.
pub fn list_non_wifi_up() -> Result<Vec<String>, String> {
    let out = run_ps(
        "Get-NetAdapter | Where-Object { $_.Name -ne 'Wi-Fi' -and $_.Status -eq 'Up' } | Select-Object -ExpandProperty Name",
    )?;
    Ok(out.lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
}

/// Returns the names of currently disabled adapters.
pub fn list_disabled() -> Result<Vec<String>, String> {
    let out = run_ps(
        "Get-NetAdapter | Where-Object { $_.Status -eq 'Disabled' } | Select-Object -ExpandProperty Name",
    )?;
    Ok(out.lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
}

pub fn disable_adapter(name: &str) -> Result<(), String> {
    let script = format!("Disable-NetAdapter -Name '{}' -Confirm:$false", name.replace('\'', "''"));
    run_ps(&script)?;
    Ok(())
}

pub fn enable_adapter(name: &str) -> Result<(), String> {
    let script = format!("Enable-NetAdapter -Name '{}' -Confirm:$false", name.replace('\'', "''"));
    run_ps(&script)?;
    Ok(())
}

/// Returns all network adapters with name, status, and interface description.
pub fn list_all_adapters() -> Result<Vec<AdapterInfo>, String> {
    let out = run_ps(
        "Get-NetAdapter | Select-Object Name, Status, InterfaceDescription | ConvertTo-Json -Compress",
    )?;
    if out.is_empty() {
        return Ok(Vec::new());
    }
    // Handle both single object and array
    let json_str = if out.starts_with('[') {
        out
    } else {
        format!("[{out}]")
    };
    let raw: Vec<serde_json::Value> =
        serde_json::from_str(&json_str).map_err(|e| format!("Failed to parse adapter JSON: {e}"))?;
    Ok(raw
        .into_iter()
        .map(|v| {
            let name = v["Name"].as_str().unwrap_or("").to_string();
            let status = v["Status"].as_str().unwrap_or("Unknown").to_string();
            let iface = v["InterfaceDescription"].as_str().unwrap_or("");
            let adapter_type = classify_adapter(&name, iface);
            AdapterInfo { name, status, adapter_type }
        })
        .collect())
}

fn classify_adapter(name: &str, iface: &str) -> String {
    let combined = format!("{name} {iface}").to_lowercase();
    if combined.contains("wi-fi") || combined.contains("wifi") || combined.contains("wireless") {
        "wifi".to_string()
    } else if combined.contains("virtual") || combined.contains("bridge") || combined.contains("hyper-v") || combined.contains("vpn") {
        "virtual".to_string()
    } else {
        "ethernet".to_string()
    }
}
