use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use hickory_resolver::net::NetError;
use hickory_resolver::proto::ProtoError;
use hickory_resolver::proto::rr::{Name, RData};
use log::debug;
use tokio::task::JoinSet;

use crate::MinecraftPinger;
use crate::common::ip_filter::IpFilter;
use crate::error::PingError;

fn is_ldh(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_' || byte == b'.'
}

fn parse_host(host: &str) -> Result<Name, PingError> {
    if host.contains(['\\', '*']) {
        return Err(PingError::DnsParse(NetError::Proto(ProtoError::from(
            format!("hostname contains an illegal character: {host}"),
        ))));
    }

    if host.is_ascii() && host.bytes().all(is_ldh) {
        return Name::from_ascii(host).map_err(|e| PingError::DnsParse(e.into()));
    }

    Name::from_utf8(host).map_err(|e| PingError::DnsParse(e.into()))
}

pub(crate) async fn resolve_filtered_addrs(
    pinger: &MinecraftPinger,
    host: &str,
    default_port: u16,
    protocol: &str,
    ip_filter: Option<&Arc<IpFilter>>,
) -> Result<(Option<String>, Vec<SocketAddr>), PingError> {
    let (override_hostname, addrs) =
        resolve_all_addrs(pinger, host, default_port, protocol).await?;

    let Some(filter) = ip_filter.map(Arc::as_ref) else {
        return Ok((override_hostname, addrs));
    };

    let (allowed, rejected): (Vec<SocketAddr>, Vec<SocketAddr>) =
        addrs.into_iter().partition(|addr| filter(addr.ip()));

    if !rejected.is_empty() {
        debug!("IP filter rejected addresses: {:?}", rejected);
    }

    if allowed.is_empty() {
        return Err(PingError::BlockedEndpoint(
            rejected.iter().map(|addr| addr.ip()).collect(),
        ));
    }

    Ok((override_hostname, allowed))
}

async fn resolve_all_addrs(
    pinger: &MinecraftPinger,
    host: &str,
    default_port: u16,
    protocol: &str,
) -> Result<(Option<String>, Vec<SocketAddr>), PingError> {
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Ok((None, vec![SocketAddr::new(ip, default_port)]));
    }

    // SRV resolve
    let srv_record = format!("_minecraft._{}.{}", protocol, host);
    if let Ok(srv_name) = parse_host(&srv_record)
        && let Ok(lookup) = pinger.dns_resolver.srv_lookup(srv_name).await
    {
        // Every SRV target resolves concurrently: awaiting them one by one
        // would stack a full DNS round-trip per record.
        let resolver = Arc::clone(&pinger.dns_resolver);
        let targets: Vec<(usize, Name, u16)> = lookup
            .answers()
            .iter()
            .enumerate()
            .filter_map(|(index, record)| match &record.data {
                RData::SRV(srv) => Some((index, srv.target.clone(), srv.port)),
                _ => None,
            })
            .collect();
        let lookup_ip = Arc::new(move |target: Name, port: u16| {
            let resolver = Arc::clone(&resolver);
            async move {
                resolver
                    .lookup_ip(target)
                    .await
                    .map(|ip_lookup| {
                        ip_lookup
                            .iter()
                            .map(|ip| SocketAddr::new(ip, port))
                            .collect()
                    })
                    .unwrap_or_default()
            }
        });

        let resolved = lookup_all(targets, lookup_ip).await;
        let (override_hostname, all_addrs) = collect_srv_addrs(resolved);
        if !all_addrs.is_empty() {
            return Ok((override_hostname, all_addrs));
        }
    }

    // Fallback - Resolve from A or AAAA ... Not SRV
    let host_name = parse_host(host)?;
    let ip_lookup = pinger
        .dns_resolver
        .lookup_ip(host_name)
        .await
        .map_err(PingError::DnsParse)?;

    let all_addrs: Vec<SocketAddr> = ip_lookup
        .iter()
        .map(|ip| SocketAddr::new(ip, default_port))
        .collect();
    if all_addrs.is_empty() {
        return Err(PingError::DnsIpNotFound);
    }
    Ok((None, all_addrs))
}

