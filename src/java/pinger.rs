use crate::MinecraftPinger;
use crate::common::dns::resolve_filtered_addrs;
use crate::common::protocol::read_string;
use crate::error::PingError;
use crate::java::config::JavaPingConfig;
use crate::java::model::JavaPing;
use crate::java::protocol::{read_packet, write_ping_handshake, write_ping_request};
use bytes::BytesMut;
use log::debug;
use std::time::Instant;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time::{Instant as TokioInstant, timeout_at};

impl MinecraftPinger {
    pub async fn ping_java_server(
        &self,
        ip: &str,
        port: u16,
        config: &JavaPingConfig,
    ) -> Result<JavaPing, PingError> {
        let deadline = TokioInstant::now() + config.common().timeout();
        self.ping_java_server_internal(ip, port, config, deadline)
            .await
    }

    async fn ping_java_server_internal(
        &self,
        ip: &str,
        port: u16,
        config: &JavaPingConfig,
        deadline: TokioInstant,
    ) -> Result<JavaPing, PingError> {
        debug!("Pinging server {}:{}", ip, port);

        let (hostname, addrs) = timeout_at(
            deadline,
            resolve_filtered_addrs(self, ip, port, "tcp", config.common().ip_filter()),
        )
        .await??;

        let mut stream = timeout_at(deadline, TcpStream::connect(&addrs[..]))
            .await?
            .map_err(|e| {
                debug!("Connection error: {}", e);
                PingError::ConnectionRefused
            })?;
        stream.set_nodelay(true).unwrap_or_default();

        debug!("Stream connected to {:?}", stream.peer_addr().ok());

        let start_time = Instant::now();

        let mut buffer = BytesMut::with_capacity(256);

        let classic_hostname = hostname.as_deref().unwrap_or(ip);

        let handshake_host = config.hostname().as_deref().unwrap_or(classic_hostname);
        write_ping_handshake(
            &mut buffer,
            handshake_host,
            &port,
            &config.protocol_version(),
        );
        write_ping_request(&mut buffer);

        timeout_at(deadline, stream.write_all(&buffer.freeze()))
            .await?
            .map_err(|_| PingError::SendPacket)?;
        debug!("Stream all packets !");

        let connected_addr = stream.peer_addr().ok();

        let mut packet = timeout_at(deadline, read_packet(&mut stream)).await??;
        debug!("Received Packet ID: {}", packet.id());

        let json = read_string(&mut packet.data)?;
        let latency = start_time.elapsed().as_millis() as u32;

        let mut as_res = serde_json::from_str::<JavaPing>(&json)?;

        as_res.latency = latency;
        as_res.connected_addr = connected_addr;
        Ok(as_res)
    }
}
