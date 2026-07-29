#[cfg(any(target_os = "linux", target_os = "macos"))]
use super::dto::NetworkMonitorWarning;
use super::dto::{
    InterfaceKind, InterfaceState, RawInterfaceCounters, RawNetworkSample, RawProxyState,
};
use super::error::{NetworkMonitorError, NetworkMonitorErrorCode, NetworkMonitorResult};
use sha2::{Digest, Sha256};

pub trait PlatformNetworkCollector: Send {
    fn collect(&mut self) -> NetworkMonitorResult<RawNetworkSample>;
}

pub fn create_platform_collector() -> Box<dyn PlatformNetworkCollector> {
    #[cfg(windows)]
    return Box::new(HostPlatformCollector);
    #[cfg(not(windows))]
    Box::new(HostPlatformCollector::default())
}

fn stable_id(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    format!("{:x}", hasher.finalize())
}

#[cfg(windows)]
#[derive(Default)]
struct HostPlatformCollector;

#[cfg(windows)]
impl PlatformNetworkCollector for HostPlatformCollector {
    fn collect(&mut self) -> NetworkMonitorResult<RawNetworkSample> {
        use std::{ptr::null_mut, slice};
        use windows::Win32::NetworkManagement::IpHelper::{
            FreeMibTable, GetIfTable2, MIB_IF_TABLE2,
        };

        let mut table: *mut MIB_IF_TABLE2 = null_mut();
        unsafe {
            let error = GetIfTable2(&mut table);
            if error.0 != 0 {
                return Err(NetworkMonitorError::new(
                    NetworkMonitorErrorCode::CollectorUnavailable,
                    format!("GetIfTable2 failed with code {}", error.0),
                ));
            }

            if table.is_null() {
                return Err(NetworkMonitorError::new(
                    NetworkMonitorErrorCode::CollectorUnavailable,
                    "GetIfTable2 returned a null table",
                ));
            }

            let rows = slice::from_raw_parts((*table).Table.as_ptr(), (*table).NumEntries as usize);
            let interfaces: Vec<RawInterfaceCounters> = rows
                .iter()
                .map(|row| {
                    let name = utf16_buffer_to_string(&row.Alias);
                    let interface_type = row.Type;
                    let (kind, looks_virtual, is_tunnel) =
                        classify_windows_interface(interface_type, &name);
                    let guid = format!("{:?}", row.InterfaceGuid);
                    RawInterfaceCounters {
                        stable_id: stable_id(&["windows", &guid]),
                        interface_index: row.InterfaceIndex,
                        name,
                        kind,
                        state: if row.OperStatus.0 == 1 {
                            InterfaceState::Up
                        } else {
                            InterfaceState::Down
                        },
                        is_virtual: looks_virtual,
                        tunnel_type: is_tunnel.then(|| format!("{}", row.TunnelType.0)),
                        received_bytes: row.InOctets,
                        transmitted_bytes: row.OutOctets,
                    }
                })
                .collect();
            FreeMibTable(table.cast());
            let route_mode = detect_windows_route_mode(&interfaces);

            Ok(RawNetworkSample {
                interfaces,
                proxy: read_windows_proxy_state(),
                route_mode,
                warnings: Vec::new(),
            })
        }
    }
}

#[cfg(windows)]
fn utf16_buffer_to_string(buffer: &[u16]) -> String {
    let length = buffer
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..length])
}

