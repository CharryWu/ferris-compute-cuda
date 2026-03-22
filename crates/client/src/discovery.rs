use mdns_sd::{ServiceDaemon, ServiceEvent};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

const MDNS_SERVICE_TYPE: &str = "_ferris-compute._tcp.local.";

#[derive(Debug, Clone)]
pub struct DiscoveredHost {
    pub address: String,
    pub hostname: String,
    pub port: u16,
}

impl DiscoveredHost {
    pub fn url(&self) -> String {
        format!("http://{}:{}", self.address, self.port)
    }
}

/// Scans the local network for ferris-compute hosts using mDNS.
/// Blocks for up to `timeout_secs` seconds, then returns all discovered hosts.
pub fn discover_hosts(timeout_secs: u64) -> Vec<DiscoveredHost> {
    let mut hosts = discover_hosts_for_type(MDNS_SERVICE_TYPE, timeout_secs);
    if hosts.is_empty() {
        if let Some(local) = detect_local_host(50051) {
            hosts.push(local);
        }
    }
    hosts
}

fn detect_local_host(port: u16) -> Option<DiscoveredHost> {
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().ok()?;
    let timeout = Duration::from_millis(250);
    if TcpStream::connect_timeout(&addr, timeout).is_ok() {
        Some(DiscoveredHost {
            address: "127.0.0.1".to_string(),
            hostname: "localhost".to_string(),
            port,
        })
    } else {
        None
    }
}

fn discover_hosts_for_type(service_type: &str, timeout_secs: u64) -> Vec<DiscoveredHost> {
    let mdns = match ServiceDaemon::new() {
        Ok(d) => d,
        Err(_) => return vec![],
    };

    let receiver = match mdns.browse(service_type) {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    let mut hosts = Vec::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(timeout_secs);

    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            break;
        }

        match receiver.recv_timeout(remaining) {
            Ok(ServiceEvent::ServiceResolved(info)) => {
                let addrs = info.get_addresses();
                if let Some(addr) = addrs.iter().next() {
                    let hostname = info.get_hostname().trim_end_matches('.').to_string();
                    let host = DiscoveredHost {
                        address: addr.to_string(),
                        hostname,
                        port: info.get_port(),
                    };
                    if !hosts.iter().any(|h: &DiscoveredHost| h.url() == host.url()) {
                        hosts.push(host);
                    }
                }
            }
            Err(_) => break,
            _ => {}
        }
    }

    let _ = mdns.shutdown();
    hosts
}
