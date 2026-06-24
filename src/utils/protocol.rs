use crate::error::PingError;
use bytes::{Buf, BufMut, Bytes, BytesMut};

pub const NEXT_BYTE_MASK: u32 = 0xFFFFFF80;
pub const CONTINUATION_BIT: u8 = 0x80;
pub const DATA_MASK: u8 = 0x7F;

pub fn write_var_int(buffer: &mut BytesMut, x: i32) {
    let mut ux = x as u32; // cast en u32 pour le logical shift
    while (ux & NEXT_BYTE_MASK) != 0 {
        buffer.put_u8((ux as u8 & DATA_MASK) | CONTINUATION_BIT); // (ux as u8 & DATA_MASK) get the 7 bits of data, | CONTINUATION_BIT set the continuation to 1
        ux >>= 7; // remove the 7 bits
    }
    buffer.put_u8(ux as u8); // put the last, often 0x000..
}

pub fn write_string(buffer: &mut BytesMut, string: &str) {
    write_var_int(buffer, string.len() as i32);
    buffer.put_slice(string.as_bytes());
}

pub fn read_var_int(buf: &mut Bytes) -> Result<i32, PingError> {
    let mut result = 0i32;
    let mut shift = 0;
    loop {
        let byte = buf.try_get_u8().map_err(|_| PingError::ReadPacketError)?;
        result |= ((byte & DATA_MASK) as i32) << shift;
        if byte & CONTINUATION_BIT == 0 {
            break;
        }
        shift += 7;

        // Security
        if shift >= 35 {
            return Err(PingError::ReadPacketError);
        }
    }
    Ok(result)
}

// Todo: work on code duplication
pub async fn read_var_int_custom<F>(mut extractor: F) -> Result<i32, PingError>
where F: AsyncFnMut() -> Result<u8, PingError>
{
    let mut result = 0i32;
    let mut shift = 0;
    loop {
        let byte = extractor().await?;
        result |= ((byte & DATA_MASK) as i32) << shift;
        if byte & CONTINUATION_BIT == 0 {
            break;
        }
        shift += 7;

        // Security
        if shift >= 35 {
            return Err(PingError::ReadPacketError);
        }
    }
    Ok(result)
}

pub fn read_string(buf: &mut Bytes) -> Result<String, PingError> {
    let len = read_var_int(buf)? as usize;
    if buf.remaining() < len {
        return Err(PingError::ReadPacketError);
    }
    let bytes = buf.split_to(len);
    String::from_utf8(bytes.into())
        .map_err(|_| PingError::ReadPacketError)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn test_var_int_serialization() {
        let mut buf = BytesMut::new();
        write_var_int(&mut buf, 255);
        let mut data = buf.freeze();
        assert_eq!(read_var_int(&mut data).unwrap(), 255);
    }
}
