use mdns_sd::{ServiceDaemon, ServiceEvent};
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
    let mdns = match ServiceDaemon::new() {
        Ok(d) => d,
        Err(_) => return vec![],
    };

    let receiver = match mdns.browse(MDNS_SERVICE_TYPE) {
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
