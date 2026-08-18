use std::time::Duration;

#[cfg(feature = "java")]
use crate::java::config::JavaPingConfig;

pub const DEFAULT_PROTOCOL_VERSION: i32 = 775;
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(1);

pub struct PingConfig {
    timeout: Duration,
    #[cfg(feature = "java")]
    java_config: JavaPingConfig,
}

impl PingConfig {
    pub fn builder() -> PingConfigBuilder {
        PingConfigBuilder::new()
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    #[cfg(feature = "java")]
    pub fn java_config(&self) -> &JavaPingConfig {
        &self.java_config
    }
}

impl Default for PingConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

pub struct PingConfigBuilder {
    timeout: Duration,
    #[cfg(feature = "java")]
    java_config: JavaPingConfig,
}

impl PingConfigBuilder {
    pub fn new() -> PingConfigBuilder {
        PingConfigBuilder {
            timeout: DEFAULT_TIMEOUT,
            #[cfg(feature = "java")]
            java_config: JavaPingConfig::default()
        }
    }

    pub fn set_timeout(mut self, timeout: Duration) -> PingConfigBuilder {
        self.timeout = timeout;
        self
    }

    #[cfg(feature = "java")]
    pub fn set_java_config(mut self, java_config: JavaPingConfig) -> PingConfigBuilder{
        self.java_config = java_config;
        self
    }

    pub fn build(self) -> PingConfig {
        PingConfig {
            timeout: self.timeout,
            #[cfg(feature = "java")]
            java_config: self.java_config
        }
    }
}