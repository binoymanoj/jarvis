use clap::Parser;
use jarvis::cli::{run_cli, CliArgs};
use jarvis::core::config::Settings;
use jarvis::core::logging::init_logging;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = CliArgs::parse();
    init_logging(args.daemon);

    let settings = Settings::load()?;
    run_cli(args, settings).await?;
    Ok(())
}
