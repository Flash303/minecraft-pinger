use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Predicate applied to resolved IPs before any connection is made.
pub type IpFilter = dyn Fn(IpAddr) -> bool + Send + Sync;

/// Returns true if the IP belongs to the public internet: excludes private
/// ranges, loopback, link-local, CGNAT, documentation, benchmarking, multicast
/// and reserved blocks.
pub fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_public_ipv4(v4),
        IpAddr::V6(v6) => is_public_ipv6(v6),
    }
}

fn is_public_ipv4(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    !(
        // 0.0.0.0/8 - "this network"
        o[0] == 0
        // 10.0.0.0/8 - private
        || o[0] == 10
        // 100.64.0.0/10 - CGNAT / shared address space
        || (o[0] == 100 && (o[1] & 0b1100_0000) == 64)
        // 127.0.0.0/8 - loopback
        || o[0] == 127
        // 169.254.0.0/16 - link-local (incl. cloud metadata endpoints)
        || (o[0] == 169 && o[1] == 254)
        // 172.16.0.0/12 - private
        || (o[0] == 172 && (o[1] & 0xF0) == 16)
        // 192.0.0.0/24 - IETF protocol assignments
        || (o[0] == 192 && o[1] == 0 && o[2] == 0)
        // 192.0.2.0/24 - TEST-NET-1
        || (o[0] == 192 && o[1] == 0 && o[2] == 2)
        // 192.168.0.0/16 - private
        || (o[0] == 192 && o[1] == 168)
        // 198.18.0.0/15 - benchmarking
        || (o[0] == 198 && (o[1] & 0xFE) == 18)
        // 198.51.100.0/24 - TEST-NET-2
        || (o[0] == 198 && o[1] == 51 && o[2] == 100)
        // 203.0.113.0/24 - TEST-NET-3
        || (o[0] == 203 && o[1] == 0 && o[2] == 113)
        // 224.0.0.0/4 multicast + 240.0.0.0/4 reserved (incl. broadcast)
        || o[0] >= 224
    )
}

fn is_public_ipv6(ip: Ipv6Addr) -> bool {
    // ::ffff:x.y.z.w - IPv4-mapped: apply IPv4 rules
    if let Some(mapped) = ip.to_ipv4_mapped() {
        return is_public_ipv4(mapped);
    }

    let s = ip.segments();
    !(
        ip.is_loopback()
        || ip.is_unspecified()
        // ::/8 - reserved (incl. :: and deprecated IPv4-compatible addresses)
        || s[0] == 0
        // fc00::/7 - unique local
        || (s[0] & 0xfe00) == 0xfc00
        // fe80::/10 - link-local
        || (s[0] & 0xffc0) == 0xfe80
        // ff00::/8 - multicast
        || (s[0] & 0xff00) == 0xff00
        // 100::/8 - discard-only / reserved
        || s[0] == 0x0100
        // 2001:0000::/24 - IANA special-purpose block (Teredo, benchmarking,
        // ORCHID...); global allocations start at 2001:0200::
        || (s[0] == 0x2001 && (s[1] & 0xff00) == 0x0000)
        // 2001:db8::/32 - documentation
        || (s[0] == 0x2001 && s[1] == 0x0db8)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;

    fn allowed(s: &str) -> bool {
        is_public_ip(s.parse::<IpAddr>().unwrap())
    }

    #[test]
    fn rejects_private_and_reserved_ipv4() {
        assert!(!allowed("127.0.0.1"));
        assert!(!allowed("10.0.0.1"));
        assert!(!allowed("172.16.0.1"));
        assert!(!allowed("172.31.255.255"));
        assert!(!allowed("192.168.1.1"));
        assert!(!allowed("169.254.169.254")); // cloud metadata endpoint
        assert!(!allowed("0.0.0.0"));
        assert!(!allowed("255.255.255.255"));
        assert!(!allowed("100.64.0.1")); // CGNAT
        assert!(!allowed("192.0.2.1")); // TEST-NET-1
        assert!(!allowed("198.51.100.7")); // TEST-NET-2
        assert!(!allowed("203.0.113.9")); // TEST-NET-3
        assert!(!allowed("198.18.0.1")); // benchmarking
        assert!(!allowed("224.0.0.1")); // multicast
        assert!(!allowed("240.0.0.1")); // reserved
        assert!(!allowed("192.0.0.8")); // IETF assignments
    }

    #[test]
    fn accepts_public_ipv4() {
        assert!(allowed("8.8.8.8"));
        assert!(allowed("1.1.1.1"));
        assert!(allowed("51.75.20.104"));
        assert!(allowed("172.32.0.1")); // outside private range
        assert!(allowed("100.128.0.1")); // outside CGNAT range
        assert!(allowed("9.9.9.9"));
    }

    #[test]
    fn rejects_private_and_reserved_ipv6() {
        assert!(!allowed("::1"));
        assert!(!allowed("::"));
        assert!(!allowed("fe80::1"));
        assert!(!allowed("fc00::1"));
        assert!(!allowed("fd12:3456:789a::1"));
        assert!(!allowed("ff02::1"));
        assert!(!allowed("2001:db8::1"));
        assert!(!allowed("2001::1")); // Teredo
        assert!(!allowed("::ffff:127.0.0.1")); // IPv4-mapped loopback
        assert!(!allowed("::ffff:10.0.0.1"));
        assert!(!allowed("::ffff:169.254.169.254"));
    }

    #[test]
    fn accepts_public_ipv6() {
        assert!(allowed("2606:4700:4700::1111"));
        assert!(allowed("2001:4860:4860::8888"));
        assert!(allowed("::ffff:8.8.8.8")); // mapped public: IPv4 rules apply
    }
}
