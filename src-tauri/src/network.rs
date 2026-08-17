use serde::Serialize;
use windows::core::BSTR;
use windows::Win32::System::Com::*;
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::System::Wmi::*;

macro_rules! log {
    ($($arg:tt)*) => {
        eprintln!("[network] {}", format!($($arg)*));
    };
}

#[derive(Serialize, Clone)]
pub struct AdapterInfo {
    pub name: String,
    pub status: String,
    pub adapter_type: String,
}

struct WmiSession {
    services: ISWbemServices,
}

impl WmiSession {
    fn connect() -> Result<Self, String> {
        log!("WmiSession::connect - initializing COM...");
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

            log!("WmiSession::connect - creating SWbemLocator...");
            let locator: ISWbemLocator = CoCreateInstance(
                &SWbemLocator,
                None,
                CLSCTX_INPROC_SERVER,
            )
            .map_err(|e| {
                let msg = format!("CoCreateInstance failed: {e}");
                log!("{msg}");
                msg
            })?;

            log!("WmiSession::connect - connecting to ROOT\\CIMV2...");
            let services = locator.ConnectServer(
                &BSTR::from("."),
                &BSTR::from("ROOT\\CIMV2"),
                &BSTR::new(),
                &BSTR::new(),
                &BSTR::new(),
                &BSTR::new(),
                0,
                None,
            )
            .map_err(|e| {
                let msg = format!("ConnectServer failed: {e}");
                log!("{msg}");
                msg
            })?;

            log!("WmiSession::connect - OK");
            Ok(Self { services })
        }
    }

    fn query(&self, wql: &str) -> Result<ISWbemObjectSet, String> {
        unsafe {
            self.services
                .ExecQuery(
                    &BSTR::from(wql),
                    &BSTR::from("WQL"),
                    0,
                    None,
                )
                .map_err(|e| format!("ExecQuery failed: {e}"))
        }
    }

    fn call_method(obj: &ISWbemObject, method: &str) -> Result<i32, String> {
        log!("call_method - calling '{method}'...");
        unsafe {
            let result = obj.ExecMethod_(
                &BSTR::from(method),
                None,
                0,
                None,
            )
            .map_err(|e| {
                let msg = format!("ExecMethod '{method}' failed: {e}");
                log!("{msg}");
                msg
            })?;

            let props = result.Properties_()
                .map_err(|e| {
                    let msg = format!("Properties_ failed: {e}");
                    log!("{msg}");
                    msg
                })?;

            let prop = props.Item(&BSTR::from("ReturnValue"), 0)
                .map_err(|e| {
                    let msg = format!("Item 'ReturnValue' failed: {e}");
                    log!("{msg}");
                    msg
                })?;

            let val = prop.Value()
                .map_err(|e| {
                    let msg = format!("Value failed: {e}");
                    log!("{msg}");
                    msg
                })?;

            let ret = variant_to_i32(&val)?;
            log!("call_method - '{method}' returned {ret}");
            Ok(ret)
        }
    }

    fn get_property_string(obj: &ISWbemObject, name: &str) -> Option<String> {
        unsafe {
            let props = obj.Properties_().ok()?;
            let prop = props.Item(&BSTR::from(name), 0).ok()?;
            let val = prop.Value().ok()?;
            variant_to_string(&val)
        }
    }

    fn get_property_u32(obj: &ISWbemObject, name: &str) -> Option<u32> {
        unsafe {
            let props = obj.Properties_().ok()?;
            let prop = props.Item(&BSTR::from(name), 0).ok()?;
            let val = prop.Value().ok()?;
            variant_to_u32(&val)
        }
    }
}

unsafe fn variant_to_string(val: &VARIANT) -> Option<String> {
    // VT_BSTR = 8
    if val.Anonymous.Anonymous.vt.0 == 8 {
        let bstr = &val.Anonymous.Anonymous.Anonymous.bstrVal;
        Some(bstr.to_string())
    } else {
        None
    }
}

unsafe fn variant_to_u32(val: &VARIANT) -> Option<u32> {
    let vt = val.Anonymous.Anonymous.vt.0;
    match vt {
        3 => Some(val.Anonymous.Anonymous.Anonymous.lVal as u32), // VT_I4
        19 => Some(val.Anonymous.Anonymous.Anonymous.ulVal),       // VT_UI4
        _ => None,
    }
}

