use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    time::Duration,
};

use crate::error::Error;
use trust_dns_resolver::{
    config::ResolverOpts as TrustResolverOpts,
    config::{NameServerConfig, ResolverConfig as TrustResolverConfig},
    lookup_ip::LookupIp as TrustLookupIp,
    Resolver as TrustResolver,
};

#[derive(Debug, Clone, Default)]
pub struct Config {
    doh_servers: Vec<(SocketAddr, String)>,
    bind_address: Option<IpAddr>,
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_suggested_servers(mut self) -> Self {
        self.add_doh_server(
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(9, 9, 9, 10), 443)),
            "dns10.quad9.net",
        );
        self.add_doh_server(
            SocketAddr::V6(SocketAddrV6::new(
                Ipv6Addr::new(0x2620, 0xfe, 0, 0, 0, 0, 0, 0x10),
                443,
                0,
                0,
            )),
            "dns10.quad9.net",
        );

        self.add_doh_server(
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(1, 1, 1, 1), 443)),
            "cloudflare-dns.com",
        );
        self.add_doh_server(
            SocketAddr::V6(SocketAddrV6::new(
                Ipv6Addr::new(0x2606, 0x4700, 0x4700, 0, 0, 0, 0, 0x1111),
                443,
                0,
                0,
            )),
            "cloudflare-dns.com",
        );

        self.add_doh_server(
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(8, 8, 8, 8), 443)),
            "dns.google",
        );
        self.add_doh_server(
            SocketAddr::V6(SocketAddrV6::new(
                Ipv6Addr::new(0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8888),
                443,
                0,
                0,
            )),
            "dns.google",
        );
        self
    }

    pub fn doh_servers(&self) -> &[(SocketAddr, String)] {
        self.doh_servers.as_ref()
    }

    pub fn set_doh_servers(&mut self, servers: Vec<(SocketAddr, String)>) -> &mut Self {
        self.doh_servers = servers;
        self
    }

    pub fn add_doh_server<N: Into<String>>(&mut self, address: SocketAddr, domain: N) -> &mut Self {
        self.doh_servers.push((address, domain.into()));
        self
    }

    pub fn bind_address(&self) -> Option<IpAddr> {
        self.bind_address
    }

    pub fn set_bind_address(&mut self, address: Option<IpAddr>) -> &mut Self {
        self.bind_address = address;
        self
    }
}

pub struct Resolver {
    inner: TrustResolver,
}

impl Resolver {
    pub fn new(config: Config) -> Result<Self, Error> {
        let mut trust_config = TrustResolverConfig::new();

        for doh_server in config.doh_servers {
            let protocol = trust_dns_resolver::config::Protocol::Https;
            let mut name_server = NameServerConfig::new(doh_server.0, protocol);
            name_server.tls_dns_name = Some(doh_server.1);
            name_server.bind_addr = config.bind_address.map(|v| SocketAddr::new(v, 0));

            trust_config.add_name_server(name_server);
        }

        let mut trust_options = TrustResolverOpts::default();
        trust_options.timeout = Duration::from_secs(20);
        trust_options.use_hosts_file = false;

        let inner = TrustResolver::new(trust_config, trust_options)?;

        Ok(Self { inner })
    }

    pub fn lookup_ip_address<S: AsRef<str>>(&self, name: S) -> Result<IpAddressLookup, Error> {
        let span = tracing::info_span!("resolver lookup IP address", name = name.as_ref());
        let _guard = span.enter();

        tracing::debug!("lookup IP address start");

        let lookup = self.inner.lookup_ip(name.as_ref())?;

        tracing::debug!(len = lookup.iter().count(), "lookup IP address ok");

        Ok(IpAddressLookup {
            addresses: lookup.iter().collect(),
            inner: lookup,
        })
    }
}

#[derive(Debug, Clone)]
pub struct IpAddressLookup {
    inner: TrustLookupIp,
    addresses: Vec<IpAddr>,
}

impl IpAddressLookup {
    pub fn ip_addresses(&self) -> &[IpAddr] {
        self.addresses.as_ref()
    }

    pub fn to_record_string(&self) -> String {
        let mut buf = String::new();
        for record in self.inner.as_lookup().records() {
            buf.push_str(&record.to_string());
            buf.push_str("\r\n");
        }
        buf
    }
}
