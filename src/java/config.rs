use crate::config::{DEFAULT_PROTOCOL_VERSION, PingConfig, PingConfigBuilder};
use std::time::Duration;

#[derive(Clone)]
pub struct JavaPingConfig {
    common: PingConfig,
    protocol_version: i32,
    hostname: Option<String>,
}

impl JavaPingConfig {
    pub fn builder() -> JavaPingConfigBuilder {
        JavaPingConfigBuilder::new()
    }

    pub fn from(config: &PingConfigBuilder) -> JavaPingConfigBuilder {
        JavaPingConfigBuilder::from(config)
    }

    pub fn protocol_version(&self) -> i32 {
        self.protocol_version
    }

    pub fn common(&self) -> &PingConfig {
        &self.common
    }

    pub fn hostname(&self) -> &Option<String> {
        &self.hostname
    }
}

impl Default for JavaPingConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

pub struct JavaPingConfigBuilder {
    common: PingConfigBuilder,
    protocol_version: i32,
    hostname: Option<String>,
}

impl Default for JavaPingConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl JavaPingConfigBuilder {
    pub fn new() -> Self {
        JavaPingConfigBuilder {
            common: PingConfigBuilder::new(),
            protocol_version: DEFAULT_PROTOCOL_VERSION,
            hostname: None,
        }
    }

    pub fn from(config: &PingConfigBuilder) -> Self {
        JavaPingConfigBuilder {
            common: config.clone(),
            protocol_version: DEFAULT_PROTOCOL_VERSION,
            hostname: None,
        }
    }

    pub fn set_protocol_version(mut self, protocol_version: i32) -> Self {
        self.protocol_version = protocol_version;
        self
    }

    pub fn set_hostname(mut self, hostname: Option<String>) -> Self {
        self.hostname = hostname;
        self
    }

    pub fn set_timeout(mut self, timeout: Duration) -> Self {
        self.common = self.common.set_timeout(timeout);
        self
    }

    pub fn deny_non_public_ips(mut self) -> Self {
        self.common = self.common.deny_non_public_ips();
        self
    }

    pub fn build(self) -> JavaPingConfig {
        JavaPingConfig {
            common: self.common.build(),
            hostname: self.hostname,
            protocol_version: self.protocol_version,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_java_ping_config_builder_default() {
        let config = JavaPingConfigBuilder::new().build();
        assert_eq!(config.protocol_version(), DEFAULT_PROTOCOL_VERSION);
        assert_eq!(config.common().timeout(), Duration::from_secs(1));
        assert!(config.hostname().is_none());
    }

    #[test]
    fn test_java_ping_config_builder_custom_protocol() {
        let config = JavaPingConfigBuilder::new()
            .set_protocol_version(765)
            .build();
        assert_eq!(config.protocol_version(), 765);
    }

    #[test]
    fn test_java_ping_config_builder_custom_hostname() {
        let config = JavaPingConfigBuilder::new()
            .set_hostname(Some("custom.example.com".to_string()))
            .build();
        assert_eq!(config.hostname(), &Some("custom.example.com".to_string()));
    }

    #[test]
    fn test_java_ping_config_builder_custom_timeout() {
        let config = JavaPingConfigBuilder::new()
            .set_timeout(Duration::from_secs(5))
            .build();
        assert_eq!(config.common().timeout(), Duration::from_secs(5));
    }

    #[test]
    fn test_java_ping_config_builder_deny_non_public_ips() {
        let config = JavaPingConfigBuilder::new().deny_non_public_ips().build();
        assert!(config.common().ip_filter().is_some());
    }

    #[test]
    fn test_java_ping_config_builder_from_ping_config_builder() {
        let base = PingConfigBuilder::new()
            .set_timeout(Duration::from_secs(10))
            .deny_non_public_ips();
        let config = JavaPingConfigBuilder::from(&base).build();
        assert_eq!(config.common().timeout(), Duration::from_secs(10));
        assert!(config.common().ip_filter().is_some());
    }

    #[test]
    fn test_java_ping_config_builder_chaining() {
        let config = JavaPingConfigBuilder::new()
            .set_protocol_version(766)
            .set_hostname(Some("test.com".to_string()))
            .set_timeout(Duration::from_millis(500))
            .deny_non_public_ips()
            .build();
        assert_eq!(config.protocol_version(), 766);
        assert_eq!(config.hostname(), &Some("test.com".to_string()));
        assert_eq!(config.common().timeout(), Duration::from_millis(500));
        assert!(config.common().ip_filter().is_some());
    }

    #[test]
    fn test_java_ping_config_default() {
        let config = JavaPingConfig::default();
        assert_eq!(config.protocol_version(), DEFAULT_PROTOCOL_VERSION);
        assert_eq!(config.common().timeout(), Duration::from_secs(1));
    }
}
