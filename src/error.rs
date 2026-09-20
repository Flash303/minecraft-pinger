use hickory_resolver::net::NetError;
use std::string::FromUtf8Error;
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

    #[error("Endpoint blocked by IP filter: {0}")]
    BlockedEndpoint(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::io;

    #[test]
    fn test_ping_error_io() {
        let io_error = io::Error::new(io::ErrorKind::ConnectionRefused, "connection refused");
        let ping_error: PingError = io_error.into();

        match ping_error {
            PingError::Io(_) => {}
            _ => panic!("Expected Io error"),
        }
        assert!(ping_error.to_string().contains("I/O error"));
    }

    #[test]
    fn test_ping_error_timeout() {
        // Test that the timeout error type exists
        // Note: Elapsed has no public constructor, so we can't easily create this variant
        // We just verify the enum variant exists
        let _ = PingError::Timeout;
        // If this compiles, the variant exists
    }

    #[test]
    fn test_ping_error_connection_refused() {
        let ping_error = PingError::ConnectionRefused;
        assert_eq!(ping_error.to_string(), "Connection refused");
    }

    #[test]
    fn test_ping_error_send_packet() {
        let ping_error = PingError::SendPacket;
        assert_eq!(ping_error.to_string(), "Failed to send packet");
    }

    #[test]
    fn test_ping_error_read_packet() {
        let ping_error = PingError::ReadPacket("test error".to_string());
        assert_eq!(ping_error.to_string(), "Failed to read packet: test error");
    }

    #[test]
    fn test_ping_error_utf8_error() {
        let utf8_error = String::from_utf8(vec![0xFF, 0xFE]).unwrap_err();
        let ping_error: PingError = utf8_error.into();

        match ping_error {
            PingError::Utf8Error(_) => {}
            _ => panic!("Expected Utf8Error"),
        }
        assert!(ping_error.to_string().contains("Invalid UTF-8"));
    }

    #[test]
    fn test_ping_error_serialization() {
        let json_error = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let ping_error: PingError = json_error.into();

        match ping_error {
            PingError::Serialization(_) => {}
            _ => panic!("Expected Serialization error"),
        }
        assert!(ping_error.to_string().contains("Serialization error"));
    }

    #[test]
    fn test_ping_error_dns_parse() {
        let ping_error = PingError::DnsParse("test".into());
        assert!(ping_error.to_string().contains("DNS parse error"));
    }

    #[test]
    fn test_ping_error_dns_ip_not_found() {
        let ping_error = PingError::DnsIpNotFound;
        assert_eq!(ping_error.to_string(), "DNS IP not found");
    }

    #[test]
    fn test_ping_error_address_parse() {
        let ping_error = PingError::AddressParse("test".into());
        assert!(ping_error.to_string().contains("Address parse error"));
    }

    #[test]
    fn test_ping_error_parse_response() {
        let ping_error = PingError::ParseResponse;
        assert_eq!(ping_error.to_string(), "Failed to parse response");
    }

    #[test]
    fn test_ping_error_init() {
        let ping_error = PingError::Init("test init error".to_string());
        assert_eq!(
            ping_error.to_string(),
            "Initialization error: test init error"
        );
    }

    #[test]
    fn test_ping_error_blocked_endpoint() {
        let ping_error = PingError::BlockedEndpoint("10.0.0.1".to_string());
        assert_eq!(
            ping_error.to_string(),
            "Endpoint blocked by IP filter: 10.0.0.1"
        );
    }

    #[test]
    fn test_ping_error_debug() {
        let ping_error = PingError::ConnectionRefused;
        let debug_str = format!("{:?}", ping_error);
        assert!(debug_str.contains("ConnectionRefused"));
    }

    #[test]
    fn test_ping_error_source() {
        let io_error = io::Error::new(io::ErrorKind::ConnectionRefused, "test");
        let ping_error: PingError = io_error.into();
        assert!(Error::source(&ping_error).is_some());
    }
}
