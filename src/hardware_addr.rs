use std::{fmt, num::ParseIntError, str::FromStr};

use thiserror::Error;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct HardwareAddr([u8; 6]);

impl fmt::Debug for HardwareAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

impl HardwareAddr {
    pub fn octets(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParseError {
    #[error("expected octet")]
    ExpectedOctet,
    #[error("invalid octet: {0}")]
    InvalidOctet(ParseIntError),
}

impl FromStr for HardwareAddr {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut octets = [0u8; 6];
        let mut parts = s.splitn(octets.len(), ':');

        for x in octets.iter_mut() {
            let octet = parts.next().ok_or(ParseError::ExpectedOctet)?;
            let octet = u8::from_str_radix(octet, 16).map_err(ParseError::InvalidOctet)?;
            *x = octet;
        }

        Ok(Self(octets))
    }
}

impl<'de> serde::Deserialize<'de> for HardwareAddr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}
