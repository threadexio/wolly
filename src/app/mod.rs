use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use eyre::{Context, Result, bail};
use tokio::net::TcpListener;

use crate::config::Config;
use crate::config::address::Port;
use crate::signal::Signals;

mod mapping;
mod upstream;

use self::mapping::{Mapping, MappingKind};
use self::upstream::{ConnectOpts, Upstream};

#[derive(Debug)]
pub struct App {
    upstream: HashMap<IpAddr, Upstream>,
    mappings: Vec<Mapping>,
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

            if !upstream.contains_key(&x.to.ip) {
                bail!("no upstream directive found for {}", x.to.ip);
            }

            let kind = match (x.from.port.clone(), x.to.port) {
                (Single(from_port), Single(to_port)) => MappingKind::OneToOne {
                    from: SocketAddr::new(x.from.ip, from_port),
                    to: SocketAddr::new(x.to.ip, to_port),
                },

                (Range(from_range), Single(to_port)) => MappingKind::ManyToOne {
                    from_ip: x.from.ip,
                    from_ports: from_range,
                    to: SocketAddr::new(x.to.ip, to_port),
                },

                (Range(from_range), Range(to_range)) => {
                    if from_range.len() != to_range.len() {
                        bail!("{:?}: 'from' and 'to' ranges do not match in size", x.from)
                    } else {
                        MappingKind::ManyToMany {
                            from_ip: x.from.ip,
                            from_port_range_start: from_range.start,
                            to_ip: x.to.ip,
                            to_port_range_start: to_range.start,
                            port_range_len: from_range
                                .len()
                                .try_into()
                                .expect("the length of Range<u16> can never be greater than u16"),
                        }
                    }
                }

                (Single(_), Range(_)) => {
                    bail!("{:?}: cannot map a single port to a range of ports", x.from)
                }
            };

            let opts = ConnectOpts {
                wakeup_delay: x.wait_for,
                max_attempts: x.max_attempts,
                initial_retry_delay: x.retry_delay,
                retry_delay_grow_factor: 2.0,
            };

            let mapping = Mapping { kind, opts };
            mappings.push(mapping);
        }

        Ok(Self { upstream, mappings })
    }
}

impl App {
    pub async fn run(self: Arc<Self>) -> Result<()> {
        info!("starting...");
        let mut signals = Signals::new().context("failed to register signal handlers")?;

        for mapping in self.mappings.iter().cloned() {
            let spawn_tunnel =
                async |from, to, opts| spawn_tunnel(from, to, opts, Arc::clone(&self)).await;

            match mapping.kind {
                MappingKind::OneToOne { from, to } => spawn_tunnel(from, to, mapping.opts).await?,

                MappingKind::ManyToOne {
                    from_ip,
                    from_ports,
                    to,
                } => {
                    for port in from_ports {
                        let from = SocketAddr::new(from_ip, port);

                        spawn_tunnel(from, to, mapping.opts.clone()).await?
                    }
                }

                MappingKind::ManyToMany {
                    from_ip,
                    from_port_range_start,
                    to_ip,
                    to_port_range_start,
                    port_range_len,
                } => {
                    for i in 0..port_range_len {
                        let from = SocketAddr::new(from_ip, from_port_range_start + i);
                        let to = SocketAddr::new(to_ip, to_port_range_start + i);

                        spawn_tunnel(from, to, mapping.opts.clone()).await?;
                    }
                }
            }
        }

        signals.wait_terminate().await;
        info!("exiting...");
        Ok(())
    }
}

async fn spawn_tunnel(
    from: SocketAddr,
    to: SocketAddr,
    opts: ConnectOpts,
    app: Arc<App>,
) -> Result<()> {
    let listener = TcpListener::bind(from)
        .await
        .with_context(|| format!("failed to bind listener on {from}"))?;

    tokio::spawn(async move {
        loop {
            let (a, addr) = match listener.accept().await {
                Ok(x) => x,
                Err(_) => continue,
            };

            let opts = opts.clone();
            let app = Arc::clone(&app);
            tokio::spawn(async move {
                let span = error_span!("tunnel", from = addr.to_string(), to = to.to_string());
                let _enter = span.enter();

                info!("connected");

                let upstream = app
                    .upstream
                    .get(&to.ip())
                    .expect("upstream should be known");

                let r: Result<()> = async move {
                    let mut a = a;
                    let mut b = upstream.connect(to.port(), &opts).await?;

                    let _ = tokio::io::copy_bidirectional(&mut a, &mut b).await;
                    info!("disconnected");
                    Ok(())
                }
                .await;

                if let Err(e) = r {
                    error!("{e:#}");
                }
            });
        }
    });

    Ok(())
}
