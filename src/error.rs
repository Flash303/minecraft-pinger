use std::string::FromUtf8Error;
use hickory_resolver::net::NetError;
use thiserror::Error;
use tokio::time::error::Elapsed;

#[derive(Debug, Error)]
pub enum PingError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Global ping timeout for {0}")]
    Timeout(#[from] Elapsed),

    #[error("Connection refused")]
    ConnectionRefused,
    
    #[error("Failed to send packet")]
    SendPacket,
    
    #[error("Failed to read packet: {0}")]
    ReadPacket(String),
    
    #[error("Invalid UTF-8: {0}")]
    Utf8Error(#[from] FromUtf8Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("DNS parse error: {0}")]
    DnsParse(#[source] NetError),
    
    #[error("DNS IP not found")]
    DnsIpNotFound,
    
    #[error("Address parse error: {0}")]
    AddressParse(#[source] NetError),
    
    #[error("Failed to parse response")]
    ParseResponse,

    #[error("Initialization error: {0}")]
    Init(String),
}