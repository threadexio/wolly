use std::net::{IpAddr, SocketAddr};
use std::ops::Range;

use super::upstream::ConnectOpts;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MappingKind {
    OneToOne {
        from: SocketAddr,
        to: SocketAddr,
    },

    ManyToOne {
        from_ip: IpAddr,
        from_ports: Range<u16>,
        to: SocketAddr,
    },

    ManyToMany {
        from_ip: IpAddr,
        from_port_range_start: u16,
        to_ip: IpAddr,
        to_port_range_start: u16,
        port_range_len: u16,
    },
}

impl MappingKind {
    pub fn upstream(&self) -> IpAddr {
        match self {
            Self::OneToOne { to, .. } => to.ip(),
            Self::ManyToOne { to, .. } => to.ip(),
            Self::ManyToMany { to_ip, .. } => *to_ip,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Mapping {
    pub kind: MappingKind,
    pub opts: ConnectOpts,
}
