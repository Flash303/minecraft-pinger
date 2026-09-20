use serde::{Deserialize, Serialize};
use crate::error::PingError;

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct BedrockPing {
    pub edition: String,
    pub motd: String,
    pub protocol_version: u32,
    pub version: String,
    pub current_players: u32,
    pub max_players: u32,
    pub server_id: String,
    pub map_name: String,
    pub game_mode: String,
    pub numeric_id: Option<u8>,
    pub port: Option<u16>,
    pub unknown_val: Option<u32>,

    #[serde(skip_deserializing)]
    pub latency: u32,
}

impl TryFrom<String> for BedrockPing {
    type Error = PingError;

    fn try_from(data: String) -> Result<Self, Self::Error> {
        let parts: Vec<&str> = data.split(';').collect();
        if parts.len() < 8 {
            return Err(PingError::ParseResponse);
        }
        Ok(BedrockPing {
            edition: parts[0].to_string(),
            motd: parts[1].to_string(),
            protocol_version: parts[2].parse().unwrap_or(0),
            version: parts[3].to_string(),
            current_players: parts[4].parse().unwrap_or(0),
            max_players: parts[5].parse().unwrap_or(0),
            server_id: parts[6].to_string(),
            map_name: parts[7].to_string(),
            game_mode: parts.get(8).map(|s| s.to_string()).unwrap_or_default(),
            numeric_id: parts.get(9).and_then(|s| s.parse().ok()),
            port: parts.get(10).and_then(|s| s.parse().ok()),
            unknown_val: parts.get(11).and_then(|s| s.parse().ok()),
            latency: 0,
        })
    }
}