use minecraft_pinger::{
    MinecraftPinger, bedrock::model::BedrockPing, common::ip_filter::is_public_ip,
    config::PingConfigBuilder, java::config::JavaPingConfigBuilder,
};
use std::time::Duration;

#[test]
fn test_full_java_config_builder() {
    let config = JavaPingConfigBuilder::new()
        .set_protocol_version(765)
        .set_hostname(Some("custom.example.com".to_string()))
        .set_timeout(Duration::from_secs(5))
        .deny_non_public_ips()
        .build();

    assert_eq!(config.protocol_version(), 765);
    assert_eq!(config.hostname(), &Some("custom.example.com".to_string()));
    assert_eq!(config.common().timeout(), Duration::from_secs(5));
    assert!(config.common().ip_filter().is_some());
}

#[test]
fn test_full_ping_config_builder() {
    let config = PingConfigBuilder::new()
        .set_timeout(Duration::from_millis(500))
        .deny_non_public_ips()
        .build();

    assert_eq!(config.timeout(), Duration::from_millis(500));
    assert!(config.ip_filter().is_some());
}

#[test]
fn test_ip_filter_public_ips() {
    let public_ips = vec![
        "8.8.8.8",
        "1.1.1.1",
        "51.75.20.104",
        "172.32.0.1",
        "100.128.0.1",
        "9.9.9.9",
        "2606:4700:4700::1111",
        "2001:4860:4860::8888",
        "::ffff:8.8.8.8",
    ];

    for ip_str in public_ips {
        let ip = ip_str.parse().unwrap();
        assert!(is_public_ip(ip), "Expected {} to be public", ip_str);
    }
}

#[test]
fn test_ip_filter_private_ips() {
    let private_ips = vec![
        "127.0.0.1",
        "10.0.0.1",
        "172.16.0.1",
        "172.31.255.255",
        "192.168.1.1",
        "169.254.169.254",
        "0.0.0.0",
        "255.255.255.255",
        "100.64.0.1",
        "192.0.2.1",
        "198.51.100.7",
        "203.0.113.9",
        "198.18.0.1",
        "224.0.0.1",
        "240.0.0.1",
        "192.0.0.8",
        "::1",
        "::",
        "fe80::1",
        "fc00::1",
        "fd12:3456:789a::1",
        "ff02::1",
        "2001:db8::1",
        "2001::1",
        "::ffff:127.0.0.1",
        "::ffff:10.0.0.1",
        "::ffff:169.254.169.254",
    ];

    for ip_str in private_ips {
        let ip = ip_str.parse().unwrap();
        assert!(!is_public_ip(ip), "Expected {} to be private", ip_str);
    }
}

#[test]
fn test_bedrock_ping_parsing() {
    let response_str = "MCPE;Test Server;589;1.20.0;10;20;1234567890;world;Survival;1;19132;123";
    let result = BedrockPing::try_from(response_str.to_string()).unwrap();

    assert_eq!(result.edition, "MCPE");
    assert_eq!(result.motd, "Test Server");
    assert_eq!(result.protocol_version, 589);
    assert_eq!(result.version, "1.20.0");
    assert_eq!(result.current_players, 10);
    assert_eq!(result.max_players, 20);
    assert_eq!(result.server_id, "1234567890");
    assert_eq!(result.map_name, "world");
    assert_eq!(result.game_mode, "Survival");
    assert_eq!(result.numeric_id, Some(1));
    assert_eq!(result.port, Some(19132));
    assert_eq!(result.unknown_val, Some(123));
}

#[test]
fn test_minecraft_pinger_creation() {
    let pinger = MinecraftPinger::new();
    assert!(pinger.is_ok());
}

#[test]
fn test_config_builder_chaining() {
    let config = PingConfigBuilder::new()
        .set_timeout(Duration::from_secs(1))
        .set_ip_filter(|_| true)
        .build();

    assert_eq!(config.timeout(), Duration::from_secs(1));
    assert!(config.ip_filter().is_some());
}

#[test]
fn test_java_ping_config_from_ping_config() {
    let base = PingConfigBuilder::new()
        .set_timeout(Duration::from_secs(10))
        .deny_non_public_ips();

    let config = JavaPingConfigBuilder::from(&base).build();

    assert_eq!(config.common().timeout(), Duration::from_secs(10));
    assert!(config.common().ip_filter().is_some());
}
