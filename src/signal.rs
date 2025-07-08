use eyre::Result;
use tokio::signal::unix::{Signal, SignalKind, signal};

#[derive(Debug)]
pub struct Signals {
    int: Signal,
    term: Signal,
}

impl Signals {
    pub fn new() -> Result<Self> {
        Ok(Self {
            int: signal(SignalKind::interrupt())?,
            term: signal(SignalKind::terminate())?,
        })
    }

    pub async fn wait_terminate(&mut self) {
        tokio::select! {
            _ = self.int.recv() => {},
            _ = self.term.recv() => {}
        }
    }
}
