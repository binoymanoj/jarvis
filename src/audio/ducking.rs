use crate::core::config::Settings;
use crate::core::is_test_environment;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tracing::{debug, info};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DuckMode {
    Mute,
    Lower(u32), // percentage, e.g. 15%
}

/// Automatically ducks or mutes media audio output during voice capture
/// to prevent speaker acoustic bleed from confusing Silero VAD and delaying silence detection.
pub struct AudioDucker {
    enabled: bool,
    mode: DuckMode,
    is_ducked: Arc<AtomicBool>,
    was_originally_muted: Arc<AtomicBool>,
    original_volume: Arc<Mutex<Option<f32>>>,
    wpctl_bin: Option<PathBuf>,
    pactl_bin: Option<PathBuf>,
}

impl AudioDucker {
    pub fn new(enabled: bool, mode: DuckMode) -> Self {
        let wpctl_bin = which::which("wpctl").ok();
        let pactl_bin = which::which("pactl").ok();

        Self {
            enabled,
            mode,
            is_ducked: Arc::new(AtomicBool::new(false)),
            was_originally_muted: Arc::new(AtomicBool::new(false)),
            original_volume: Arc::new(Mutex::new(None)),
            wpctl_bin,
            pactl_bin,
        }
    }

    pub fn from_settings(settings: &Settings) -> Self {
        let mode = if settings.duck_mode.eq_ignore_ascii_case("lower") {
            DuckMode::Lower(15)
        } else {
            DuckMode::Mute
        };

        Self::new(settings.duck_media, mode)
    }

    /// Ducks or mutes audio output during voice recording
    pub async fn duck(&self) {
        if !self.enabled || is_test_environment() {
            return;
        }

        if self.is_ducked.load(Ordering::SeqCst) {
            return;
        }

        // Try wpctl first (PipeWire standard)
        if let Some(ref wpctl) = self.wpctl_bin {
            if let Ok(output) = tokio::process::Command::new(wpctl)
                .arg("get-volume")
                .arg("@DEFAULT_AUDIO_SINK@")
                .output()
                .await
            {
                let text = String::from_utf8_lossy(&output.stdout);
                let (vol, is_muted) = Self::parse_wpctl_volume(&text);
                self.was_originally_muted.store(is_muted, Ordering::SeqCst);
                if let Ok(mut g) = self.original_volume.lock() {
                    *g = vol;
                }

                if is_muted {
                    // System was already muted by the user; keep state and don't touch
                    debug!("Audio ducking: Sink was already muted by user. Skipping ducking.");
                    return;
                }

                match self.mode {
                    DuckMode::Mute => {
                        let res = tokio::process::Command::new(wpctl)
                            .arg("set-mute")
                            .arg("@DEFAULT_AUDIO_SINK@")
                            .arg("1")
                            .output()
                            .await;
                        if res.is_ok() {
                            info!("Audio ducking: Muted audio output during speech recording.");
                            self.is_ducked.store(true, Ordering::SeqCst);
                            return;
                        }
                    }
                    DuckMode::Lower(pct) => {
                        let frac = pct as f32 / 100.0;
                        let res = tokio::process::Command::new(wpctl)
                            .arg("set-volume")
                            .arg("@DEFAULT_AUDIO_SINK@")
                            .arg(format!("{:.2}", frac))
                            .output()
                            .await;
                        if res.is_ok() {
                            info!(
                                "Audio ducking: Lowered sink volume to {pct}% (original: {:?}).",
                                vol
                            );
                            self.is_ducked.store(true, Ordering::SeqCst);
                            return;
                        }
                    }
                }
            }
        }

        // Fallback to pactl (PulseAudio compatibility)
        if let Some(ref pactl) = self.pactl_bin {
            if let Ok(output) = tokio::process::Command::new(pactl)
                .arg("get-sink-mute")
                .arg("@DEFAULT_SINK@")
                .output()
                .await
            {
                let text = String::from_utf8_lossy(&output.stdout);
                let is_muted = text.to_lowercase().contains("yes");
                self.was_originally_muted.store(is_muted, Ordering::SeqCst);
                if is_muted {
                    return;
                }

                let _ = tokio::process::Command::new(pactl)
                    .arg("set-sink-mute")
                    .arg("@DEFAULT_SINK@")
                    .arg("1")
                    .output()
                    .await;
                info!("Audio ducking (pactl): Muted sink during speech recording.");
                self.is_ducked.store(true, Ordering::SeqCst);
            }
        }
    }

