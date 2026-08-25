use std::time::Instant;
use bytes::BytesMut;
use log::debug;
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::timeout;
use crate::common::dns::resolve_filtered_addrs;
use crate::common::protocol::read_string;
use crate::error::PingError;
use crate::java::config::JavaPingConfig;
use crate::java::model::JavaPing;
use crate::java::protocol::{read_packet, write_ping_handshake, write_ping_request};
use crate::MinecraftPinger;

impl MinecraftPinger {
    pub async fn ping_java_server(self: &Self,
                                  ip: &str,
                                  port: u16,
                                  config: &JavaPingConfig) -> Result<JavaPing, PingError> {
        let rs = timeout(config.common().timeout(), self.ping_java_server_internal(ip, port, &config)).await??;
        Ok(rs)
    }

    async fn ping_java_server_internal(self: &Self, ip: &str, port: u16, config: &JavaPingConfig) -> Result<JavaPing, PingError> {
        debug!("Pinging server {}:{}", ip, port);

        let addrs = resolve_filtered_addrs(self, ip, port, "tcp", config.common().ip_filter()).await?;

        let stream_future = TcpStream::connect(&addrs[..]);
        let mut stream = timeout(config.common().timeout(), stream_future)
            .await?
            .map_err(|e| {
                debug!("Connection error: {}", e);
                PingError::ConnectionRefused
            })?;

        stream.set_nodelay(true).unwrap_or_default();

        debug!("Stream connected to {}", stream.peer_addr().map(|a| a.to_string()).unwrap_or_else(|_| "unknown".to_string()));

        let start_time = Instant::now();

        let mut buffer = BytesMut::with_capacity(256);

        let handshake_host = config.hostname().as_deref().unwrap_or(ip);
        write_ping_handshake(&mut buffer, handshake_host, &port, &config.protocol_version());
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