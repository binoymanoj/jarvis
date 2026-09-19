use crate::core::config::Settings;
use crate::core::error::Result;
use crate::tools::omarchy::OmarchyBridge;
use crate::tools::Tool;
use crate::ui::confirmation::{request_confirmation, ConfirmationRequest};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::info;

pub struct SystemPowerManager {
    omarchy: Arc<OmarchyBridge>,
    settings: Settings,
}

impl SystemPowerManager {
    pub fn new(omarchy: Arc<OmarchyBridge>, settings: Settings) -> Self {
        Self { omarchy, settings }
    }

    pub async fn lock_screen(&self) -> Result<String> {
        if !crate::ui::TaskNotifier::global().is_active() {
            crate::ui::send_desktop_notification(
                "󰌾",
                "Lock Screen",
                "Locking desktop session...",
                2000,
                "low",
            )
            .await;
        }

        let out = self.omarchy.run("system lock", &[]).await?;
        if out.is_empty() {
            Ok("Screen locked.".to_string())
        } else {
            Ok(format!("Screen lock status: {out}"))
        }
    }

    pub async fn logout_system(&self) -> Result<String> {
        let req = ConfirmationRequest {
            title: "Confirm System Logout",
            prompt: "Are you sure you want to log out of your session?",
            spoken_prompt: "Are you sure you want to log out? Please confirm: yes or no.",
            icon_name: "system-log-out",
            timeout_secs: 15,
        };

        let confirmed = request_confirmation(&req, Some(&self.settings)).await;
        if !confirmed {
            info!("System logout cancelled by user.");
            return Ok("Logout cancelled.".to_string());
        }

        let out = self.omarchy.run("system logout", &[]).await?;
        if out.is_empty() {
            Ok("Logging out...".to_string())
        } else {
            Ok(format!("Logout status: {out}"))
        }
    }

    pub async fn reboot_system(&self) -> Result<String> {
        let req = ConfirmationRequest {
            title: "Confirm System Reboot",
            prompt: "Are you sure you want to reboot the system?",
            spoken_prompt: "Are you sure you want to reboot the system? Please confirm: yes or no.",
            icon_name: "system-reboot",
            timeout_secs: 15,
        };

        let confirmed = request_confirmation(&req, Some(&self.settings)).await;
        if !confirmed {
            info!("System reboot cancelled by user.");
            return Ok("Reboot cancelled.".to_string());
        }

        let out = self.omarchy.run("system reboot", &[]).await?;
        if out.is_empty() {
            Ok("Rebooting system...".to_string())
        } else {
            Ok(format!("Reboot status: {out}"))
        }
    }

    pub async fn shutdown_system(&self) -> Result<String> {
        let req = ConfirmationRequest {
            title: "Confirm System Shutdown",
            prompt: "Are you sure you want to shut down the computer?",
            spoken_prompt:
                "Are you sure you want to shut down the computer? Please confirm: yes or no.",
            icon_name: "system-shutdown",
            timeout_secs: 15,
        };

        let confirmed = request_confirmation(&req, Some(&self.settings)).await;
        if !confirmed {
            info!("System shutdown cancelled by user.");
            return Ok("Shutdown cancelled.".to_string());
        }

        let out = self.omarchy.run("system shutdown", &[]).await?;
        if out.is_empty() {
            Ok("Shutting down system...".to_string())
        } else {
            Ok(format!("Shutdown status: {out}"))
        }
    }

    pub async fn get_system_stats(&self) -> Result<String> {
        let out = self.omarchy.run("system stats", &[]).await?;
        if out.is_empty() {
            Ok("System stats currently unavailable.".to_string())
        } else {
            Ok(out)
        }
    }

    pub async fn toggle_bluetooth(&self, action: &str) -> Result<String> {
        let clean_action = match action.to_lowercase().trim() {
            "on" => "on",
            "off" => "off",
            "is-on" => "is-on",
            _ => "toggle",
        };

        let out = self.omarchy.run("bluetooth power", &[clean_action]).await?;
        if clean_action == "is-on" {
            if out.is_empty() {
                Ok("Bluetooth is on.".to_string())
            } else {
                Ok(format!("Bluetooth status: {out}"))
            }
        } else {
            if !crate::ui::TaskNotifier::global().is_active() {
                crate::ui::send_desktop_notification(
                    "󰂯",
                    "Bluetooth",
                    &format!("Bluetooth power set to {clean_action}"),
                    2500,
                    "low",
                )
                .await;
            }
            Ok(format!("Bluetooth power set to {clean_action}."))
        }
    }

