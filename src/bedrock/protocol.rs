use crate::bedrock::model::BedrockPing;
use crate::error::PingError;
use bytes::{BufMut, Bytes, BytesMut};
use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};

const MAGIC: u128 = 0x00ffff00fefefefefdfdfdfd12345678;

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

pub(crate) fn read_response(buffer: &mut &[u8]) -> Result<BedrockPing, PingError> {
    let mut take = |len: usize, err: &'static str| -> Result<&[u8], PingError> {
        if buffer.len() < len {
            return Err(PingError::ReadPacket(err));
        }
        let (head, tail) = buffer.split_at(len);
        *buffer = tail;
        Ok(head)
    };

    let _packet_id = take(1, "Packet id not found")?[0];
    let _timestamp = u64::from_be_bytes(take(8, "Timestamp not found")?.try_into().unwrap());
    let _server_guid = u64::from_be_bytes(take(8, "Server guid not found")?.try_into().unwrap());
    let _magic = u128::from_be_bytes(take(16, "Magic id not found")?.try_into().unwrap());

    let string_size = u16::from_be_bytes(take(2, "String size not found")?.try_into().unwrap()) as usize;

    let string_data = take(string_size, "String data not found")?;

    let str = String::from_utf8(string_data.to_vec())?;

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

        let bytes = buffer.freeze();
        let mut slice: &[u8] = &bytes;
        let result = read_response(&mut slice).unwrap();

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

        let bytes = buffer.freeze();
        let mut slice: &[u8] = &bytes;
        let result = read_response(&mut slice).unwrap();

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

        let bytes = buffer.freeze();
        let mut slice: &[u8] = &bytes;
        let result = read_response(&mut slice);
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

        let bytes = buffer.freeze();
        let mut slice: &[u8] = &bytes;
        let result = read_response(&mut slice);
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

        let bytes = buffer.freeze();
        let mut slice: &[u8] = &bytes;
        let result = read_response(&mut slice);
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

        let bytes = buffer.freeze();
        let mut slice: &[u8] = &bytes;
        let result = read_response(&mut slice);
        assert!(result.is_err());
    }
}
