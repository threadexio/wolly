use std::num::ParseIntError;
use std::time::Duration;

use thiserror::Error;

use super::address::{Address, ParseAddressError};
use super::parse::{Parse, ParseStream};

#[derive(Debug, Clone)]
pub struct Forward {
    pub from: Address,
    pub to: Address,
    pub wait_for: Duration,
    pub max_attempts: u64,
    pub retry_delay: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParseForwardError {
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
}

impl Parse for Forward {
    type Output = Self;
    type Error = ParseForwardError;

    // Syntax: forward <address> to <address>
    fn parse(stream: &mut ParseStream<'_>) -> Result<Self::Output, Self::Error> {
        use ParseForwardError::*;

        let from = stream
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

        let to = stream
            .next()
            .ok_or(ExpectedToAddress)?
            .parse()
            .map_err(InvalidToAddress)?;

        let mut wait_for = Duration::from_secs(0);
        let mut max_attempts = 5;
        let mut retry_delay = Duration::from_secs(1);

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
                        .parse()
                        .map_err(InvalidMaxAttempts)?;
                }

                "retry-delay" => {
                    retry_delay = stream
                        .next()
                        .ok_or(ExpectedRetryDelay)?
                        .parse()
                        .map_err(InvalidRetryDelay)
                        .map(Duration::from_secs)?;
                }

                _ => {
                    warn!("ignoring unknown property '{x}'");
                }
            }
        }

        Ok(Self {
            from,
            to,
            wait_for,
            max_attempts,
            retry_delay,
        })
    }
}