    pub async fn network_speedtest(&self, direction: &str) -> Result<String> {
        let clean_dir = if direction.to_lowercase().contains("up") {
            "up"
        } else {
            "down"
        };
        let out = self.omarchy.run("network speedtest", &[clean_dir]).await?;
        if out.is_empty() {
            Ok("Speedtest completed.".to_string())
        } else {
            if !crate::ui::TaskNotifier::global().is_active() {
                crate::ui::send_desktop_notification(
                    "󰛳",
                    "Speedtest Results",
                    &format!("{clean_dir}: {out}"),
                    4000,
                    "normal",
                )
                .await;
            }
            Ok(format!("Network speedtest ({clean_dir}): {out}"))
        }
    }
}

// -----------------------------------------------------------------------------
// Tool Implementations
// -----------------------------------------------------------------------------

pub struct LockScreenTool {
    power: Arc<SystemPowerManager>,
}

impl LockScreenTool {
    pub fn new(power: Arc<SystemPowerManager>) -> Self {
        Self { power }
    }
}

#[async_trait]
impl Tool for LockScreenTool {
    fn name(&self) -> &'static str {
        "lock_screen"
    }

    fn description(&self) -> &'static str {
        "Lock the workstation and sleep displays."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {}
        })
    }

    async fn execute(&self, _args: Value) -> Result<String> {
        self.power.lock_screen().await
    }
}

pub struct LogoutSystemTool {
    power: Arc<SystemPowerManager>,
}

impl LogoutSystemTool {
    pub fn new(power: Arc<SystemPowerManager>) -> Self {
        Self { power }
    }
}

#[async_trait]
impl Tool for LogoutSystemTool {
    fn name(&self) -> &'static str {
        "logout_system"
    }

    fn description(&self) -> &'static str {
        "Log out of the desktop session."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {}
        })
    }

    async fn execute(&self, _args: Value) -> Result<String> {
        self.power.logout_system().await
    }
}

pub struct RebootSystemTool {
    power: Arc<SystemPowerManager>,
}

impl RebootSystemTool {
    pub fn new(power: Arc<SystemPowerManager>) -> Self {
        Self { power }
    }
}

#[async_trait]
impl Tool for RebootSystemTool {
    fn name(&self) -> &'static str {
        "reboot_system"
    }

    fn description(&self) -> &'static str {
        "Reboot the computer."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {}
        })
    }

    async fn execute(&self, _args: Value) -> Result<String> {
        self.power.reboot_system().await
    }
}

pub struct ShutdownSystemTool {
    power: Arc<SystemPowerManager>,
}

impl ShutdownSystemTool {
    pub fn new(power: Arc<SystemPowerManager>) -> Self {
        Self { power }
    }
}

#[async_trait]
impl Tool for ShutdownSystemTool {
    fn name(&self) -> &'static str {
        "shutdown_system"
    }

    fn description(&self) -> &'static str {
        "Shut down the computer."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {}
        })
    }

    async fn execute(&self, _args: Value) -> Result<String> {
        self.power.shutdown_system().await
    }
}

pub struct GetSystemStatsTool {
    power: Arc<SystemPowerManager>,
}

impl GetSystemStatsTool {
    pub fn new(power: Arc<SystemPowerManager>) -> Self {
        Self { power }
    }
}

#[async_trait]
impl Tool for GetSystemStatsTool {
    fn name(&self) -> &'static str {
        "get_system_stats"
    }

    fn description(&self) -> &'static str {
        "Get live system stats (CPU usage, memory usage, network interface)."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {}
        })
    }

    async fn execute(&self, _args: Value) -> Result<String> {
        self.power.get_system_stats().await
    }
}

pub struct ToggleBluetoothTool {
    power: Arc<SystemPowerManager>,
}

impl ToggleBluetoothTool {
    pub fn new(power: Arc<SystemPowerManager>) -> Self {
        Self { power }
    }
}

#[async_trait]
impl Tool for ToggleBluetoothTool {
    fn name(&self) -> &'static str {
        "toggle_bluetooth"
    }

    fn description(&self) -> &'static str {
        "Control Bluetooth power state ('on', 'off', 'toggle', 'is-on')."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "action": {
                    "type": "STRING",
                    "description": "Bluetooth state: 'on', 'off', 'toggle', or 'is-on'. Default is 'toggle'."
                }
            }
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let action = args["action"].as_str().unwrap_or("toggle");
        self.power.toggle_bluetooth(action).await
    }
}

pub struct NetworkSpeedtestTool {
    power: Arc<SystemPowerManager>,
}

impl NetworkSpeedtestTool {
    pub fn new(power: Arc<SystemPowerManager>) -> Self {
        Self { power }
    }
}

#[async_trait]
impl Tool for NetworkSpeedtestTool {
    fn name(&self) -> &'static str {
        "network_speedtest"
    }

    fn description(&self) -> &'static str {
        "Measure live internet speed ('down' or 'up')."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "direction": {
                    "type": "STRING",
                    "description": "Speedtest direction: 'down' (download) or 'up' (upload). Default is 'down'."
                }
            }
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let direction = args["direction"].as_str().unwrap_or("down");
        self.power.network_speedtest(direction).await
    }
}
