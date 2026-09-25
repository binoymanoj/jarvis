use crate::core::error::{JarvisError, Result};
use crate::tools::Tool;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::process::Command;
use tracing::warn;

#[derive(Clone)]
pub struct OmarchyBridge {
    omarchy_bin: PathBuf,
    notify_bin: PathBuf,
}

impl Default for OmarchyBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl OmarchyBridge {
    pub fn new() -> Self {
        let omarchy_bin =
            which::which("omarchy").unwrap_or_else(|_| PathBuf::from("/usr/bin/omarchy"));

        let notify_bin = which::which("omarchy-notification-send")
            .unwrap_or_else(|_| PathBuf::from("/usr/share/omarchy/bin/omarchy-notification-send"));

        Self {
            omarchy_bin,
            notify_bin,
        }
    }

    pub async fn run(&self, subcommand: &str, extra_args: &[&str]) -> Result<String> {
        let sub_parts: Vec<&str> = subcommand.split_whitespace().collect();
        let mut cmd = Command::new(&self.omarchy_bin);
        for part in sub_parts {
            cmd.arg(part);
        }
        for arg in extra_args {
            cmd.arg(arg);
        }

        let output = cmd.output().await?;
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

        if !output.status.success() && !stderr.is_empty() {
            warn!("Omarchy command failed: {stderr}");
            return Ok(stderr);
        }

        Ok(stdout)
    }

    pub async fn notify(
        &self,
        headline: &str,
        description: &str,
        glyph: &str,
        urgency: &str,
    ) -> Result<bool> {
        let mut cmd = Command::new(&self.notify_bin);
        cmd.arg("-g")
            .arg(glyph)
            .arg("-u")
            .arg(urgency)
            .arg(headline);

        if !description.is_empty() {
            cmd.arg(description);
        }

        let status = cmd.status().await?;
        Ok(status.success())
    }

    pub async fn show_osd(&self, icon: &str, message: &str, duration_ms: u64) -> Result<()> {
        let osd_bin = which::which("omarchy-osd")
            .unwrap_or_else(|_| PathBuf::from("/usr/bin/omarchy-osd"));

        let _ = Command::new(osd_bin)
            .arg("-i")
            .arg(icon)
            .arg("-m")
            .arg(message)
            .arg("-d")
            .arg(duration_ms.to_string())
            .output()
            .await;

        Ok(())
    }

    pub async fn set_volume(&self, adjustment: &str) -> Result<String> {
        self.run("audio output volume", &[adjustment]).await
    }

    pub async fn get_brightness(&self) -> Result<String> {
        self.run("brightness display", &[]).await
    }

    pub async fn set_brightness(&self, level_or_delta: &str) -> Result<String> {
        self.run("brightness display", &[level_or_delta]).await
    }

    pub async fn get_theme(&self) -> Result<String> {
        self.run("theme current", &[]).await
    }

    pub async fn set_theme(&self, theme_name: &str) -> Result<String> {
        self.run("theme set", &[theme_name]).await
    }

    pub async fn list_themes(&self) -> Result<Vec<String>> {
        let out = self.run("theme list", &[]).await?;
        Ok(out
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect())
    }

    pub async fn get_battery_status(&self) -> Result<String> {
        self.run("battery status", &[]).await
    }

    pub async fn launch_app(&self, app_name: &str, args: &[&str]) -> Result<String> {
        let mut full_args = vec![app_name];
        full_args.extend_from_slice(args);
        self.run("launch", &full_args).await
    }
}

// -----------------------------------------------------------------------------
// Tool Implementations
// -----------------------------------------------------------------------------

pub struct AdjustVolumeTool {
    omarchy: Arc<OmarchyBridge>,
}

impl AdjustVolumeTool {
    pub fn new(omarchy: Arc<OmarchyBridge>) -> Self {
        Self { omarchy }
    }
}

#[async_trait]
impl Tool for AdjustVolumeTool {
    fn name(&self) -> &'static str {
        "adjust_volume"
    }

    fn description(&self) -> &'static str {
        "Adjust audio volume. Options: 'raise', 'lower', '+5', '-10', 'mute-toggle'."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "adjustment": {
                    "type": "STRING",
                    "description": "Adjustment value or command (e.g., 'raise', 'lower', '+5', '-10', 'mute-toggle')."
                }
            },
            "required": ["adjustment"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let adjustment = args["adjustment"].as_str().ok_or_else(|| {
            JarvisError::ToolParameter(
                "adjust_volume".into(),
                "adjustment string is required".into(),
            )
        })?;

        let out = self.omarchy.set_volume(adjustment).await?;
        Ok(format!("Volume adjusted: {out}"))
    }
}

