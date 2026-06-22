use bytes::{BufMut, Bytes, BytesMut};
use log::debug;
use tokio::io::AsyncReadExt;
use crate::error::PingError;
use crate::utils::protocol::{read_var_int, write_string, write_var_int, DATA_MASK};

const CONTINUATION_BIT: u8 = 0x80;

// Packets
pub fn write_ping_handshake(buffer: &mut BytesMut, hostname: &str, port: &u16, protocol_version: &i32) {
    let mut handshake = BytesMut::with_capacity(128);
    handshake.put_u8(0x00);
    write_var_int(&mut handshake, *protocol_version); // protocol version
    write_string(&mut handshake, hostname);
    handshake.put_u16(*port); // Server Port
    write_var_int(&mut handshake, 1); // next state = 1 status

    write_var_int(buffer, handshake.len() as i32);
    buffer.put(handshake.freeze());
}

pub fn write_ping_request(buffer: &mut BytesMut) {
    write_var_int(buffer, 1); // length
    write_var_int(buffer, 0x00); // packet id
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

pub async fn read_packet<R: AsyncReadExt + Unpin>(stream: &mut R) -> Result<Packet, PingError> {
    // Lire le varint de longueur directement depuis le stream bufferisé
    let mut length: i32 = 0;
    let mut shift = 0;
    loop {
        let byte = stream.read_u8()
            .await
            .map_err(|_| PingError::ReadPacketError)?;
        length |= ((byte & DATA_MASK) as i32) << shift;
        if byte & CONTINUATION_BIT == 0 {
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