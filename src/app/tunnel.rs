use std::net::SocketAddr;
use std::sync::Arc;

use eyre::{Context, Result};
use tokio::net::TcpListener;

use super::App;
use super::upstream::ConnectOpts;

#[derive(Debug)]
pub struct Tunnels<'a> {
    app: &'a Arc<App>,
}

impl<'a> Tunnels<'a> {
    pub fn new(app: &'a Arc<App>) -> Self {
        Self { app }
    }

    pub async fn spawn(&self, from: SocketAddr, to: SocketAddr, opts: ConnectOpts) -> Result<()> {
        debug!("bind listener on {from}");

        Tunnel::spawn(from, to, opts, self.app.clone())
            .await
            .with_context(|| format!("failed to bind listener on {from}"))
    }
}

pub struct Tunnel {}

impl Tunnel {
    pub async fn spawn(
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
}
