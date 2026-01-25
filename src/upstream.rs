use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::num::NonZero;
use std::time::Duration;

use tokio::net::{TcpStream, UdpSocket};
use tokio::time::sleep;

use crate::hardware_addr::HardwareAddr;
use crate::util::DurationExt;

#[derive(Debug)]
pub struct Upstream {
    pub mac: HardwareAddr,
    pub address: IpAddr,
    pub broadcast: IpAddr,
}

impl Upstream {
    async fn wake(&self) -> io::Result<()> {
        info!("waking upstream");

        let mut packet = Vec::with_capacity(102);
        packet.extend_from_slice(&[0xff; 6]);
        for _ in 0..16 {
            packet.extend_from_slice(self.mac.octets());
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
    pub wait_for: Duration,
    pub max_attempts: NonZero<u64>,
    pub retry_delay: Duration,
    pub retry_factor: f64,
}

impl Upstream {
    pub async fn connect(&self, port: u16, opts: &ConnectOpts) -> io::Result<TcpStream> {
        let to = SocketAddr::new(self.address, port);

        match TcpStream::connect(to).await {
            Ok(x) => return Ok(x),
            Err(e) if !is_retry_error(&e) => return Err(e),
            Err(_) => {}
        }

        let mut attempts = 0;
        let mut delay = opts.retry_delay;

        loop {
            self.wake().await?;
            sleep(opts.wait_for).await;

            match TcpStream::connect(to).await {
                Ok(x) => return Ok(x),
                Err(e) if !is_retry_error(&e) => return Err(e),
                Err(e) => {
                    attempts += 1;

                    if attempts == opts.max_attempts.get() {
                        debug!("max attempts reached, will not try again");
                        return Err(e);
                    } else {
                        warn!("failed to connect to upstream: {}", display!(e));
                        debug!("retrying in {}", display!(delay));
                        sleep(delay).await;

                        delay = delay.checked_mul_f64(opts.retry_factor).unwrap_or_else(|| {
                            warn!("invalid retry factor");
                            delay
                        });

                        continue;
                    }
                }
            }
        }
    }
}

fn is_retry_error(e: &io::Error) -> bool {
    use io::ErrorKind::*;

    matches!(e.kind(), HostUnreachable | ConnectionRefused)
}
