use std::time::Duration;

pub const DEFAULT_PROTOCOL_VERSION: i32 = 763;
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(1);

pub struct PingConfig {
    timeout: Duration,
    java_config: JavaPingConfig,
}

impl PingConfig {
    pub fn builder() -> PingConfigBuilder {
        PingConfigBuilder::new()
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

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
    java_config: JavaPingConfig,
}

impl PingConfigBuilder {
    pub fn new() -> PingConfigBuilder {
        PingConfigBuilder {
            timeout: DEFAULT_TIMEOUT,
            java_config: JavaPingConfig::default()
        }
    }
    
    pub fn set_timeout(mut self, timeout: Duration) -> PingConfigBuilder {
        self.timeout = timeout;
        self
    }

    pub fn set_java_config(mut self, java_config: JavaPingConfig) -> PingConfigBuilder{
        self.java_config = java_config;
        self
    }

    pub fn build(self) -> PingConfig {
        PingConfig {
            timeout: self.timeout,
            java_config: self.java_config
        }
    }
}

// Java config
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