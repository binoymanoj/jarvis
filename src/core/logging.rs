use tracing_subscriber::{fmt, EnvFilter};

pub fn init_logging(verbose: bool) {
    let default_directive = if verbose {
        "jarvis=debug,info"
    } else {
        "jarvis=info,warn"
    };

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(default_directive));

    let _ = fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .compact()
        .try_init();
}
