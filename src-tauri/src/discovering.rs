use tauri::Emitter;
use tauri::AppHandle;
use windows::core::{w, PCWSTR};
use windows::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY, HKEY_LOCAL_MACHINE,
    KEY_READ, KEY_SET_VALUE, REG_DWORD, REG_SAM_FLAGS,
};
use windows::Win32::System::Services::{
    CloseServiceHandle, OpenSCManagerW, OpenServiceW, QueryServiceStatusEx, StartServiceW,
    SC_HANDLE, SC_STATUS_PROCESS_INFO, SERVICE_START, SERVICE_START_PENDING, SERVICE_STATUS_PROCESS,
    SERVICE_STOPPED,
};

macro_rules! log {
    ($($arg:tt)*) => {
        eprintln!("[discovering] {}", format!($($arg)*));
    };
}

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
    let service = OpenServiceW(scm, PCWSTR::from_raw(wide_name.as_ptr()), SERVICE_START);
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

// --- Public API ---

pub fn check_and_enable_discovering(app: &AppHandle) -> Result<bool, String> {
    log!("check_and_enable_discovering - starting...");
    let mut all_ok = true;

    // 1. Check & enable EnableSecuritySignature
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
                        log!("EnableSecuritySignature read failed: {e}");
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
    }

    // 2. Check & enable EncryptData
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
                                    log!("EncryptData key open failed: {e}");
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
                        log!("EncryptData read failed: {e}");
                        emit_check(app, "SMB Encryption", "failed");
                        all_ok = false;
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

    // 3. Start discovery services if stopped
    let services_to_start = [
        ("fdphost", "Function Discovery Provider Host"),
        ("FDResPub", "Function Discovery Resource Publication"),
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
                                        log!("Service '{name}' start request sent OK");
                                        emit_check(app, display_name, "started");
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

    log!("check_and_enable_discovering - complete");
    emit_check_done(app);
    Ok(all_ok)
}
