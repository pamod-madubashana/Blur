use tauri::Emitter;
use tauri::AppHandle;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::VARIANT_TRUE;
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED};
use windows::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY, HKEY_LOCAL_MACHINE,
    KEY_READ, KEY_SET_VALUE, REG_DWORD, REG_SAM_FLAGS,
};
use windows::Win32::System::Services::{
    ChangeServiceConfigW, CloseServiceHandle, OpenSCManagerW, OpenServiceW,
    QueryServiceStatusEx, StartServiceW, SC_HANDLE, SC_STATUS_PROCESS_INFO,
    SERVICE_AUTO_START, SERVICE_CHANGE_CONFIG, SERVICE_QUERY_STATUS, SERVICE_START,
    SERVICE_START_PENDING, SERVICE_STATUS_PROCESS, SERVICE_STOPPED, SERVICE_RUNNING,
    SERVICE_NO_CHANGE, SERVICE_ERROR, ENUM_SERVICE_TYPE,
};
use windows::Win32::NetworkManagement::WindowsFirewall::{INetFwPolicy2, NetFwPolicy2};
use windows::Win32::Networking::NetworkListManager::{
    INetworkListManager, NetworkListManager, INetwork,
    NLM_ENUM_NETWORK_CONNECTED,
    NLM_NETWORK_CATEGORY_PRIVATE,
    NLM_NETWORK_CATEGORY_PUBLIC,
    NLM_NETWORK_CATEGORY_DOMAIN_AUTHENTICATED,
};
use windows::Win32::Storage::FileSystem::{
    NetShareAdd, NetShareDel,
    SHARE_INFO_502, STYPE_DISKTREE, ACCESS_READ,
};
use windows::Win32::Security::PSECURITY_DESCRIPTOR;
use windows::Win32::Security::Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW;
use windows::core::BSTR;

macro_rules! log {
    ($($arg:tt)*) => {
        eprintln!("[discovering] {}", format!($($arg)*));
    };
}

// --- Error constants ---

/// Network management error: share already exists.
const NERR_DUPLICATE_SHARE: u32 = 2118;

/// Windows Firewall profile bitmask: Private + Public (not Domain).
const FW_PROFILE_PRIVATE_AND_PUBLIC: i32 = 2 | 4;

// --- Event helpers ---

#[derive(Clone, serde::Serialize)]
struct DiscoveringCheckPayload {
    rule: String,
    status: String,
}

fn emit_check(app: &AppHandle, rule: &str, status: &str) {
    let _ = app.emit(
        "discovering_check",
        DiscoveringCheckPayload {
            rule: rule.to_string(),
            status: status.to_string(),
        },
    );
}

fn emit_check_done(app: &AppHandle) {
    let _ = app.emit("discovering_check_done", ());
}

// --- Registry helpers ---

const LANMAN_SERVER_PARAMS: PCWSTR =
    w!("SYSTEM\\CurrentControlSet\\Services\\LanManServer\\Parameters");

unsafe fn reg_open_key(sam: REG_SAM_FLAGS) -> Result<HKEY, String> {
    let mut key = HKEY(std::ptr::null_mut());
    let status = RegOpenKeyExW(HKEY_LOCAL_MACHINE, LANMAN_SERVER_PARAMS, None, sam, &mut key);
    if status.is_err() {
        let code = status.0;
        return Err(format!("RegOpenKeyExW failed (code {code}): {status:?}"));
    }
    Ok(key)
}

unsafe fn reg_read_dword(key: HKEY, name: PCWSTR) -> Result<u32, String> {
    let mut value_type = REG_DWORD;
    let mut data: u32 = 0;
    let mut data_size: u32 = std::mem::size_of::<u32>() as u32;
    let status = RegQueryValueExW(
        key,
        name,
        None,
        Some(&mut value_type),
        Some(&mut data as *mut u32 as *mut u8),
        Some(&mut data_size),
    );
    if status.is_err() {
        let code = status.0;
        return Err(format!("RegQueryValueExW failed (code {code}): {status:?}"));
    }
    Ok(data)
}

unsafe fn reg_write_dword(key: HKEY, name: PCWSTR, value: u32) -> Result<(), String> {
    let bytes = value.to_ne_bytes();
    let status = RegSetValueExW(key, name, None, REG_DWORD, Some(&bytes));
    if status.is_err() {
        return Err(format!("RegSetValueExW failed: {status:?}"));
    }
    Ok(())
}

