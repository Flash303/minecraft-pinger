pub mod models;
pub mod utils;
pub mod error;
pub mod config;
pub mod protocol;

use std::sync::Arc;
use error::PingError;
use protocol::protocol::{read_string};
use std::time::{Duration, Instant};
use bytes::{Bytes, BytesMut};
use hickory_resolver::net::runtime::TokioRuntimeProvider;
use hickory_resolver::Resolver;
use log::{debug};
use tokio::io::{AsyncWriteExt, BufReader};
use crate::utils::dns::{resolve_to_addr};
use tokio::net::{TcpStream, UdpSocket};
use tokio::time::timeout;
use crate::config::PingConfig;
use crate::models::java_model::JavaPing;
use crate::models::bedrock_model::BedrockPing;
use protocol::bedrock_protocol::{create_ping, read_response};
use protocol::java_protocol::{read_packet, write_ping_handshake, write_ping_request};

pub struct MinecraftPinger {
    dns_resolver: Arc<Resolver<TokioRuntimeProvider>>,
}

impl MinecraftPinger {
    pub fn new() -> Result<Self, PingError> {
        let builder = Resolver::builder_tokio()
            .map_err(|e| PingError::Init(e.to_string()))?;
        let resolver = builder.build()
            .map_err(|e| PingError::Init(e.to_string()))?;

        Ok(Self {
            dns_resolver: Arc::new(resolver)
        })
    }

    pub async fn ping_java_server(self: &Self,
            ip: &str,
            port: u16,
            config: &PingConfig) -> Result<JavaPing, PingError> {
        let rs = timeout(config.timeout(), self.ping_java_server_internal(ip, port, &config)).await??;
        Ok(rs)
    }

    pub async fn ping_bedrock_server(self: &Self,
            ip: &str,
            port: u16,
            config: &PingConfig) -> Result<BedrockPing, PingError> {
        let rs = timeout(config.timeout(), self.ping_bedrock_server_internal(ip, port, config)).await??;
        Ok(rs)
    }

    async fn ping_bedrock_server_internal(self: &Self, ip: &str, port: u16, config: &PingConfig) -> Result<BedrockPing, PingError> {
        debug!("Pinging bedrock server {}:{}", ip, port);

        let addr = resolve_to_addr(self, ip, port, "udp").await?;
        let start_time = Instant::now();

        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        timeout(config.timeout(), socket.connect(addr))
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

    async fn ping_java_server_internal(self: &Self, ip: &str, port: u16, config: &PingConfig) -> Result<JavaPing, PingError> {
        debug!("Pinging server {}:{}", ip, port);

        let addr = resolve_to_addr(self, ip, port, "tcp").await?;

        let stream_future = TcpStream::connect(addr);
        let mut stream = timeout(config.timeout(), stream_future)
            .await?
            .map_err(|e| {
                debug!("Connection error: {}", e);
                PingError::ConnectionRefused
            })?;

        stream.set_nodelay(true).unwrap_or_default();

        debug!("Stream connected to {}", addr);

        let start_time = Instant::now();

        let mut buffer = BytesMut::with_capacity(256);

        let handshake_host = config.java_config().hostname().as_deref().unwrap_or(ip);
        write_ping_handshake(&mut buffer, handshake_host, &port, &config.java_config().protocol_version());
        write_ping_request(&mut buffer);

        stream.write_all(&buffer.freeze())
            .await
            .map_err(|_| PingError::SendPacket)?;
        debug!("Stream all packets !");

        let mut buffered_reader = BufReader::new(&mut stream);
        let mut packet = read_packet(&mut buffered_reader).await?;
        debug!("Received Packet ID: {}", packet.id());

        let json = read_string(&mut packet.data)?;
        let latency = start_time.elapsed().as_millis() as u32;

        let mut as_res = serde_json::from_str::<JavaPing>(&json)?;

        as_res.latency = latency;
        Ok(as_res)
    }
}
