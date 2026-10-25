use serde::de::value::{MapAccessDeserializer, SeqAccessDeserializer};
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

/// Server MOTD: either a legacy plain string or a JSON chat component.
/// Deserialized in a single pass (no `untagged` retry).
#[derive(Serialize)]
#[serde(untagged)]
pub enum Description {
    Plain(String),
    Component(TextComponent),
}

impl<'de> Deserialize<'de> for Description {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(DescriptionVisitor)
    }
}

struct DescriptionVisitor;

impl<'de> Visitor<'de> for DescriptionVisitor {
    type Value = Description;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string, map or sequence")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Ok(Description::Plain(value.to_owned()))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
        Ok(Description::Plain(value))
    }

    fn visit_borrowed_str<E: de::Error>(self, value: &'de str) -> Result<Self::Value, E> {
        Ok(Description::Plain(value.to_owned()))
    }

    fn visit_bytes<E: de::Error>(self, value: &[u8]) -> Result<Self::Value, E> {
        Ok(Description::Plain(
            std::str::from_utf8(value).map_err(E::custom)?.to_owned(),
        ))
    }

    fn visit_borrowed_bytes<E: de::Error>(self, value: &'de [u8]) -> Result<Self::Value, E> {
        Ok(Description::Plain(
            std::str::from_utf8(value).map_err(E::custom)?.to_owned(),
        ))
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        Ok(Description::Component(TextComponent::deserialize(
            MapAccessDeserializer::new(map),
        )?))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
        Ok(Description::Component(TextComponent::deserialize(
            SeqAccessDeserializer::new(seq),
        )?))
    }
}

impl Description {
    /// Renders the MOTD as plain text (formatting codes stripped).
    pub fn to_plain_text(&self) -> String {
        match self {
            Description::Plain(s) => s.clone(),
            Description::Component(c) => c.to_plain_text(),
        }
    }
}

/// A JSON chat component: object, raw string, or nested array.
/// Deserialized in a single pass (no `untagged` retry).
#[derive(Serialize)]
#[serde(untagged)]
pub enum TextComponent {
    Object {
        text: String,

        #[serde(skip_serializing_if = "Option::is_none")]
        hat: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        player: Option<PlayerField>,
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

#[derive(Deserialize)]
struct ComponentObject {
    #[serde(default)]
    text: String,
    hat: Option<bool>,
    player: Option<PlayerField>,
    #[serde(alias = "shadowColor")]
    shadow_color: Option<i32>,
    atlas: Option<String>,
    sprite: Option<String>,
    bold: Option<bool>,
    strikethrough: Option<bool>,
    obfuscated: Option<bool>,
    italic: Option<bool>,
    underlined: Option<bool>,
    font: Option<String>,
    color: Option<String>,
    extra: Option<Vec<TextComponent>>,
}

impl From<ComponentObject> for TextComponent {
    fn from(raw: ComponentObject) -> Self {
        TextComponent::Object {
            text: raw.text,
            hat: raw.hat,
            player: raw.player,
            shadow_color: raw.shadow_color,
            atlas: raw.atlas,
            sprite: raw.sprite,
            bold: raw.bold,
            strikethrough: raw.strikethrough,
            obfuscated: raw.obfuscated,
            italic: raw.italic,
            underlined: raw.underlined,
            font: raw.font,
            color: raw.color,
            extra: raw.extra,
        }
    }
}

impl<'de> Deserialize<'de> for TextComponent {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(TextComponentVisitor)
    }
}

struct TextComponentVisitor;

impl<'de> Visitor<'de> for TextComponentVisitor {
    type Value = TextComponent;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string, map or sequence")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Ok(TextComponent::String(value.to_owned()))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
        Ok(TextComponent::String(value))
    }

    fn visit_borrowed_str<E: de::Error>(self, value: &'de str) -> Result<Self::Value, E> {
        Ok(TextComponent::String(value.to_owned()))
    }

    fn visit_bytes<E: de::Error>(self, value: &[u8]) -> Result<Self::Value, E> {
        Ok(TextComponent::String(
            std::str::from_utf8(value).map_err(E::custom)?.to_owned(),
        ))
    }

    fn visit_borrowed_bytes<E: de::Error>(self, value: &'de [u8]) -> Result<Self::Value, E> {
        Ok(TextComponent::String(
            std::str::from_utf8(value).map_err(E::custom)?.to_owned(),
        ))
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        Ok(ComponentObject::deserialize(MapAccessDeserializer::new(map))?.into())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
        Ok(TextComponent::Array(Vec::deserialize(
            SeqAccessDeserializer::new(seq),
        )?))
    }
}

