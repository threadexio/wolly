use std::net::SocketAddr;
use std::sync::Arc;

use eyre::{Context, Result};
use tokio::net::TcpListener;

use super::App;

#[derive(Debug)]
pub struct Tunnels<'a> {
    app: &'a Arc<App>,
}

impl<'a> Tunnels<'a> {
    pub fn new(app: &'a Arc<App>) -> Self {
        Self { app }
    }

    pub async fn spawn(&self, from: SocketAddr, to: SocketAddr) -> Result<()> {
        debug!("bind listener on {from}");

        Tunnel::spawn(from, to, self.app.clone())
            .await
            .with_context(|| format!("failed to bind listener on {from}"))
    }
}

pub struct Tunnel {
    listener: TcpListener,
    to: SocketAddr,
    app: Arc<App>,
}

impl Tunnel {
    pub async fn spawn(from: SocketAddr, to: SocketAddr, app: Arc<App>) -> Result<()> {
        let listener = TcpListener::bind(from)
            .await
            .with_context(|| format!("failed to bind listener on {from}"))?;

        let tunnel = Self { listener, to, app };
        tokio::spawn(async move { tunnel.run().await });
        Ok(())
    }

    async fn run(self) {
        loop {
            let (from, from_addr) = match self.listener.accept().await {
                Ok(x) => x,
                Err(_) => continue,
            };

            let to_addr = self.to;

            let app = Arc::clone(&self.app);
            tokio::spawn(async move {
                info!("connection from {from_addr} to {to_addr}");

                let upstream = app
                    .upstream
                    .get(&to_addr.ip())
                    .expect("upstream should be known");

                let r: Result<()> = async move {
                    let mut from = from;
                    let mut to = upstream.connect(to_addr.port()).await?;

                    let _ = tokio::io::copy_bidirectional(&mut from, &mut to).await;
                    Ok(())
                }
                .await;

                if let Err(e) = r {
                    warn!("tunnel[{from_addr},{to_addr}]: {e:#}");
                }
            });
        }
    }
}
