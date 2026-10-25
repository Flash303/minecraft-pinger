use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use crate::common::ip_filter::{IpFilter, is_public_ip};

pub const DEFAULT_PROTOCOL_VERSION: i32 = 777;
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(1);

/// Common ping settings shared by the Java and Bedrock pingers.
///
/// Build with [`PingConfig::builder`]. Cheap to clone (the IP filter,
/// if any, is behind an `Arc`).
#[derive(Clone)]
pub struct PingConfig {
    timeout: Duration,
    ip_filter: Option<Arc<IpFilter>>,
}

impl PingConfig {
    pub fn builder() -> PingConfigBuilder {
        PingConfigBuilder::new()
    }

    /// Maximum time budget for a whole ping (DNS + connect + I/O).
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// The IP filter applied to resolved addresses, if any.
    /// A ping is only attempted when at least one address is accepted.
    pub fn ip_filter(&self) -> Option<&Arc<IpFilter>> {
        self.ip_filter.as_ref()
    }
}

impl Default for PingConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

/// Builder for [`PingConfig`].
#[derive(Clone)]
pub struct PingConfigBuilder {
    pub(crate) timeout: Duration,
    pub(crate) ip_filter: Option<Arc<IpFilter>>,
}

impl Default for PingConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PingConfigBuilder {
    pub fn new() -> PingConfigBuilder {
        PingConfigBuilder {
            timeout: DEFAULT_TIMEOUT,
            ip_filter: None,
        }
    }

    /// Sets the whole-ping time budget (default: 1 s).
    pub fn set_timeout(mut self, timeout: Duration) -> PingConfigBuilder {
        self.timeout = timeout;
        self
    }

    /// Rejects non-public IPs (loopback, private, link-local, …).
    /// This prevents SSRF: resolved addresses pointing at internal
    /// infrastructure are refused before any connection is made.
    pub fn deny_non_public_ips(mut self) -> PingConfigBuilder {
        self.ip_filter = Some(Arc::new(is_public_ip));
        self
    }

    /// Accepts a custom predicate over resolved IPs.
    /// Returning `false` rejects the address; when every address is
    /// rejected the ping fails with [`PingError::BlockedEndpoint`](crate::error::PingError::BlockedEndpoint).
    pub fn set_ip_filter<F>(mut self, filter: F) -> PingConfigBuilder
    where
        F: Fn(IpAddr) -> bool + Send + Sync + 'static,
    {
        self.ip_filter = Some(Arc::new(filter));
        self
    }

    pub fn build(self) -> PingConfig {
        PingConfig {
            timeout: self.timeout,
            ip_filter: self.ip_filter,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};
    use std::time::Duration;

    #[test]
    fn test_ping_config_builder_default() {
        let config = PingConfigBuilder::new().build();
        assert_eq!(config.timeout(), DEFAULT_TIMEOUT);
        assert!(config.ip_filter().is_none());
    }

    #[test]
    fn test_ping_config_builder_custom_timeout() {
        let config = PingConfigBuilder::new()
            .set_timeout(Duration::from_secs(5))
            .build();
        assert_eq!(config.timeout(), Duration::from_secs(5));
    }

    #[test]
    fn test_ping_config_builder_deny_non_public_ips() {
        let config = PingConfigBuilder::new().deny_non_public_ips().build();
        assert!(config.ip_filter().is_some());

        let filter = config.ip_filter().unwrap();
        assert!(filter(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
        assert!(!filter(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
    }

    #[test]
    fn test_ping_config_builder_custom_ip_filter() {
        let config = PingConfigBuilder::new()
            .set_ip_filter(|ip| matches!(ip, IpAddr::V4(_)))
            .build();
        assert!(config.ip_filter().is_some());

        let filter = config.ip_filter().unwrap();
        assert!(filter(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
        assert!(!filter(IpAddr::V6(std::net::Ipv6Addr::new(
            0, 0, 0, 0, 0, 0, 0, 1
        ))));
    }

    #[test]
    fn test_ping_config_builder_chaining() {
        let config = PingConfigBuilder::new()
            .set_timeout(Duration::from_millis(500))
            .deny_non_public_ips()
            .build();
        assert_eq!(config.timeout(), Duration::from_millis(500));
        assert!(config.ip_filter().is_some());
    }

    #[test]
    fn test_ping_config_default() {
        let config = PingConfig::default();
        assert_eq!(config.timeout(), DEFAULT_TIMEOUT);
        assert!(config.ip_filter().is_none());
    }
}