// --- Service helpers ---

unsafe fn open_scm() -> Result<SC_HANDLE, String> {
    let handle = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), 1); // SC_MANAGER_CONNECT = 1
    match handle {
        Ok(h) if !h.is_invalid() => Ok(h),
        Ok(_) => Err("OpenSCManagerW returned invalid handle".to_string()),
        Err(e) => Err(format!("OpenSCManagerW failed: {e}")),
    }
}

unsafe fn get_service_status(scm: SC_HANDLE, name: &str) -> Result<u32, String> {
    let wide_name: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let service = OpenServiceW(scm, PCWSTR::from_raw(wide_name.as_ptr()), SERVICE_QUERY_STATUS);
    let svc = match service {
        Ok(h) if !h.is_invalid() => h,
        Ok(_) => return Err(format!("OpenService '{name}' returned invalid handle")),
        Err(e) => return Err(format!("OpenService '{name}' failed: {e}")),
    };

    let mut status = SERVICE_STATUS_PROCESS::default();
    let mut bytes_needed: u32 = 0;
    let buf_size = std::mem::size_of::<SERVICE_STATUS_PROCESS>() as u32;
    let result = QueryServiceStatusEx(
        svc,
        SC_STATUS_PROCESS_INFO,
        Some(std::slice::from_raw_parts_mut(
            &mut status as *mut _ as *mut u8,
            buf_size as usize,
        )),
        &mut bytes_needed,
    );
    CloseServiceHandle(svc).ok();

    match result {
        Ok(_) => Ok(status.dwCurrentState.0),
        Err(e) => Err(format!("QueryServiceStatusEx '{name}' failed: {e}")),
    }
}

unsafe fn start_service(scm: SC_HANDLE, name: &str) -> Result<(), String> {
    let wide_name: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let service = OpenServiceW(scm, PCWSTR::from_raw(wide_name.as_ptr()), SERVICE_START);
    let svc = match service {
        Ok(h) if !h.is_invalid() => h,
        Ok(_) => return Err(format!("OpenService '{name}' returned invalid handle")),
        Err(e) => return Err(format!("OpenService '{name}' failed: {e}")),
    };

    let result = StartServiceW(svc, None);
    CloseServiceHandle(svc).ok();

    match result {
        Ok(()) => {
            log!("start_service: '{name}' start request sent");
            Ok(())
        }
        Err(e) => {
            let err_msg = format!("{e}");
            let code = e.code().0 as u32;
            if err_msg.contains("already") || code == 1056 {
                log!("start_service: '{name}' already running (1056)");
                Ok(())
            } else if code == 1053 {
                log!("start_service: '{name}' timeout but may be starting (1053)");
                Ok(())
            } else {
                Err(format!("StartService '{name}' failed (code {code}): {e}"))
            }
        }
    }
}

