use thiserror::Error;

use super::address::{Address, ParseAddressError};
use super::parse::{Parse, ParseStream};

#[derive(Debug, Clone)]
pub struct Forward {
    pub from: Address,
    pub to: Address,
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

        Ok(Self { from, to })
    }
}