async fn lookup_all<F, Fut>(
    targets: Vec<(usize, Name, u16)>,
    lookup: Arc<F>,
) -> Vec<(usize, Name, Vec<SocketAddr>)>
where
    F: Fn(Name, u16) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Vec<SocketAddr>> + Send + 'static,
{
    let mut lookups = JoinSet::new();
    for (index, target, port) in targets {
        let lookup = Arc::clone(&lookup);
        lookups.spawn(async move {
            let addrs = lookup(target.clone(), port).await;
            (index, target, addrs)
        });
    }

    let mut resolved = Vec::new();
    while let Some(outcome) = lookups.join_next().await {
        if let Ok(entry) = outcome {
            resolved.push(entry);
        }
    }
    resolved
}

fn collect_srv_addrs(
    mut resolved: Vec<(usize, Name, Vec<SocketAddr>)>,
) -> (Option<String>, Vec<SocketAddr>) {
    // Tasks finish in any order: restore the SRV answer order first.
    resolved.sort_by_key(|entry| entry.0);

    let mut override_hostname = None;
    let mut all_addrs = Vec::new();
    for (_, target, addrs) in resolved {
        let srv_target = target.to_string().trim_end_matches('.').to_owned();
        // if the srv target is not an ip, we use the subdomain as hostname
        if srv_target.parse::<IpAddr>().is_err() {
            override_hostname = Some(srv_target);
        }
        all_addrs.extend(addrs);
    }
    (override_hostname, all_addrs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::ip_filter::is_public_ip;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::sync::Arc;

    #[test]
    fn test_parse_host_accepts_underscore_in_label() {
        let name = parse_host("my_server.example.lan").unwrap();
        assert_eq!(name.to_string(), "my_server.example.lan");
    }

    #[test]
    fn test_parse_host_accepts_srv_prefix() {
        let name = parse_host("_minecraft._tcp.example.com").unwrap();
        assert_eq!(name.to_string(), "_minecraft._tcp.example.com");
    }

    #[test]
    fn test_parse_host_accepts_idn() {
        let name = parse_host("münchen.example.com").unwrap();
        assert_eq!(name.to_ascii(), "xn--mnchen-3ya.example.com");
    }

    #[test]
    fn test_parse_host_accepts_trailing_dot() {
        let name = parse_host("example.com.").unwrap();
        assert!(name.is_fqdn());
    }

    #[test]
    fn test_parse_host_rejects_backslash_escape() {
        assert!(parse_host("attacker.com\\.trusted.com").is_err());
        assert!(parse_host("a\\_b.com").is_err());
    }

    #[test]
    fn test_parse_host_rejects_wildcard() {
        assert!(parse_host("*.example.com").is_err());
        assert!(parse_host("*ac.example.com").is_err());
    }

    #[test]
    fn test_parse_host_rejects_control_and_separators() {
        assert!(parse_host("a b.com").is_err());
        assert!(parse_host("a;b.com").is_err());
        assert!(parse_host("a\nb.com").is_err());
        assert!(parse_host("a\tb.com").is_err());
        assert!(parse_host("a\0b.com").is_err());
        assert!(parse_host("user@host.com").is_err());
        assert!(parse_host("a\\/b.com").is_err());
    }

    #[test]
    fn test_parse_host_rejects_oversized_label() {
        let long = format!("{}.com", "a".repeat(64));
        assert!(parse_host(&long).is_err());
    }

    #[test]
    fn test_parse_host_rejects_empty_label() {
        assert!(parse_host(".example.com").is_err());
        assert!(parse_host("a..b.com").is_err());
    }

    #[test]
    fn test_parse_host_error_is_dns_parse() {
        assert!(matches!(parse_host("a b.com"), Err(PingError::DnsParse(_))));
    }

    #[test]
    fn test_ip_filter_partition() {
        let addrs = vec![
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), 25565),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 25565),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 25565),
        ];

        let filter = Arc::new(is_public_ip);
        let (allowed, rejected): (Vec<_>, Vec<_>) =
            addrs.into_iter().partition(|addr| filter(addr.ip()));

        assert_eq!(allowed.len(), 1);
        assert_eq!(allowed[0].ip(), IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)));
        assert_eq!(rejected.len(), 2);
    }

    #[test]
    fn test_ip_filter_all_rejected() {
        let addrs = vec![
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 25565),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 25565),
        ];

        let filter = Arc::new(is_public_ip);
        let (allowed, rejected): (Vec<_>, Vec<_>) =
            addrs.into_iter().partition(|addr| filter(addr.ip()));

        assert!(allowed.is_empty());
        assert_eq!(rejected.len(), 2);
    }

    #[test]
    fn test_ip_filter_none_returns_all() {
        let addrs = vec![
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), 25565),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 25565),
        ];

        let filter: Option<&Arc<IpFilter>> = None;
        // When filter is None, all addresses should be allowed
        let (allowed, rejected): (Vec<_>, Vec<_>) = addrs
            .into_iter()
            .partition(|addr| filter.is_none_or(|f| f(addr.ip())));

        assert_eq!(allowed.len(), 2);
        assert!(rejected.is_empty());
    }

    #[test]
    fn test_collect_srv_addrs_restores_answer_order() {
        let resolved = vec![
            (
                1,
                parse_host("mc2.example.com").unwrap(),
                vec![SocketAddr::new(
                    IpAddr::V4(Ipv4Addr::new(9, 9, 9, 9)),
                    25566,
                )],
            ),
            (
                0,
                parse_host("mc1.example.com").unwrap(),
                vec![SocketAddr::new(
                    IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
                    25565,
                )],
            ),
        ];

        let (hostname, addrs) = collect_srv_addrs(resolved);

        assert_eq!(hostname, Some("mc2.example.com".to_string()));
        assert_eq!(
            addrs,
            vec![
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), 25565),
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(9, 9, 9, 9)), 25566),
            ]
        );
    }

    #[test]
    fn test_collect_srv_addrs_empty() {
        let (hostname, addrs) = collect_srv_addrs(Vec::new());
        assert!(hostname.is_none());
        assert!(addrs.is_empty());
    }

    #[tokio::test]
    async fn test_srv_lookups_run_concurrently() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::time::Duration;

        let active = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let lookup = {
            let active = Arc::clone(&active);
            let peak = Arc::clone(&peak);
            Arc::new(move |target: Name, port: u16| {
                let active = Arc::clone(&active);
                let peak = Arc::clone(&peak);
                async move {
                    let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(current, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    active.fetch_sub(1, Ordering::SeqCst);
                    let _ = target;
                    vec![SocketAddr::new(
                        IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
                        port,
                    )]
                }
            })
        };

        let targets = vec![
            (0, parse_host("mc1.example.com").unwrap(), 25565),
            (1, parse_host("mc2.example.com").unwrap(), 25566),
            (2, parse_host("mc3.example.com").unwrap(), 25567),
        ];
        let resolved = lookup_all(targets, lookup).await;
        assert_eq!(peak.load(Ordering::SeqCst), 3);

        let (hostname, addrs) = collect_srv_addrs(resolved);
        assert_eq!(hostname, Some("mc3.example.com".to_string()));
        assert_eq!(
            addrs.iter().map(|addr| addr.port()).collect::<Vec<_>>(),
            vec![25565, 25566, 25567]
        );
    }

    #[tokio::test]
    async fn test_resolve_ip_literal_skips_dns() {
        let pinger = MinecraftPinger::new().unwrap();
        let (hostname, addrs) = resolve_all_addrs(&pinger, "127.0.0.1", 25565, "tcp")
            .await
            .unwrap();
        assert!(hostname.is_none());
        assert_eq!(
            addrs,
            vec![SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                25565
            )]
        );
    }

    #[tokio::test]
    async fn test_resolve_filtered_addrs_blocks_private_ip() {
        let pinger = MinecraftPinger::new().unwrap();
        let filter: Arc<IpFilter> = Arc::new(is_public_ip);
        let result =
            resolve_filtered_addrs(&pinger, "127.0.0.1", 25565, "tcp", Some(&filter)).await;
        assert!(matches!(result, Err(PingError::BlockedEndpoint(_))));
    }
}
