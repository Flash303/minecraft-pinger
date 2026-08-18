use crate::config::{PingConfig, DEFAULT_PROTOCOL_VERSION, PingConfigBuilder};

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

impl JavaPingConfigBuilder {
    pub fn new() -> Self {
        JavaPingConfigBuilder {
            common: PingConfigBuilder::new(),
            protocol_version: DEFAULT_PROTOCOL_VERSION,
            hostname: None
        }
    }

    pub fn from(config: &PingConfigBuilder) -> Self {
        JavaPingConfigBuilder {
            common: config.clone(),
            protocol_version: DEFAULT_PROTOCOL_VERSION,
            hostname: None
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

    pub fn build(self) -> JavaPingConfig {
        JavaPingConfig {
            common: self.common.build(),
            hostname: self.hostname,
            protocol_version: self.protocol_version
        }
    }
}