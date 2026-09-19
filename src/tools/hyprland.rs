use crate::core::error::{JarvisError, Result};
use crate::tools::Tool;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::process::Command;
use tracing::debug;

#[derive(Clone)]
pub struct HyprlandController {
    cmd_socket_path: Option<PathBuf>,
}

impl Default for HyprlandController {
    fn default() -> Self {
        Self::new()
    }
}

impl HyprlandController {
    pub fn new() -> Self {
        let sig = env::var("HYPRLAND_INSTANCE_SIGNATURE").ok();
        let runtime_dir = env::var("XDG_RUNTIME_DIR")
            .unwrap_or_else(|_| format!("/run/user/{}", nix::unistd::getuid().as_raw()));

        let cmd_socket_path = sig.map(|s| {
            PathBuf::from(runtime_dir)
                .join("hypr")
                .join(s)
                .join(".socket.sock")
        });

        Self { cmd_socket_path }
    }

    pub async fn send_request(&self, payload: &[u8]) -> Result<String> {
        let socket_path =
            self.cmd_socket_path
                .as_ref()
                .ok_or_else(|| JarvisError::ToolExecution {
                    tool: "hyprland".to_string(),
                    message: "HYPRLAND_INSTANCE_SIGNATURE environment variable is not set."
                        .to_string(),
                })?;

        if !socket_path.exists() {
            return Err(JarvisError::Socket {
                path: socket_path.clone(),
                message: "Hyprland socket does not exist. Is Hyprland running?".to_string(),
            });
        }

        let mut stream = UnixStream::connect(socket_path).await?;
        stream.write_all(payload).await?;
        stream.shutdown().await?;

        let mut buffer = Vec::new();
        stream.read_to_end(&mut buffer).await?;
        let resp = String::from_utf8_lossy(&buffer).trim().to_string();
        Ok(resp)
    }

    pub async fn get_json(&self, command: &str) -> Result<Value> {
        let payload = format!("j/{command}").into_bytes();
        let raw = self.send_request(&payload).await?;
        if raw.is_empty() {
            return Ok(json!({}));
        }
        serde_json::from_str(&raw).map_err(JarvisError::Serialization)
    }

    pub async fn get_active_window(&self) -> Result<Value> {
        self.get_json("activewindow").await
    }

    pub async fn get_clients(&self) -> Result<Value> {
        self.get_json("clients").await
    }

    pub async fn get_workspaces(&self) -> Result<Value> {
        self.get_json("workspaces").await
    }

    pub fn translate_legacy_command(cmd: &str) -> String {
        let trimmed = cmd.trim();
        if trimmed.starts_with("hl.") || trimmed.starts_with("hl.dsp.") {
            return trimmed.to_string();
        }

        if let Some(arg) = trimmed.strip_prefix("workspace ") {
            return format!("hl.dsp.focus({{ workspace = \"{}\" }})", arg.trim());
        }
        if let Some(arg) = trimmed.strip_prefix("movetoworkspacesilent ") {
            return format!(
                "hl.dsp.window.move({{ workspace = \"{}\", follow = false }})",
                arg.trim()
            );
        }
        if let Some(arg) = trimmed.strip_prefix("movetoworkspace ") {
            return format!("hl.dsp.window.move({{ workspace = \"{}\" }})", arg.trim());
        }
        if let Some(arg) = trimmed.strip_prefix("focuswindow ") {
            return format!("hl.dsp.focus({{ window = \"{}\" }})", arg.trim());
        }
        if trimmed == "killactive" {
            return "hl.dsp.window.close()".to_string();
        }
        if trimmed == "fullscreen" {
            return "hl.dsp.window.fullscreen({ mode = \"fullscreen\" })".to_string();
        }
        if trimmed == "togglesplit" {
            return "hl.dsp.layout(\"togglesplit\")".to_string();
        }

        trimmed.to_string()
    }

