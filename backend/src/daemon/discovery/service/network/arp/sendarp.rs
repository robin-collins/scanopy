//! Windows SendARP implementation using the native iphlpapi API.
//! This module provides ARP scanning without requiring Npcap installation.

#[cfg(target_family = "windows")]
use std::net::Ipv4Addr;

#[cfg(target_family = "windows")]
use anyhow::Result;
#[cfg(target_family = "windows")]
use mac_address::MacAddress;

#[cfg(target_family = "windows")]
use super::types::ArpScanResult;

#[cfg(target_family = "windows")]
const SENDARP_CONCURRENCY: usize = 50;

/// Scan targets using Windows SendARP API, returning results via a streaming channel.
/// Matches the broadcast ARP interface so callers get results as they arrive.
/// Uses scoped threads in chunks for concurrency without requiring a tokio runtime.
#[cfg(target_family = "windows")]
pub fn scan_subnet_streaming(
    targets: Vec<Ipv4Addr>,
) -> Result<std::sync::mpsc::Receiver<ArpScanResult>> {
    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        for chunk in targets.chunks(SENDARP_CONCURRENCY) {
            std::thread::scope(|s| {
                for &ip in chunk {
                    let tx = tx.clone();
                    s.spawn(move || {
                        if let Some(result) = send_arp_single(ip) {
                            let _ = tx.send(result);
                        }
                    });
                }
            });
        }
    });

    Ok(rx)
}

#[cfg(target_family = "windows")]
fn send_arp_single(target_ip: Ipv4Addr) -> Option<ArpScanResult> {
    use windows::Win32::NetworkManagement::IpHelper::SendARP;

    // Convert IP to the format expected by SendARP (network byte order u32)
    let dest_ip = u32::from_ne_bytes(target_ip.octets());
    let mut mac_addr: [u8; 8] = [0; 8];
    let mut mac_len: u32 = 6;

    // SAFETY: SendARP is a well-defined Windows API function.
    // We pass valid pointers and sizes.
    let result = unsafe { SendARP(dest_ip, 0, mac_addr.as_mut_ptr() as *mut _, &mut mac_len) };

    if result == 0 && mac_len >= 6 {
        tracing::trace!(ip = %target_ip, "SendARP success");
        let mac = MacAddress::new([
            mac_addr[0],
            mac_addr[1],
            mac_addr[2],
            mac_addr[3],
            mac_addr[4],
            mac_addr[5],
        ]);
        Some(ArpScanResult { ip: target_ip, mac })
    } else {
        tracing::trace!(ip = %target_ip, result = result, "SendARP failed or no response");
        None
    }
}

// Stub for non-Windows platforms
#[cfg(not(target_family = "windows"))]
pub fn scan_subnet_streaming(
    _targets: Vec<std::net::Ipv4Addr>,
) -> anyhow::Result<std::sync::mpsc::Receiver<super::types::ArpScanResult>> {
    Err(anyhow::anyhow!("SendARP is only available on Windows"))
}
