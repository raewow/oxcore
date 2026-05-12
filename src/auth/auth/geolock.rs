use std::net::IpAddr;

/// Check if an IP address is allowed based on geolocking rules
/// This is a simplified implementation - a full implementation would use
/// a geolocation database (like MaxMind GeoIP) to determine country
pub struct Geolock {
    enabled: bool,
    // In a full implementation, this would contain geolocation data
}

impl Geolock {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Check if IP is allowed for the account
    /// account_last_ip: Last known IP for the account (from database)
    /// current_ip: Current connection IP
    pub fn is_ip_allowed(&self, account_last_ip: Option<&str>, current_ip: &IpAddr) -> bool {
        if !self.enabled {
            return true; // Geolocking disabled
        }

        // If account has no previous IP, allow first connection
        let Some(last_ip_str) = account_last_ip else {
            return true;
        };

        // Parse last IP
        let Ok(last_ip) = last_ip_str.parse::<IpAddr>() else {
            // If we can't parse, allow connection (could be IPv6 or invalid format)
            return true;
        };

        // For now, simple check: same IP or same subnet (first 3 octets for IPv4)
        // A full implementation would check country/region
        match (last_ip, current_ip) {
            (IpAddr::V4(last_v4), IpAddr::V4(current_v4)) => {
                let last_octets = last_v4.octets();
                let current_octets = current_v4.octets();

                // Allow if same IP
                if last_v4 == *current_v4 {
                    return true;
                }

                // Allow if same subnet (first 3 octets match)
                // This is a simplified check - real geolocking would use country/region
                last_octets[0] == current_octets[0]
                    && last_octets[1] == current_octets[1]
                    && last_octets[2] == current_octets[2]
            }
            (IpAddr::V6(last_v6), IpAddr::V6(current_v6)) => {
                // For IPv6, check if same /64 subnet
                // Simplified: just check if first 64 bits match
                let last_bytes = last_v6.octets();
                let current_bytes = current_v6.octets();
                last_bytes[..8] == current_bytes[..8]
            }
            _ => {
                // Mixed IPv4/IPv6 - be permissive
                true
            }
        }
    }

    /// Get country code for an IP (placeholder - would use GeoIP database)
    #[allow(dead_code)]
    pub fn get_country_code(&self, _ip: &IpAddr) -> Option<String> {
        // In a full implementation, this would query a GeoIP database
        // For now, return None (unknown)
        None
    }

    /// Check if country is allowed for account
    #[allow(dead_code)]
    pub fn is_country_allowed(
        &self,
        _account_country: Option<&str>,
        _current_country: &str,
    ) -> bool {
        // In a full implementation, this would check if the country matches
        // or is in an allowed list
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geolock_disabled() {
        let geolock = Geolock::new(false);
        let ip: IpAddr = "192.168.1.1".parse().unwrap();
        assert!(geolock.is_ip_allowed(None, &ip));
        assert!(geolock.is_ip_allowed(Some("10.0.0.1"), &ip));
    }

    #[test]
    fn test_geolock_same_ip() {
        let geolock = Geolock::new(true);
        let ip: IpAddr = "192.168.1.1".parse().unwrap();
        assert!(geolock.is_ip_allowed(Some("192.168.1.1"), &ip));
    }

    #[test]
    fn test_geolock_same_subnet() {
        let geolock = Geolock::new(true);
        let ip: IpAddr = "192.168.1.100".parse().unwrap();
        // Same subnet (192.168.1.x) should be allowed
        assert!(geolock.is_ip_allowed(Some("192.168.1.1"), &ip));
        // Different subnet should be denied
        assert!(!geolock.is_ip_allowed(Some("192.168.2.1"), &ip));
    }

    #[test]
    fn test_geolock_no_previous_ip() {
        let geolock = Geolock::new(true);
        let ip: IpAddr = "192.168.1.1".parse().unwrap();
        // First connection should be allowed
        assert!(geolock.is_ip_allowed(None, &ip));
    }
}
