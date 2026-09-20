pub mod common;
pub mod config;
pub mod error;

#[cfg(feature = "bedrock")]
pub mod bedrock;
#[cfg(feature = "java")]
pub mod java;

// Require at least one feature to be enabled
#[cfg(not(any(feature = "java", feature = "bedrock")))]
compile_error!("At least one of the 'java' or 'bedrock' features must be enabled");

use error::PingError;
use hickory_resolver::Resolver;
use hickory_resolver::config::{CLOUDFLARE, ResolverConfig};
use hickory_resolver::net::runtime::TokioRuntimeProvider;
use std::sync::Arc;

pub struct MinecraftPinger {
    dns_resolver: Arc<Resolver<TokioRuntimeProvider>>,
}

impl MinecraftPinger {
    pub fn new() -> Result<Self, PingError> {
        let resolver = Resolver::builder_with_config(
            ResolverConfig::udp_and_tcp(&CLOUDFLARE),
            TokioRuntimeProvider::default(),
        )
        .build()
        .map_err(|e| PingError::Init(e.to_string()))?;

        Ok(Self {
            dns_resolver: Arc::new(resolver),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minecraft_pinger_new() {
        let pinger = MinecraftPinger::new();
        assert!(pinger.is_ok());
    }

    #[test]
    fn test_minecraft_pinger_new_multiple() {
        let pinger1 = MinecraftPinger::new();
        let pinger2 = MinecraftPinger::new();
        assert!(pinger1.is_ok());
        assert!(pinger2.is_ok());
    }
}