#[cfg(windows)]
fn detect_windows_route_mode(interfaces: &[RawInterfaceCounters]) -> Option<super::dto::RouteMode> {
    use super::dto::RouteMode;
    use std::{collections::HashSet, ptr::null_mut, slice};
    use windows::Win32::NetworkManagement::IpHelper::{
        FreeMibTable, GetIpForwardTable2, MIB_IPFORWARD_TABLE2,
    };
    use windows::Win32::Networking::WinSock::AF_UNSPEC;

    let tunnel_indices: HashSet<_> = interfaces
        .iter()
        .filter(|interface| interface.kind == InterfaceKind::Tunnel)
        .map(|interface| interface.interface_index)
        .collect();
    if tunnel_indices.is_empty() {
        return Some(RouteMode::Direct);
    }

    let mut table: *mut MIB_IPFORWARD_TABLE2 = null_mut();
    unsafe {
        if GetIpForwardTable2(AF_UNSPEC, &mut table).0 != 0 || table.is_null() {
            return None;
        }
        let routes = slice::from_raw_parts((*table).Table.as_ptr(), (*table).NumEntries as usize);
        let mut tunnel_default = false;
        let mut physical_default = false;
        for route in routes
            .iter()
            .filter(|route| route.DestinationPrefix.PrefixLength == 0)
        {
            if tunnel_indices.contains(&route.InterfaceIndex) {
                tunnel_default = true;
            } else {
                physical_default = true;
            }
        }
        FreeMibTable(table.cast());
        Some(if tunnel_default && !physical_default {
            RouteMode::FullTunnel
        } else if tunnel_default {
            RouteMode::SplitTunnel
        } else {
            RouteMode::Direct
        })
    }
}

#[cfg(windows)]
fn read_windows_proxy_state() -> RawProxyState {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};

    let settings = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings");
    let Ok(settings) = settings else {
        return RawProxyState::default();
    };

    let proxy_enabled = settings.get_value::<u32, _>("ProxyEnable").unwrap_or(0) != 0;
    let proxy_server = settings
        .get_value::<String, _>("ProxyServer")
        .unwrap_or_default();
    let pac_enabled = settings
        .get_value::<String, _>("AutoConfigURL")
        .is_ok_and(|value| !value.trim().is_empty());
    summarize_windows_proxy(proxy_enabled, &proxy_server, pac_enabled)
}

#[cfg(any(windows, test))]
fn summarize_windows_proxy(
    proxy_enabled: bool,
    proxy_server: &str,
    pac_enabled: bool,
) -> RawProxyState {
    let lowered = proxy_server.to_ascii_lowercase();
    let mut kinds = Vec::new();
    for kind in ["http", "https", "socks", "ftp"] {
        if lowered.contains(&format!("{kind}="))
            || (proxy_enabled && !lowered.contains('=') && kind == "http")
        {
            kinds.push(kind.to_owned());
        }
    }

    RawProxyState {
        configured: proxy_enabled || pac_enabled,
        kinds,
        pac_enabled,
    }
}

#[cfg(any(windows, test))]
fn classify_windows_interface(interface_type: u32, name: &str) -> (InterfaceKind, bool, bool) {
    let lowered_name = name.to_ascii_lowercase();
    let is_tunnel = interface_type == 131
        || lowered_name.contains("vpn")
        || lowered_name.contains("wireguard")
        || lowered_name.contains("tun")
        || lowered_name.contains("tap");
    let is_virtual = is_tunnel
        || interface_type == 53
        || lowered_name.contains("virtual")
        || lowered_name.contains("hyper-v")
        || lowered_name.contains("wsl");
    let kind = match interface_type {
        _ if is_tunnel => InterfaceKind::Tunnel,
        6 => InterfaceKind::Ethernet,
        71 => InterfaceKind::Wifi,
        24 => InterfaceKind::Loopback,
        _ if is_virtual => InterfaceKind::Virtual,
        _ => InterfaceKind::Other,
    };
    (kind, is_virtual, is_tunnel)
}

#[cfg(target_os = "linux")]
#[derive(Default)]
struct HostPlatformCollector;

#[cfg(target_os = "linux")]
impl PlatformNetworkCollector for HostPlatformCollector {
    fn collect(&mut self) -> NetworkMonitorResult<RawNetworkSample> {
        use std::fs;

        let entries = fs::read_dir("/sys/class/net").map_err(|error| {
            NetworkMonitorError::new(
                NetworkMonitorErrorCode::CollectorUnavailable,
                format!("failed to enumerate /sys/class/net: {error}"),
            )
        })?;
        let mut interfaces = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = entry.path();
            let received_bytes = read_u64(path.join("statistics/rx_bytes"));
            let transmitted_bytes = read_u64(path.join("statistics/tx_bytes"));
            let index = read_string(path.join("ifindex"));
            let operstate = read_string(path.join("operstate"));
            let kind_hint = read_string(path.join("type"));
            let has_physical_device = fs::canonicalize(path.join("device"))
                .map(|value| !value.to_string_lossy().contains("/virtual/"))
                .unwrap_or(false);
            let (kind, is_virtual, is_tunnel) =
                classify_linux_interface(&name, &kind_hint, has_physical_device);
            interfaces.push(RawInterfaceCounters {
                stable_id: stable_id(&["linux", &index, &name, &kind_hint]),
                interface_index: index.parse().unwrap_or_default(),
                name,
                kind,
                state: match operstate.as_str() {
                    "up" => InterfaceState::Up,
                    "down" => InterfaceState::Down,
                    _ => InterfaceState::Unknown,
                },
                is_virtual,
                tunnel_type: is_tunnel.then(|| "linux-tunnel".to_owned()),
                received_bytes,
                transmitted_bytes,
            });
        }

