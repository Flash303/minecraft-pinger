use std::net::{IpAddr, SocketAddr};
use hickory_resolver::proto::rr::RData;
use crate::MinecraftPinger;
use crate::error::PingError;

pub(crate) async fn resolve_to_addr(pinger: &MinecraftPinger, host: &str, default_port: u16, protocol: &str) -> Result<SocketAddr, PingError> {
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Ok(SocketAddr::new(ip, default_port));
    }

    let srv_record = format!("_minecraft._{}.{}", protocol, host);
    if let Ok(lookup) = pinger.dns_resolver.srv_lookup(srv_record.as_str()).await {
        for record in lookup.answers() {
            if let RData::SRV(srv) = &record.data {
                if let Ok(ip_lookup) = pinger.dns_resolver.lookup_ip(srv.target.clone()).await {
                    if let Some(ip) = ip_lookup.iter().next() {
                        return Ok(SocketAddr::new(ip, srv.port));
                    }
                }
            }
        }
    }

    // Fallback
    let ip_lookup = pinger.dns_resolver.lookup_ip(host).await
        .map_err(|e| PingError::DnsParse(e))?;
    
    let ip = ip_lookup.iter().next().ok_or(PingError::DnsIpNotFound)?;
    Ok(SocketAddr::new(ip, default_port))
}