#[macro_use]
extern crate tracing;

#[macro_use]
extern crate derive_more;

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt;
use std::net::{IpAddr, SocketAddr};
use std::process::ExitCode;
use std::sync::Arc;

use config::address::Port;
use eyre::{Context, Result, bail};
use tokio::net::TcpListener;

mod config;
mod hardware_addr;
mod signal;
mod upstream;

use self::config::Config;
use self::signal::Signals;
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
    pub async fn run(self: Arc<Self>) -> Result<()> {
        let mut signals = Signals::new().context("failed to register signal handlers")?;

        for mapping in self.mappings.iter().cloned() {
            info!("forwarding {mapping:?}");

            match mapping {
                Mapping::Single { from, to } => {
                    self.clone().forward(from, to).await?;
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

                        self.clone().forward(from, to).await?;
                    }
                }
            }
        }

        signals.wait_terminate().await;
        info!("exiting...");
        Ok(())
    }

    async fn forward(self: Arc<Self>, from: SocketAddr, to: SocketAddr) -> Result<()> {
        let listener = TcpListener::bind(from)
            .await
            .with_context(|| format!("failed to bind listener on {from}"))?;

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
}

impl TryFrom<Config> for App {
    type Error = eyre::Report;

    fn try_from(config: Config) -> Result<Self> {
        let mut upstream = HashMap::new();
        let mut mappings = Vec::new();

        for x in config.upstream {
            let Entry::Vacant(entry) = upstream.entry(x.address) else {
                bail!("duplicate upstream directives for {}", x.address);
            };

            entry.insert(Upstream {
                hardware_addr: x.mac,
                address: x.address,
                broadcast: x.broadcast,
            });
        }

        for x in config.forward {
            use Port::{Range, Single};

            let mapping = match (x.from.port.clone(), x.to.port) {
                (Single(from_port), Single(to_port)) => Mapping::Single {
                    from: SocketAddr::new(x.from.ip, from_port),
                    to: SocketAddr::new(x.to.ip, to_port),
                },

                (Range(from_range), Range(to_range)) => {
                    if from_range.len() != to_range.len() {
                        bail!("{:?}: 'from' and 'to' ranges do not match in size", x.from)
                    } else {
                        Mapping::Range {
                            from: x.from.ip,
                            to: x.to.ip,
                            from_port_base: from_range.start,
                            to_port_base: to_range.start,
                            range_len: from_range
                                .len()
                                .try_into()
                                .expect("the length of Range<u16> can never be greater than u16"),
                        }
                    }
                }

                (Single(_), Range(_)) => {
                    bail!("{:?}: cannot map a single port to a range of ports", x.from)
                }

                (Range(_), Single(_)) => {
                    bail!("{:?}: cannot map a range of ports to a single port", x.from)
                }
            };

            mappings.push(mapping);
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

    let config = Config::read(path).await.with_context(|| path)?;
    let app = App::try_from(config).map(Arc::new)?;

    app.run().await
}
