pub mod models;
pub mod utils;
pub mod error;

use std::sync::Arc;
use crate::models::model::PingResponse;
use error::PingError;
use crate::utils::protocol::{create_ping_handshake, create_ping_request, read_packet, read_string};
use std::time::Duration;
use bytes::{BufMut, BytesMut};
use hickory_resolver::net::runtime::TokioRuntimeProvider;
use hickory_resolver::Resolver;
use log::debug;
use tokio::io::{AsyncWriteExt, BufReader};
use crate::utils::dns::{resolve_to_addr};
use tokio::net::{TcpStream};
use tokio::time::timeout;

pub struct PingConfig {
    pub protocol_version: i32,
    pub timeout: Duration,
    pub hostname: Option<String>,
}

impl Default for PingConfig {
    fn default() -> Self {
        Self {
            protocol_version: 763,
            timeout: Duration::from_secs(5),
            hostname: None,
        }
    }
}

pub struct MinecraftPinger {
    dns_resolver: Arc<Resolver<TokioRuntimeProvider>>,
}

impl MinecraftPinger {
    pub fn new() -> Result<Self, String> {
        let result = Resolver::builder_tokio();
        if let Err(_) = result {
            return Err(String::from("Failed to create resolver builder"));
        }

        let builder = result.unwrap();
        let result = builder.build();
        if let Err(_) = result {
            return Err(String::from("Failed to create resolver"));
        }

        Ok(Self {
            dns_resolver: Arc::new(result.unwrap())
        })
    }

    pub async fn ping_server(self: &Self, ip: &str, port: u16, config: PingConfig) -> Result<PingResponse, PingError> {
        match timeout(config.timeout, self.ping_server_internal(ip, port, &config)).await {
            Ok(result) => result,
            Err(_) => {
                debug!("Global ping timeout for {}:{}", ip, port);
                Err(PingError::TimeoutError)
            }
        }
    }

    async fn ping_server_internal(self: &Self, ip: &str, port: u16, config: &PingConfig) -> Result<PingResponse, PingError> {
        debug!("Pinging server {}:{}", ip, port);

        let addr = resolve_to_addr(self, ip, port).await?;

        let stream_future = TcpStream::connect(addr);
        let mut stream = timeout(Duration::from_secs(1), stream_future)
            .await
            .map_err(|_| {
                debug!("Connection timeout error");
                PingError::ConnectionRefused
            })?
            .map_err(|e| {
                debug!("Connection error: {}", e);
                PingError::ConnectionRefused
            })?;

        stream.set_nodelay(true).unwrap_or_default();

        debug!("Stream connected to {}", addr);

        let mut merged_packets = BytesMut::new();
        merged_packets.put(create_ping_handshake(&config.hostname.as_deref().unwrap_or(ip).to_string(), &port, &config.protocol_version));
        merged_packets.put(create_ping_request());

        stream.write_all(&merged_packets.freeze())
            .await
            .map_err(|_| PingError::SendPacketError)?;
        debug!("Stream all packets !");

        let mut buffered_reader = BufReader::new(&mut stream);
        let mut packet = read_packet(&mut buffered_reader).await?;
        debug!("Received Packet ID: {}", packet.id());

        let json = read_string(&mut packet.data)?;

        let as_res = serde_json::from_str::<PingResponse>(&json)
            .map_err(|e| {
                debug!("Error deserializing ping response: {}, json {}", e, json);
                PingError::SerializationError
            })?;
        Ok(as_res)
    }
}