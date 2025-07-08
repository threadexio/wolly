use std::net::{AddrParseError, IpAddr};

use thiserror::Error;

use crate::hardware_addr::{self, HardwareAddr};

use super::parse::{Parse, ParseStream};

#[derive(Debug, Clone)]
pub struct Upstream {
    pub address: IpAddr,
    pub mac: HardwareAddr,
    pub broadcast: IpAddr,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParseUpstreamError {
    #[error("expected upstream address")]
    ExpectedAddress,

    #[error("invalid upstream address: {0}")]
    InvalidAddress(AddrParseError),

    #[error("expected mac address")]
    ExpectedMac,

    #[error("invalid mac address: {0}")]
    InvalidMacAddress(hardware_addr::ParseError),

    #[error("expected broadcast address")]
    ExpectedBroadcast,

    #[error("invalid broadcast address: {0}")]
    InvalidBroadcast(AddrParseError),
}

impl Parse for Upstream {
    type Output = Self;
    type Error = ParseUpstreamError;

    // Syntax: upstream <ip> [mac <mac>] [brd <ip>]
    fn parse(stream: &mut ParseStream<'_>) -> Result<Self::Output, Self::Error> {
        use ParseUpstreamError::*;

        let address = stream
            .next()
            .ok_or(ExpectedAddress)?
            .parse()
            .map_err(InvalidAddress)?;

        let mut mac = None;
        let mut broadcast = None;

        while let Some(x) = stream.next() {
            match x {
                "mac" => {
                    mac = Some(
                        stream
                            .next()
                            .ok_or(ExpectedMac)?
                            .parse()
                            .map_err(InvalidMacAddress)?,
                    );
                }

                "brd" => {
                    broadcast = Some(
                        stream
                            .next()
                            .ok_or(ExpectedBroadcast)?
                            .parse()
                            .map_err(InvalidBroadcast)?,
                    );
                }

                _ => {
                    warn!("ignoring unknown property '{x}'");
                }
            }
        }

        let mac = mac.ok_or(ExpectedMac)?;
        let broadcast = broadcast.ok_or(ExpectedBroadcast)?;

        Ok(Self {
            address,
            mac,
            broadcast,
        })
    }
}
