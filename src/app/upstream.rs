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
        info!("waking upstream {}", self.address);

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

    async fn try_connect(&self, port: u16) -> io::Result<Option<TcpStream>> {
        match TcpStream::connect((self.address, port)).await {
            Ok(s) => Ok(Some(s)),
            Err(e)
                if e.kind() == io::ErrorKind::HostUnreachable
                    || e.kind() == io::ErrorKind::ConnectionRefused =>
            {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    pub async fn connect(&self, port: u16) -> Result<TcpStream> {
        if let Some(s) = self.try_connect(port).await? {
            return Ok(s);
        }

        self.wake().await.context("cannot wake upstream")?;

        let mut attempts = 0;
        let mut delay = Duration::from_secs(1);

        loop {
            attempts += 1;
            match self.try_connect(port).await? {
                Some(s) => return Ok(s),
                None => {
                    if attempts == 5 {
                        bail!("cannot connect to upstream");
                    } else {
                        sleep(delay).await;
                        delay *= 2;
                    }
                }
            }
        }
    }
}
