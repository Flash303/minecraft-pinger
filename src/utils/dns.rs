use crate::MinecraftPinger;
use hickory_resolver::proto::rr::RData;
use std::net::IpAddr;

pub async fn resolve_srv(pinger: &MinecraftPinger, ip: &str, default_port: u16) -> (String, u16) {
    if ip.parse::<IpAddr>().is_ok() {
        return (ip.to_string(), default_port);
    }

    let srv_record = format!("_minecraft._tcp.{}", ip);

    if let Ok(lookup) = pinger.dns_resolver.srv_lookup(srv_record.as_str()).await {
        for record in lookup.answers() {
            if let RData::SRV(srv) = &record.data {
                let target = srv.target.to_string();
                let clean_target = target.trim_end_matches('.').to_string();

                return (clean_target, srv.port);
            }
        }
    }

    // Fallback
    (ip.to_string(), default_port)
}