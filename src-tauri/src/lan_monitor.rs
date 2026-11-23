use serde::Serialize;
use socket2::{Domain, Protocol, Socket, Type};
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

const LAN_PORT: u16 = 50001;
const PEER_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Serialize, Clone)]
pub struct LanPeer {
    pub address: String,
    pub port: u16,
    pub name: String,
    pub strings: Vec<String>,
    pub packets: u32,
    pub last_seen: u64,
}

#[derive(Serialize, Clone)]
pub struct LanSnapshot {
    pub listening: bool,
    pub error: Option<String>,
    pub interface: String,
    pub peers: Vec<LanPeer>,
}

struct PeerState {
    port: u16,
    name: String,
    strings: Vec<String>,
    packets: u32,
    last_seen: Instant,
}

pub struct LanMonitor {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

fn printable(byte: u8) -> bool {
    byte == b' ' || byte.is_ascii_alphanumeric() || b"-_.[]()'+#/:& ".contains(&byte)
}

fn extract_strings(payload: &[u8]) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();

    let mut current = String::new();
    for &byte in payload {
        if printable(byte) {
            current.push(byte as char);
        } else {
            if current.trim().len() >= 4 {
                found.push(current.trim().to_string());
            }
            current.clear();
        }
    }
    if current.trim().len() >= 4 {
        found.push(current.trim().to_string());
    }

    let mut wide = String::new();
    for pair in payload.chunks_exact(2) {
        if pair[1] == 0 && printable(pair[0]) {
            wide.push(pair[0] as char);
        } else {
            if wide.trim().len() >= 4 {
                found.push(wide.trim().to_string());
            }
            wide.clear();
        }
    }
    if wide.trim().len() >= 4 {
        found.push(wide.trim().to_string());
    }

    found.retain(|s| {
        !s.chars().all(|c| c.is_ascii_digit())
            && !s.contains("0.0.0.0")
            && !s.contains("255.255")
            && s.len() <= 64
    });
    found.sort_by_key(|s| std::cmp::Reverse(s.len()));
    found.dedup();
    found.truncate(3);
    found
}

fn bind_socket() -> Result<UdpSocket, String> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))
        .map_err(|e| format!("could not create UDP socket: {e}"))?;
    socket
        .set_reuse_address(true)
        .map_err(|e| format!("could not set SO_REUSEADDR: {e}"))?;
    let address: SocketAddr = ([0, 0, 0, 0], LAN_PORT).into();
    socket
        .bind(&address.into())
        .map_err(|e| format!("could not bind UDP {LAN_PORT}: {e}"))?;
    let socket: UdpSocket = socket.into();
    socket
        .set_read_timeout(Some(Duration::from_millis(300)))
        .map_err(|e| e.to_string())?;
    Ok(socket)
}

impl LanMonitor {
    pub fn start(app: AppHandle, local_ip: String) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();

        let handle = thread::spawn(move || {
            let mut listening = false;
            let socket = match bind_socket() {
                Ok(s) => {
                    listening = true;
                    Some(s)
                }
                Err(e) => {
                    let _ = app.emit("lan_peers", LanSnapshot {
                        listening: false,
                        error: Some(e),
                        interface: local_ip.clone(),
                        peers: vec![],
                    });
                    None
                }
            };

            let capture_path = if std::env::var("BLUR_LAN_CAPTURE").as_deref() == Ok("1") {
                app.path().app_data_dir().ok().map(|dir| dir.join("lan_capture.log"))
            } else {
                None
            };

            let mut peers: HashMap<String, PeerState> = HashMap::new();
            let mut last_emit = Instant::now();

            while !thread_stop.load(Ordering::Relaxed) {
                if let Some(ref socket) = socket {
                    let mut buffer = [0u8; 2048];
                    match socket.recv_from(&mut buffer) {
                        Ok((size, source)) => {
                            let ip = source.ip().to_string();
                            if ip == local_ip || ip.starts_with("127.") {
                                continue;
                            }
                            if let Some(ref path) = capture_path {
                                if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
                                    use std::io::Write;
                                    let sample = &buffer[..size.min(256)];
                                    let hex: String = sample.iter().map(|b| format!("{b:02x}")).collect();
                                    let _ = writeln!(file, "{} {}:{} {}", ip, source.port(), size, hex);
                                }
                            }
                            let strings = extract_strings(&buffer[..size]);
                            let name = strings.first().cloned().unwrap_or_else(|| format!("LAN HOST {ip}"));
                            let entry = peers.entry(ip.clone()).or_insert_with(|| PeerState {
                                port: source.port(),
                                name: name.clone(),
                                strings: strings.clone(),
                                packets: 0,
                                last_seen: Instant::now(),
                            });
                            entry.port = source.port();
                            if !strings.is_empty() {
                                entry.strings = strings;
                                entry.name = name;
                            }
                            entry.packets += 1;
                            entry.last_seen = Instant::now();
                        }
                        Err(ref e)
                            if e.kind() == std::io::ErrorKind::WouldBlock
                                || e.kind() == std::io::ErrorKind::TimedOut => {}
                        Err(e) => {
                            let _ = app.emit("lan_peers", LanSnapshot {
                                listening: false,
                                error: Some(format!("UDP receive failed: {e}")),
                                interface: local_ip.clone(),
                                peers: vec![],
                            });
                            break;
                        }
                    }
                }

                if last_emit.elapsed() >= Duration::from_secs(1) {
                    last_emit = Instant::now();
                    peers.retain(|_, peer| peer.last_seen.elapsed() < PEER_TIMEOUT);
                    let mut list: Vec<LanPeer> = peers
                        .iter()
                        .map(|(ip, peer)| LanPeer {
                            address: ip.clone(),
                            port: peer.port,
                            name: peer.name.clone(),
                            strings: peer.strings.clone(),
                            packets: peer.packets,
                            last_seen: peer.last_seen.elapsed().as_secs(),
                        })
                        .collect();
                    list.sort_by(|a, b| a.last_seen.cmp(&b.last_seen).then(a.address.cmp(&b.address)));
                    let _ = app.emit("lan_peers", LanSnapshot {
                        listening,
                        error: None,
                        interface: local_ip.clone(),
                        peers: list,
                    });
                }
            }
        });

        Self {
            stop,
            handle: Some(handle),
        }
    }

    fn shutdown(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for LanMonitor {
    fn drop(&mut self) {
        self.shutdown();
    }
}
