use crate::error::PingError;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// Parsed Bedrock (RakNet unconnected pong) status.
///
/// Field order follows the `;`-separated pong string. Optional trailing
/// fields are `None` when the server omits them. Parse from `&str`
/// without intermediate allocation via `TryFrom<&str>` (`String` kept
/// as a convenience wrapper).
#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct BedrockPing {
    pub edition: String,
    pub motd: String,
    pub protocol_version: u32,
    pub version: String,
    pub current_players: i32,
    pub max_players: u32,
    pub server_id: String,
    pub map_name: String,
    pub game_mode: String,
    pub numeric_id: Option<u8>,
    pub port: Option<u16>,
    pub unknown_val: Option<u32>,

    #[serde(skip_deserializing)]
    pub latency: u32,

    #[serde(skip_serializing, skip_deserializing)]
    pub connected_addr: Option<SocketAddr>,
}

impl TryFrom<&str> for BedrockPing {
    type Error = PingError;

    fn try_from(data: &str) -> Result<Self, Self::Error> {
        let mut parts = data.split(';');

        let edition = parts.next().ok_or(PingError::ParseResponse)?.to_owned();
        let motd = parts.next().ok_or(PingError::ParseResponse)?.to_owned();
        let protocol_version = parts
            .next()
            .ok_or(PingError::ParseResponse)?
            .parse()
            .unwrap_or(0);
        let version = parts.next().ok_or(PingError::ParseResponse)?.to_owned();
        let current_players = parts
            .next()
            .ok_or(PingError::ParseResponse)?
            .parse()
            .unwrap_or(0);
        let max_players = parts
            .next()
            .ok_or(PingError::ParseResponse)?
            .parse()
            .unwrap_or(0);
        let server_id = parts.next().ok_or(PingError::ParseResponse)?.to_owned();
        let map_name = parts.next().ok_or(PingError::ParseResponse)?.to_owned();

        Ok(BedrockPing {
            edition,
            motd,
            protocol_version,
            version,
            current_players,
            max_players,
            server_id,
            map_name,
            game_mode: parts.next().unwrap_or_default().to_owned(),
            numeric_id: parts.next().and_then(|part| part.parse().ok()),
            port: parts.next().and_then(|part| part.parse().ok()),
            unknown_val: parts.next().and_then(|part| part.parse().ok()),
            latency: 0,
            connected_addr: None,
        })
    }
}

impl TryFrom<String> for BedrockPing {
    type Error = PingError;

