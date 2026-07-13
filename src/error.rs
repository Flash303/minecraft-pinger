use hickory_resolver::net::NetError;
use thiserror::Error;
use tokio::time::error::Elapsed;

#[derive(Debug, Error)]
pub enum PingError {
    #[error("Global ping timeout for {0}")]
    Timeout(#[from] Elapsed),

    #[error("ConnectionRefused")]
    ConnectionRefused,
    
    #[error("SendPacket")]
    SendPacket,
    
    #[error("ReadPacket: {0}")]
    ReadPacket(String),
    
    #[error("Serialization")]
    Serialization,
    
    #[error("DnsParse")]
    DnsParse(NetError),
    
    #[error("Dns ip not found")]
    DnsIpNotFound(),
    
    #[error("AddressParse")]
    AddressParse(#[from] NetError),
    
    #[error("ParseResponse")]
    ParseResponse,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("DnsResolverError: {0}")]
    DnsResolverError(#[from] NetError),
}