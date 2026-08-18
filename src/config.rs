use std::time::Duration;

pub const DEFAULT_PROTOCOL_VERSION: i32 = 775;
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Clone)]
pub struct PingConfig {
    timeout: Duration,
}

impl PingConfig {
    pub fn builder() -> PingConfigBuilder {
        PingConfigBuilder::new()
    }

    // Remove it ?
    pub fn to_builder(&self) -> PingConfigBuilder {
        PingConfigBuilder {
            timeout: self.timeout
        }
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

impl Default for PingConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

#[derive(Clone)]
pub struct PingConfigBuilder {
    timeout: Duration
}

impl PingConfigBuilder {
    pub fn new() -> PingConfigBuilder {
        PingConfigBuilder {
            timeout: DEFAULT_TIMEOUT,
        }
    }

    pub fn set_timeout(mut self, timeout: Duration) -> PingConfigBuilder {
        self.timeout = timeout;
        self
    }

    pub fn build(self) -> PingConfig {
        PingConfig {
            timeout: self.timeout,
        }
    }
}