    fn try_from(data: String) -> Result<Self, Self::Error> {
        Self::try_from(data.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bedrock_ping_try_from_full() {
        let data = "MCPE;Test Server;589;1.20.0;10;20;1234567890;world;Survival;1;19132;123";
        let ping = BedrockPing::try_from(data).unwrap();

        assert_eq!(ping.edition, "MCPE");
        assert_eq!(ping.motd, "Test Server");
        assert_eq!(ping.protocol_version, 589);
        assert_eq!(ping.version, "1.20.0");
        assert_eq!(ping.current_players, 10);
        assert_eq!(ping.max_players, 20);
        assert_eq!(ping.server_id, "1234567890");
        assert_eq!(ping.map_name, "world");
        assert_eq!(ping.game_mode, "Survival");
        assert_eq!(ping.numeric_id, Some(1));
        assert_eq!(ping.port, Some(19132));
        assert_eq!(ping.unknown_val, Some(123));
        assert_eq!(ping.latency, 0);
    }

    #[test]
    fn test_bedrock_ping_try_from_minimal() {
        let data = "MCPE;Test;589;1.20.0;0;20;1234567890;world;Creative";
        let ping = BedrockPing::try_from(data).unwrap();

        assert_eq!(ping.edition, "MCPE");
        assert_eq!(ping.motd, "Test");
        assert_eq!(ping.protocol_version, 589);
        assert_eq!(ping.version, "1.20.0");
        assert_eq!(ping.current_players, 0);
        assert_eq!(ping.max_players, 20);
        assert_eq!(ping.game_mode, "Creative");
        assert_eq!(ping.numeric_id, None);
        assert_eq!(ping.port, None);
        assert_eq!(ping.unknown_val, None);
    }

    #[test]
    fn test_bedrock_ping_try_from_empty_game_mode() {
        let data = "MCPE;Test;589;1.20.0;0;20;1234567890;world";
        let ping = BedrockPing::try_from(data).unwrap();

        assert_eq!(ping.game_mode, "");
    }

    #[test]
    fn test_bedrock_ping_try_from_invalid_protocol_version() {
        let data = "MCPE;Test;invalid;1.20.0;0;20;1234567890;world";
        let ping = BedrockPing::try_from(data).unwrap();

        assert_eq!(ping.protocol_version, 0);
    }

    #[test]
    fn test_bedrock_ping_try_from_invalid_players() {
        let data = "MCPE;Test;589;1.20.0;invalid;invalid;1234567890;world";
        let ping = BedrockPing::try_from(data).unwrap();

        assert_eq!(ping.current_players, 0);
        assert_eq!(ping.max_players, 0);
    }

    #[test]
    fn test_bedrock_ping_try_from_string_compat() {
        let ping =
            BedrockPing::try_from("MCPE;Test;589;1.20.0;0;20;123;world".to_string()).unwrap();
        assert_eq!(ping.edition, "MCPE");
        assert_eq!(ping.game_mode, "");
    }

    #[test]
    fn test_bedrock_ping_try_from_too_few_parts() {
        let data = "MCPE;Test;589;1.20.0;0;20;1234567890";
        let result = BedrockPing::try_from(data);

        assert!(result.is_err());
        match result.unwrap_err() {
            PingError::ParseResponse => {}
            _ => panic!("Expected ParseResponse error"),
        }
    }

    #[test]
    fn test_bedrock_ping_serialize_deserialize() {
        let ping = BedrockPing {
            edition: "MCPE".to_string(),
            motd: "Test".to_string(),
            protocol_version: 589,
            version: "1.20.0".to_string(),
            current_players: 10,
            max_players: 20,
            server_id: "1234567890".to_string(),
            map_name: "world".to_string(),
            game_mode: "Survival".to_string(),
            numeric_id: Some(1),
            port: Some(19132),
            unknown_val: Some(123),
            latency: 50,
            connected_addr: None,
        };

        let json = serde_json::to_string(&ping).unwrap();
        let deserialized: BedrockPing = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.edition, ping.edition);
        assert_eq!(deserialized.motd, ping.motd);
        assert_eq!(deserialized.protocol_version, ping.protocol_version);
        assert_eq!(deserialized.version, ping.version);
        assert_eq!(deserialized.current_players, ping.current_players);
        assert_eq!(deserialized.max_players, ping.max_players);
        assert_eq!(deserialized.server_id, ping.server_id);
        assert_eq!(deserialized.map_name, ping.map_name);
        assert_eq!(deserialized.game_mode, ping.game_mode);
        assert_eq!(deserialized.numeric_id, ping.numeric_id);
        assert_eq!(deserialized.port, ping.port);
        assert_eq!(deserialized.unknown_val, ping.unknown_val);
        // latency is skipped in deserialization
        assert_eq!(deserialized.latency, 0);
    }

    #[test]
    fn test_bedrock_ping_default() {
        let ping = BedrockPing::default();
        assert_eq!(ping.edition, "");
        assert_eq!(ping.motd, "");
        assert_eq!(ping.protocol_version, 0);
        assert_eq!(ping.version, "");
        assert_eq!(ping.current_players, 0);
        assert_eq!(ping.max_players, 0);
        assert_eq!(ping.server_id, "");
        assert_eq!(ping.map_name, "");
        assert_eq!(ping.game_mode, "");
        assert_eq!(ping.numeric_id, None);
        assert_eq!(ping.port, None);
        assert_eq!(ping.unknown_val, None);
        assert_eq!(ping.latency, 0);
    }
}
