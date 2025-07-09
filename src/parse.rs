use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt;
use std::iter::Peekable;
use std::net::{AddrParseError, IpAddr, SocketAddr};
use std::num::{NonZero, ParseFloatError, ParseIntError};
use std::ops::Range;
use std::str::FromStr;
use std::time::Duration;

use miniarg::split_args::SplitArgs;
use thiserror::Error;

use crate::hardware_addr;

use super::App;
use super::mapping::{Mapping, MappingKind};
use super::upstream::{ConnectOpts, Upstream};

type ParseStream<'a> = Peekable<SplitArgs<'a>>;

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

impl Upstream {
    fn parse(stream: &mut ParseStream<'_>) -> Result<Self, ParseUpstreamError> {
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

#[derive(Debug, Clone)]
pub enum Port {
    Single(u16),
    Range(Range<u16>),
}

impl fmt::Display for Port {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Port::Single(x) => write!(f, "{x}"),
            Port::Range(x) => write!(f, "{}-{}", x.start, x.end - 1),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Address {
    pub ip: IpAddr,
    pub port: Port,
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.ip, self.port)
    }
}

display!(@impl Address as address);

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

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParseMappingError {
    #[error("expected 'from' address")]
    ExpectedFromAddress,

    #[error("invalid 'from' address: {0}")]
    InvalidFromAddress(ParseAddressError),

    #[error("expected literal 'to'")]
    ExpectedTo,

    #[error("expected 'to' address")]
    ExpectedToAddress,

    #[error("invalid 'to' address: {0}")]
    InvalidToAddress(ParseAddressError),

    #[error("expected a delay for 'wait-for'")]
    ExpectedWaitFor,

    #[error("invalid wait delay: {0}")]
    InvalidWaitFor(ParseIntError),

    #[error("expected a number for 'max-attempts'")]
    ExpectedMaxAttempts,

    #[error("invalid max attempts: {0}")]
    InvalidMaxAttempts(ParseIntError),

    #[error("expected a delay for 'retry-delay'")]
    ExpectedRetryDelay,

    #[error("invalid retry delay")]
    InvalidRetryDelay(ParseIntError),

    #[error("expected 'retry-factor'")]
    ExpectedRetryFactor,

    #[error("invalid retry factor: {0}")]
    InvalidRetryFactor(ParseFloatError),

    #[error("'from' and 'to' ranges do not match in size")]
    InvalidPortRanges,

    #[error("cannot map a single port to a range of ports")]
    InvalidMappingType,
}

impl Mapping {
    fn parse(stream: &mut ParseStream<'_>) -> Result<Self, ParseMappingError> {
        use ParseMappingError::*;
        use Port::{Range, Single};

        let from: Address = stream
            .next()
            .ok_or(ExpectedFromAddress)?
            .parse()
            .map_err(InvalidFromAddress)?;

        {
            let to = stream.next().ok_or(ExpectedTo)?;
            if to != "to" {
                return Err(ExpectedTo);
            }
        }

        let to: Address = stream
            .next()
            .ok_or(ExpectedToAddress)?
            .parse()
            .map_err(InvalidToAddress)?;

        let mut wait_for = Duration::from_secs(0);
        let mut max_attempts = 5;
        let mut retry_delay = Duration::from_secs(1);
        let mut retry_factor = 2.0;

        while let Some(x) = stream.next() {
            match x {
                "wait-for" => {
                    wait_for = stream
                        .next()
                        .ok_or(ExpectedWaitFor)?
                        .parse()
                        .map_err(InvalidWaitFor)
                        .map(Duration::from_secs)?;
                }

                "max-attempts" => {
                    max_attempts = stream
                        .next()
                        .ok_or(ExpectedMaxAttempts)?
                        .parse::<u64>()
                        .map_err(InvalidMaxAttempts)?
                }

                "retry-delay" => {
                    retry_delay = stream
                        .next()
                        .ok_or(ExpectedRetryDelay)?
                        .parse()
                        .map_err(InvalidRetryDelay)
                        .map(Duration::from_secs)?
                }

                "retry-factor" => {
                    retry_factor = stream
                        .next()
                        .ok_or(ExpectedRetryFactor)?
                        .parse()
                        .map_err(InvalidRetryFactor)?
                }

                _ => {
                    warn!("ignoring unknown property '{x}'");
                }
            }
        }

        let max_attempts = match NonZero::new(max_attempts) {
            Some(x) => x,
            None => {
                warn!(
                    "{}: 'max-attempts' cannot be 0, will try to connect at least once",
                    display!(from)
                );
                NonZero::new(1).expect("1 is not 0")
            }
        };

        let opts = ConnectOpts {
            wait_for,
            max_attempts,
            retry_delay,
            retry_factor,
        };

        let kind = match (from.port.clone(), to.port) {
            (Single(from_port), Single(to_port)) => MappingKind::OneToOne {
                from: SocketAddr::new(from.ip, from_port),
                to: SocketAddr::new(to.ip, to_port),
            },

            (Range(from_range), Single(to_port)) => MappingKind::ManyToOne {
                from_ip: from.ip,
                from_ports: from_range,
                to: SocketAddr::new(to.ip, to_port),
            },

            (Range(from_range), Range(to_range)) => {
                if from_range.len() != to_range.len() {
                    return Err(InvalidPortRanges);
                } else {
                    MappingKind::ManyToMany {
                        from_ip: from.ip,
                        from_port_range_start: from_range.start,
                        to_ip: to.ip,
                        to_port_range_start: to_range.start,
                        port_range_len: from_range
                            .len()
                            .try_into()
                            .expect("the length of Range<u16> can never be greater than u16"),
                    }
                }
            }

            (Single(_), Range(_)) => return Err(InvalidMappingType),
        };

        Ok(Self { kind, opts })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    Upstream(ParseUpstreamError),
    Mapping(ParseMappingError),

    DuplicateUpstreamDirectives,
}

impl fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Upstream(x) => x.fmt(f),
            Self::Mapping(x) => x.fmt(f),
            Self::DuplicateUpstreamDirectives => f.write_str("duplicate upstream directives"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("line {line}: {kind}")]
pub struct ParseError {
    pub line: usize,
    pub kind: ParseErrorKind,
}

impl FromStr for App {
    type Err = ParseError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let mut upstream = HashMap::new();
        let mut mappings = Vec::new();

        let lines = s
            .lines()
            .map(str::trim)
            .enumerate()
            .filter(|(_, line)| !line.is_empty())
            .filter(|(_, line)| !line.starts_with('#'));

        for (i, line) in lines {
            let parse_error =
                |kind: ParseErrorKind| -> ParseError { ParseError { line: i + 1, kind } };

            let mut stream = SplitArgs::new(line).peekable();

            match stream.next().expect("line is not empty") {
                "upstream" => {
                    let x = Upstream::parse(&mut stream)
                        .map_err(ParseErrorKind::Upstream)
                        .map_err(parse_error)?;

                    match upstream.entry(x.address) {
                        Entry::Vacant(e) => {
                            e.insert(x);
                        }
                        Entry::Occupied(_) => {
                            return Err(parse_error(ParseErrorKind::DuplicateUpstreamDirectives));
                        }
                    }
                }

                "forward" => {
                    let x = Mapping::parse(&mut stream)
                        .map_err(ParseErrorKind::Mapping)
                        .map_err(parse_error)?;

                    mappings.push(x);
                }

                x => {
                    warn!("skipping unknown directive '{x}'")
                }
            }
        }

        Ok(Self { upstream, mappings })
    }
}
