use crate::MinecraftPinger;
use crate::common::dns::resolve_filtered_addrs;
use crate::common::protocol::read_string;
use crate::error::PingError;
use crate::java::config::JavaPingConfig;
use crate::java::model::{JavaPing, PlayerInfo, RawJavaPing};
use crate::java::protocol::{read_packet, write_ping_handshake, write_ping_request};
use bytes::BytesMut;
use log::debug;
use serde::de::IgnoredAny;
use std::time::Instant;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time::{Instant as TokioInstant, timeout_at};

impl MinecraftPinger {
    /// Pings a Java-edition server (status handshake + request).
    ///
    /// Resolves `_minecraft._tcp.<host>` SRV first, falls back to A/AAAA,
    /// applies the configured IP filter, then reports `latency` (ms) and
    /// the address actually connected to. The whole call is bounded by
    /// a single deadline derived from the configured timeout.
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
        let _ = stream.set_nodelay(true);

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

        let mut as_res: JavaPing = match (config.skip_favicon(), config.skip_sample()) {
            (false, false) => serde_json::from_str(&json)?,
            (true, false) => serde_json::from_str::<
                RawJavaPing<Option<IgnoredAny>, Option<Vec<PlayerInfo>>>,
            >(&json)?
            .into(),
            (false, true) => {
                serde_json::from_str::<RawJavaPing<Option<String>, Option<IgnoredAny>>>(&json)?
                    .into()
            }
            (true, true) => {
                serde_json::from_str::<RawJavaPing<Option<IgnoredAny>, Option<IgnoredAny>>>(&json)?
                    .into()
            }
        };

        as_res.latency = latency;
        as_res.connected_addr = connected_addr;
        Ok(as_res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::protocol::{write_string, write_var_int};
    use tokio::io::AsyncReadExt;
    use tokio::net::TcpListener;

    const STATUS_BODY: &str = "{\"version\":{\"name\":\"1.20.1\",\"protocol\":775},\"players\":{\"online\":5,\"max\":20,\"sample\":[{\"name\":\"Player1\",\"id\":\"uuid1\"}]},\"description\":\"Test\",\"favicon\":\"data:image/png;base64,abc123\"}";

    async fn fake_server(listener: TcpListener, pings: usize) {
        fake_server_with_body(listener, pings, STATUS_BODY).await;
    }

    async fn fake_server_with_body(listener: TcpListener, pings: usize, body: &'static str) {
        for _ in 0..pings {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 512];
            let _ = socket.read(&mut buf).await.unwrap();

            let mut payload = BytesMut::new();
            write_var_int(&mut payload, 0);
            write_string(&mut payload, body);
            let mut response = BytesMut::new();
            write_var_int(&mut response, payload.len() as i32);
            response.extend_from_slice(&payload);
            socket.write_all(&response.freeze()).await.unwrap();
        }
    }

    #[tokio::test]
    async fn ping_loopback_skip_flags() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(fake_server(listener, 4));

        let pinger = MinecraftPinger::new().unwrap();

        let full = pinger
            .ping_java_server("127.0.0.1", port, &JavaPingConfig::default())
            .await
            .unwrap();
        assert!(full.favicon.is_some());
        assert_eq!(
            full.players
                .as_ref()
                .expect("players should be present")
                .sample
                .as_ref()
                .unwrap()
                .len(),
            1
        );

        let no_favicon = pinger
            .ping_java_server(
                "127.0.0.1",
                port,
                &JavaPingConfig::builder().skip_favicon().build(),
            )
            .await
            .unwrap();
        assert!(no_favicon.favicon.is_none());
        assert!(
            no_favicon
                .players
                .as_ref()
                .expect("players should be present")
                .sample
                .is_some()
        );

        let no_sample = pinger
            .ping_java_server(
                "127.0.0.1",
                port,
                &JavaPingConfig::builder().skip_sample().build(),
            )
            .await
            .unwrap();
        assert!(no_sample.favicon.is_some());
        assert!(
            no_sample
                .players
                .as_ref()
                .expect("players should be present")
                .sample
                .is_none()
        );

        let neither = pinger
            .ping_java_server(
                "127.0.0.1",
                port,
                &JavaPingConfig::builder()
                    .skip_favicon()
                    .skip_sample()
                    .build(),
            )
            .await
            .unwrap();
        assert!(neither.favicon.is_none());
        assert!(
            neither
                .players
                .as_ref()
                .expect("players should be present")
                .sample
                .is_none()
        );
        assert_eq!(neither.version.name, "1.20.1");
    }

    const STATUS_BODY_NO_PLAYERS: &str = "{\"version\":{\"name\":\"Eldium\",\"protocol\":766},\"description\":\"play.eldium.fr\"}";

    #[tokio::test]
    async fn ping_loopback_without_players() {
        // Regression test for `missing field 'players'` (Eldium-style payload).
        for (skip_favicon, skip_sample) in
            [(false, false), (true, false), (false, true), (true, true)]
        {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            tokio::spawn(fake_server_with_body(
                listener,
                1,
                STATUS_BODY_NO_PLAYERS,
            ));

            let pinger = MinecraftPinger::new().unwrap();
            let mut builder = JavaPingConfig::builder();
            if skip_favicon {
                builder = builder.skip_favicon();
            }
            if skip_sample {
                builder = builder.skip_sample();
            }
            let res = pinger
                .ping_java_server("127.0.0.1", port, &builder.build())
                .await
                .unwrap_or_else(|e| {
                    panic!("skip_favicon={skip_favicon} skip_sample={skip_sample}: {e:?}")
                });
            assert!(
                res.players.is_none(),
                "skip_favicon={skip_favicon} skip_sample={skip_sample}"
            );
            assert_eq!(res.version.name, "Eldium");
        }
    }

    #[tokio::test]
    async fn ping_loopback_with_null_players() {
        const BODY: &str = "{\"version\":{\"name\":\"1.20.1\",\"protocol\":775},\"players\":null,\"description\":\"Test\"}";
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(fake_server_with_body(listener, 1, BODY));

        let pinger = MinecraftPinger::new().unwrap();
        let res = pinger
            .ping_java_server("127.0.0.1", port, &JavaPingConfig::default())
            .await
            .unwrap();
        assert!(res.players.is_none());
    }
}