        let route_mode = detect_linux_route_mode(&interfaces);
        Ok(RawNetworkSample {
            interfaces,
            proxy: RawProxyState::default(),
            route_mode,
            warnings: vec![NetworkMonitorWarning::new("proxyConfigurationUnavailable")],
        })
    }
}

#[cfg(target_os = "linux")]
fn read_string(path: impl AsRef<std::path::Path>) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .trim()
        .to_owned()
}

#[cfg(target_os = "linux")]
fn read_u64(path: impl AsRef<std::path::Path>) -> u64 {
    read_string(path).parse().unwrap_or(0)
}

#[cfg(any(target_os = "linux", test))]
fn classify_linux_interface(
    name: &str,
    kind_hint: &str,
    has_physical_device: bool,
) -> (InterfaceKind, bool, bool) {
    let is_loopback = name == "lo" || kind_hint == "772";
    let is_tunnel = name.starts_with("tun")
        || name.starts_with("tap")
        || name.starts_with("wg")
        || name.starts_with("ppp");
    let is_virtual = is_tunnel || !has_physical_device;
    let kind = if is_loopback {
        InterfaceKind::Loopback
    } else if is_tunnel {
        InterfaceKind::Tunnel
    } else if name.starts_with("wl") {
        InterfaceKind::Wifi
    } else if is_virtual {
        InterfaceKind::Virtual
    } else {
        InterfaceKind::Ethernet
    };
    (kind, is_virtual, is_tunnel)
}

#[cfg(target_os = "linux")]
fn detect_linux_route_mode(interfaces: &[RawInterfaceCounters]) -> Option<super::dto::RouteMode> {
    use super::dto::RouteMode;
    let routes = std::fs::read_to_string("/proc/net/route").ok()?;
    let default_interfaces: Vec<_> = routes
        .lines()
        .skip(1)
        .filter_map(|line| {
            let columns: Vec<_> = line.split_whitespace().collect();
            (columns.len() > 2 && columns[1] == "00000000").then(|| columns[0].to_owned())
        })
        .collect();
    let has_tunnel = interfaces
        .iter()
        .any(|interface| interface.kind == InterfaceKind::Tunnel);
    if !has_tunnel {
        return Some(RouteMode::Direct);
    }
    let tunnel_default = default_interfaces.iter().any(|name| {
        interfaces
            .iter()
            .any(|interface| interface.name == *name && interface.kind == InterfaceKind::Tunnel)
    });
    let physical_default = default_interfaces.iter().any(|name| {
        interfaces
            .iter()
            .any(|interface| interface.name == *name && interface.kind != InterfaceKind::Tunnel)
    });
    Some(if tunnel_default && !physical_default {
        RouteMode::FullTunnel
    } else if tunnel_default {
        RouteMode::SplitTunnel
    } else {
        RouteMode::Direct
    })
}

#[cfg(target_os = "macos")]
struct HostPlatformCollector {
    networks: sysinfo::Networks,
}

#[cfg(target_os = "macos")]
impl Default for HostPlatformCollector {
    fn default() -> Self {
        Self {
            networks: sysinfo::Networks::new_with_refreshed_list(),
        }
    }
}

