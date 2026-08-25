use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use crate::common::ip_filter::{IpFilter, is_public_ip};

pub const DEFAULT_PROTOCOL_VERSION: i32 = 775;
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Clone)]
pub struct PingConfig {
    timeout: Duration,
    ip_filter: Option<Arc<IpFilter>>,
}

impl PingConfig {
    pub fn builder() -> PingConfigBuilder {
        PingConfigBuilder::new()
    }

    // Remove it ?
    pub fn to_builder(&self) -> PingConfigBuilder {
        PingConfigBuilder {
            timeout: self.timeout,
            ip_filter: self.ip_filter.clone(),
        }
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }
    
    pub fn ip_filter(&self) -> Option<&Arc<IpFilter>> {
        self.ip_filter.as_ref()
    }
}

impl Default for PingConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

#[derive(Clone)]
pub struct PingConfigBuilder {
    timeout: Duration,
    ip_filter: Option<Arc<IpFilter>>,
}

impl PingConfigBuilder {
    pub fn new() -> PingConfigBuilder {
        PingConfigBuilder {
            timeout: DEFAULT_TIMEOUT,
            ip_filter: None,
        }
    }

    pub fn set_timeout(mut self, timeout: Duration) -> PingConfigBuilder {
        self.timeout = timeout;
        self
    }
    
    pub fn deny_non_public_ips(mut self) -> PingConfigBuilder {
        self.ip_filter = Some(Arc::new(is_public_ip));
        self
    }
    
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
