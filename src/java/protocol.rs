use crate::common::protocol::{read_var_int, read_var_int_stream, write_string, write_var_int};
use crate::error::PingError;
use bytes::{BufMut, Bytes, BytesMut};
use log::debug;
use tokio::io::AsyncReadExt;

// Packets
pub(crate) fn write_ping_handshake(buffer: &mut BytesMut, hostname: &str, port: &u16, protocol_version: &i32) {
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
    pub data: Bytes,
}

impl Packet {
    pub fn new(id: u8, data: Bytes) -> Packet {
        Packet { id, data }
    }

    pub fn id(&self) -> u8 {
        self.id
    }
}

pub(crate) async fn read_packet<R: AsyncReadExt + Unpin>(stream: &mut R) -> Result<Packet, PingError> {
    let length = read_var_int_stream(stream).await?;

    // Lire exactement `length` bytes
    let mut buf = vec![0u8; length as usize];
    stream.read_exact(&mut buf)
        .await
        .map_err(|e| {
            debug!("Read packet error 2 {e}");
            PingError::ReadPacket(e.to_string())
        })?;

    let mut data = Bytes::from(buf);

    Ok(Packet::new(read_var_int(&mut data)? as u8, data))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;
    use tokio::io::BufReader;

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
        let packet = Packet::new(0x01, Bytes::from_static(b"test"));
        assert_eq!(packet.id(), 0x01);
    }

    #[tokio::test]
    async fn test_read_packet_valid() {
        let mut buffer = BytesMut::new();
        write_ping_request(&mut buffer);
        let data = buffer.freeze();
        
        let mut reader = BufReader::new(&data[..]);
        let packet = read_packet(&mut reader).await.unwrap();
        
        assert_eq!(packet.id(), 0x00);
    }

    #[tokio::test]
    async fn test_read_packet_invalid_length() {
        let data = Bytes::from_static(&[0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x01]);
        let mut reader = BufReader::new(&data[..]);
        let result = read_packet(&mut reader).await;
        assert!(result.is_err());
    }
}