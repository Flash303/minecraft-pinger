use crate::MinecraftPinger;
use crate::bedrock::model::BedrockPing;
use crate::bedrock::protocol::{create_ping, read_response};
use crate::common::dns::resolve_filtered_addrs;
use crate::config::PingConfig;
use crate::error::PingError;
use log::debug;
use std::net::SocketAddr;
use std::time::Instant;
use tokio::net::UdpSocket;
use tokio::time::{Instant as TokioInstant, timeout_at};

impl MinecraftPinger {
    /// Pings a Bedrock server with a fresh UDP socket.
    ///
    /// For batch scans, prefer [`ping_bedrock_with_socket`](Self::ping_bedrock_with_socket)
    /// on a shared socket instead of binding one socket per ping.
    pub async fn ping_bedrock_server(
        &self,
        ip: &str,
        port: u16,
        config: &PingConfig,
    ) -> Result<BedrockPing, PingError> {
        let deadline = TokioInstant::now() + config.timeout();
        self.ping_bedrock_server_internal(ip, port, config, deadline)
            .await
    }

    async fn ping_bedrock_server_internal(
        &self,
        ip: &str,
        port: u16,
        config: &PingConfig,
        deadline: TokioInstant,
    ) -> Result<BedrockPing, PingError> {
        debug!("Pinging bedrock server {}:{}", ip, port);

        let (_, addrs) = timeout_at(
            deadline,
            resolve_filtered_addrs(self, ip, port, "udp", config.ip_filter()),
        )
        .await??;

        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        Self::exchange(&socket, &addrs, deadline).await
    }

    /// Pings a Bedrock server through a caller-provided UDP socket.
    ///
    /// Share one (or a pool of) `Arc<UdpSocket>` across scans to avoid
    /// binding a socket and consuming an ephemeral port per ping.
    /// Replies are matched by sender address; concurrent pings to the
    /// *same* address on one socket cannot be told apart (the client
    /// GUID is not echoed by the pong), so use one socket per concurrent
    /// target or accept that ambiguity.
    pub async fn ping_bedrock_with_socket(
        &self,
        socket: &UdpSocket,
        ip: &str,
        port: u16,
        config: &PingConfig,
    ) -> Result<BedrockPing, PingError> {
        let deadline = TokioInstant::now() + config.timeout();
        self.ping_bedrock_with_socket_internal(socket, ip, port, config, deadline)
            .await
    }

    async fn ping_bedrock_with_socket_internal(
        &self,
        socket: &UdpSocket,
        ip: &str,
        port: u16,
        config: &PingConfig,
        deadline: TokioInstant,
    ) -> Result<BedrockPing, PingError> {
        debug!("Pinging bedrock server {}:{} (shared socket)", ip, port);

        let (_, addrs) = timeout_at(
            deadline,
            resolve_filtered_addrs(self, ip, port, "udp", config.ip_filter()),
        )
        .await??;

        Self::exchange(socket, &addrs, deadline).await
    }

    async fn exchange(
        socket: &UdpSocket,
        addrs: &[SocketAddr],
        deadline: TokioInstant,
    ) -> Result<BedrockPing, PingError> {
        let start_time = Instant::now();
        let target = addrs.first().copied().ok_or(PingError::ConnectionRefused)?;

        timeout_at(deadline, socket.send_to(&create_ping()?, target))
            .await?
            .map_err(|_| PingError::SendPacket)?;

        // A shared socket may carry other traffic: only accept the target reply.
        // A full buffer means the datagram may have been cut: reject it.
        let mut buffer = [0u8; 4096];
        let len = loop {
            let (len, from) = timeout_at(deadline, socket.recv_from(&mut buffer))
                .await?
                .map_err(|_| PingError::ConnectionRefused)?;
            if from == target {
                break len;
            }
        };
        if len == buffer.len() {
            return Err(PingError::ReadPacket("truncated response"));
        }

        let mut response_bytes = &buffer[..len];
        let latency = start_time.elapsed().as_millis() as u32;

        let mut rs = read_response(&mut response_bytes)?;
        rs.latency = latency;
        rs.connected_addr = Some(target);

        Ok(rs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PingConfig;
    use tokio::net::UdpSocket;

    fn pong_bytes() -> Vec<u8> {
        let mut pong = Vec::new();
        pong.push(0x1C);
        pong.extend_from_slice(&[0u8; 8]);
        pong.extend_from_slice(&[0u8; 8]);
        pong.extend_from_slice(&[0u8; 16]);
        let body = "MCPE;Test Server;589;1.20.0;10;20;123;world;Survival";
        pong.extend_from_slice(&(body.len() as u16).to_be_bytes());
        pong.extend_from_slice(body.as_bytes());
        pong
    }

    #[tokio::test]
    async fn ping_loopback_default_and_shared_socket() {
        let server = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let port = server.local_addr().unwrap().port();

        tokio::spawn(async move {
            for _ in 0..2 {
                let mut buf = [0u8; 128];
                let (len, from) = server.recv_from(&mut buf).await.unwrap();
                assert_eq!(len, 33);
                assert_eq!(buf[0], 0x01);
                server.send_to(&pong_bytes(), from).await.unwrap();
            }
        });

        let pinger = MinecraftPinger::new().unwrap();
        let config = PingConfig::builder().build();

        let res = pinger
            .ping_bedrock_server("127.0.0.1", port, &config)
            .await
            .unwrap();
        assert_eq!(res.motd, "Test Server");
        assert_eq!(res.current_players, 10);

        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let res = pinger
            .ping_bedrock_with_socket(&socket, "127.0.0.1", port, &config)
            .await
            .unwrap();
        assert_eq!(res.motd, "Test Server");
        assert_eq!(res.current_players, 10);
        assert_eq!(res.connected_addr.unwrap().port(), port);
    }

    #[tokio::test]
    async fn ping_rejects_truncated_response() {
        let server = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let port = server.local_addr().unwrap().port();

        tokio::spawn(async move {
            let mut buf = [0u8; 128];
            let (_, from) = server.recv_from(&mut buf).await.unwrap();
            server.send_to(&[0u8; 4096], from).await.unwrap();
        });

        let pinger = MinecraftPinger::new().unwrap();
        let config = PingConfig::builder().build();
        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

        let result = pinger
            .ping_bedrock_with_socket(&socket, "127.0.0.1", port, &config)
            .await;
        assert!(matches!(
            result,
            Err(PingError::ReadPacket("truncated response"))
        ));
    }
}