    pub async fn dispatch(&self, cmd: &str) -> Result<String> {
        let lua_cmd = Self::translate_legacy_command(cmd);

        // 1. Try lua command via socket
        let payload = format!("dispatch {lua_cmd}").into_bytes();
        if let Ok(res) = self.send_request(&payload).await {
            let lower = res.to_lowercase();
            if !lower.contains("error:") && !lower.contains("unknown request") {
                return Ok(res);
            }
        }

        // 2. Try raw command via socket if different
        if lua_cmd != cmd {
            let raw_payload = format!("dispatch {cmd}").into_bytes();
            if let Ok(res) = self.send_request(&raw_payload).await {
                let lower = res.to_lowercase();
                if !lower.contains("error:") && !lower.contains("unknown request") {
                    return Ok(res);
                }
            }
        }

        // 3. Fallback to hyprctl with lua_cmd
        debug!("Direct socket dispatch failed for '{lua_cmd}', falling back to hyprctl");
        let output = Command::new("hyprctl")
            .arg("dispatch")
            .arg(&lua_cmd)
            .output()
            .await;

        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if out.status.success() && !stdout.to_lowercase().contains("error:") {
                return Ok(stdout);
            }
        }

        // 4. Fallback to hyprctl with raw cmd
        let raw_output = Command::new("hyprctl")
            .arg("dispatch")
            .arg(cmd)
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&raw_output.stdout)
            .trim()
            .to_string();
        let stderr = String::from_utf8_lossy(&raw_output.stderr)
            .trim()
            .to_string();
        if !stdout.is_empty() {
            Ok(stdout)
        } else {
            Ok(stderr)
        }
    }

    pub async fn change_workspace(&self, workspace_id: i32) -> Result<String> {
        self.dispatch(&format!("workspace {workspace_id}")).await
    }

    pub async fn focus_window(&self, address: &str) -> Result<String> {
        let clean_addr = if address.starts_with("address:") {
            address.to_string()
        } else {
            format!("address:{address}")
        };
        self.dispatch(&format!("focuswindow {clean_addr}")).await
    }

    pub async fn focus_app(&self, app_name: &str) -> Result<bool> {
        let clients = self.get_clients().await?;
        let query = app_name.to_lowercase();

        if let Some(client_list) = clients.as_array() {
            for client in client_list {
                let c_class = client["class"].as_str().unwrap_or("").to_lowercase();
                let c_title = client["title"].as_str().unwrap_or("").to_lowercase();
                let c_initial = client["initialClass"].as_str().unwrap_or("").to_lowercase();

                if c_class.contains(&query)
                    || c_title.contains(&query)
                    || c_initial.contains(&query)
                {
                    if let Some(addr) = client["address"].as_str() {
                        self.focus_window(addr).await?;
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }

    pub async fn close_active_window(&self) -> Result<String> {
        self.dispatch("killactive").await
    }

    pub async fn toggle_split(&self) -> Result<String> {
        self.dispatch("togglesplit").await
    }

    pub async fn toggle_fullscreen(&self) -> Result<String> {
        self.dispatch("fullscreen").await
    }

    pub async fn move_window_to_workspace(
        &self,
        workspace_id: i32,
        silent: bool,
    ) -> Result<String> {
        let cmd = if silent {
            format!("movetoworkspacesilent {workspace_id}")
        } else {
            format!("movetoworkspace {workspace_id}")
        };
        self.dispatch(&cmd).await
    }
}

// -----------------------------------------------------------------------------
// Tool Implementations
// -----------------------------------------------------------------------------

pub struct SwitchWorkspaceTool {
    hyprland: Arc<HyprlandController>,
}

impl SwitchWorkspaceTool {
    pub fn new(hyprland: Arc<HyprlandController>) -> Self {
        Self { hyprland }
    }
}

#[async_trait]
impl Tool for SwitchWorkspaceTool {
    fn name(&self) -> &'static str {
        "switch_workspace"
    }

    fn description(&self) -> &'static str {
        "Switch to a specific Hyprland workspace (e.g., 1, 2, 3)."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "workspace_id": {
                    "type": "INTEGER",
                    "description": "The workspace number to switch to (e.g. 1, 2, 3)."
                }
            },
            "required": ["workspace_id"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let workspace_id = args["workspace_id"].as_i64().ok_or_else(|| {
            JarvisError::ToolParameter(
                "switch_workspace".into(),
                "workspace_id integer is required".into(),
            )
        })? as i32;

        self.hyprland.change_workspace(workspace_id).await?;
        Ok(format!("Switched to workspace {workspace_id}."))
    }
}

pub struct FocusApplicationTool {
    hyprland: Arc<HyprlandController>,
}

impl FocusApplicationTool {
    pub fn new(hyprland: Arc<HyprlandController>) -> Self {
        Self { hyprland }
    }
}

#[async_trait]
impl Tool for FocusApplicationTool {
    fn name(&self) -> &'static str {
        "focus_application"
    }

    fn description(&self) -> &'static str {
        "Focus an open application window by name or class (e.g., 'terminal', 'browser', 'foot', 'slack')."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "app_name": {
                    "type": "STRING",
                    "description": "Name or class of the open application to focus."
                }
            },
            "required": ["app_name"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let app_name = args["app_name"].as_str().ok_or_else(|| {
            JarvisError::ToolParameter(
                "focus_application".into(),
                "app_name string is required".into(),
            )
        })?;

        let found = self.hyprland.focus_app(app_name).await?;
        if found {
            Ok(format!("Focused {app_name}."))
        } else {
            Ok(format!(
                "Could not find open application matching '{app_name}'."
            ))
        }
    }
}

