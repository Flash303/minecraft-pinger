# Minecraft Pinger

A high-performance, asynchronous Rust library for pinging Minecraft servers to retrieve status, players, and metadata.

## Features

- **Asynchronous I/O**: Built on top of `tokio` for maximum efficiency.
- **Optimized Performance**: Uses `BufReader`/`BufWriter` and zero-copy string parsing.
- **Robust DNS Support**: Full support for SRV record resolution (`_minecraft._tcp`).
- **Rich Metadata**: Parses MOTD (plain and JSON components), player samples, versions, and ModInfo (Forge/Fabric).
- **Resource Efficient**: Shared DNS resolver to minimize overhead during batch pings.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
minecraft-pinger = { git = "https://github.com/Flash303/minecraft-pinger" }
tokio = { version = "1.0", features = ["full"] }
```

## Quick Start

```rust
use minecraft_pinger::{MinecraftPinger, PingConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize the pinger (manages shared resources like DNS resolver)
    let pinger = MinecraftPinger::new()
        .map_err(|e| format!("Failed to init pinger: {}", e))?;

    // 2. Configure your ping settings (optional)
    let config = PingConfig {
        timeout: Duration::from_secs(3),
        protocol_version: 763, // 1.20.1
        ..Default::default()
    };

    // 3. Ping a server
    match pinger.ping_server("mc.hypixel.net", 25565, config).await {
        Ok(response) => {
            println!("Server: {}", response.version.name);
            println!("Players: {}/{}", response.players.online, response.players.max);
            println!("MOTD: {}", response.description.to_plain_text());
        }
        Err(e) => eprintln!("Ping failed: {:?}", e),
    }

    Ok(())
}
```

## Configuration

| Field | Type | Description |
|-------|------|-------------|
| `protocol_version` | `i32` | Protocol ID sent in the handshake. |
| `timeout` | `Duration` | Maximum time to wait for a response. |
| `hostname` | `Option<String>` | Override the hostname sent in the handshake (defaults to the IP provided). |

## Error Handling

The library uses a custom `PingError` enum to categorize failures:
- `ConnectionRefused`: Network-level connection issues.
- `TimeoutError`: Request exceeded the configured duration.
- `SerializationError`: Failed to parse the server's JSON response.
- `ReadPacketError`: Protocol-level parsing failure.

## License

MIT