unsafe fn variant_to_i32(val: &VARIANT) -> Result<i32, String> {
    let vt = val.Anonymous.Anonymous.vt.0;
    match vt {
        3 => Ok(val.Anonymous.Anonymous.Anonymous.lVal),  // VT_I4
        19 => Ok(val.Anonymous.Anonymous.Anonymous.ulVal as i32), // VT_UI4
        _ => Err(format!("Unexpected VARIANT type: {vt}")),
    }
}

pub fn disable_adapter(name: &str) -> Result<(), String> {
    log!("disable_adapter: '{name}' - connecting to WMI...");
    let session = WmiSession::connect()?;

    let wql = format!(
        "SELECT * FROM Win32_NetworkAdapter WHERE Name = '{}'",
        name.replace('\'', "''")
    );
    log!("disable_adapter: query = {wql}");
    let results = session.query(&wql)?;

    let count = unsafe { results.Count().map_err(|e| format!("Count failed: {e}"))? };
    log!("disable_adapter: found {count} matching adapter(s)");
    if count == 0 {
        return Err(format!("Adapter '{}' not found", name));
    }

    let adapter = unsafe {
        results.ItemIndex(0)
            .map_err(|e| format!("ItemIndex failed: {e}"))?
    };

    let ret = WmiSession::call_method(&adapter, "Disable")?;
    if ret != 0 {
        log!("disable_adapter: ERROR - Disable returned {ret}");
        return Err(format!("Disable returned error code: {ret}"));
    }

    log!("disable_adapter: '{name}' disabled successfully");
    Ok(())
}

pub fn enable_adapter(name: &str) -> Result<(), String> {
    log!("enable_adapter: '{name}' - connecting to WMI...");
    let session = WmiSession::connect()?;

    let wql = format!(
        "SELECT * FROM Win32_NetworkAdapter WHERE Name = '{}'",
        name.replace('\'', "''")
    );
    log!("enable_adapter: query = {wql}");
    let results = session.query(&wql)?;

    let count = unsafe { results.Count().map_err(|e| format!("Count failed: {e}"))? };
    log!("enable_adapter: found {count} matching adapter(s)");
    if count == 0 {
        return Err(format!("Adapter '{}' not found", name));
    }

    let adapter = unsafe {
        results.ItemIndex(0)
            .map_err(|e| format!("ItemIndex failed: {e}"))?
    };

    let ret = WmiSession::call_method(&adapter, "Enable")?;
    if ret != 0 {
        log!("enable_adapter: ERROR - Enable returned {ret}");
        return Err(format!("Enable returned error code: {ret}"));
    }

    log!("enable_adapter: '{name}' enabled successfully");
    Ok(())
}

pub fn list_all_adapters() -> Result<Vec<AdapterInfo>, String> {
    let session = WmiSession::connect()?;
    let results = session.query("SELECT * FROM Win32_NetworkAdapter")?;

    let count = unsafe { results.Count().map_err(|e| format!("Count failed: {e}"))? };
    let mut adapters = Vec::new();

    for i in 0..count {
        let adapter = unsafe {
            results.ItemIndex(i)
                .map_err(|e| format!("ItemIndex failed: {e}"))?
        };

        let name = match WmiSession::get_property_string(&adapter, "Name") {
            Some(n) => n,
            None => continue,
        };

        let status = WmiSession::get_property_u32(&adapter, "NetConnectionStatus")
            .map(|s| match s {
                0 => "Disconnected",
                1 => "Connecting",
                2 => "Connected",
                7 => "Disabled",
                _ => "Unknown",
            })
            .unwrap_or("Unknown")
            .to_string();

        let adapter_type = classify_adapter(&name);
        adapters.push(AdapterInfo {
            name,
            status,
            adapter_type,
        });
    }

    Ok(adapters)
}

pub fn list_virtual_adapters() -> Result<Vec<String>, String> {
    let adapters = list_all_adapters()?;
    Ok(adapters
        .into_iter()
        .filter(|a| a.adapter_type == "virtual")
        .map(|a| a.name)
        .collect())
}

fn classify_adapter(name: &str) -> String {
    let combined = name.to_lowercase();
    if combined.contains("wi-fi") || combined.contains("wifi") || combined.contains("wireless") {
        "wifi".to_string()
    } else if combined.contains("bluetooth") {
        "bluetooth".to_string()
    } else if combined.contains("vmware")
        || combined.contains("vmnet")
        || combined.contains("virtualbox")
        || combined.contains("vbox")
        || combined.contains("hyper-v")
        || combined.contains("vpn")
        || combined.contains("bridge")
        || combined.contains("virtual")
    {
        "virtual".to_string()
    } else {
        "ethernet".to_string()
    }
}
