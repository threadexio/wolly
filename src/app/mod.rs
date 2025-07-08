use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use eyre::{Context, Result, bail};

use crate::config::Config;
use crate::config::address::Port;
use crate::signal::Signals;

mod mapping;
mod tunnel;
mod upstream;

use self::mapping::Mapping;
use self::tunnel::Tunnels;
use self::upstream::Upstream;

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

            let mapping = match (x.from.port.clone(), x.to.port) {
                (Single(from_port), Single(to_port)) => Mapping::OneToOne {
                    from: SocketAddr::new(x.from.ip, from_port),
                    to: SocketAddr::new(x.to.ip, to_port),
                },

                (Range(from_range), Single(to_port)) => Mapping::ManyToOne {
                    from_ip: x.from.ip,
                    from_ports: from_range,
                    to: SocketAddr::new(x.to.ip, to_port),
                },

                (Range(from_range), Range(to_range)) => {
                    if from_range.len() != to_range.len() {
                        bail!("{:?}: 'from' and 'to' ranges do not match in size", x.from)
                    } else {
                        Mapping::ManyToMany {
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
            let tunnels = Tunnels::new(&self);

            match mapping {
                Mapping::OneToOne { from, to } => tunnels.spawn(from, to).await?,

                Mapping::ManyToOne {
                    from_ip,
                    from_ports,
                    to,
                } => {
                    for port in from_ports {
                        let from = SocketAddr::new(from_ip, port);

                        tunnels.spawn(from, to).await?
                    }
                }

                Mapping::ManyToMany {
                    from_ip,
                    from_port_range_start,
                    to_ip,
                    to_port_range_start,
                    port_range_len,
                } => {
                    for i in 0..port_range_len {
                        let from = SocketAddr::new(from_ip, from_port_range_start + i);
                        let to = SocketAddr::new(to_ip, to_port_range_start + i);

                        tunnels.spawn(from, to).await?;
                    }
                }
            }
        }

        signals.wait_terminate().await;
        info!("exiting...");
        Ok(())
    }
}
