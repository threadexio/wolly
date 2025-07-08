use std::net::{IpAddr, SocketAddr};
use std::ops::Range;

#[derive(Debug, Display, Clone, PartialEq, Eq)]
pub enum Mapping {
    #[display("{from}:{to}")]
    OneToOne { from: SocketAddr, to: SocketAddr },

    #[display("{from_ip}:{}-{}:{to}", from_ports.start, from_ports.end)]
    ManyToOne {
        from_ip: IpAddr,
        from_ports: Range<u16>,
        to: SocketAddr,
    },

    #[display("{from_ip}:{}-{}:{to_ip}:{}-{}",
        from_port_range_start, from_port_range_start + port_range_len - 1,
        to_port_range_start, to_port_range_start + port_range_len - 1
    )]
    ManyToMany {
        from_ip: IpAddr,
        from_port_range_start: u16,
        to_ip: IpAddr,
        to_port_range_start: u16,
        port_range_len: u16,
    },
}
