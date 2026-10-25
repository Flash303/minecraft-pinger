use crate::common::protocol::{
    CONTINUATION_BIT, DATA_MASK, read_var_int, write_string, write_var_int,
};
use crate::error::PingError;
use crate::error::PingError::ReadPacket;
use bytes::{Buf, BufMut, BytesMut};
use log::debug;
use tokio::io::AsyncReadExt;

const MAX_PACKET_SIZE: usize = 1 << 20;

// Packets
pub(crate) fn write_ping_handshake(
    buffer: &mut BytesMut,
    hostname: &str,
    port: &u16,
    protocol_version: &i32,
) {
    let mut handshake = BytesMut::with_capacity(128);
    handshake.put_u8(0x00);
    write_var_int(&mut handshake, *protocol_version); // protocol version
    write_string(&mut handshake, hostname);
    handshake.put_u16(*port); // Server Port
    write_var_int(&mut handshake, 1); // next state = 1 status

    write_var_int(buffer, handshake.len() as i32);
    buffer.put(handshake.freeze());
}

pub(crate) fn write_ping_request(buffer: &mut BytesMut) {
    write_var_int(buffer, 1); // length
    write_var_int(buffer, 0x00); // packet id
}

pub(crate) struct Packet {
    id: u8,
    pub data: BytesMut,
}

impl Packet {
    pub fn new(id: u8, data: BytesMut) -> Packet {
        Packet { id, data }
    }

    pub fn id(&self) -> u8 {
        self.id
    }
}

fn decode_var_int_prefix(buf: &[u8]) -> Result<Option<(i32, usize)>, PingError> {
    let mut result = 0i32;
    for (index, byte) in buf.iter().enumerate().take(5) {
        result |= ((*byte & DATA_MASK) as i32) << (index * 7);
        if *byte & CONTINUATION_BIT == 0 {
            return Ok(Some((result, index + 1)));
        }
    }
    if buf.len() >= 5 {
        return Err(ReadPacket("Shift security"));
    }
    Ok(None)
}

#[inline]
async fn fill_buf<R: AsyncReadExt + Unpin>(
    stream: &mut R,
    buf: &mut BytesMut,
) -> Result<(), PingError> {
    if stream.read_buf(buf).await.map_err(|e| {
        debug!("Read packet error 2 {e}");
        e
    })? == 0
    {
        return Err(ReadPacket("Unexpected EOF"));
    }
    Ok(())
}

pub(crate) async fn read_packet<R: AsyncReadExt + Unpin>(
    stream: &mut R,
) -> Result<Packet, PingError> {
    let mut buf = BytesMut::with_capacity(256);

    // Reads in large chunks: the first read already holds
    // the length VarInt and usually the whole packet.
    let length = loop {
        if let Some((value, width)) = decode_var_int_prefix(&buf)? {
            buf.advance(width);
            break value;
        }
        fill_buf(stream, &mut buf).await?;
    };
    if length < 0 || length as usize > MAX_PACKET_SIZE {
        return Err(ReadPacket("packet too big"));
    }

    let length = length as usize;
    buf.reserve(length.saturating_sub(buf.len()));

    // The payload start may already sit in memory,
    // so only the missing bytes get fetched.
    while buf.len() < length {
        fill_buf(stream, &mut buf).await?;
    }

    // Take the whole packet without splitting: split would share the
    // allocation and force a copy later when building the JSON string.
    let mut data = std::mem::replace(&mut buf, BytesMut::new());
    data.truncate(length);
    let packet_id = read_var_int(&mut data)? as u8;

    Ok(Packet::new(packet_id, data))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn test_write_ping_handshake() {
        let mut buffer = BytesMut::new();
        write_ping_handshake(&mut buffer, "example.com", &25565, &775);

        // Should have length prefix + handshake data
        assert!(!buffer.is_empty());

        // Verify we can read the length
        let mut data = buffer.clone().freeze();
        let length = read_var_int(&mut data).unwrap();
        assert_eq!(length as usize, data.len());
    }

    #[test]
    fn test_write_ping_request() {
        let mut buffer = BytesMut::new();
        write_ping_request(&mut buffer);

        let mut data = buffer.freeze();
        let length = read_var_int(&mut data).unwrap();
        assert_eq!(length, 1); // packet length

        let packet_id = read_var_int(&mut data).unwrap();
        assert_eq!(packet_id, 0x00); // status request packet id
    }

    #[test]
    fn test_packet_new_and_id() {
        let packet = Packet::new(0x01, BytesMut::from(&b"test"[..]));
        assert_eq!(packet.id(), 0x01);
    }

    #[tokio::test]
    async fn test_read_packet_valid() {
        let mut buffer = BytesMut::new();
        write_ping_request(&mut buffer);
        let data = buffer.freeze();

        let mut reader = &data[..];
        let packet = read_packet(&mut reader).await.unwrap();

        assert_eq!(packet.id(), 0x00);
    }

    #[tokio::test]
    async fn test_read_packet_invalid_length() {
        let data = BytesMut::from(
            &[
                0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x01,
            ][..],
        );
        let mut reader = &data[..];
        let result = read_packet(&mut reader).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_packet_too_big() {
        // VarInt(MAX_PACKET_SIZE + 1): rejected before any payload read.
        let data = BytesMut::from(&[0x81, 0x80, 0x40, 0x00][..]);
        let mut reader = &data[..];
        let result = read_packet(&mut reader).await;
        assert!(matches!(
            result,
            Err(PingError::ReadPacket("packet too big"))
        ));
    }

    #[tokio::test]
    async fn test_read_packet_negative_length() {
        // VarInt(-1): rejected by the same guard.
        let data = BytesMut::from(&[0xFF, 0xFF, 0xFF, 0xFF, 0x0F][..]);
        let mut reader = &data[..];
        let result = read_packet(&mut reader).await;
        assert!(matches!(
            result,
            Err(PingError::ReadPacket("packet too big"))
        ));
    }
}
