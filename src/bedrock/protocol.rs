use crate::bedrock::model::BedrockPing;
use crate::error::PingError;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};

const MAGIC: u128 = 0x00ffff00fefefefefdfdfdfd12345678;

fn try_split_to(buffer: &mut Bytes, size: usize) -> Option<Bytes> {
    if buffer.len() >= size {
        Some(buffer.split_to(size))
    } else {
        None
    }
}

pub(crate) fn create_ping() -> Result<Bytes, PingError> {
    let mut data = BytesMut::with_capacity(33);
    data.put_u8(0x1);

    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| PingError::Init("SystemTime before UNIX_EPOCH".into()))?
        .as_millis() as u64;

    data.put_u64(time);
    data.put_u128(MAGIC);

    let mut guid = [0u8; 8];
    rand::rng().fill_bytes(&mut guid);

    data.put_slice(&guid);

    Ok(data.freeze())
}

pub(crate) fn read_response(buffer: &mut Bytes) -> Result<BedrockPing, PingError> {
    let _packet_id = buffer
        .try_get_u8()
        .map_err(|_| PingError::ReadPacket("Packet id not found".to_string()))?;

    let _timestamp = buffer
        .try_get_u64()
        .map_err(|_| PingError::ReadPacket("Timestamp not found".to_string()))?;

    let _server_guid = buffer
        .try_get_u64()
        .map_err(|_| PingError::ReadPacket("Server guid not found".to_string()))?;

    let _magic = buffer
        .try_get_u128()
        .map_err(|_| PingError::ReadPacket("Magic id not found".to_string()))?;

    let string_size = buffer
        .try_get_u16()
        .map_err(|_| PingError::ReadPacket("String size not found".to_string()))?;

    let string_data = try_split_to(buffer, string_size as usize)
        .ok_or(PingError::ReadPacket("String data not found".to_string()))?;

    let str = String::from_utf8(string_data.into())?;

    str.try_into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn test_create_ping() {
        let ping = create_ping().unwrap();
        assert!(!ping.is_empty());
        assert_eq!(ping.len(), 33); // 1 + 8 + 16 + 8
        assert_eq!(ping[0], 0x01);
    }

    #[test]
    fn test_read_response_valid() {
        let mut buffer = BytesMut::new();
        buffer.put_u8(0x02); // packet id
        buffer.put_u64(1234567890); // timestamp
        buffer.put_u64(9876543210); // server guid
        buffer.put_u128(MAGIC); // magic

        let response_str =
            "MCPE;Test Server;589;1.20.0;10;20;1234567890;world;Survival;1;19132;123";
        buffer.put_u16(response_str.len() as u16);
        buffer.put_slice(response_str.as_bytes());

        let mut bytes = buffer.freeze();
        let result = read_response(&mut bytes).unwrap();

        assert_eq!(result.edition, "MCPE");
        assert_eq!(result.motd, "Test Server");
        assert_eq!(result.protocol_version, 589);
        assert_eq!(result.version, "1.20.0");
        assert_eq!(result.current_players, 10);
        assert_eq!(result.max_players, 20);
        assert_eq!(result.server_id, "1234567890");
        assert_eq!(result.map_name, "world");
        assert_eq!(result.game_mode, "Survival");
        assert_eq!(result.numeric_id, Some(1));
        assert_eq!(result.port, Some(19132));
        assert_eq!(result.unknown_val, Some(123));
    }

    #[test]
    fn test_read_response_minimal() {
        let mut buffer = BytesMut::new();
        buffer.put_u8(0x02);
        buffer.put_u64(1234567890);
        buffer.put_u64(9876543210);
        buffer.put_u128(MAGIC);

        let response_str = "MCPE;Test;589;1.20.0;0;20;1234567890;world;Creative";
        buffer.put_u16(response_str.len() as u16);
        buffer.put_slice(response_str.as_bytes());

        let mut bytes = buffer.freeze();
        let result = read_response(&mut bytes).unwrap();

        assert_eq!(result.edition, "MCPE");
        assert_eq!(result.motd, "Test");
        assert_eq!(result.game_mode, "Creative");
        assert_eq!(result.numeric_id, None);
        assert_eq!(result.port, None);
        assert_eq!(result.unknown_val, None);
    }

    #[test]
    fn test_read_response_insufficient_data() {
        let mut buffer = BytesMut::new();
        buffer.put_u8(0x02);
        buffer.put_u64(1234567890);
        // Missing fields

        let mut bytes = buffer.freeze();
        let result = read_response(&mut bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_response_invalid_string_size() {
        let mut buffer = BytesMut::new();
        buffer.put_u8(0x02);
        buffer.put_u64(1234567890);
        buffer.put_u64(9876543210);
        buffer.put_u128(MAGIC);
        buffer.put_u16(100); // String size larger than actual data
        buffer.put_slice(b"short");

        let mut bytes = buffer.freeze();
        let result = read_response(&mut bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_response_invalid_utf8() {
        let mut buffer = BytesMut::new();
        buffer.put_u8(0x02);
        buffer.put_u64(1234567890);
        buffer.put_u64(9876543210);
        buffer.put_u128(MAGIC);
        buffer.put_u16(4);
        buffer.put_slice(&[0xFF, 0xFF, 0xFF, 0xFF]); // Invalid UTF-8

        let mut bytes = buffer.freeze();
        let result = read_response(&mut bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_response_too_few_parts() {
        let mut buffer = BytesMut::new();
        buffer.put_u8(0x02);
        buffer.put_u64(1234567890);
        buffer.put_u64(9876543210);
        buffer.put_u128(MAGIC);

        let response_str = "MCPE;Test;589"; // Less than 8 parts
        buffer.put_u16(response_str.len() as u16);
        buffer.put_slice(response_str.as_bytes());

        let mut bytes = buffer.freeze();
        let result = read_response(&mut bytes);
        assert!(result.is_err());
    }
}
