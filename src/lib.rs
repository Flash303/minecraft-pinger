pub mod error;
pub mod config;
pub mod common;

#[cfg(feature = "java")]
pub mod java;
#[cfg(feature = "bedrock")]
pub mod bedrock;

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
            TokioRuntimeProvider::default()
        )
            .build()
            .map_err(|e| PingError::Init(e.to_string()))?;

        Ok(Self {
            dns_resolver: Arc::new(resolver)
        })
    }

    #[deprecated(note="Some problems with default DNS of OVH for some servers.")]
    pub fn new_legacy() -> Result<Self, PingError> {
        let builder = Resolver::builder_tokio()
            .map_err(|e| PingError::Init(e.to_string()))?;
        let resolver = builder.build()
            .map_err(|e| PingError::Init(e.to_string()))?;

        Ok(Self {
            dns_resolver: Arc::new(resolver)
        })
    }
}
