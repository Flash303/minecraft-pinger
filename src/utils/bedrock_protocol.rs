use bytes::{Buf, BufMut, Bytes, BytesMut};
use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::error::PingError;
use crate::models::bedrock_model::BedrockPing;

const MAGIC: u128 = 0x00ffff00fefefefefdfdfdfd12345678;

fn try_split_to(buffer: &mut Bytes, size: usize) -> Option<Bytes> {
    if buffer.len() >= size {
        Some(buffer.split_to(size))
    } else {
        None
    }
}

pub fn create_ping() -> Bytes {
    let mut data = BytesMut::with_capacity(33);
    data.put_u8(0x1);
    data.put_u64(SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64);
    data.put_u128(MAGIC);

    let mut guid = [0u8; 8];
    rand::rng().fill_bytes(&mut guid);

    data.put_slice(&guid);

    data.freeze()
}

pub fn read_response(buffer: &mut Bytes) -> Result<BedrockPing, PingError> {
    let _packet_id = buffer.try_get_u8()
        .map_err(|_| PingError::ReadPacketError)?;
    let _timestamp = buffer.try_get_u64().map_err(|_| PingError::ReadPacketError)?;
    let _server_guid = buffer.try_get_u64().map_err(|_| PingError::ReadPacketError)?;
    let _magic = buffer.try_get_u128().map_err(|_| PingError::ReadPacketError)?;

    let string_size = buffer.try_get_u16().map_err(|_| PingError::ReadPacketError)?;
    let string_data = try_split_to(buffer, string_size as usize).ok_or(PingError::ReadPacketError)?;
    let str = String::from_utf8(string_data.into()).unwrap();

    // let bedrock_rs = BedrockPacket {
    //     packet_id,
    //     timestamp,
    //     server_guid,
    //     magic,
    //     str_size: string_size,
    //     string: str
    // };

    Ok(str.try_into()?)
}

// struct BedrockPacket {
//     packet_id: u8,
//     timestamp: u64,
//     server_guid: u64,
//     magic: u128,
//     str_size: u16,
//     string: String
// }