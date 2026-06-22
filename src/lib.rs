pub mod models;
pub mod utils;
pub mod error;
pub mod config;

use std::sync::Arc;
use error::PingError;
use crate::utils::protocol::{read_string};
use std::time::Duration;
use bytes::{Bytes, BytesMut};
use hickory_resolver::net::runtime::TokioRuntimeProvider;
use hickory_resolver::Resolver;
use log::debug;
use tokio::io::{AsyncWriteExt, BufReader};
use crate::utils::dns::{resolve_to_addr};
use tokio::net::{TcpStream, UdpSocket};
use tokio::time::timeout;
use crate::config::PingConfig;
use crate::models::bedrock_model::BedrockPing;
use crate::models::java_model::JavaPing;
use crate::utils::bedrock_protocol::{create_ping, read_response};
use crate::utils::java_protocol::{read_packet, write_ping_handshake, write_ping_request};

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

    pub async fn ping_java_server(self: &Self, ip: &str, port: u16, config: &PingConfig) -> Result<JavaPing, PingError> {
        match timeout(config.timeout(), self.ping_java_server_internal(ip, port, &config)).await {
            Ok(result) => result,
            Err(_) => {
                debug!("Global ping timeout for {}:{}", ip, port);
                Err(PingError::TimeoutError)
            }
        }
    }

    pub async fn ping_bedrock_server(self: &Self, ip: &str, port: u16, config: &PingConfig) -> Result<BedrockPing, PingError> {
        match timeout(config.timeout(), self.ping_bedrock_server_internal(ip, port)).await {
            Ok(result) => result,
            Err(_) => {
                debug!("Global ping timeout for {}:{}", ip, port);
                Err(PingError::TimeoutError)
            }
        }
    }

    async fn ping_bedrock_server_internal(self: &Self, ip: &str, port: u16) -> Result<BedrockPing, PingError> {
        debug!("Pinging bedrock server {}:{}", ip, port);

        let addr = resolve_to_addr(self, ip, port).await?;

        let socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        timeout(Duration::from_secs(1), socket.connect(addr))
            .await
            .map_err(|_| {
                debug!("Connection timeout error");
                PingError::ConnectionRefused
            })?
            .map_err(|e| {
                debug!("Connection error: {}", e);
                PingError::ConnectionRefused
            })?;

        let _ = socket.send(&create_ping()).await;

        let mut buffer = [0u8; 1024];
        let len = timeout(Duration::from_secs(5), socket.recv(&mut buffer))
            .await
            .map_err(|_| PingError::TimeoutError)?
            .map_err(|_| PingError::ConnectionRefused)?;

        let mut response_bytes = Bytes::copy_from_slice(&buffer[..len]);
        let rs = read_response(&mut response_bytes)?;

        Ok(rs)
    }

    async fn ping_java_server_internal(self: &Self, ip: &str, port: u16, config: &PingConfig) -> Result<JavaPing, PingError> {
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

        let mut buffer = BytesMut::with_capacity(256);

        let handshake_host = config.java_config().hostname().as_deref().unwrap_or(ip);
        write_ping_handshake(&mut buffer, handshake_host, &port, &config.java_config().protocol_version());
        write_ping_request(&mut buffer);

        stream.write_all(&buffer.freeze())
            .await
            .map_err(|_| PingError::SendPacketError)?;
        debug!("Stream all packets !");

        let mut buffered_reader = BufReader::new(&mut stream);
        let mut packet = read_packet(&mut buffered_reader).await?;
        debug!("Received Packet ID: {}", packet.id());

        let json = read_string(&mut packet.data)?;

        let as_res = serde_json::from_str::<JavaPing>(&json)
            .map_err(|e| {
                debug!("Error deserializing ping response: {}, json {}", e, json);
                PingError::SerializationError
            })?;
        Ok(as_res)
    }
}
