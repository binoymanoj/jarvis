use crate::core::error::{JarvisError, Result};
use crate::ui::theme::load_omarchy_theme;
use std::env;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::process::Command;
use tokio::time::sleep;
use tracing::{debug, warn};

pub struct JarvisHUD {
    ui_path: PathBuf,
    qs_bin: PathBuf,
    is_started: AtomicBool,
}

impl JarvisHUD {
    pub fn new() -> Self {
        let qs_bin = which::which("qs").unwrap_or_else(|_| PathBuf::from("/usr/bin/qs"));
        let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());

        let home_path = PathBuf::from(&home);

        let candidates = [
            home_path.join(".config/jarvis/ui"),
            home_path.join(".local/share/jarvis/ui"),
            PathBuf::from("./ui"),
            PathBuf::from("/usr/share/jarvis/ui"),
            PathBuf::from("/usr/local/share/jarvis/ui"),
        ];

        let mut ui_path = candidates
            .iter()
            .find(|p| p.join("shell.qml").is_file())
            .cloned();

        if ui_path.is_none() {
            if let Ok(exe) = env::current_exe() {
                if let Some(parent) = exe.parent() {
                    let exe_ui = parent.join("ui");
                    if exe_ui.join("shell.qml").is_file() {
                        ui_path = Some(exe_ui);
                    } else if let Some(grandparent) = parent.parent() {
                        let share_ui = grandparent.join("share/jarvis/ui");
                        if share_ui.join("shell.qml").is_file() {
                            ui_path = Some(share_ui);
                        }
                    }
                }
            }
        }

        let ui_path = ui_path.unwrap_or_else(|| candidates[0].clone());

        Self {
            ui_path,
            qs_bin,
            is_started: AtomicBool::new(false),
        }
    }

    pub fn with_ui_path(ui_path: PathBuf) -> Self {
        let qs_bin = which::which("qs").unwrap_or_else(|_| PathBuf::from("/usr/bin/qs"));
        Self {
            ui_path,
            qs_bin,
            is_started: AtomicBool::new(false),
        }
    }

    /// Check if Quickshell HUD is running or start it in background detached mode
    pub async fn ensure_running(&self) -> bool {
        if self.is_started.load(Ordering::SeqCst) {
            return true;
        }

        // 1. Try pinging existing instance
        let ping_res = tokio::time::timeout(
            Duration::from_millis(500),
            Command::new(&self.qs_bin)
                .arg("ipc")
                .arg("-p")
                .arg(&self.ui_path)
                .arg("call")
                .arg("jarvis")
                .arg("ping")
                .output(),
        )
        .await;

        if let Ok(Ok(output)) = ping_res {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains("pong") {
                self.is_started.store(true, Ordering::SeqCst);
                return true;
            }
        }

        // 2. Launch detached instance if shell.qml is found
        if !self.ui_path.join("shell.qml").is_file() {
            debug!(
                "Quickshell HUD shell.qml not found at {:?}, skipping HUD launch",
                self.ui_path
            );
            return false;
        }

        debug!("Spawning Quickshell HUD daemon from {:?}", self.ui_path);
        let launch = tokio::time::timeout(
            Duration::from_millis(1500),
            Command::new(&self.qs_bin)
                .arg("-d")
                .arg("-n")
                .arg("-p")
                .arg(&self.ui_path)
                .output(),
        )
        .await;

        match launch {
            Ok(Ok(_)) => {
                sleep(Duration::from_millis(250)).await;
                self.is_started.store(true, Ordering::SeqCst);
                true
            }
            _ => {
                self.is_started.store(false, Ordering::SeqCst);
                false
            }
        }
    }

    /// Low-level IPC call to Quickshell HUD
    async fn call_ipc(&self, method: &str, args: &[&str]) -> Result<String> {
        if !self.ensure_running().await {
            return Ok(String::new());
        }

        let mut cmd = Command::new(&self.qs_bin);
        cmd.arg("ipc")
            .arg("-p")
            .arg(&self.ui_path)
            .arg("call")
            .arg("jarvis")
            .arg(method);

        for arg in args {
            cmd.arg(arg);
        }

        let output_res = tokio::time::timeout(Duration::from_secs(2), cmd.output()).await;
        match output_res {
            Ok(Ok(output)) => {
                if !output.status.success() {
                    self.is_started.store(false, Ordering::SeqCst);
                }
                Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
            }
            Ok(Err(e)) => {
                self.is_started.store(false, Ordering::SeqCst);
                Err(JarvisError::Io(e))
            }
            Err(_) => {
                warn!("Quickshell HUD IPC call '{method}' timed out after 2s");
                self.is_started.store(false, Ordering::SeqCst);
                Ok(String::new())
            }
        }
    }

    /// Sync currently active Omarchy theme colors with the HUD
    pub async fn sync_theme(&self) {
        let theme = load_omarchy_theme();
        let _ = self
            .call_ipc(
                "setTheme",
                &[
                    &theme.accent,
                    &theme.foreground,
                    &theme.background,
                    &theme.dark_background,
                    &theme.cyan,
                    &theme.magenta,
                    &theme.blue,
                    &theme.name,
                ],
            )
            .await;
    }

    /// Transition HUD to listening state
    pub async fn show_listening(&self, caption: Option<&str>) {
        self.sync_theme().await;
        let text = caption.unwrap_or("Listening...");
        let _ = self.call_ipc("setListening", &[text]).await;
    }

    /// Real-time 20fps volume metering (0.0 to 1.0)
    pub async fn update_volume(&self, volume: f32) {
        let vol_str = format!("{:.2}", volume.clamp(0.0, 1.0));
        let _ = self.call_ipc("setVolume", &[&vol_str]).await;
    }

    /// Transition HUD to thinking state
    pub async fn show_thinking(&self, caption: Option<&str>) {
        let text = caption.unwrap_or("Thinking...");
        let _ = self.call_ipc("setThinking", &[text]).await;
    }

    /// Transition HUD to speaking state
    pub async fn show_speaking(&self, caption: Option<&str>) {
        let text = caption.unwrap_or("Speaking...");
        let _ = self.call_ipc("setSpeaking", &[text]).await;
    }

    /// Hide HUD smoothly
    pub async fn hide(&self) {
        let _ = self.call_ipc("hide", &[]).await;
    }
}

impl Default for JarvisHUD {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hud_initialization() {
        let hud = JarvisHUD::new();
        assert!(!hud.qs_bin.as_os_str().is_empty());
    }
}
