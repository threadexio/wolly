impl App {
    pub async fn run(self: Arc<Self>) {
        for mapping in self.mappings.iter().cloned() {
            match mapping {
                Mapping::Single { from, to } => {
                    forward(from, to, self.clone()).await.unwrap();
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

                        forward(from, to, self.clone()).await.unwrap();
                    }
                }
            }
        }

        loop {
            sleep(Duration::from_secs(10)).await;
        }
    }
}

async fn forward(from: SocketAddr, to: SocketAddr, config: Arc<App>) -> io::Result<()> {
    let listener = TcpListener::bind(from).await?;

    tokio::spawn(async move {
        eprintln!("forwarding {from} to {to}...");

        loop {
            let mut a = match listener.accept().await {
                Ok((s, addr)) => {
                    eprintln!("connected {addr}");
                    s
                }
                Err(e) => {
                    eprintln!("Failed to accept client: {e}");
                    continue;
                }
            };

            let config = Arc::clone(&config);
            tokio::spawn(async move {
                let r: io::Result<()> = async move {
                    let upstream = config
                        .upstream
                        .get(&to.ip())
                        .expect("upstream should be in the map");

                    let mut b = upstream.connect(to.port()).await?;

                    tokio::io::copy_bidirectional(&mut a, &mut b).await?;
                    Ok(())
                }
                .await;

                if let Err(e) = r {
                    eprintln!("warn: {e}");
                }
            });
        }
    });

    Ok(())
}
