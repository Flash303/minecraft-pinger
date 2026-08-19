use std::time::Instant;
use bytes::Bytes;
use log::debug;
use tokio::net::UdpSocket;
use tokio::time::timeout;
use crate::bedrock::model::BedrockPing;
use crate::bedrock::protocol::{create_ping, read_response};
use crate::common::dns::resolve_to_addrs;
use crate::config::PingConfig;
use crate::error::PingError;
use crate::MinecraftPinger;

impl MinecraftPinger {
    pub async fn ping_bedrock_server(self: &Self,
                                     ip: &str,
                                     port: u16,
                                     config: &PingConfig) -> Result<BedrockPing, PingError> {
        let rs = timeout(config.timeout(), self.ping_bedrock_server_internal(ip, port, config)).await??;
        Ok(rs)
    }

    async fn ping_bedrock_server_internal(self: &Self, ip: &str, port: u16, config: &PingConfig) -> Result<BedrockPing, PingError> {
        debug!("Pinging bedrock server {}:{}", ip, port);

        let addrs = resolve_to_addrs(self, ip, port, "udp").await?;
        let start_time = Instant::now();

        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        timeout(config.timeout(), socket.connect(&addrs[..]))
            .await?
            .map_err(|e| {
                debug!("Connection error: {}", e);
                PingError::ConnectionRefused
            })?;

        let _ = socket.send(&create_ping()?).await;

        let mut buffer = [0u8; 1024];
        let len = timeout(config.timeout(), socket.recv(&mut buffer))
            .await?
            .map_err(|_| PingError::ConnectionRefused)?;

        let mut response_bytes = Bytes::copy_from_slice(&buffer[..len]);
        let latency = start_time.elapsed().as_millis() as u32;

        let mut rs = read_response(&mut response_bytes)?;
        rs.latency = latency;

        Ok(rs)
    }
}

