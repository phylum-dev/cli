//! Ip address utilities to determine if a address is routable beyond the local
//! network segment or localhost.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

mod ipv4 {
    use std::net::Ipv4Addr;

    use cidr::Ipv4Cidr;
    use static_init::dynamic;

    #[dynamic(lazy)]
    static HOST_LOOPBACK: Ipv4Cidr = Ipv4Cidr::new(Ipv4Addr::new(127, 0, 0, 0), 8).unwrap();
    #[dynamic(lazy)]
    static LINK_LOCAL: Ipv4Cidr = Ipv4Cidr::new(Ipv4Addr::new(169, 254, 0, 0), 16).unwrap();
    #[dynamic(lazy)]
    static SOFTWARE_SCOPE: Ipv4Cidr = Ipv4Cidr::new(Ipv4Addr::new(0, 0, 0, 0), 8).unwrap();

    /// Determine if a address is possibly routable beyond the local network
    /// segment. This method considers ANY ip address that is not software scope
    /// (0.0.0.0 / ::::), loopback, or link_local to be potentially routable
    pub fn is_routable(ip_address: &Ipv4Addr) -> bool {
        let is_not_routable = HOST_LOOPBACK.contains(ip_address)
            || LINK_LOCAL.contains(ip_address)
            || SOFTWARE_SCOPE.contains(ip_address);
        !is_not_routable
    }
}

mod ipv6 {
    use std::net::Ipv6Addr;

    use cidr::Ipv6Cidr;
    use static_init::dynamic;

    #[dynamic(lazy)]
    static SOFTWARE_SCOPE: Ipv6Cidr =
        Ipv6Cidr::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0), 128).unwrap();
    #[dynamic(lazy)]
    static HOST_LOOPBACK: Ipv6Cidr =
        Ipv6Cidr::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1), 128).unwrap();
    #[dynamic(lazy)]
    static LINK_LOCAL: Ipv6Cidr =
        Ipv6Cidr::new(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 0), 10).unwrap();

    /// Determine if a address is possibly routable beyond the local network
    /// segment. This method considers ANY ip address that is not software scope
    /// (0.0.0.0 / ::::), loopback, or link_local to be potentially routable
    pub fn is_routable(ip_address: &Ipv6Addr) -> bool {
        let is_not_routable = HOST_LOOPBACK.contains(ip_address)
            || LINK_LOCAL.contains(ip_address)
            || SOFTWARE_SCOPE.contains(ip_address);
        !is_not_routable
    }
}

pub trait IpAddrExt {
    /// Determine if a address is possibly routable beyond the local network
    /// segment. This method considers ANY ip address that is not software scope
    /// (0.0.0.0 / ::::), loopback, or link_local to be potentially routable
    fn is_routable(&self) -> bool;
}

impl IpAddrExt for IpAddr {
    fn is_routable(&self) -> bool {
        match self {
            Self::V4(ipv4) => ipv4.is_routable(),
            Self::V6(ipv6) => ipv6.is_routable(),
        }
    }
}

impl IpAddrExt for Ipv4Addr {
    fn is_routable(&self) -> bool {
        ipv4::is_routable(self)
    }
}

impl IpAddrExt for Ipv6Addr {
    fn is_routable(&self) -> bool {
        ipv6::is_routable(self)
    }
}
