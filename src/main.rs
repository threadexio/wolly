#[macro_use]
extern crate tracing;

#[macro_use]
extern crate derive_more;

use std::process::ExitCode;
use std::sync::Arc;

use eyre::{Context, Result};

mod app;
mod config;
mod hardware_addr;
mod signal;

use self::app::App;
use self::config::Config;

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .compact()
        .with_target(false)
        .init();

    match try_main().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            error!("{e:#}");
            ExitCode::FAILURE
        }
    }
}

async fn try_main() -> Result<()> {
    let path = "wolly.conf";

    let config = Config::read(path).await.with_context(|| path)?;
    let app = App::try_from(config).map(Arc::new)?;

    app.run().await
}
