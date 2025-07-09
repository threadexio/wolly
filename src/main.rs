#[macro_use]
extern crate tracing;

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use clap::Parser;
use eyre::{Context, Result};
use tracing::level_filters::LevelFilter;

#[macro_use]
mod display;
#[macro_use]
mod util;

mod app;
mod config;
mod hardware_addr;
mod signal;

use self::app::App;
use self::config::Config;

#[derive(Debug, Parser)]
struct CliArgs {
    #[clap(short, help = "Be verbose.")]
    verbose: bool,

    #[clap(
        help = "Specify the config file.",
        value_name = "config",
        default_value = "wolly.conf"
    )]
    config_path: PathBuf,
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = CliArgs::parse();

    let max_level = if args.verbose {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };

    tracing_subscriber::fmt()
        .compact()
        .with_target(false)
        .with_max_level(max_level)
        .init();

    match try_main(args).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            error!("{e:#}");
            ExitCode::FAILURE
        }
    }
}

async fn try_main(args: CliArgs) -> Result<()> {
    let config = Config::read(&args.config_path)
        .await
        .with_context(|| format!("{}", args.config_path.display()))?;

    let app = App::try_from(config).map(Arc::new)?;

    app.run().await
}
