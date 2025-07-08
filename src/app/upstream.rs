use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

use eyre::{Context, Result, bail};
use tokio::net::{TcpStream, UdpSocket};
use tokio::time::sleep;

use crate::hardware_addr::HardwareAddr;

#[derive(Debug)]
pub struct Upstream {
    pub hardware_addr: HardwareAddr,
    pub address: IpAddr,
    pub broadcast: IpAddr,
}

impl Upstream {
    async fn wake(&self) -> io::Result<()> {
        info!("waking upstream");

        let mut packet = Vec::with_capacity(102);
        packet.extend_from_slice(&[0xff; 6]);
        for _ in 0..16 {
            packet.extend_from_slice(self.hardware_addr.octets());
        }

        let bind_on = match self.address {
            IpAddr::V4(_) => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            IpAddr::V6(_) => IpAddr::V6(Ipv6Addr::UNSPECIFIED),
        };

        let s = UdpSocket::bind((bind_on, 0)).await?;
        s.set_broadcast(true)?;

        s.send_to(&packet, (self.broadcast, 9)).await?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ConnectOpts {
    pub wakeup_delay: Duration,
    pub max_attempts: u64,
    pub initial_retry_delay: Duration,
    pub retry_delay_grow_factor: f64,
}

impl Upstream {
    pub async fn connect(&self, port: u16, opts: &ConnectOpts) -> Result<TcpStream> {
        let to = SocketAddr::new(self.address, port);

        if let Some(s) = try_connect(to).await {
            return s.map_err(Into::into);
        }

        self.wake().await.context("cannot wake upstream")?;
        sleep(opts.wakeup_delay).await;

        let mut attempts = 0;
        let mut delay = opts.initial_retry_delay;

        loop {
            match try_connect(to).await {
                Some(Ok(x)) => return Ok(x),
                Some(Err(e)) => return Err(e.into()),
                None => {
                    if attempts == opts.max_attempts {
                        bail!("cannot connect to upstream (max attempts reached)");
                    } else {
                        warn!("failed to connect to upstream");
                        attempts += 1;

                        sleep(delay).await;
                        delay = delay.mul_f64(opts.retry_delay_grow_factor);
                    }
                }
            }
        }
    }
}

async fn try_connect(addr: SocketAddr) -> Option<io::Result<TcpStream>> {
    match TcpStream::connect(addr).await {
        Ok(x) => Some(Ok(x)),
        Err(e) => match e.kind() {
            io::ErrorKind::HostUnreachable | io::ErrorKind::ConnectionRefused => None,
            _ => Some(Err(e)),
        },
    }
}
