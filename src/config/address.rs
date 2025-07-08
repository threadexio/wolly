use std::net::{AddrParseError, IpAddr};
use std::num::ParseIntError;
use std::ops::Range;
use std::str::FromStr;

use thiserror::Error;

#[derive(Debug, Clone)]
#[debug("{ip}:{port:?}")]
pub struct Address {
    pub ip: IpAddr,
    pub port: Port,
}

#[derive(Debug, Clone)]
pub enum Port {
    #[debug("{_0}")]
    Single(u16),
    #[debug("{}-{}", _0.start, _0.end - 1)]
    Range(Range<u16>),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParseAddressError {
    #[error("missing ':' separator")]
    MissingSeparator,

    #[error("{0}")]
    InvalidAddress(AddrParseError),

    #[error("{0}")]
    InvalidPort(ParseIntError),
}

impl FromStr for Address {
    type Err = ParseAddressError;

    // Syntax: <ip>:<port> | <ip>:<port>-<port>
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use ParseAddressError::*;

        let (ip, port) = s.split_once(':').ok_or(MissingSeparator)?;

        let ip = ip.parse().map_err(InvalidAddress)?;

        let port = match port.split_once('-') {
            None => {
                let port = port.parse().map_err(InvalidPort)?;

                Port::Single(port)
            }

            Some((start, end)) => {
                let start: u16 = start.parse().map_err(InvalidPort)?;
                let end: u16 = end.parse().map_err(InvalidPort)?;

                if start == end {
                    Port::Single(start)
                } else {
                    Port::Range(start..end + 1)
                }
            }
        };

        Ok(Self { ip, port })
    }
}
