/*

{
    "upstream": {
        "10.0.0.17": {
            "mac": "01:02:03:04:05:06",
            "brd": "10.0.0.255",
        }
    },

    "forward": {
        "0.0.0.0:8888": "10.0.0.17:4444",
        "0.0.0.0:9000-9010": "10.0.0.17:10000-10010"
    }
}

*/

use std::{
    collections::HashMap,
    net::{AddrParseError, IpAddr},
    num::ParseIntError,
    ops::Range,
    str::FromStr,
};

use serde::Deserialize;
use thiserror::Error;

use crate::hardware_addr::HardwareAddr;

#[derive(Deserialize)]
pub struct Config {
    pub upstream: HashMap<IpAddr, Upstream>,
    pub forward: HashMap<ForwardSpec, ForwardSpec>,
}

#[derive(Deserialize)]
pub struct Upstream {
    pub mac: HardwareAddr,
    pub brd: IpAddr,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PortSpec {
    Single(u16),
    Range(Range<u16>),
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ForwardSpec {
    pub addr: IpAddr,
    pub port: PortSpec,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ForwardParseError {
    #[error("missing separator ':'")]
    MissingSeparator,

    #[error("invalid address: {0}")]
    InvalidAddress(AddrParseError),

    #[error("invalid port: {0}")]
    InvalidPort(ParseIntError),
}

impl FromStr for ForwardSpec {
    type Err = ForwardParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (addr, port) = s
            .split_once(':')
            .ok_or(ForwardParseError::MissingSeparator)?;

        let addr = addr
            .parse::<IpAddr>()
            .map_err(ForwardParseError::InvalidAddress)?;

        let port = match port.split_once('-') {
            None => {
                let port: u16 = port.parse().map_err(ForwardParseError::InvalidPort)?;
                PortSpec::Single(port)
            }
            Some((start, end)) => {
                let start: u16 = start.parse().map_err(ForwardParseError::InvalidPort)?;
                let end: u16 = end.parse().map_err(ForwardParseError::InvalidPort)?;

                PortSpec::Range(Range {
                    start,
                    end: end + 1,
                })
            }
        };

        Ok(Self { addr, port })
    }
}

impl<'de> serde::Deserialize<'de> for ForwardSpec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}
