pub mod config;
pub mod error;
pub mod logging;
pub mod state;

pub use config::Settings;
pub use error::{JarvisError, Result};
pub use logging::init_logging;
pub use state::Status;

/// Returns true if running under a cargo test runner or explicit test environment.
pub fn is_test_environment() -> bool {
    if std::env::var("JARVIS_LIVE_TEST").is_ok() {
        return false;
    }
    if std::env::var("JARVIS_TEST_MODE").is_ok() {
        return true;
    }
    if let Ok(exe) = std::env::current_exe() {
        let exe_str = exe.to_string_lossy();
        if exe_str.contains("/deps/test_") || exe_str.contains("/deps/jarvis-") {
            return true;
        }
    }
    false
}