    /// Restores audio output volume and unmute state immediately after recording finishes
    pub async fn restore(&self) {
        if !self.enabled || is_test_environment() {
            return;
        }

        if !self.is_ducked.load(Ordering::SeqCst) {
            return;
        }

        self.restore_sync();
    }

    /// Synchronous restore helper used by async restore() and Drop
    pub fn restore_sync(&self) {
        if !self.is_ducked.swap(false, Ordering::SeqCst) {
            return;
        }

        let was_muted = self.was_originally_muted.load(Ordering::SeqCst);
        if was_muted {
            // Keep muted if user had it muted initially
            return;
        }

        if let Some(ref wpctl) = self.wpctl_bin {
            match self.mode {
                DuckMode::Mute => {
                    let _ = Command::new(wpctl)
                        .arg("set-mute")
                        .arg("@DEFAULT_AUDIO_SINK@")
                        .arg("0")
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .status();
                    info!("Audio ducking: Restored audio output (unmuted).");
                }
                DuckMode::Lower(_) => {
                    if let Ok(g) = self.original_volume.lock() {
                        if let Some(orig) = *g {
                            let _ = Command::new(wpctl)
                                .arg("set-volume")
                                .arg("@DEFAULT_AUDIO_SINK@")
                                .arg(format!("{:.2}", orig))
                                .stdout(Stdio::null())
                                .stderr(Stdio::null())
                                .status();
                            info!("Audio ducking: Restored audio sink volume to {:.2}.", orig);
                        }
                    }
                }
            }
            return;
        }

        if let Some(ref pactl) = self.pactl_bin {
            let _ = Command::new(pactl)
                .arg("set-sink-mute")
                .arg("@DEFAULT_SINK@")
                .arg("0")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            info!("Audio ducking (pactl): Unmuted audio sink.");
        }
    }

    /// Parses wpctl get-volume output, e.g. "Volume: 0.55" or "Volume: 0.55 [MUTED]"
    pub fn parse_wpctl_volume(text: &str) -> (Option<f32>, bool) {
        let is_muted = text.contains("[MUTED]");
        let vol = text
            .split_whitespace()
            .find_map(|s| s.parse::<f32>().ok());
        (vol, is_muted)
    }
}

impl Drop for AudioDucker {
    fn drop(&mut self) {
        self.restore_sync();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_wpctl_volume_normal() {
        let (vol, muted) = AudioDucker::parse_wpctl_volume("Volume: 0.55\n");
        assert_eq!(vol, Some(0.55));
        assert!(!muted);
    }

    #[test]
    fn test_parse_wpctl_volume_muted() {
        let (vol, muted) = AudioDucker::parse_wpctl_volume("Volume: 0.55 [MUTED]\n");
        assert_eq!(vol, Some(0.55));
        assert!(muted);
    }

    #[test]
    fn test_parse_wpctl_volume_full() {
        let (vol, muted) = AudioDucker::parse_wpctl_volume("Volume: 1.00");
        assert_eq!(vol, Some(1.00));
        assert!(!muted);
    }

    #[test]
    fn test_ducker_initialization() {
        let ducker = AudioDucker::new(true, DuckMode::Mute);
        assert!(ducker.enabled);
        assert_eq!(ducker.mode, DuckMode::Mute);
    }
}
