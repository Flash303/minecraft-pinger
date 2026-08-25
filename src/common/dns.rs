use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use hickory_resolver::proto::rr::RData;
use log::debug;

use crate::common::ip_filter::IpFilter;
use crate::MinecraftPinger;
use crate::error::PingError;

pub(crate) async fn resolve_filtered_addrs(pinger: &MinecraftPinger,
                                           host: &str,
                                           default_port: u16,
                                           protocol: &str,
                                           ip_filter: Option<&Arc<IpFilter>>) -> Result<Vec<SocketAddr>, PingError> {
    let addrs = resolve_all_addrs(pinger, host, default_port, protocol).await?;

    let Some(filter) = ip_filter.map(Arc::as_ref) else {
        return Ok(addrs);
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

    Ok(allowed)
}

async fn resolve_all_addrs(pinger: &MinecraftPinger,
                           host: &str,
                           default_port: u16,
                           protocol: &str) -> Result<Vec<SocketAddr>, PingError> {
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Ok(vec![SocketAddr::new(ip, default_port)]);
    }

    let srv_record = format!("_minecraft._{}.{}", protocol, host);
    if let Ok(lookup) = pinger.dns_resolver.srv_lookup(srv_record.as_str()).await {
        let mut all_addrs = Vec::new();
        for record in lookup.answers() {
            if let RData::SRV(srv) = &record.data {
                if let Ok(ip_lookup) = pinger.dns_resolver.lookup_ip(srv.target.clone()).await {
                    for ip in ip_lookup.iter() {
                        all_addrs.push(SocketAddr::new(ip, srv.port));
                    }
                }
            }
        }
        if !all_addrs.is_empty() {
            return Ok(all_addrs);
        }
    }

    // Fallback
    let ip_lookup = pinger.dns_resolver.lookup_ip(host).await
        .map_err(|e| PingError::DnsParse(e))?;

    let all_addrs: Vec<SocketAddr> = ip_lookup.iter().map(|ip| SocketAddr::new(ip, default_port)).collect();
    if all_addrs.is_empty() {
        return Err(PingError::DnsIpNotFound);
    }
    Ok(all_addrs)
}
