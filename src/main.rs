use clap::Parser;
use jarvis::cli::{run_cli, CliArgs};
use jarvis::core::config::Settings;
use jarvis::core::logging::init_logging;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> anyhow::Result<()> {
    // Initialize global ONNX runtime environment with minimal thread pool & spin control disabled
    let _ = (|| -> ort::Result<()> {
        let pool = ort::environment::GlobalThreadPoolOptions::default()
            .with_spin_control(false)?
            .with_intra_threads(1)?
            .with_inter_threads(1)?;
        ort::init()
            .with_name("jarvis")
            .with_telemetry(false)
            .with_global_thread_pool(pool)
            .commit();
        Ok(())
    })();

    let args = CliArgs::parse();
    init_logging(args.daemon);

    let settings = Settings::load()?;
    run_cli(args, settings).await?;
    Ok(())
}
