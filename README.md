<p align="center">
  <img src=".github/assets/logo.webp" alt="Minecraft Pinger Logo" width="256" />
</p>

<h1 align="center">Minecraft Pinger</h1>

A high-performance, asynchronous Rust library for pinging Minecraft servers to retrieve status, players, and metadata.

## ✨ Features

- **Asynchronous I/O**: Built on top of `tokio` for maximum efficiency.
- **Feature Flags**: Modular design with `java` and `bedrock` features (both enabled by default).
- **Java & Bedrock Support**: Supports pinging both Java and Bedrock (MCPE) edition servers.
- **Optimized Performance**: Uses `BufReader`/`BufWriter` and zero-copy string parsing.
- **Robust DNS Support**: Full support for SRV record resolution (`_minecraft._tcp`).
- **Rich Metadata**: Parses MOTD (plain and JSON components), player samples, versions, and ModInfo (Forge/Fabric).
- **Resource Efficient**: Shared DNS resolver to minimize overhead during batch pings.
- **Security**: Built-in IP filtering capability to prevent SSRF (Server-Side Request Forgery) attacks by blocking private or reserved IP addresses.

## ⚙️ Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
minecraft-pinger = { git = "https://github.com/Flash303/minecraft-pinger" }
tokio = { version = "1.0", features = ["full"] }
```

> **Note**: Both `java` and `bedrock` features are enabled by default. If you only need one, you can disable the default features:
> ```toml
> [dependencies]
> minecraft-pinger = { git = "https://github.com/Flash303/minecraft-pinger", default-features = false, features = ["java"] }
> ```

## 🚀 Quick Start

```rust
use minecraft_pinger::MinecraftPinger;
use minecraft_pinger::config::PingConfig;
use minecraft_pinger::java::config::JavaPingConfig;
use std::time::Duration;

#[tokio::main]
async fn main() {
    // 1. Initialize the pinger (manages shared resources like DNS resolver)
    let pinger = MinecraftPinger::new().expect("Failed to create pinger");

    // 2. Configure your ping settings using the Builder pattern
    let config = PingConfig::builder()
        .set_timeout(Duration::from_secs(3))
        .deny_non_public_ips() // Prevent SSRF by dropping local/private IPs
        .build();

    let java_config = JavaPingConfig::from(&config.to_builder())
        .set_protocol_version(763) // 1.20.1
        .build();

    // 3. Ping a Java server
    match pinger.ping_java_server("mc.hypixel.net", 25565, &java_config).await {
        Ok(response) => {
            println!("Server: {}", response.version.name);
            println!("Players: {}/{}", response.players.online, response.players.max);
            println!("MOTD: {}", response.description.to_plain_text());
        }
        Err(e) => eprintln!("Java Ping failed: {:?}", e),
    }
    
    // 4. Ping a Bedrock server
    match pinger.ping_bedrock_server("bedrock.nationsglory.fr", 19132, &config).await {
        Ok(pong) => {
            println!("Bedrock Server: {}", pong.motd);
            println!("Bedrock Players: {}/{}", pong.current_players, pong.max_players);
        }
        Err(e) => eprintln!("Bedrock Ping failed: {:?}", e),
    }
}
```

## 🛠️ Configuration

Configuration is managed via `PingConfig` for common properties and `JavaPingConfig` for Java-specific options, both utilizing the Builder pattern.

### PingConfig

| Builder Method | Type | Description |
|----------------|------|-------------|
| `set_timeout` | `Duration` | Maximum time to wait for a response. |
| `deny_non_public_ips` | `self` | Rejects non-public IPs (e.g. 127.0.0.1, 192.168.0.1), preventing SSRF attacks. |
| `set_ip_filter` | `Fn(IpAddr) -> bool` | Apply a custom filter to resolved IPs. Returning `false` blocks the connection. |

### JavaPingConfig

Created using `JavaPingConfig::from(&config.to_builder())` or `JavaPingConfig::builder()`.

| Builder Method | Type | Description |
|----------------|------|-------------|
| `set_protocol_version` | `i32` | Protocol ID sent in the handshake. |
| `set_hostname` | `Option<String>` | Override the hostname sent in the handshake (defaults to the IP provided). |

## 🚨 Error Handling

The library uses a custom `PingError` enum to categorize failures:
- `ConnectionRefused`: Network-level connection issues.
- `TimeoutError`: Request exceeded the configured duration.
- `SerializationError`: Failed to parse the server's JSON response.
- `ReadPacketError`: Protocol-level parsing failure.
- `BlockedEndpoint`: The resolved IP was blocked by the configured IP filter.

## 📄 License

MIT
