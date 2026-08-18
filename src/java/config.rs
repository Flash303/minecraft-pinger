use crate::config::DEFAULT_PROTOCOL_VERSION;

pub struct JavaPingConfig {
    protocol_version: i32,
    hostname: Option<String>,
}

impl JavaPingConfig {
    pub fn builder() -> JavaPingConfigBuilder {
        JavaPingConfigBuilder::new()
    }

    pub fn protocol_version(&self) -> i32 {
        self.protocol_version
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
    protocol_version: i32,
    hostname: Option<String>,
}

impl JavaPingConfigBuilder {
    pub fn new() -> Self {
        JavaPingConfigBuilder {
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
            hostname: self.hostname,
            protocol_version: self.protocol_version
        }
    }
}