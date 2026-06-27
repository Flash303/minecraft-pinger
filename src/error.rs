#[derive(Debug)]
pub enum PingError {
    ConnectionRefused,
    SendPacketError,
    ReadPacketError(String),
    SerializationError,
    AddressParseError,
    TimeoutError,
    ParseResponseError,
}