pub struct CloseActiveWindowTool {
    hyprland: Arc<HyprlandController>,
}

impl CloseActiveWindowTool {
    pub fn new(hyprland: Arc<HyprlandController>) -> Self {
        Self { hyprland }
    }
}

#[async_trait]
impl Tool for CloseActiveWindowTool {
    fn name(&self) -> &'static str {
        "close_active_window"
    }

    fn description(&self) -> &'static str {
        "Close the currently focused window."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {}
        })
    }

    async fn execute(&self, _args: Value) -> Result<String> {
        self.hyprland.close_active_window().await?;
        Ok("Closed active window.".to_string())
    }
}

pub struct ToggleLayoutSplitTool {
    hyprland: Arc<HyprlandController>,
}

impl ToggleLayoutSplitTool {
    pub fn new(hyprland: Arc<HyprlandController>) -> Self {
        Self { hyprland }
    }
}

#[async_trait]
impl Tool for ToggleLayoutSplitTool {
    fn name(&self) -> &'static str {
        "toggle_layout_split"
    }

    fn description(&self) -> &'static str {
        "Toggle the split orientation between vertical and horizontal in dwindle layout."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {}
        })
    }

    async fn execute(&self, _args: Value) -> Result<String> {
        self.hyprland.toggle_split().await?;
        Ok("Toggled window split orientation.".to_string())
    }
}

pub struct ToggleFullscreenTool {
    hyprland: Arc<HyprlandController>,
}

impl ToggleFullscreenTool {
    pub fn new(hyprland: Arc<HyprlandController>) -> Self {
        Self { hyprland }
    }
}

#[async_trait]
impl Tool for ToggleFullscreenTool {
    fn name(&self) -> &'static str {
        "toggle_fullscreen"
    }

    fn description(&self) -> &'static str {
        "Toggle fullscreen state of the currently active window."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {}
        })
    }

    async fn execute(&self, _args: Value) -> Result<String> {
        self.hyprland.toggle_fullscreen().await?;
        Ok("Toggled fullscreen.".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_schemas() {
        let hl = Arc::new(HyprlandController::new());
        let switch_tool = SwitchWorkspaceTool::new(hl.clone());
        assert_eq!(switch_tool.name(), "switch_workspace");
        let schema = switch_tool.parameters_schema();
        assert_eq!(schema["type"], "OBJECT");
        assert!(schema["properties"]["workspace_id"].is_object());
    }
}
