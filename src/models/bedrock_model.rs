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
    pub server_id: i64,
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
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .delimiter(b';')
            .flexible(true)
            .from_reader(data.as_bytes());

        if let Some(result) = reader.deserialize::<BedrockPing>().next() {
            return result.map_err(|_| PingError::ParseResponse);
        }

        Err(PingError::ParseResponse)
    }
}