impl TextComponent {
    /// Renders the component (and nested `extra`) as plain text.
    pub fn to_plain_text(&self) -> String {
        let mut out = String::new();
        self.write_plain_text(&mut out);
        out
    }

    fn write_plain_text(&self, out: &mut String) {
        match self {
            TextComponent::String(s) => out.push_str(s),
            TextComponent::Array(items) => {
                for item in items {
                    item.write_plain_text(out);
                }
            }
            TextComponent::Object { text, extra, .. } => {
                out.push_str(text);
                if let Some(extra) = extra {
                    for item in extra {
                        item.write_plain_text(out);
                    }
                }
            }
        }
    }
}

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
    pub properties: Option<Vec<Property>>,
}

#[derive(Serialize, Deserialize)]
pub struct Property {
    pub name: String,
    pub value: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
    fn test_description_to_plain_text_plain() {
        let description = Description::Plain("A Minecraft Server".to_string());
        assert_eq!(description.to_plain_text(), "A Minecraft Server");
    }

    #[test]
    fn test_description_to_plain_text_component_with_extra() {
        let json = json!({
            "text": "Hello ",
            "color": "green",
            "extra": [{"text": "World"}, ["!", {"text": "!"}]]
        });
        let component: TextComponent = serde_json::from_value(json).unwrap();
        let description = Description::Component(component);
        assert_eq!(description.to_plain_text(), "Hello World!!");
    }

    #[test]
    fn test_text_component_to_plain_text_string_variant() {
        let component = TextComponent::String("simple".to_string());
        assert_eq!(component.to_plain_text(), "simple");
    }

    #[test]
    fn test_description_serialize_keeps_content_shape() {
        let plain = Description::Plain("hi".to_string());
        assert_eq!(serde_json::to_value(&plain).unwrap(), json!("hi"));

        let component = Description::Component(TextComponent::Object {
            text: "hi".to_string(),
            hat: None,
            player: None,
            shadow_color: None,
            atlas: None,
            sprite: None,
            bold: None,
            strikethrough: None,
            obfuscated: None,
            italic: None,
            underlined: None,
            font: None,
            color: Some("green".to_string()),
            extra: None,
        });
        assert_eq!(
            serde_json::to_value(&component).unwrap(),
            json!({"text": "hi", "color": "green"})
        );
    }

    #[test]
    fn test_description_empty_object() {
        let description: Description = serde_json::from_value(json!({})).unwrap();
        match description {
            Description::Component(TextComponent::Object { text, .. }) => {
                assert_eq!(text, "")
            }
            _ => panic!("Expected component with empty text"),
        }
    }

    #[test]
    fn test_description_ignores_unknown_fields() {
        let description: Description =
            serde_json::from_value(json!({"text": "a", "unknown": 1})).unwrap();
        match description {
            Description::Component(TextComponent::Object { text, .. }) => assert_eq!(text, "a"),
            _ => panic!("Expected component"),
        }
    }

    #[test]
    fn test_description_rejects_duplicate_field() {
        let result: Result<Description, _> = serde_json::from_str(r#"{"text": "a", "text": "b"}"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_description_rejects_scalars() {
        for raw in ["5", "true", "null"] {
            let result: Result<Description, _> = serde_json::from_str(raw);
            assert!(result.is_err(), "expected error for {raw}");
        }
    }

    #[test]
    fn test_description_rejects_null_text() {
        let result: Result<Description, _> = serde_json::from_str(r#"{"text": null}"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_description_accepts_padded_string() {
        let description: Description = serde_json::from_str(r#"  "hi"  "#).unwrap();
        assert!(matches!(description, Description::Plain(s) if s == "hi"));
    }

    #[test]
    fn test_description_nested_mixed_extra() {
        let description: Description = serde_json::from_value(json!({
            "text": "a",
            "extra": ["b", {"text": "c"}, ["d"]]
        }))
        .unwrap();
        assert_eq!(description.to_plain_text(), "abcd");
    }

    #[test]
    fn test_description_shadow_color_alias() {
        let description: Description =
            serde_json::from_value(json!({"text": "a", "shadowColor": 1})).unwrap();
        match description {
            Description::Component(TextComponent::Object { shadow_color, .. }) => {
                assert_eq!(shadow_color, Some(1))
            }
            _ => panic!("Expected component"),
        }
    }
}
