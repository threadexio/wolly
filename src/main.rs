#[macro_use]
extern crate tracing;

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::ops::Range;
use std::path::Path;
use std::process::ExitCode;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use std::{fmt, io};

use eyre::{Context, ContextCompat, Result, bail};
use hardware_addr::HardwareAddr;
use miniarg::split_args::SplitArgs;
use thiserror::Error;
use tokio::net::TcpListener;
use tokio::time::sleep;

mod hardware_addr;
mod upstream;

use self::upstream::Upstream;

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Mapping {
    Single {
        from: SocketAddr,
        to: SocketAddr,
    },
    Range {
        from: IpAddr,
        to: IpAddr,
        from_port_base: u16,
        to_port_base: u16,
        range_len: u16,
    },
}

impl fmt::Debug for Mapping {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Single { from, to } => {
                write!(f, "{from}:{to}")
            }
            Self::Range {
                from,
                to,
                from_port_base,
                to_port_base,
                range_len,
            } => {
                let from_port_end = from_port_base + range_len;
                let to_port_end = to_port_base + range_len;

                write!(
                    f,
                    "{from}:{from_port_base}-{from_port_end}:{to}:{to_port_base}-{to_port_end}"
                )
            }
        }
    }
}

#[derive(Debug)]
struct App {
    upstream: HashMap<IpAddr, Upstream>,
    mappings: Vec<Mapping>,
}

impl App {
    pub async fn run(self: Arc<Self>) {
        for mapping in self.mappings.iter().cloned() {
            info!("forwarding {mapping:?}");

            match mapping {
                Mapping::Single { from, to } => {
                    self.clone().forward(from, to).await.unwrap();
                }

                Mapping::Range {
                    from,
                    to,
                    from_port_base,
                    to_port_base,
                    range_len,
                } => {
                    for i in 0..range_len {
                        let from = SocketAddr::new(from, from_port_base + i);
                        let to = SocketAddr::new(to, to_port_base + i);

                        self.clone().forward(from, to).await.unwrap();
                    }
                }
            }
        }

        loop {
            sleep(Duration::from_secs(10)).await;
        }
    }

    async fn forward(self: Arc<Self>, from: SocketAddr, to: SocketAddr) -> io::Result<()> {
        let listener = TcpListener::bind(from).await?;

        tokio::spawn(async move {
            loop {
                let (mut a, addr) = match listener.accept().await {
                    Ok(x) => x,
                    Err(e) => {
                        warn!("failed to accept connection: {e:#}");
                        continue;
                    }
                };

                let me = Arc::clone(&self);
                tokio::spawn(async move {
                    let r: Result<()> = async move {
                        let upstream = me
                            .upstream
                            .get(&to.ip())
                            .expect("upstream should be in the map");

                        info!("new connection from {addr}, forwarding to {to}");

                        let mut b = upstream.connect(to.port()).await?;

                        tokio::io::copy_bidirectional(&mut a, &mut b)
                            .await
                            .context("cannot forward connection")?;
                        Ok(())
                    }
                    .await;

                    if let Err(e) = r {
                        warn!("{addr}<->{to}: {e}");
                    }
                });
            }
        });

        Ok(())
    }

    pub async fn read<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        let data = tokio::fs::read_to_string(path).await?;
        data.parse()
    }
}

impl FromStr for App {
    type Err = eyre::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // upstream <ip> [mac <mac>] [brd <ip>]
        #[derive(Debug)]
        struct UpstreamStmt {
            addr: IpAddr,
            mac: HardwareAddr,
            brd: IpAddr,
        }

        impl UpstreamStmt {
            fn parse(args: &mut SplitArgs) -> Result<Self> {
                let addr = args
                    .next()
                    .context("missing upstream <ip> address")?
                    .parse()
                    .context("invalid upstream <ip> address")?;

                let mut mac = None;
                let mut brd = None;

                while let Some(arg) = args.next() {
                    match arg {
                        "mac" => {
                            mac = Some(
                                args.next()
                                    .context("expected upstream <mac> address")?
                                    .parse()
                                    .context("invalid upstream <mac> address")?,
                            );
                        }

                        "brd" => {
                            brd = Some(
                                args.next()
                                    .context("expected upstream broadcast <ip> address")?
                                    .parse()
                                    .context("invalid upstream broadcast <ip> address")?,
                            );
                        }

                        x => {
                            warn!("ignoring unknown argument '{x}'");
                        }
                    }
                }

                Ok(Self {
                    addr,
                    mac: mac.context("missing <mac>")?,
                    brd: brd.context("missing <brd>")?,
                })
            }
        }

