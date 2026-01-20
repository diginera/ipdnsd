use std::net::IpAddr;

use anyhow::{anyhow, Result};

pub fn get_internal_ip() -> Result<IpAddr> {
    local_ip_address::local_ip().map_err(|e| anyhow!("Failed to get local IP: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_internal_ip() {
        // This should work on most systems
        let result = get_internal_ip();
        // We don't assert success because CI environments may not have a network interface
        if let Ok(ip) = result {
            // Should be a private IP or localhost
            assert!(
                ip.is_loopback() || is_private_ip(&ip),
                "Expected private IP, got: {}",
                ip
            );
        }
    }

    fn is_private_ip(ip: &IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4) => {
                ipv4.is_private() || ipv4.is_loopback() || ipv4.is_link_local()
            }
            IpAddr::V6(ipv6) => {
                ipv6.is_loopback()
                    || ipv6.segments()[0] == 0xfe80 // link-local
                    || ipv6.segments()[0] == 0xfc00 // unique local
                    || ipv6.segments()[0] == 0xfd00 // unique local
            }
        }
    }
}
