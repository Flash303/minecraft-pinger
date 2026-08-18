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
    let _packet_id = buffer.try_get_u8()
        .map_err(|_| PingError::ReadPacket("Packet id not found".to_string()))?;

    let _timestamp = buffer.try_get_u64()
        .map_err(|_| PingError::ReadPacket("Timestamp not found".to_string()))?;

    let _server_guid = buffer.try_get_u64()
        .map_err(|_| PingError::ReadPacket("Server guid not found".to_string()))?;

    let _magic = buffer.try_get_u128()
        .map_err(|_| PingError::ReadPacket("Magic id not found".to_string()))?;


    let string_size = buffer.try_get_u16()
        .map_err(|_| PingError::ReadPacket("String size not found".to_string()))?;

    let string_data = try_split_to(buffer, string_size as usize)
        .ok_or(PingError::ReadPacket("String data not found".to_string()))?;

    let str = String::from_utf8(string_data.into())?;

    Ok(str.try_into()?)
}