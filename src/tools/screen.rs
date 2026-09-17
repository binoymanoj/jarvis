use crate::core::error::{JarvisError, Result};
use crate::tools::hyprland::HyprlandController;
use crate::tools::Tool;
use async_trait::async_trait;
use chrono::Local;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::process::Command;
use tracing::{debug, warn};

pub struct ScreenPerception {
    grim_bin: PathBuf,
    storage_dir: PathBuf,
    hyprland: Arc<HyprlandController>,
}

impl ScreenPerception {
    pub fn new(hyprland: Arc<HyprlandController>) -> Self {
        let grim_bin = which::which("grim").unwrap_or_else(|_| PathBuf::from("/usr/bin/grim"));
        let storage_dir = PathBuf::from("/tmp/jarvis_captures");
        let _ = fs::create_dir_all(&storage_dir);
        Self {
            grim_bin,
            storage_dir,
            hyprland,
        }
    }

    fn generate_capture_path(&self, prefix: &str) -> PathBuf {
        let timestamp = Local::now().format("%Y%m%d_%H%M%S_%f").to_string();
        self.storage_dir.join(format!("{prefix}_{timestamp}.png"))
    }

    pub async fn capture_full_screen(&self, output_path: Option<&Path>) -> Result<PathBuf> {
        let target = output_path
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| self.generate_capture_path("fullscreen"));

        let output = Command::new(&self.grim_bin)
            .arg("-t")
            .arg("png")
            .arg(&target)
            .output()
            .await?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(JarvisError::ToolExecution {
                tool: "screen".to_string(),
                message: format!("grim full screenshot failed: {err}"),
            });
        }
        Ok(target)
    }

    pub async fn capture_active_window(&self, output_path: Option<&Path>) -> Result<PathBuf> {
        let win = self.hyprland.get_active_window().await.unwrap_or(json!({}));
        let at = win["at"].as_array();
        let size = win["size"].as_array();

        let valid_geom = match (at, size) {
            (Some(a), Some(s)) if a.len() >= 2 && s.len() >= 2 => {
                let w = s[0].as_i64().unwrap_or(0);
                let h = s[1].as_i64().unwrap_or(0);
                let x = a[0].as_i64().unwrap_or(0);
                let y = a[1].as_i64().unwrap_or(0);
                if w > 0 && h > 0 {
                    Some(format!("{x},{y} {w}x{h}"))
                } else {
                    None
                }
            }
            _ => None,
        };

        let target = output_path
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| self.generate_capture_path("activewindow"));

        if let Some(geometry) = valid_geom {
            let output = Command::new(&self.grim_bin)
                .arg("-g")
                .arg(&geometry)
                .arg("-t")
                .arg("png")
                .arg(&target)
                .output()
                .await?;

            if output.status.success() {
                return Ok(target);
            }
            warn!("grim active window geometry failed, falling back to fullscreen capture");
        }

        self.capture_full_screen(Some(&target)).await
    }
}

// -----------------------------------------------------------------------------
// Tool Implementations
// -----------------------------------------------------------------------------

pub struct InspectScreenTool {
    screen: Arc<ScreenPerception>,
}

impl InspectScreenTool {
    pub fn new(screen: Arc<ScreenPerception>) -> Self {
        Self { screen }
    }
}

#[async_trait]
impl Tool for InspectScreenTool {
    fn name(&self) -> &'static str {
        "inspect_screen"
    }

    fn description(&self) -> &'static str {
        "Capture the screen or active window and analyze it visually."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "query": {
                    "type": "STRING",
                    "description": "The visual question or perception goal (e.g. 'what error is showing?')."
                },
                "target": {
                    "type": "STRING",
                    "description": "Capture target: 'active_window' or 'fullscreen'. Default is 'active_window'."
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let query = args["query"]
            .as_str()
            .ok_or_else(|| JarvisError::ToolParameter("inspect_screen".into(), "query string is required".into()))?;
        let target = args["target"].as_str().unwrap_or("active_window");

        let img_path = if target == "fullscreen" {
            self.screen.capture_full_screen(None).await?
        } else {
            self.screen.capture_active_window(None).await?
        };

        debug!("Captured screen image at: {:?}", img_path);
        // Returns the image path and status; the AI agent coordinator replaces this with multimodal vision payload
        Ok(format!("[Screenshot captured at {:?}] User query: '{query}'", img_path))
    }
}
