//! IP address detection utilities
//!
//! Provides functionality to discover local network IP addresses
//! for ICE candidate gathering.

use std::net::Ipv4Addr;

/// Detects the local IP address for LAN connections.
///
/// Uses platform-specific methods to discover the actual network IP:
/// 1. Linux: 'ip addr' or 'hostname -I' commands
/// 2. macOS/BSD: 'ifconfig' command
/// 3. Windows: 'ipconfig' command
/// 4. Falls back to 0.0.0.0 if all methods fail
///
/// Prioritizes private network ranges (192.168.x.x, 10.x.x.x, 172.16-31.x.x)
/// and avoids loopback (127.x.x.x) and link-local (169.254.x.x) addresses.
///
/// # Returns
/// The local IP address as a string (e.g., "192.168.1.100")
pub fn detect_local_ip() -> String {
    #[cfg(target_family = "unix")]
    {
        if let Some(ip) = detect_local_ip_unix() {
            return ip;
        }
    }

    #[cfg(target_family = "windows")]
    {
        if let Some(ip) = detect_local_ip_windows() {
            return ip;
        }
    }

    "0.0.0.0".to_string()
}

/// Detects local IP on Unix/Linux systems
#[cfg(target_family = "unix")]
fn detect_local_ip_unix() -> Option<String> {
    use std::process::Command;

    let mut candidate_ips = Vec::new();

    if let Ok(output) = Command::new("ip").arg("addr").output()
        && output.status.success()
        && let Ok(result) = String::from_utf8(output.stdout)
    {
        for line in result.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("inet ") && !trimmed.starts_with("inet6") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 {
                    let ip_part = parts[1].split('/').next().unwrap_or("");
                    if let Ok(ip) = ip_part.parse::<Ipv4Addr>()
                        && is_valid_lan_ip(&ip)
                    {
                        candidate_ips.push(ip);
                    }
                }
            }
        }
    }

    // Try using 'hostname -I' command (Linux)
    if candidate_ips.is_empty()
        && let Ok(output) = Command::new("hostname").arg("-I").output()
        && output.status.success()
        && let Ok(result) = String::from_utf8(output.stdout)
    {
        for ip_str in result.split_whitespace() {
            if let Ok(ip) = ip_str.parse::<Ipv4Addr>()
                && is_valid_lan_ip(&ip)
            {
                candidate_ips.push(ip);
            }
        }
    }

    // Try using 'ifconfig' command
    if candidate_ips.is_empty()
        && let Ok(output) = Command::new("ifconfig").output()
        && output.status.success()
        && let Ok(result) = String::from_utf8(output.stdout)
    {
        for line in result.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("inet ") && !trimmed.starts_with("inet6") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2
                    && let Ok(ip) = parts[1].parse::<Ipv4Addr>()
                    && is_valid_lan_ip(&ip)
                {
                    candidate_ips.push(ip);
                }
            }
        }
    }

    select_best_ip(&candidate_ips)
}

/// Detects local IP on Windows systems
#[cfg(target_family = "windows")]
fn detect_local_ip_windows() -> Option<String> {
    use std::process::Command;

    let mut candidate_ips = Vec::new();

    // Try using 'ipconfig' command
    if let Ok(output) = Command::new("ipconfig").output() {
        if output.status.success() {
            if let Ok(result) = String::from_utf8(output.stdout) {
                for line in result.lines() {
                    if line.contains("IPv4 Address") || line.contains("IPv4 address") {
                        if let Some(colon_pos) = line.rfind(':') {
                            let ip_str = line[colon_pos + 1..].trim();
                            if let Ok(ip) = ip_str.parse::<Ipv4Addr>() {
                                if is_valid_lan_ip(&ip) {
                                    candidate_ips.push(ip);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Return the best candidate
    select_best_ip(&candidate_ips)
}

/// Checks if an IP is valid for LAN connections
fn is_valid_lan_ip(ip: &Ipv4Addr) -> bool {
    let octets = ip.octets();

    if ip.is_loopback() {
        return false;
    }

    if octets[0] == 169 && octets[1] == 254 {
        return false;
    }

    if octets == [0, 0, 0, 0] {
        return false;
    }

    true
}

/// Selects the best IP from a list of candidates
/// Prioritizes private network ranges (192.168.x.x, 10.x.x.x, 172.16-31.x.x)
fn select_best_ip(candidates: &[Ipv4Addr]) -> Option<String> {
    if candidates.is_empty() {
        return None;
    }

    for ip in candidates {
        let octets = ip.octets();
        if octets[0] == 192 && octets[1] == 168 {
            return Some(ip.to_string());
        }
    }

    for ip in candidates {
        let octets = ip.octets();
        if octets[0] == 10 {
            return Some(ip.to_string());
        }
    }

    for ip in candidates {
        let octets = ip.octets();
        if octets[0] == 172 && (octets[1] >= 16 && octets[1] <= 31) {
            return Some(ip.to_string());
        }
    }

    Some(candidates[0].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_local_ip() {
        let ip = detect_local_ip();

        assert!(ip.parse::<Ipv4Addr>().is_ok() || ip == "0.0.0.0");

        if ip != "0.0.0.0" {
            assert_ne!(ip, "127.0.0.1");
        }
    }
}