/// Poll until a service reaches SERVICE_RUNNING or timeout.
unsafe fn wait_for_service_running(scm: SC_HANDLE, name: &str, timeout_ms: u32) -> Result<(), String> {
    let mut elapsed = 0u32;
    loop {
        let state = get_service_status(scm, name)?;
        if state == SERVICE_RUNNING.0 {
            return Ok(());
        }
        if elapsed >= timeout_ms {
            return Err(format!(
                "Service '{name}' not running after {timeout_ms}ms (state={state})"
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
        elapsed += 500;
    }
}

/// Set a service's startup type to SERVICE_AUTO_START.
/// Requires SERVICE_CHANGE_CONFIG access on the service handle.
unsafe fn set_service_startup_type(scm: SC_HANDLE, name: &str) -> Result<(), String> {
    let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let service = OpenServiceW(scm, PCWSTR::from_raw(wide.as_ptr()), SERVICE_CHANGE_CONFIG);
    let svc = match service {
        Ok(h) if !h.is_invalid() => h,
        Ok(_) => return Err(format!("OpenService '{name}' for config returned invalid handle")),
        Err(e) => return Err(format!("OpenService '{name}' for config failed: {e}")),
    };

    let result = ChangeServiceConfigW(
        svc,
        ENUM_SERVICE_TYPE(SERVICE_NO_CHANGE), // service type unchanged
        SERVICE_AUTO_START,                    // start type
        SERVICE_ERROR(SERVICE_NO_CHANGE),      // error control unchanged (SERVICE_NO_CHANGE = 0xFFFFFFFF)
        PCWSTR::null(),                       // binary path unchanged
        PCWSTR::null(),                       // load order group unchanged
        None,                                 // tag id unchanged
        PCWSTR::null(),                       // dependencies unchanged
        PCWSTR::null(),                       // start name unchanged
        PCWSTR::null(),                       // password unchanged
        PCWSTR::null(),                       // display name unchanged
    );

    CloseServiceHandle(svc).ok();

    match result {
        Ok(()) => {
            log!("set_service_startup_type: '{name}' -> SERVICE_AUTO_START");
            Ok(())
        }
        Err(e) => Err(format!("ChangeServiceConfigW '{name}': {e}")),
    }
}

// --- Firewall rule group helpers ---

/// Enable a Windows Firewall rule group for Private + Public profiles.
/// Returns Ok(true) if changed, Ok(false) if already enabled.
fn enable_firewall_rule_group(app: &AppHandle, label: &str, group_name: &str) -> Result<bool, String> {
    unsafe {
        let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
        if hr.is_err() && hr.0 != 1 { // S_FALSE = already initialized
            return Err(format!("CoInitializeEx failed: {hr:?}"));
        }

        let policy: INetFwPolicy2 = CoCreateInstance(&NetFwPolicy2, None, CLSCTX_INPROC_SERVER)
            .map_err(|e| format!("CoCreateInstance INetFwPolicy2: {e}"))?;

        let group_bstr = BSTR::from(group_name);

        let currently = policy.get_IsRuleGroupCurrentlyEnabled(&group_bstr)
            .map_err(|e| format!("IsRuleGroupCurrentlyEnabled: {e}"))?;

        if currently == VARIANT_TRUE {
            log!("{label}: rule group '{group_name}' already enabled");
            emit_check(app, label, "ok");
            return Ok(false);
        }

        log!("{label}: enabling rule group '{group_name}'...");
        emit_check(app, label, "enabling");
        policy.EnableRuleGroup(FW_PROFILE_PRIVATE_AND_PUBLIC, &group_bstr, VARIANT_TRUE)
            .map_err(|e| format!("EnableRuleGroup: {e}"))?;

        log!("{label}: rule group '{group_name}' enabled");
        emit_check(app, label, "enabled");
        Ok(true)
    }
}

// --- Public folder sharing ---

/// Ensure the "Public" share exists for %SystemDrive%\Users\Public with read access.
/// Uses delete+recreate to guarantee security descriptor is applied on rerun.
fn ensure_public_folder_share(app: &AppHandle) -> Result<(), String> {
    emit_check(app, "Public Folder Sharing", "checking");

    let public_path = std::env::var("SystemDrive")
        .map(|d| format!("{d}\\Users\\Public"))
        .unwrap_or_else(|_| "C:\\Users\\Public".to_string());

    if !std::path::Path::new(&public_path).exists() {
        log!("Public folder '{public_path}' does not exist - creating");
        if let Err(e) = std::fs::create_dir_all(&public_path) {
            let msg = format!("create public folder: {e}");
            emit_check(app, "Public Folder Sharing", "failed");
            return Err(msg);
        }
    }

    // SDDL: D:(A;;FR;;;WD) = Everyone: File Read
    let mut sd: PSECURITY_DESCRIPTOR = PSECURITY_DESCRIPTOR(std::ptr::null_mut());
    unsafe {
        if let Err(e) = ConvertStringSecurityDescriptorToSecurityDescriptorW(
            w!("D:(A;;FR;;;WD)"),
            1, // SDDL_REVISION_1
            &mut sd,
            None,
        ) {
            let msg = format!("SDDL conversion: {e}");
            emit_check(app, "Public Folder Sharing", "failed");
            return Err(msg);
        }
    }

    let share_name: Vec<u16> = "Public\0".encode_utf16().collect();
    let path_wide: Vec<u16> = public_path.encode_utf16().chain(std::iter::once(0)).collect();
    let remark: Vec<u16> = "Public Files\0".encode_utf16().collect();

    let info = SHARE_INFO_502 {
        shi502_netname: windows::core::PWSTR(share_name.as_ptr() as *mut u16),
        shi502_type: STYPE_DISKTREE,
        shi502_remark: windows::core::PWSTR(remark.as_ptr() as *mut u16),
        shi502_permissions: ACCESS_READ,
        shi502_max_uses: 0xFFFFFFFF, // unlimited
        shi502_current_uses: 0,
        shi502_path: windows::core::PWSTR(path_wide.as_ptr() as *mut u16),
        shi502_passwd: windows::core::PWSTR::null(),
        shi502_reserved: 0,
        shi502_security_descriptor: sd,
    };
    let info_ptr = &info as *const SHARE_INFO_502 as *const u8;

    unsafe {
        let result = NetShareAdd(None, 502, info_ptr, None);
        if result == 0 {
            log!("Public share created successfully");
            emit_check(app, "Public Folder Sharing", "enabled");
            return Ok(());
        }
        if result == NERR_DUPLICATE_SHARE {
            // NetShareSetInfo doesn't apply SD changes — delete and recreate
            log!("Public share exists - deleting to apply current settings");
            emit_check(app, "Public Folder Sharing", "enabling");
            NetShareDel(None, PCWSTR::from_raw(share_name.as_ptr()), None);
            let result2 = NetShareAdd(None, 502, info_ptr, None);
            if result2 == 0 {
                log!("Public share recreated successfully");
                emit_check(app, "Public Folder Sharing", "enabled");
                return Ok(());
            }
            let msg = format!("NetShareAdd after delete: code {result2}");
            emit_check(app, "Public Folder Sharing", "failed");
            return Err(msg);
        }
        let msg = format!("NetShareAdd: code {result}");
        emit_check(app, "Public Folder Sharing", "failed");
        Err(msg)
    }
}

// --- Password protected sharing ---

/// Set forceguest registry value for password-protected sharing.
/// 0 = Classic model (password-protected ON), 1 = Guest-only model (OFF).
///
/// Document: forceguest controls the machine-wide local-account network
/// authentication model. When 0 (Classic), remote users authenticate
/// with their own credentials (password-protected). When 1 (Guest-only),
/// all remote connections use the Guest account (no password required).
/// This affects all inbound network services that use local accounts,
/// not just file sharing.
fn ensure_password_protected_sharing(app: &AppHandle) -> Result<(), String> {
    emit_check(app, "Password Protected Sharing", "checking");

    const FORCEGUEST_PATH: PCWSTR = w!("SYSTEM\\CurrentControlSet\\Control\\Lsa");
    let desired: u32 = 0; // 0 = Classic (password-protected ON)

    unsafe {
        let mut key = HKEY(std::ptr::null_mut());
        let status = RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            FORCEGUEST_PATH,
            None,
            REG_SAM_FLAGS((KEY_READ.0 | KEY_SET_VALUE.0) as u32),
            &mut key,
        );
        if status.is_err() {
            let msg = format!("Open Lsa key: {status:?}");
            emit_check(app, "Password Protected Sharing", "failed");
            return Err(msg);
        }

        // Read current value
        let mut value_type = REG_DWORD;
        let mut current: u32 = 0;
        let mut data_size: u32 = std::mem::size_of::<u32>() as u32;
        let qs = RegQueryValueExW(
            key,
            w!("forceguest"),
            None,
            Some(&mut value_type),
            Some(&mut current as *mut u32 as *mut u8),
            Some(&mut data_size),
        );

        if qs.is_ok() && current == desired {
            let _ = RegCloseKey(key);
            log!("forceguest already set to {desired} (Classic)");
            emit_check(app, "Password Protected Sharing", "ok");
            return Ok(());
        }

        log!("Setting forceguest = {desired} (Classic/password-protected ON)");
        emit_check(app, "Password Protected Sharing", "enabling");
        let bytes = desired.to_ne_bytes();
        let ws = RegSetValueExW(key, w!("forceguest"), None, REG_DWORD, Some(&bytes));
        let _ = RegCloseKey(key);

        if ws.is_err() {
            let msg = format!("RegSetValueExW forceguest: {ws:?}");
            emit_check(app, "Password Protected Sharing", "failed");
            return Err(msg);
        }

        log!("forceguest set to {desired}");
        emit_check(app, "Password Protected Sharing", "enabled");
        Ok(())
    }
}

// --- Network category ---

/// Network info returned to the frontend.
#[derive(Clone, serde::Serialize)]
pub struct NetworkInfo {
    pub name: String,
    pub category: String,
    pub is_connected: bool,
}

/// Read-only: enumerate connected networks and their categories.
fn get_connected_networks() -> Result<Vec<NetworkInfo>, String> {
    unsafe {
        let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
        if hr.is_err() && hr.0 != 1 {
            return Err(format!("CoInitializeEx failed: {hr:?}"));
        }

        let nlm: INetworkListManager =
            CoCreateInstance(&NetworkListManager, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| format!("CoCreateInstance INetworkListManager: {e}"))?;

        let networks = nlm
            .GetNetworks(NLM_ENUM_NETWORK_CONNECTED)
            .map_err(|e| format!("GetNetworks: {e}"))?;

        let mut result = Vec::new();
        let mut buf: Vec<Option<INetwork>> = vec![None; 1];
        loop {
            match networks.Next(&mut buf, None) {
                Ok(()) => {
                    if let Some(ref network) = buf[0] {
                        let name = network
                            .GetName()
                            .map(|b| b.to_string())
                            .unwrap_or_default();
                        let cat = network.GetCategory().map(|c| c.0).unwrap_or(-1);
                        let category = match cat {
                            c if c == NLM_NETWORK_CATEGORY_PRIVATE.0 => "Private",
                            c if c == NLM_NETWORK_CATEGORY_PUBLIC.0 => "Public",
                            c if c == NLM_NETWORK_CATEGORY_DOMAIN_AUTHENTICATED.0 => "Domain",
                            _ => "Unknown",
                        }
                        .to_string();
                        let connected = network
                            .IsConnected()
                            .map(|v| v == VARIANT_TRUE)
                            .unwrap_or(false);
                        result.push(NetworkInfo {
                            name,
                            category,
                            is_connected: connected,
                        });
                    }
                }
                Err(_) => break,
            }
        }

        log!(
            "get_connected_networks: found {} network(s)",
            result.len()
        );
        for n in &result {
            log!(
                "  - '{}' category={} connected={}",
                n.name,
                n.category,
                n.is_connected
            );
        }
        Ok(result)
    }
}

// --- Structured result types ---

/// Full sharing state for the frontend (read-side).
#[derive(Clone, serde::Serialize)]
pub struct SharingState {
    pub network_discovery_enabled: bool,
    pub file_sharing_enabled: bool,
    pub public_folder_sharing: bool,
    pub password_protected_sharing: bool,
    pub networks: Vec<NetworkInfo>,
    pub services: Vec<ServiceInfo>,
}

#[derive(Clone, serde::Serialize)]
pub struct ServiceInfo {
    pub name: String,
    pub display_name: String,
    pub running: bool,
}

/// Read-side: check current sharing state without modifying anything.
pub fn get_sharing_state(_app: &AppHandle) -> Result<SharingState, String> {
    let mut nd_enabled = false;
    let mut fs_enabled = false;

    // Check firewall rule groups
    unsafe {
        let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
        if hr.is_ok() || hr.0 == 1 {
            if let Ok(policy) =
                CoCreateInstance::<_, INetFwPolicy2>(&NetFwPolicy2, None, CLSCTX_INPROC_SERVER)
            {
                let nd = BSTR::from("Network Discovery");
                if let Ok(v) = policy.get_IsRuleGroupCurrentlyEnabled(&nd) {
                    nd_enabled = v == VARIANT_TRUE;
                }
                let fs = BSTR::from("File and Printer Sharing");
                if let Ok(v) = policy.get_IsRuleGroupCurrentlyEnabled(&fs) {
                    fs_enabled = v == VARIANT_TRUE;
                }
            }
        }
    }

    // Check public folder share
    let public_share_exists = std::path::Path::new(
        &std::env::var("SystemDrive")
            .map(|d| format!("{d}\\Users\\Public"))
            .unwrap_or_else(|_| "C:\\Users\\Public".to_string()),
    )
    .exists();

    // Check forceguest
    let password_protected = unsafe {
        let mut key = HKEY(std::ptr::null_mut());
        let status = RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            w!("SYSTEM\\CurrentControlSet\\Control\\Lsa"),
            None,
            REG_SAM_FLAGS(KEY_READ.0),
            &mut key,
        );
        if status.is_ok() {
            let mut value_type = REG_DWORD;
            let mut data: u32 = 1; // default: guest-only (password-protected OFF)
            let mut data_size: u32 = std::mem::size_of::<u32>() as u32;
            let qs = RegQueryValueExW(
                key,
                w!("forceguest"),
                None,
                Some(&mut value_type),
                Some(&mut data as *mut u32 as *mut u8),
                Some(&mut data_size),
            );
            let _ = RegCloseKey(key);
            qs.is_ok() && data == 0 // 0 = Classic = password-protected ON
        } else {
            false
        }
    };

    // Check services
    let service_names = [
        ("fdphost", "Function Discovery Provider Host"),
        ("FDResPub", "Function Discovery Resource Publication"),
        ("SSDPSRV", "SSDP Discovery"),
        ("upnphost", "UPnP Device Host"),
    ];
    let services = unsafe {
        match open_scm() {
            Ok(scm) => {
                let mut svcs = Vec::new();
                for (name, display_name) in &service_names {
                    let running = get_service_status(scm, name)
                        .map(|s| s == SERVICE_RUNNING.0)
                        .unwrap_or(false);
                    svcs.push(ServiceInfo {
                        name: name.to_string(),
                        display_name: display_name.to_string(),
                        running,
                    });
                }
                CloseServiceHandle(scm).ok();
                svcs
            }
            Err(_) => service_names
                .iter()
                .map(|(n, d)| ServiceInfo {
                    name: n.to_string(),
                    display_name: d.to_string(),
                    running: false,
                })
                .collect(),
        }
    };

    let networks = get_connected_networks().unwrap_or_default();

    Ok(SharingState {
        network_discovery_enabled: nd_enabled,
        file_sharing_enabled: fs_enabled,
        public_folder_sharing: public_share_exists,
        password_protected_sharing: password_protected,
        networks,
        services,
    })
}

// --- Main public API ---

pub fn check_and_enable_discovering(app: &AppHandle, persist: bool) -> Result<bool, String> {
    log!("check_and_enable_discovering - starting...");
    let mut all_ok = true;

    // Emit initial "checking" status so progress bar starts at 0%
    emit_check(app, "SMB Signing", "checking");
    emit_check(app, "SMB Encryption", "checking");
    emit_check(app, "Function Discovery Provider Host", "checking");
    emit_check(app, "Function Discovery Resource Publication", "checking");
    emit_check(app, "SSDP Discovery", "checking");
    emit_check(app, "UPnP Device Host", "checking");
    emit_check(app, "Network Discovery", "checking");
    emit_check(app, "File and Printer Sharing", "checking");
    emit_check(app, "Public Folder Sharing", "checking");
    emit_check(app, "Password Protected Sharing", "checking");
    std::thread::sleep(std::time::Duration::from_millis(400));

    // ================================================================
    // 1. SMB Signing
    // ================================================================
    emit_check(app, "SMB Signing", "checking");
    unsafe {
        match reg_open_key(REG_SAM_FLAGS(KEY_READ.0)) {
            Ok(key) => {
                match reg_read_dword(key, w!("EnableSecuritySignature")) {
                    Ok(current) => {
                        let _ = RegCloseKey(key);
                        if current == 0 {
                            log!("EnableSecuritySignature is OFF - turning ON");
                            emit_check(app, "SMB Signing", "enabling");
                            match reg_open_key(REG_SAM_FLAGS((KEY_READ.0 | KEY_SET_VALUE.0) as u32)) {
                                Ok(wkey) => {
                                    match reg_write_dword(wkey, w!("EnableSecuritySignature"), 1) {
                                        Ok(()) => {
                                            let _ = RegCloseKey(wkey);
                                            log!("EnableSecuritySignature enabled");
                                            emit_check(app, "SMB Signing", "enabled");
                                        }
                                        Err(e) => {
                                            let _ = RegCloseKey(wkey);
                                            log!("EnableSecuritySignature write failed: {e}");
                                            emit_check(app, "SMB Signing", "failed");
                                            all_ok = false;
                                        }
                                    }
                                }
                                Err(e) => {
                                    log!("EnableSecuritySignature key open failed: {e}");
                                    emit_check(app, "SMB Signing", "failed");
                                    all_ok = false;
                                }
                            }
                        } else {
                            log!("EnableSecuritySignature is already ON");
                            emit_check(app, "SMB Signing", "ok");
                        }
                    }
                    Err(e) => {
                        let _ = RegCloseKey(key);
                        if e.contains("code 2") || e.contains("WIN32_ERROR(2)") {
                            log!("EnableSecuritySignature value not found - creating with value 1");
                            emit_check(app, "SMB Signing", "enabling");
                            match reg_open_key(REG_SAM_FLAGS((KEY_READ.0 | KEY_SET_VALUE.0) as u32)) {
                                Ok(wkey) => {
                                    match reg_write_dword(wkey, w!("EnableSecuritySignature"), 1) {
                                        Ok(()) => {
                                            let _ = RegCloseKey(wkey);
                                            log!("EnableSecuritySignature created and enabled");
                                            emit_check(app, "SMB Signing", "enabled");
                                        }
                                        Err(e) => {
                                            let _ = RegCloseKey(wkey);
                                            log!("EnableSecuritySignature create failed: {e}");
                                            emit_check(app, "SMB Signing", "failed");
                                            all_ok = false;
                                        }
                                    }
                                }
                                Err(e) => {
                                    log!("EnableSecuritySignature key open for create failed: {e}");
                                    emit_check(app, "SMB Signing", "failed");
                                    all_ok = false;
                                }
                            }
                        } else {
                            log!("EnableSecuritySignature read failed: {e}");
                            emit_check(app, "SMB Signing", "failed");
                            all_ok = false;
                        }
                    }
                }
            }
            Err(e) => {
                log!("EnableSecuritySignature key open failed: {e}");
                emit_check(app, "SMB Signing", "failed");
                all_ok = false;
            }
        }
    }

    // ================================================================
    // 2. SMB Encryption
    // ================================================================
    emit_check(app, "SMB Encryption", "checking");
    unsafe {
        match reg_open_key(REG_SAM_FLAGS(KEY_READ.0)) {
            Ok(key) => {
                match reg_read_dword(key, w!("EncryptData")) {
                    Ok(current) => {
                        let _ = RegCloseKey(key);
                        if current == 0 {
                            log!("EncryptData is OFF - turning ON");
                            emit_check(app, "SMB Encryption", "enabling");
                            match reg_open_key(REG_SAM_FLAGS((KEY_READ.0 | KEY_SET_VALUE.0) as u32)) {
                                Ok(wkey) => {
                                    match reg_write_dword(wkey, w!("EncryptData"), 1) {
                                        Ok(()) => {
                                            let _ = RegCloseKey(wkey);
                                            log!("EncryptData enabled");
                                            emit_check(app, "SMB Encryption", "enabled");
                                        }
                                        Err(e) => {
                                            let _ = RegCloseKey(wkey);
                                            log!("EncryptData write failed: {e}");
                                            emit_check(app, "SMB Encryption", "failed");
                                            all_ok = false;
                                        }
                                    }
                                }
                                Err(e) => {
                                    log!("EncryptData key open for write failed: {e}");
                                    emit_check(app, "SMB Encryption", "failed");
                                    all_ok = false;
                                }
                            }
                        } else {
                            log!("EncryptData is already ON");
                            emit_check(app, "SMB Encryption", "ok");
                        }
                    }
                    Err(e) => {
                        let _ = RegCloseKey(key);
                        if e.contains("code 2") || e.contains("WIN32_ERROR(2)") {
                            log!("EncryptData value not found - creating with value 1");
                            emit_check(app, "SMB Encryption", "enabling");
                            match reg_open_key(REG_SAM_FLAGS((KEY_READ.0 | KEY_SET_VALUE.0) as u32)) {
                                Ok(wkey) => {
                                    match reg_write_dword(wkey, w!("EncryptData"), 1) {
                                        Ok(()) => {
                                            let _ = RegCloseKey(wkey);
                                            log!("EncryptData created and enabled");
                                            emit_check(app, "SMB Encryption", "enabled");
                                        }
                                        Err(e) => {
                                            let _ = RegCloseKey(wkey);
                                            log!("EncryptData create failed: {e}");
                                            emit_check(app, "SMB Encryption", "failed");
                                            all_ok = false;
                                        }
                                    }
                                }
                                Err(e) => {
                                    log!("EncryptData key open for create failed: {e}");
                                    emit_check(app, "SMB Encryption", "failed");
                                    all_ok = false;
                                }
                            }
                        } else {
                            log!("EncryptData read failed: {e}");
                            emit_check(app, "SMB Encryption", "failed");
                            all_ok = false;
                        }
                    }
                }
            }
            Err(e) => {
                log!("EncryptData key open failed: {e}");
                emit_check(app, "SMB Encryption", "failed");
                all_ok = false;
            }
        }
    }

    // ================================================================
    // 3. Discovery services — start if stopped, optionally persist
    // ================================================================
    let services_to_start = [
        ("fdphost", "Function Discovery Provider Host"),
        ("FDResPub", "Function Discovery Resource Publication"),
        ("SSDPSRV", "SSDP Discovery"),
        ("upnphost", "UPnP Device Host"),
    ];

    unsafe {
        match open_scm() {
            Ok(scm) => {
                for (name, display_name) in &services_to_start {
                    emit_check(app, display_name, "checking");
                    match get_service_status(scm, name) {
                        Ok(status) => {
                            if status == SERVICE_STOPPED.0 || status == SERVICE_START_PENDING.0 {
                                log!("Service '{name}' is stopped - starting...");
                                emit_check(app, display_name, "starting");
                                match start_service(scm, name) {
                                    Ok(()) => {
                                        match wait_for_service_running(scm, name, 15_000) {
                                            Ok(()) => {
                                                log!("Service '{name}' confirmed running");
                                                emit_check(app, display_name, "started");
                                            }
                                            Err(e) => {
                                                log!("Service '{name}' polling: {e}");
                                                emit_check(app, display_name, "started");
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        log!("Service '{name}' start failed: {e}");
                                        emit_check(app, display_name, "failed");
                                        all_ok = false;
                                    }
                                }
                            } else {
                                log!("Service '{name}' is already running (state={status})");
                                emit_check(app, display_name, "ok");
                            }
                        }
                        Err(e) => {
                            log!("Service '{name}' status query failed: {e}");
                            emit_check(app, display_name, "failed");
                            all_ok = false;
                        }
                    }

                    // Optionally persist startup type (continue on failure)
                    if persist {
                        if let Err(e) = set_service_startup_type(scm, name) {
                            log!("Failed to persist startup type for '{name}': {e}");
                            // non-fatal — don't set all_ok = false
                        }
                    }
                }
                CloseServiceHandle(scm).ok();
            }
            Err(e) => {
                log!("SCM open failed: {e} - skipping all services");
                for (_, display_name) in &services_to_start {
                    emit_check(app, display_name, "failed");
                }
                all_ok = false;
            }
        }
    }

    // ================================================================
    // 4. Network Discovery firewall rules
    // ================================================================
    match enable_firewall_rule_group(app, "Network Discovery", "Network Discovery") {
        Ok(_) => {}
        Err(e) => {
            log!("Network Discovery firewall rules failed: {e}");
            emit_check(app, "Network Discovery", "failed");
            all_ok = false;
        }
    }

    // ================================================================
    // 5. File and Printer Sharing firewall rules
    // ================================================================
    match enable_firewall_rule_group(app, "File and Printer Sharing", "File and Printer Sharing") {
        Ok(_) => {}
        Err(e) => {
            log!("File and Printer Sharing firewall rules failed: {e}");
            emit_check(app, "File and Printer Sharing", "failed");
            all_ok = false;
        }
    }

    // ================================================================
    // 6. Public folder sharing
    // ================================================================
    if let Err(e) = ensure_public_folder_share(app) {
        log!("Public folder sharing failed: {e}");
        all_ok = false;
    }

    // ================================================================
    // 7. Password protected sharing
    // ================================================================
    if let Err(e) = ensure_password_protected_sharing(app) {
        log!("Password protected sharing failed: {e}");
        all_ok = false;
    }

    log!("check_and_enable_discovering - complete");
    std::thread::sleep(std::time::Duration::from_millis(800));
    emit_check_done(app);
    Ok(all_ok)
}