#[cfg(target_os = "macos")]
impl PlatformNetworkCollector for HostPlatformCollector {
    fn collect(&mut self) -> NetworkMonitorResult<RawNetworkSample> {
        self.networks.refresh(true);
        let interfaces = self
            .networks
            .iter()
            .map(|(name, data)| {
                let interface_index = std::ffi::CString::new(name.as_str())
                    .ok()
                    .map(|name| unsafe { libc::if_nametoindex(name.as_ptr()) })
                    .unwrap_or_default();
                let (kind, is_virtual, is_tunnel) = classify_macos_interface(name);
                RawInterfaceCounters {
                    stable_id: stable_id(&["macos", &interface_index.to_string(), name]),
                    interface_index,
                    name: name.to_owned(),
                    kind,
                    state: InterfaceState::Unknown,
                    is_virtual,
                    tunnel_type: is_tunnel.then(|| "darwin-tunnel".to_owned()),
                    received_bytes: data.total_received(),
                    transmitted_bytes: data.total_transmitted(),
                }
            })
            .collect();
        Ok(RawNetworkSample {
            interfaces,
            proxy: RawProxyState::default(),
            route_mode: None,
            warnings: vec![
                NetworkMonitorWarning::new("proxyConfigurationUnavailable"),
                NetworkMonitorWarning::new("routeModeUnavailable"),
            ],
        })
    }
}

#[cfg(any(target_os = "macos", test))]
fn classify_macos_interface(name: &str) -> (InterfaceKind, bool, bool) {
    let is_loopback = name == "lo0";
    let is_tunnel =
        name.starts_with("utun") || name.starts_with("ppp") || name.starts_with("ipsec");
    let is_virtual = is_tunnel
        || name.starts_with("bridge")
        || name.starts_with("awdl")
        || name.starts_with("llw");
    let kind = if is_loopback {
        InterfaceKind::Loopback
    } else if is_tunnel {
        InterfaceKind::Tunnel
    } else if is_virtual {
        InterfaceKind::Virtual
    } else {
        InterfaceKind::Other
    };
    (kind, is_virtual, is_tunnel)
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
#[derive(Default)]
struct HostPlatformCollector;

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
impl PlatformNetworkCollector for HostPlatformCollector {
    fn collect(&mut self) -> NetworkMonitorResult<RawNetworkSample> {
        Err(NetworkMonitorError::new(
            NetworkMonitorErrorCode::UnsupportedPlatform,
            "network monitoring is not implemented on this platform",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        classify_linux_interface, classify_macos_interface, classify_windows_interface, stable_id,
        summarize_windows_proxy,
    };
    use crate::network_monitor::dto::InterfaceKind;

    #[test]
    fn stable_ids_are_deterministic_and_do_not_expose_source_metadata() {
        let first = stable_id(&["platform", "guid", "10"]);
        let second = stable_id(&["platform", "guid", "10"]);
        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
        assert!(!first.contains("guid"));
    }

    #[test]
    fn platform_metadata_fixtures_classify_tunnels_and_virtual_interfaces() {
        assert_eq!(
            classify_windows_interface(6, "WireGuard Tunnel"),
            (InterfaceKind::Tunnel, true, true)
        );
        assert_eq!(
            classify_linux_interface("wlp2s0", "1", true),
            (InterfaceKind::Wifi, false, false)
        );
        assert_eq!(
            classify_linux_interface("tun0", "65534", false),
            (InterfaceKind::Tunnel, true, true)
        );
        assert_eq!(
            classify_macos_interface("utun4"),
            (InterfaceKind::Tunnel, true, true)
        );
    }

    #[test]
    fn windows_proxy_fixture_keeps_only_type_and_state_summary() {
        let proxy = summarize_windows_proxy(
            true,
            "http=proxy.example:8080;https=secure.example:8443",
            true,
        );
        assert!(proxy.configured);
        assert!(proxy.pac_enabled);
        assert_eq!(proxy.kinds, ["http", "https"]);
    }

    #[cfg(windows)]
    #[test]
    fn windows_collector_reads_real_ip_helper_interfaces() {
        use super::{HostPlatformCollector, PlatformNetworkCollector};

        let mut collector = HostPlatformCollector;
        let sample = collector.collect().expect("GetIfTable2 should succeed");
        assert!(!sample.interfaces.is_empty());
        assert!(sample
            .interfaces
            .iter()
            .all(|interface| interface.stable_id.len() == 64));
    }
}
