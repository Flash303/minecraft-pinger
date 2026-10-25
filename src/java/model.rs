pub use super::component::{Description, Player, PlayerField, Property, TextComponent};
use serde::de::IgnoredAny;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// Parsed Java status response plus measured `latency` (ms).
#[derive(Serialize, Deserialize)]
pub struct JavaPing {
    pub version: Version,
    pub players: Players,
    pub description: Description,
    pub favicon: Option<String>,
    #[serde(rename = "modinfo")]
    pub mod_info: Option<ModInfo>,

    #[serde(default)]
    pub latency: u32,

    #[serde(skip_serializing, skip_deserializing)]
    pub connected_addr: Option<SocketAddr>,
}

// Same payload as JavaPing, but favicon and player sample can be
// replaced with IgnoredAny to skip their allocation while parsing.
#[derive(Deserialize)]
pub(crate) struct RawJavaPing<F, S> {
    version: Version,
    players: RawPlayers<S>,
    description: Description,
    favicon: F,
    #[serde(rename = "modinfo")]
    mod_info: Option<ModInfo>,
}

#[derive(Deserialize)]
pub(crate) struct RawPlayers<S> {
    online: i32,
    max: i32,
    sample: S,
}

impl From<RawJavaPing<Option<IgnoredAny>, Option<IgnoredAny>>> for JavaPing {
    fn from(raw: RawJavaPing<Option<IgnoredAny>, Option<IgnoredAny>>) -> Self {
        JavaPing {
            version: raw.version,
            players: Players {
                online: raw.players.online,
                max: raw.players.max,
                sample: None,
            },
            description: raw.description,
            favicon: None,
            mod_info: raw.mod_info,
            latency: 0,
            connected_addr: None,
        }
    }
}

impl From<RawJavaPing<Option<IgnoredAny>, Option<Vec<PlayerInfo>>>> for JavaPing {
    fn from(raw: RawJavaPing<Option<IgnoredAny>, Option<Vec<PlayerInfo>>>) -> Self {
        JavaPing {
            version: raw.version,
            players: Players {
                online: raw.players.online,
                max: raw.players.max,
                sample: raw.players.sample,
            },
            description: raw.description,
            favicon: None,
            mod_info: raw.mod_info,
            latency: 0,
            connected_addr: None,
        }
    }
}

impl From<RawJavaPing<Option<String>, Option<IgnoredAny>>> for JavaPing {
    fn from(raw: RawJavaPing<Option<String>, Option<IgnoredAny>>) -> Self {
        JavaPing {
            version: raw.version,
            players: Players {
                online: raw.players.online,
                max: raw.players.max,
                sample: None,
            },
            description: raw.description,
            favicon: raw.favicon,
            mod_info: raw.mod_info,
            latency: 0,
            connected_addr: None,
        }
    }
}

// Components live in `super::component`, re-exported above.
#[derive(Serialize, Deserialize)]
pub struct Version {
    pub name: String,
    pub protocol: Option<i32>,
}

#[derive(Serialize, Deserialize)]
pub struct Players {
    pub online: i32,
    pub max: i32,
    pub sample: Option<Vec<PlayerInfo>>,
}

#[derive(Serialize, Deserialize)]
pub struct PlayerInfo {
    pub name: String,
    pub id: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct ModInfo {
    #[serde(rename = "type", default)]
    pub name: String,
    #[serde(rename = "modList", default)]
    pub mod_list: Vec<Mod>,
}

#[derive(Serialize, Deserialize)]
pub struct Mod {
    #[serde(rename = "modid")]
    pub mod_id: String,
    pub version: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_java_ping_deserialize_plain_description() {
        let json = json!({
            "version": {"name": "1.20.1", "protocol": 775},
            "players": {"online": 5, "max": 20, "sample": null},
            "description": "A Minecraft Server",
            "favicon": null,
            "modinfo": null
        });

        let ping: JavaPing = serde_json::from_value(json).unwrap();
        assert_eq!(ping.version.name, "1.20.1");
        assert_eq!(ping.version.protocol, Some(775));
        assert_eq!(ping.players.online, 5);
        assert_eq!(ping.players.max, 20);

        match ping.description {
            Description::Plain(s) => assert_eq!(s, "A Minecraft Server"),
            Description::Component(_) => panic!("Expected plain description"),
        }
    }

    #[test]
    fn test_java_ping_deserialize_component_description() {
        let json = json!({
            "version": {"name": "1.20.1", "protocol": 775},
            "players": {"online": 5, "max": 20, "sample": null},
            "description": {"text": "A Minecraft Server", "color": "green"},
            "favicon": null,
            "modinfo": null
        });

        let ping: JavaPing = serde_json::from_value(json).unwrap();

        match ping.description {
            Description::Component(TextComponent::Object { text, color, .. }) => {
                assert_eq!(text, "A Minecraft Server");
                assert_eq!(color, Some("green".to_string()));
            }
            _ => panic!("Expected component description"),
        }
    }

