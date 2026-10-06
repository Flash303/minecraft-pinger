use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use hickory_resolver::proto::rr::RData;
use log::debug;

use crate::MinecraftPinger;
use crate::common::ip_filter::IpFilter;
use crate::error::PingError;

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
        return Err(PingError::BlockedEndpoint(format!(
            "every resolved address was rejected: {:?}",
            rejected.iter().map(|a| a.ip()).collect::<Vec<_>>()
        )));
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
    if let Ok(lookup) = pinger.dns_resolver.srv_lookup(srv_record.as_str()).await {
        let mut override_hostname: Option<String> = None;

        let mut all_addrs = Vec::new();
        for record in lookup.answers() {
            if let RData::SRV(srv) = &record.data {
                let srv_target = srv.target.to_string().trim_end_matches('.').to_string();
                // if the srv target is not an ip, we use the subdomain as hostname
                if !srv_target.parse::<IpAddr>().is_ok() {
                    override_hostname = Some(srv_target.clone());
                }

                if let Ok(ip_lookup) = pinger.dns_resolver.lookup_ip(srv.target.clone()).await {
                    let ips: Vec<SocketAddr> = ip_lookup
                        .iter()
                        .map(|ip| SocketAddr::new(ip, srv.port))
                        .collect();

                    all_addrs.extend(ips);
                }
            }
        }
        if !all_addrs.is_empty() {
            return Ok((override_hostname, all_addrs));
        }
    }

    // Fallback - Resolve from A or AAAA ... Not SRV
    let ip_lookup = pinger
        .dns_resolver
        .lookup_ip(host)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::ip_filter::is_public_ip;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::sync::Arc;

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
}
