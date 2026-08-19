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
    Component(TextComponent),
    Plain(String),
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
        player: Option<Player>,
        #[serde(skip_serializing_if = "Option::is_none")]
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