    #[test]
    fn test_java_ping_with_favicon() {
        let json = json!({
            "version": {"name": "1.20.1", "protocol": 775},
            "players": {"online": 5, "max": 20, "sample": null},
            "description": "Test",
            "favicon": "data:image/png;base64,abc123",
            "modinfo": null
        });

        let ping: JavaPing = serde_json::from_value(json).unwrap();
        assert_eq!(
            ping.favicon,
            Some("data:image/png;base64,abc123".to_string())
        );
    }

    #[test]
    fn test_java_ping_with_modinfo() {
        let json = json!({
            "version": {"name": "1.20.1", "protocol": 775},
            "players": {"online": 5, "max": 20, "sample": null},
            "description": "Test",
            "favicon": null,
            "modinfo": {"type": "FML", "modList": [{"modid": "testmod", "version": "1.0"}]}
        });

        let ping: JavaPing = serde_json::from_value(json).unwrap();
        assert!(ping.mod_info.is_some());
        let mod_info = ping.mod_info.unwrap();
        assert_eq!(mod_info.name, "FML");
        assert_eq!(mod_info.mod_list.len(), 1);
        assert_eq!(mod_info.mod_list[0].mod_id, "testmod");
        assert_eq!(mod_info.mod_list[0].version, "1.0");
    }

    #[test]
    fn test_java_ping_with_player_sample() {
        let json = json!({
            "version": {"name": "1.20.1", "protocol": 775},
            "players": {
                "online": 5,
                "max": 20,
                "sample": [
                    {"name": "Player1", "id": "uuid1"},
                    {"name": "Player2", "id": "uuid2"}
                ]
            },
            "description": "Test",
            "favicon": null,
            "modinfo": null
        });

        let ping: JavaPing = serde_json::from_value(json).unwrap();
        assert_eq!(ping.players.sample.as_ref().unwrap().len(), 2);
        assert_eq!(ping.players.sample.as_ref().unwrap()[0].name, "Player1");
    }

    #[test]
    fn test_version_deserialize() {
        let json = json!({"name": "1.20.1", "protocol": 775});
        let version: Version = serde_json::from_value(json).unwrap();
        assert_eq!(version.name, "1.20.1");
        assert_eq!(version.protocol, Some(775));
    }

    #[test]
    fn test_version_deserialize_without_protocol() {
        let json = json!({"name": "1.20.1"});
        let version: Version = serde_json::from_value(json).unwrap();
        assert_eq!(version.name, "1.20.1");
        assert_eq!(version.protocol, None);
    }

    #[test]
    fn test_players_deserialize() {
        let json = json!({"online": 10, "max": 20, "sample": []});
        let players: Players = serde_json::from_value(json).unwrap();
        assert_eq!(players.online, 10);
        assert_eq!(players.max, 20);
        assert!(players.sample.is_some());
    }

    fn skipped_json() -> serde_json::Value {
        json!({
            "version": {"name": "1.20.1", "protocol": 775},
            "players": {
                "online": 5,
                "max": 20,
                "sample": [{"name": "Player1", "id": "uuid1"}]
            },
            "description": "Test",
            "favicon": "data:image/png;base64,abc123",
            "modinfo": null
        })
    }

    #[test]
    fn test_raw_ping_skip_favicon_only() {
        let raw: RawJavaPing<Option<IgnoredAny>, Option<Vec<PlayerInfo>>> =
            serde_json::from_value(skipped_json()).unwrap();
        let ping = JavaPing::from(raw);

        assert!(ping.favicon.is_none());
        assert_eq!(ping.players.sample.as_ref().unwrap().len(), 1);
        assert_eq!(ping.version.name, "1.20.1");
    }

    #[test]
    fn test_raw_ping_skip_sample_only() {
        let raw: RawJavaPing<Option<String>, Option<IgnoredAny>> =
            serde_json::from_value(skipped_json()).unwrap();
        let ping = JavaPing::from(raw);

        assert_eq!(
            ping.favicon,
            Some("data:image/png;base64,abc123".to_string())
        );
        assert!(ping.players.sample.is_none());
    }

    #[test]
    fn test_raw_ping_skip_both() {
        let raw: RawJavaPing<Option<IgnoredAny>, Option<IgnoredAny>> =
            serde_json::from_value(skipped_json()).unwrap();
        let ping = JavaPing::from(raw);

        assert!(ping.favicon.is_none());
        assert!(ping.players.sample.is_none());
        assert_eq!(ping.players.online, 5);
    }
}
