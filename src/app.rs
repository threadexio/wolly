use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use eyre::{Context, Result, bail};
use owo_colors::OwoColorize;
use tokio::net::TcpListener;
use tracing::Instrument;

use crate::mapping::{Mapping, MappingKind};
use crate::signal::Signals;
use crate::upstream::{ConnectOpts, Upstream};

#[derive(Debug)]
pub struct App {
    pub upstream: HashMap<IpAddr, Upstream>,
    pub mappings: Vec<Mapping>,
}

impl App {
    pub async fn run(self) -> Result<()> {
        if self.mappings.is_empty() {
            bail!("no forwarding rules configured");
        }

        let me = Arc::new(self);
        me._run().await
    }

    async fn _run(self: Arc<Self>) -> Result<()> {
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
        .with_context(|| format!("failed to bind listener on {}", display!(from)))?;

    tokio::spawn(async move {
        loop {
            let (a, addr) = match listener.accept().await {
                Ok(x) => x,
                Err(_) => continue,
            };

            let opts = opts.clone();
            let app = Arc::clone(&app);

            let span = error_span!("tunnel", from = addr.to_string(), to = to.to_string());
            tokio::spawn(
                async move {
                    info!("connected");

                    let upstream = app
                        .upstream
                        .get(&to.ip())
                        .expect("upstream should be known");

                    let mut a = a;
                    let mut b = match upstream.connect(to.port(), &opts).await {
                        Ok(x) => x,
                        Err(e) => {
                            error!("cannot connect to upstream: {}", display!(e));
                            return;
                        }
                    };

                    info!("{} to upstream", "connected".bright_green());
                    let _ = tokio::io::copy_bidirectional(&mut a, &mut b).await;
                    info!("disconnected");
                }
                .instrument(span),
            );
        }
    });

    Ok(())
}
