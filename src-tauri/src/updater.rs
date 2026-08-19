use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::Emitter;

const REPO: &str = "pamod-madubashana/Blur";
const ASSET_NAME: &str = "Blur-Launcher.exe";

#[derive(Clone, serde::Serialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub download_url: String,
    pub digest: String,
    pub size: u64,
}

#[derive(Clone, serde::Serialize)]
pub struct UpdateProgress {
    pub phase: String,
    pub percent: f64,
    pub message: String,
}

fn get_current_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn parse_version(v: &str) -> Option<semver::Version> {
    let v = v.trim().strip_prefix('v').unwrap_or(v.trim());
    semver::Version::parse(v).ok()
}

fn get_updates_dir() -> Result<PathBuf, String> {
    let local_app_data = std::env::var("LOCALAPPDATA").map_err(|e| e.to_string())?;
    let dir = PathBuf::from(local_app_data).join("Blur").join("updates");
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create updates dir: {e}"))?;
    Ok(dir)
}

fn get_exe_dir() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    Ok(exe.parent().unwrap_or(Path::new(".")).to_path_buf())
}

pub fn cleanup_old_exe() {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let old = dir.join("Blur-Launcher.exe.old");
            if old.exists() {
                let _ = fs::remove_file(&old);
            }
        }
    }
}

fn build_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("Blur-Updater")
        .no_gzip()
        .no_brotli()
        .no_deflate()
        .build()
        .map_err(|e| e.to_string())
}

pub async fn check_for_update() -> Result<UpdateInfo, String> {
    let current_version_str = get_current_version();
    let current_version = parse_version(&current_version_str)
        .ok_or_else(|| format!("Invalid current version: {current_version_str}"))?;

    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let client = build_client()?;

    let response = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("Failed to check for updates: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("GitHub API returned status: {}", response.status()));
    }

    let release: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

    let tag_name = release["tag_name"]
        .as_str()
        .ok_or("Missing tag_name in release")?;
    let latest_version = parse_version(tag_name)
        .ok_or_else(|| format!("Invalid version in tag: {tag_name}"))?;

    if latest_version <= current_version {
        return Err("No update available".to_string());
    }

    let assets = release["assets"]
        .as_array()
        .ok_or("Missing assets in release")?;

    let asset = assets
        .iter()
        .find(|a| a["name"].as_str() == Some(ASSET_NAME))
        .ok_or_else(|| format!("{ASSET_NAME} not found in release assets"))?;

    let download_url = asset["browser_download_url"]
        .as_str()
        .ok_or("Missing download URL")?
        .to_string();

    let digest = asset["digest"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let size = asset["size"].as_u64().unwrap_or(0);

    Ok(UpdateInfo {
        current_version: current_version_str,
        latest_version: latest_version.to_string(),
        download_url,
        digest,
        size,
    })
}

fn verify_sha256(file_path: &Path, expected_digest: &str) -> Result<bool, String> {
    if expected_digest.is_empty() {
        return Ok(true);
    }

    let expected = expected_digest
        .strip_prefix("sha256:")
        .unwrap_or(expected_digest);

    let bytes = fs::read(file_path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let result = hasher.finalize();
    let actual = format!("{:x}", result);

    if !actual.eq_ignore_ascii_case(expected) {
        return Err(format!(
            "SHA-256 mismatch: expected {}, got {} (size: {} bytes)",
            expected, actual, bytes.len()
        ));
    }

    Ok(true)
}

pub async fn download_update(info: &UpdateInfo, app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let updates_dir = get_updates_dir()?;
    let download_path = updates_dir.join(format!("{ASSET_NAME}.download"));

    if download_path.exists() {
        fs::remove_file(&download_path).map_err(|e| e.to_string())?;
    }

    let _ = app.emit("update_progress", UpdateProgress {
        phase: "downloading".to_string(),
        percent: 0.0,
        message: "Starting download...".to_string(),
    });

    let client = build_client()?;

    let response = client
        .get(&info.download_url)
        .header("Accept", "application/octet-stream")
        .send()
        .await
        .map_err(|e| format!("Failed to start download: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Download failed with status: {}", response.status()));
    }

    let total_size = info.size;
    let mut downloaded: u64 = 0;
    let mut file = fs::File::create(&download_path).map_err(|e| e.to_string())?;

    use std::io::Write;
    let mut stream = response;
    let mut last_emit = std::time::Instant::now();

    while let Some(chunk) = stream.chunk().await.map_err(|e| format!("Download error: {e}"))? {
        use std::io::Write;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;

        if total_size > 0 && last_emit.elapsed().as_millis() > 200 {
            let percent = (downloaded as f64 / total_size as f64) * 100.0;
            let _ = app.emit("update_progress", UpdateProgress {
                phase: "downloading".to_string(),
                percent,
                message: format!("Downloading... {percent:.0}%"),
            });
            last_emit = std::time::Instant::now();
        }
    }

    file.flush().map_err(|e| e.to_string())?;
    drop(file);

    let _ = app.emit("update_progress", UpdateProgress {
        phase: "verifying".to_string(),
        percent: 100.0,
        message: "Verifying download...".to_string(),
    });

    if !verify_sha256(&download_path, &info.digest)? {
        let _ = fs::remove_file(&download_path);
        return Err("SHA-256 verification failed".to_string());
    }

    Ok(download_path)
}

pub fn install_update(download_path: &PathBuf, app: &tauri::AppHandle) -> Result<(), String> {
    let exe_dir = get_exe_dir()?;
    let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let old_exe = exe_dir.join("Blur-Launcher.exe.old");
    let target_exe = exe_dir.join(ASSET_NAME);

    let _ = app.emit("update_progress", UpdateProgress {
        phase: "installing".to_string(),
        percent: 0.0,
        message: "Preparing update...".to_string(),
    });

    if old_exe.exists() {
        fs::remove_file(&old_exe).map_err(|e| format!("Failed to remove old backup: {e}"))?;
    }

    fs::rename(&current_exe, &old_exe)
        .map_err(|e| format!("Failed to backup current exe: {e}"))?;

    fs::rename(download_path, &target_exe)
        .map_err(|e| {
            let _ = fs::rename(&old_exe, &current_exe);
            format!("Failed to replace exe: {e}")
        })?;

    let _ = app.emit("update_progress", UpdateProgress {
        phase: "restarting".to_string(),
        percent: 100.0,
        message: "Restarting...".to_string(),
    });

    Command::new(&target_exe)
        .spawn()
        .map_err(|e| format!("Failed to launch new version: {e}"))?;

    std::process::exit(0);
}
