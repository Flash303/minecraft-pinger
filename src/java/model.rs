use serde::{Serialize, Deserialize};

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
}

// Components
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum Description {
    Plain(String),
    Component(TextComponent),
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum TextComponent {
    Object {
        #[serde(default)]
        text: String,

        #[serde(skip_serializing_if = "Option::is_none")]
        hat: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        player: Option<PlayerField>,
        #[serde(skip_serializing_if = "Option::is_none", alias = "shadowColor")]
        shadow_color: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        atlas: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        sprite: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        bold: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        strikethrough: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        obfuscated: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        italic: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        underlined: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        font: Option<String>,

        #[serde(skip_serializing_if = "Option::is_none")]
        color: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        extra: Option<Vec<TextComponent>>,
    },
    String(String),
    Array(Vec<TextComponent>),
}
// Components end

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum PlayerField {
    Name(String),
    Full(Player),
}

#[derive(Serialize, Deserialize)]
pub struct Player {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<Vec<Property>>
}

#[derive(Serialize, Deserialize)]
pub struct Property {
    pub name: String,
    pub value: String
}

#[derive(Serialize, Deserialize)]
pub struct Version {
    pub name: String,
    pub protocol: Option<i32>,
}

#[derive(Serialize, Deserialize)]
pub struct Players {
    pub online: u32,
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
        assert_eq!(ping.favicon, Some("data:image/png;base64,abc123".to_string()));
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
    fn test_text_component_string_variant() {
        let json = json!("simple string");
        let component: TextComponent = serde_json::from_value(json).unwrap();
        
        match component {
            TextComponent::String(s) => assert_eq!(s, "simple string"),
            _ => panic!("Expected string variant"),
        }
    }

    #[test]
    fn test_text_component_array_variant() {
        let json = json!([{"text": "part1"}, {"text": "part2"}]);
        let component: TextComponent = serde_json::from_value(json).unwrap();
        
        match component {
            TextComponent::Array(arr) => {
                assert_eq!(arr.len(), 2);
                match &arr[0] {
                    TextComponent::Object { text, .. } => assert_eq!(text, "part1"),
                    _ => panic!("Expected object"),
                }
            }
            _ => panic!("Expected array variant"),
        }
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
}