pub struct SetBrightnessTool {
    omarchy: Arc<OmarchyBridge>,
}

impl SetBrightnessTool {
    pub fn new(omarchy: Arc<OmarchyBridge>) -> Self {
        Self { omarchy }
    }
}

#[async_trait]
impl Tool for SetBrightnessTool {
    fn name(&self) -> &'static str {
        "set_brightness"
    }

    fn description(&self) -> &'static str {
        "Set display brightness. Options: '+10', '-10', or absolute '50'."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "level": {
                    "type": "STRING",
                    "description": "Brightness level (e.g., '50', '+10', '-10')."
                }
            },
            "required": ["level"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let level = args["level"].as_str().ok_or_else(|| {
            JarvisError::ToolParameter("set_brightness".into(), "level string is required".into())
        })?;

        let out = self.omarchy.set_brightness(level).await?;
        Ok(format!("Brightness updated: {out}"))
    }
}

pub struct SetThemeTool {
    omarchy: Arc<OmarchyBridge>,
}

impl SetThemeTool {
    pub fn new(omarchy: Arc<OmarchyBridge>) -> Self {
        Self { omarchy }
    }
}

#[async_trait]
impl Tool for SetThemeTool {
    fn name(&self) -> &'static str {
        "set_theme"
    }

    fn description(&self) -> &'static str {
        "Apply an Omarchy system theme (e.g., 'tokyo-night', 'catppuccin', 'solitude', 'gruvbox')."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "theme_name": {
                    "type": "STRING",
                    "description": "Name of the theme to apply."
                }
            },
            "required": ["theme_name"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let theme_name = args["theme_name"].as_str().ok_or_else(|| {
            JarvisError::ToolParameter("set_theme".into(), "theme_name string is required".into())
        })?;

        let out = self.omarchy.set_theme(theme_name).await?;
        Ok(format!("Theme changed: {out}"))
    }
}

pub struct GetBatteryTool {
    omarchy: Arc<OmarchyBridge>,
}

impl GetBatteryTool {
    pub fn new(omarchy: Arc<OmarchyBridge>) -> Self {
        Self { omarchy }
    }
}

#[async_trait]
impl Tool for GetBatteryTool {
    fn name(&self) -> &'static str {
        "get_battery"
    }

    fn description(&self) -> &'static str {
        "Check system battery percentage and remaining run time."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {}
        })
    }

    async fn execute(&self, _args: Value) -> Result<String> {
        self.omarchy.get_battery_status().await
    }
}

pub struct LaunchApplicationTool {
    omarchy: Arc<OmarchyBridge>,
}

impl LaunchApplicationTool {
    pub fn new(omarchy: Arc<OmarchyBridge>) -> Self {
        Self { omarchy }
    }
}

#[async_trait]
impl Tool for LaunchApplicationTool {
    fn name(&self) -> &'static str {
        "launch_application"
    }

    fn description(&self) -> &'static str {
        "Launch an app via Omarchy launcher (e.g. 'terminal', 'browser', 'nautilus')."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "app_name": {
                    "type": "STRING",
                    "description": "Name of the application to launch."
                }
            },
            "required": ["app_name"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let app_name = args["app_name"].as_str().ok_or_else(|| {
            JarvisError::ToolParameter(
                "launch_application".into(),
                "app_name string is required".into(),
            )
        })?;

        self.omarchy.launch_app(app_name, &[]).await?;
        Ok(format!("Launched {app_name}."))
    }
}

pub struct NotifyTool {
    omarchy: Arc<OmarchyBridge>,
}

impl NotifyTool {
    pub fn new(omarchy: Arc<OmarchyBridge>) -> Self {
        Self { omarchy }
    }
}

#[async_trait]
impl Tool for NotifyTool {
    fn name(&self) -> &'static str {
        "notify"
    }

    fn description(&self) -> &'static str {
        "Send an Omarchy desktop notification."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "headline": {
                    "type": "STRING",
                    "description": "Main notification title."
                },
                "description": {
                    "type": "STRING",
                    "description": "Optional secondary description text."
                }
            },
            "required": ["headline"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let headline = args["headline"].as_str().ok_or_else(|| {
            JarvisError::ToolParameter("notify".into(), "headline string is required".into())
        })?;
        let description = args["description"].as_str().unwrap_or("");

        self.omarchy
            .notify(headline, description, "󰚩", "normal")
            .await?;
        Ok("Notification displayed.".to_string())
    }
}
