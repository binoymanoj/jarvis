use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter, Registry};

/// Returns the standard XDG state directory for Jarvis logs (~/.local/state/jarvis)
pub fn get_log_dir() -> PathBuf {
    if let Ok(state_home) = std::env::var("XDG_STATE_HOME") {
        let p = PathBuf::from(state_home).join("jarvis");
        if !p.as_os_str().is_empty() {
            return p;
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home)
            .join(".local")
            .join("state")
            .join("jarvis")
    } else {
        PathBuf::from(".")
    }
}

/// Returns the path to the primary jarvis.log file
pub fn get_log_file_path() -> PathBuf {
    get_log_dir().join("jarvis.log")
}

pub fn init_logging(verbose: bool) {
    let default_directive = if verbose {
        "jarvis=debug,info"
    } else {
        "jarvis=info,warn"
    };

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_directive));

    let log_dir = get_log_dir();
    let _ = fs::create_dir_all(&log_dir);
    let log_path = get_log_file_path();

    // Rotate log if larger than 10MB to avoid unbound disk usage
    if let Ok(meta) = fs::metadata(&log_path) {
        if meta.len() > 10 * 1024 * 1024 {
            let backup = log_path.with_extension("log.1");
            let _ = fs::rename(&log_path, backup);
        }
    }

    // Create convenience symlink ~/.config/jarvis/jarvis.log -> ~/.local/state/jarvis/jarvis.log
    if let Ok(home) = std::env::var("HOME") {
        let config_link = PathBuf::from(home)
            .join(".config")
            .join("jarvis")
            .join("jarvis.log");
        if !config_link.exists() {
            #[cfg(unix)]
            let _ = std::os::unix::fs::symlink(&log_path, &config_link);
        }
    }

    let stdout_layer = fmt::layer()
        .compact()
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false);

    let file_result = OpenOptions::new().create(true).append(true).open(&log_path);

    if let Ok(file) = file_result {
        let file_layer = fmt::layer()
            .with_ansi(false)
            .with_target(false)
            .with_thread_ids(false)
            .with_thread_names(false)
            .with_writer(Arc::new(file));

        let _ = Registry::default()
            .with(filter)
            .with(stdout_layer)
            .with(file_layer)
            .try_init();
    } else {
        let _ = Registry::default()
            .with(filter)
            .with(stdout_layer)
            .try_init();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_paths() {
        let log_dir = get_log_dir();
        let log_file = get_log_file_path();
        assert!(log_file.ends_with("jarvis.log"));
        assert_eq!(log_file.parent().unwrap(), log_dir.as_path());
    }
}
