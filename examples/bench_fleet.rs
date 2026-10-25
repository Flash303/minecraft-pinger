//! Fleet bench: pings every server in a JSON list and writes results.
//! Usage: bench_fleet <servers.json> <out.json> [limit] [concurrency]
//! Input: [{"ip": "...", "port": 25565, "type": "java"|"bedrock"}, ...]
//! Same file is used against the published crate (old API) and this repo.
use minecraft_pinger::MinecraftPinger;
use minecraft_pinger::config::PingConfig;
use minecraft_pinger::java::config::JavaPingConfig;
use std::time::{Duration, Instant};

#[derive(serde::Deserialize)]
struct Server {
    ip: String,
    port: u16,
    #[serde(rename = "type")]
    kind: String,
}

#[derive(serde::Serialize)]
struct Row {
    ip: String,
    port: u16,
    kind: String,
    ok: bool,
    latency_ms: Option<u32>,
    elapsed_ms: u64,
    err: Option<String>,
}

#[derive(serde::Serialize)]
struct Report {
    wall_ms: u64,
    success: usize,
    failed: usize,
    rows: Vec<Row>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let list_path = args.get(1).cloned().expect("missing <servers.json>");
    let out_path = args.get(2).cloned().expect("missing <out.json>");
    let limit: usize = args
        .get(3)
        .map(|s| s.parse().expect("bad limit"))
        .unwrap_or(usize::MAX);
    let concurrency: usize = args
        .get(4)
        .map(|s| s.parse().expect("bad concurrency"))
        .unwrap_or(64);

    let data = std::fs::read_to_string(&list_path).expect("read server list");
    let mut servers: Vec<Server> = serde_json::from_str(&data).expect("parse server list");
    servers.truncate(limit);
    eprintln!(
        "bench_fleet: {} servers, concurrency {}",
        servers.len(),
        concurrency
    );

    let pinger = MinecraftPinger::new().expect("pinger");
    let config = PingConfig::builder()
        .set_timeout(Duration::from_secs(4))
        .build();
    let java_config = JavaPingConfig::from(&config)
        .set_protocol_version(777)
        .build();

    let t0 = Instant::now();
    let mut set = tokio::task::JoinSet::new();
    let mut rows: Vec<Row> = Vec::with_capacity(servers.len());

    for s in servers {
        while set.len() >= concurrency {
            rows.push(set.join_next().await.expect("task").expect("row"));
        }
        let pinger = pinger.clone();
        let config = config.clone();
        let java_config = java_config.clone();
        set.spawn(async move {
            let t = Instant::now();
            let base =
                |ok: bool, latency_ms: Option<u32>, err: Option<String>, elapsed_ms: u64| Row {
                    ip: s.ip.clone(),
                    port: s.port,
                    kind: s.kind.clone(),
                    ok,
                    latency_ms,
                    elapsed_ms,
                    err,
                };
            if s.kind == "bedrock" {
                match pinger.ping_bedrock_server(&s.ip, s.port, &config).await {
                    Ok(r) => base(true, Some(r.latency), None, t.elapsed().as_millis() as u64),
                    Err(e) => base(
                        false,
                        None,
                        Some(format!("{e:?}")),
                        t.elapsed().as_millis() as u64,
                    ),
                }
            } else {
                match pinger.ping_java_server(&s.ip, s.port, &java_config).await {
                    Ok(r) => base(true, Some(r.latency), None, t.elapsed().as_millis() as u64),
                    Err(e) => base(
                        false,
                        None,
                        Some(format!("{e:?}")),
                        t.elapsed().as_millis() as u64,
                    ),
                }
            }
        });
    }
    while let Some(r) = set.join_next().await {
        rows.push(r.expect("row"));
    }

    let report = Report {
        wall_ms: t0.elapsed().as_millis() as u64,
        success: rows.iter().filter(|r| r.ok).count(),
        failed: rows.iter().filter(|r| !r.ok).count(),
        rows,
    };
    std::fs::write(&out_path, serde_json::to_string(&report).unwrap()).expect("write report");
    eprintln!(
        "done: {}/{} ok in {} ms -> {}",
        report.success,
        report.success + report.failed,
        report.wall_ms,
        out_path
    );
}
