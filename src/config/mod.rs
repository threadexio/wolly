use std::fmt;
use std::path::Path;
use std::str::FromStr;

use parse::Parse;
use thiserror::Error;

mod parse;
use self::parse::parse_stream;

pub mod address;
pub mod forward;
pub mod upstream;

use self::forward::Forward;
use self::upstream::Upstream;

#[derive(Debug)]
pub struct Config {
    pub upstream: Vec<Upstream>,
    pub forward: Vec<Forward>,
}

impl Config {
    pub async fn read<P>(path: P) -> eyre::Result<Self>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        let data = tokio::fs::read_to_string(path).await?;
        let x = data.parse()?;
        Ok(x)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    Upstream(upstream::ParseUpstreamError),
    Forward(forward::ParseForwardError),
}

impl fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Upstream(x) => x.fmt(f),
            Self::Forward(x) => x.fmt(f),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("line {line}: {kind}")]
pub struct ParseError {
    line: usize,
    kind: ParseErrorKind,
}

impl FromStr for Config {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut upstream = Vec::new();
        let mut forward = Vec::new();

        let lines = s
            .lines()
            .map(str::trim)
            .enumerate()
            .filter(|(_, line)| !line.is_empty())
            .filter(|(_, line)| !line.starts_with('#'));

        for (i, line) in lines {
            let parse_error =
                |kind: ParseErrorKind| -> ParseError { ParseError { line: i + 1, kind } };

            let mut stream = parse_stream(line);

            match stream.next().expect("line is not empty") {
                "upstream" => {
                    let x = Upstream::parse(&mut stream)
                        .map_err(ParseErrorKind::Upstream)
                        .map_err(parse_error)?;

                    upstream.push(x);
                }

                "forward" => {
                    let x = Forward::parse(&mut stream)
                        .map_err(ParseErrorKind::Forward)
                        .map_err(parse_error)?;

                    forward.push(x);
                }

                x => {
                    warn!("skipping unknown directive '{x}'")
                }
            }
        }

        Ok(Self { upstream, forward })
    }
}
