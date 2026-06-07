use bytes::{Buf, BufMut, Bytes, BytesMut};
use log::debug;
use tokio::io::{AsyncReadExt, BufReader};
use tokio::net::tcp::{OwnedReadHalf};
use crate::error::PingError;

pub fn write_var_int(buffer: &mut BytesMut, x: i32) {
    let mut ux = x as u32; // cast en u32 pour le logical shift
    while (ux & 0xFFFFFF80) != 0 {
        buffer.put_u8((ux as u8 & 0x7F) | 0x80);
        ux >>= 7;
    }
    buffer.put_u8(ux as u8);
}

pub fn write_string(buffer: &mut BytesMut, string: &str) {
    write_var_int(buffer, string.len() as i32);
    buffer.put_slice(string.as_bytes());
}

pub fn read_var_int(buf: &mut Bytes) -> Result<i32, PingError> {
    let mut result = 0i32;
    let mut shift = 0;
    loop {
        let byte = buf.try_get_u8()
            .map_err(|_| PingError::ReadPacketError)?;
        result |= ((byte & 0x7F) as i32) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
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

// Packets
pub fn create_ping_handshake(hostname: &String, port: &u16, protocol_version: &i32) -> Bytes {
    let mut handshake = BytesMut::new();
    handshake.put_u8(0x00);
    write_var_int(&mut handshake, *protocol_version); // protocol version
    write_string(&mut handshake, hostname);
    handshake.put_u16(*port); // Server Port
    write_var_int(&mut handshake, 1); // next state = 1 status

    create_packet_header(handshake.freeze())
}

pub fn create_ping_request() -> Bytes {
    let mut packet = BytesMut::new();
    write_var_int(&mut packet, 1); // lenght
    write_var_int(&mut packet, 0x00); // packet id
    
    packet.freeze()
}

fn create_packet_header(packet: Bytes) -> Bytes {
    let mut data = BytesMut::new();
    write_var_int(&mut data, packet.len() as i32);
    data.extend_from_slice(&packet);
    data.freeze()
}

pub struct Packet {
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

pub async fn read_packet(stream: &mut BufReader<OwnedReadHalf>) -> Result<Packet, PingError> {
    // Lire le varint de longueur directement depuis le stream bufferisé
    let mut length: i32 = 0;
    let mut shift = 0;
    loop {
        let byte = stream.read_u8()
            .await
            .map_err(|_| PingError::ReadPacketError)?;
        length |= ((byte & 0x7F) as i32) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 35 {
            return Err(PingError::ReadPacketError);
        }
    }

    // Lire exactement `length` bytes
    let mut buf = vec![0u8; length as usize];
    stream.read_exact(&mut buf)
        .await
        .map_err(|e| {
        debug!("Read packet error 2 {e}");
        PingError::ReadPacketError
    })?;
    
    let mut data = Bytes::from(buf);
    
    Ok(Packet::new(read_var_int(&mut data)? as u8, data))
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