        #[derive(Debug)]
        enum Port {
            Single(u16),
            Range(Range<u16>),
        }

        #[derive(Debug)]
        struct AddrSpec {
            addr: IpAddr,
            port: Port,
        }

        impl FromStr for AddrSpec {
            type Err = eyre::Report;

            fn from_str(s: &str) -> Result<Self> {
                let (addr, port) = s.split_once(':').context("missing ':' separator")?;

                let addr = addr.parse().context("invalid <ip> address")?;

                let port = match port.split_once('-') {
                    Some((start, end)) => {
                        let start: u16 = start.parse().context("invalid start port")?;
                        let end: u16 = end.parse().context("invalid end port")?;

                        Port::Range(start..end + 1)
                    }

                    None => Port::Single(port.parse().context("invalid port")?),
                };

                Ok(Self { addr, port })
            }
        }

        // forward <ip>:<port> to <ip>:<port>
        #[derive(Debug)]
        struct ForwardStmt {
            from: AddrSpec,
            to: AddrSpec,
        }

        impl ForwardStmt {
            fn parse(args: &mut SplitArgs) -> Result<Self> {
                let from = args
                    .next()
                    .context("expected forward source address")?
                    .parse()
                    .context("invalid forward source address")?;

                let to = args.next().context("expected 'to'")?;
                if to != "to" {
                    bail!("expected 'to', not '{to}'");
                }

                let to = args
                    .next()
                    .context("expected forward destination address")?
                    .parse()
                    .context("invalid forward destination address")?;

                Ok(Self { from, to })
            }
        }

        #[derive(Debug)]
        enum Stmt {
            Upstream(UpstreamStmt),
            Forward(ForwardStmt),
        }

        impl Stmt {
            fn parse(args: &mut SplitArgs) -> Result<Self> {
                match args.next().expect("stmt should not be empty") {
                    "upstream" => UpstreamStmt::parse(args).map(Self::Upstream),
                    "forward" => ForwardStmt::parse(args).map(Self::Forward),
                    _ => bail!("unknown statement"),
                }
            }
        }

        let mut upstream = HashMap::new();
        let mut mappings = Vec::new();

        let lines = s
            .lines()
            .enumerate()
            .map(|(i, line)| (i, line.trim()))
            .filter(|(_, line)| !line.is_empty())
            .filter(|(_, line)| !line.starts_with('#'))
            .map(|(i, line)| (i, SplitArgs::new(line)));

        for (i, mut args) in lines {
            let stmt = Stmt::parse(&mut args).with_context(|| format!("line {}", i + 1))?;

            match stmt {
                Stmt::Upstream(UpstreamStmt { addr, mac, brd }) => {
                    upstream.insert(
                        addr,
                        Upstream {
                            address: addr,
                            hardware_addr: mac,
                            broadcast: brd,
                        },
                    );
                }

                Stmt::Forward(ForwardStmt { from, to }) => {
                    let mapping = match (from.port, to.port) {
                        (Port::Single(from_port), Port::Single(to_port)) => Mapping::Single {
                            from: SocketAddr::new(from.addr, from_port),
                            to: SocketAddr::new(to.addr, to_port),
                        },
                        (Port::Range(a), Port::Range(b)) => {
                            if a.len() != b.len() {
                                bail!("port ranges are not of the same length");
                            }

                            Mapping::Range {
                                from: from.addr,
                                to: to.addr,
                                from_port_base: a.start,
                                to_port_base: b.start,
                                range_len: a.len().try_into().context("port range too big")?,
                            }
                        }
                        (Port::Single(_), Port::Range(_)) => {
                            bail!("cannot forward one port to many")
                        }
                        (Port::Range(_), Port::Single(_)) => {
                            bail!("cannot forward many ports to one")
                        }
                    };

                    if !upstream.contains_key(&to.addr) {
                        bail!(
                            "upstream {} is unknown. perhaps you are missing an 'upstream' statement?",
                            to.addr
                        );
                    }

                    mappings.push(mapping);
                }
            }
        }

        Ok(Self { upstream, mappings })
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .compact()
        .with_target(false)
        .init();

    match try_main().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            error!("{e:#}");
            ExitCode::FAILURE
        }
    }
}

async fn try_main() -> Result<()> {
    let path = "wolly.conf";

    let app = App::read(path).await.with_context(|| format!("{path}"))?;

    let app = Arc::new(app);
    app.run().await;
    Ok(())
}
