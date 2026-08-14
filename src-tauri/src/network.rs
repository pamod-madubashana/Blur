use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

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
