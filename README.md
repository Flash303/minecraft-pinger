<p align="center">
  <img src=".github/assets/logo.webp" alt="Minecraft Pinger Logo" width="256" />
</p>

<h1 align="center">Minecraft Pinger</h1>

A high-performance, asynchronous Rust library for pinging Minecraft servers to retrieve status, players, and metadata.

## ✨ Features

- **Asynchronous I/O**: Built on top of `tokio` for maximum efficiency.
- **Feature Flags**: Modular design with `java` and `bedrock` features (both enabled by default).
- **Java & Bedrock Support**: Supports pinging both Java and Bedrock (MCPE) edition servers.
- **Optimized Performance**: Zero-copy packet framing, single-pass MOTD deserialization, and a single deadline timer per ping (see [Performance](#-performance)).
- **Robust DNS Support**: Full support for SRV record resolution (`_minecraft._tcp`), resolved concurrently.
- **Rich Metadata**: Parses MOTD (plain and JSON components), player samples, versions, and ModInfo (Forge/Fabric).
- **Resource Efficient**: Shared DNS resolver to minimize overhead during batch pings; optional shared UDP socket for Bedrock scans; optional skipping of heavy fields (`favicon`, player sample).
- **Security**: Built-in IP filtering capability to prevent SSRF (Server-Side Request Forgery) attacks by blocking private or reserved IP addresses.

## ⚙️ Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
minecraft-pinger = "1.0"
tokio = { version = "1.0", features = ["full"] }
```

> **Note**: Both `java` and `bedrock` features are enabled by default. If you only need one, you can disable the default features:
> ```toml
> [dependencies]
> minecraft-pinger = { version = "1.0", default-features = false, features = ["java"] }
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

    let java_config = JavaPingConfig::from(&config)
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

### Fleet scans: skip heavy fields (Java)

The `favicon` (base64, ~20–130 KB) and the player `sample` list dominate
parsing cost. Skip what you don't display:

```rust
let java_config = JavaPingConfig::from(&config)
    .skip_favicon()
    .skip_sample()
    .build();
```

### Fleet scans: share one UDP socket (Bedrock)

Binding a socket per ping burns an ephemeral port each time. Share one
(or a pool of) sockets instead:

```rust
use tokio::net::UdpSocket;
use std::sync::Arc;

let socket = Arc::new(UdpSocket::bind("0.0.0.0:0").await.unwrap());
let pong = pinger.ping_bedrock_with_socket(&socket, "play.example.net", 19132, &config).await?;
```

> Replies are matched by sender address. Concurrent pings to the *same*
> address over one socket cannot be told apart (the client GUID is not
> echoed by the pong) — use one socket per concurrent target in that case.

## 🛠️ Configuration

Configuration is managed via `PingConfig` for common properties and `JavaPingConfig` for Java-specific options, both utilizing the Builder pattern.

### PingConfig

| Builder Method | Type | Description |
|----------------|------|-------------|
| `set_timeout` | `Duration` | Maximum time budget for a whole ping (DNS + connect + I/O). |
| `deny_non_public_ips` | `self` | Rejects non-public IPs (e.g. 127.0.0.1, 192.168.0.1), preventing SSRF attacks. |
| `set_ip_filter` | `Fn(IpAddr) -> bool` | Apply a custom filter to resolved IPs. Returning `false` blocks the connection. |

### JavaPingConfig

Created using `JavaPingConfig::from(&config)` or `JavaPingConfig::builder()`.

| Builder Method | Type | Description |
|----------------|------|-------------|
| `set_protocol_version` | `i32` | Protocol ID sent in the handshake. |
| `set_hostname` | `Option<String>` | Override the hostname sent in the handshake (defaults to the IP provided). |
| `skip_favicon` | `self` | Skip parsing the `favicon` field (saves a ~20–130 KB allocation per server). |
| `skip_sample` | `self` | Skip parsing the `players.sample` list. |

## ⏱️ Performance

Measured 2026-10-25, release profile, same machine/network, alternating
order (see `examples/bench_fleet.rs`):

**Fleet: 10 rounds × 500 public servers (488 Java + 12 Bedrock), concurrency 64, timeout 4 s**

| | crates.io 0.1.10 | 1.0 (this) |
|---|---|---|
| Wall time (mean of 10 runs) | 6328 ms | **6170 ms (−2,5 %)** |
| Success (mean of 10 runs) | 469,5/500 | **470,1/500** |
| Servers OK in all 10 runs | 431 | **439 (+8)** |
| Mean per-ping time (4681 paired samples) | 298,8 ms | **278,9 ms (−6,7 %)** |
| Median / p90 / p95 / p99 | 219 / 591 / 1107 / 1519 ms | **209 / 544 / 1090 / 1377 ms** |
| Faster on same server | — | **54 % of samples** |

Reliability detail: 16 servers succeed more often with 1.0 (including
several that reset the connection mid-handshake — the old byte-at-a-time
reader lost the race, the new bulk reader gets the payload first), versus
10 in the other direction (network flakiness noise).

**Heap (`dhat`, same workload both sides, 2000 iterations):**

| Path | 0.1.10 | 1.0 |
|---|---|---|
| Full ping path (framing + Bedrock + serde + MOTD text) | 4524 B / 51 blocks per iter | **2420 B / 28 blocks (−46 %)** |
| Serde status parse | 2491 B / 25 blocks per parse | **1245 B / 12 blocks (−50 %)** |
| Serde parse with `skip_favicon` + `skip_sample` | n/a | **900 B / 9 blocks per parse** |

## 🚨 Error Handling

The library uses a custom `PingError` enum to categorize failures:
- `Io`: Underlying I/O error.
- `Timeout`: Request exceeded the configured duration.
- `ConnectionRefused`: Network-level connection issues.
- `SendPacket`: Failed to send the request packet.
- `ReadPacket`: Protocol-level parsing failure.
- `ParseResponse`: Bedrock pong did not match the expected shape.
- `Serialization`: Failed to parse the server's JSON response.
- `BlockedEndpoint(Vec<IpAddr>)`: Every resolved IP was rejected by the IP filter (lists the culprits).
- `DnsParse` / `DnsIpNotFound` / `AddressParse`: DNS resolution failures.
- `Init`: Resolver initialization failure.

## 🔼 Migrating from 0.x to 1.0

Breaking changes (the reason for the major bump):

- `JavaPingConfig::from(&PingConfigBuilder)` → `JavaPingConfig::from(&PingConfig)`.
  `PingConfig::to_builder()` was removed; build the `PingConfig` first, then derive from it:
  ```rust
  // before
  let java_config = JavaPingConfig::from(&config.to_builder()).build();
  // after
  let java_config = JavaPingConfig::from(&config).build();
  ```
- `PingError::BlockedEndpoint(String)` → `BlockedEndpoint(Vec<IpAddr>)`: match on a list of rejected IPs instead of a pre-formatted string.
- `PingError::ReadPacket(String)` → `ReadPacket(&'static str)`: no allocation on the error path.
- `BedrockPing::try_from` now takes `&str` (a `TryFrom<String>` shim delegates to it).
- Dependency `rand` removed (Bedrock GUIDs use an atomic counter).

## 📄 License

MIT
