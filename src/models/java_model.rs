use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct JavaPing {
    pub version: Version,
    pub players: Players,
    pub description: Description,
    pub favicon: Option<String>,
    #[serde(rename = "modinfo")]
    pub mod_info: Option<ModInfo>,
}

// Components
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum Description {
    Component(TextComponent),
    Plain(String),
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum TextComponent {
    Object {
        #[serde(default)]
        text: String,
        color: Option<String>,
        extra: Option<Vec<TextComponent>>,
    },
    String(String),
    Array(Vec<TextComponent>),
}
// Components end

#[derive(Serialize, Deserialize)]
pub struct Version {
    pub name: String,
    pub protocol: i32,
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

#[derive(Serialize, Deserialize)]
pub struct ModInfo {
    #[serde(rename = "type")]
    pub name: String,
    #[serde(rename = "modList")]
    pub mod_list: Vec<ModInfo>,
}

#[derive(Serialize, Deserialize)]
pub struct Mod {
    #[serde(rename = "modid")]
    pub mod_id: String,
    pub version: String,
}