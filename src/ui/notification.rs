use std::process::Stdio;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::LazyLock;
use tokio::process::Command;
use tracing::debug;

pub struct TaskNotifier {
    active_id: AtomicU32,
    last_glyph: std::sync::RwLock<String>,
    last_headline: std::sync::RwLock<String>,
}

static NOTIFIER: LazyLock<TaskNotifier> = LazyLock::new(|| TaskNotifier {
    active_id: AtomicU32::new(0),
    last_glyph: std::sync::RwLock::new(String::from("󰚩")),
    last_headline: std::sync::RwLock::new(String::from("Jarvis: Complete")),
});

#[derive(Debug, Clone)]
pub struct ToolNotificationInfo {
    pub glyph: &'static str,
    pub headline: &'static str,
    pub description: String,
    pub completion_headline: &'static str,
}

/// Map tool name and parameters to rich icon, active headline, friendly description, and completion headline
pub fn tool_notification_info(
    call_name: &str,
    call_args: &serde_json::Value,
) -> ToolNotificationInfo {
    match call_name {
        "get_battery" => ToolNotificationInfo {
            glyph: "󰁹",
            headline: "󰁹 Battery Health",
            description: "Checking battery health & power metrics...".to_string(),
            completion_headline: "󰁹 Battery Status",
        },
        "get_system_stats" => ToolNotificationInfo {
            glyph: "󰍛",
            headline: "󰍛 System Stats",
            description: "Measuring CPU, RAM & system diagnostics...".to_string(),
            completion_headline: "󰍛 System Diagnostics",
        },
        "network_speedtest" => ToolNotificationInfo {
            glyph: "󰛳",
            headline: "󰛳 Network Speedtest",
            description: "Testing network throughput...".to_string(),
            completion_headline: "󰛳 Speedtest Results",
        },
        "display_research_in_neovim" => {
            let title = call_args["title"].as_str().unwrap_or("Research");
            ToolNotificationInfo {
                glyph: "󰈙",
                headline: "󰈙 Research Assistant",
                description: format!("Generating research on '{title}' in Neovim..."),
                completion_headline: "󰈙 Research Ready",
            }
        }
        "create_project" => {
            let desc = call_args["description"].as_str().unwrap_or("project");
            ToolNotificationInfo {
                glyph: "󰲋",
                headline: "󰲋 Project Scaffolding",
                description: format!("Scaffolding autonomous code project: {desc}..."),
                completion_headline: "󰲋 Project Created",
            }
        }
        "delegate_to_antigravity" => ToolNotificationInfo {
            glyph: "󰲋",
            headline: "󰲋 Autonomous Coding",
            description: "Executing autonomous coding task...".to_string(),
            completion_headline: "󰲋 Task Delegated",
        },
        "open_file_in_editor" => {
            let file = call_args["file_path"].as_str().unwrap_or("file");
            ToolNotificationInfo {
                glyph: "󰏪",
                headline: "󰏪 Code Editor",
                description: format!("Opening {file} in Neovim..."),
                completion_headline: "󰏪 Editor Ready",
            }
        }
        "play_media" => {
            let query = call_args["query"].as_str().unwrap_or("media");
            ToolNotificationInfo {
                glyph: "󰐌",
                headline: "󰐌 Media Player",
                description: format!("Launching '{query}' in fullscreen..."),
                completion_headline: "󰐌 Now Playing",
            }
        }
        "resume_media" => {
            let query = call_args["query"].as_str().unwrap_or("media");
            ToolNotificationInfo {
                glyph: "󰐌",
                headline: "󰐌 Media Player",
                description: format!("Resuming playback for '{query}'..."),
                completion_headline: "󰐌 Playback Resumed",
            }
        }
        "localsend_share" => {
            let item = call_args["item"].as_str().unwrap_or("item");
            ToolNotificationInfo {
                glyph: "󰄬",
                headline: "󰄬 LocalSend",
                description: format!("Sharing '{item}' via LocalSend..."),
                completion_headline: "󰄬 LocalSend Share",
            }
        }
        "create_note" => ToolNotificationInfo {
            glyph: "󰏪",
            headline: "󰏪 Notes",
            description: "Saving markdown note to disk...".to_string(),
            completion_headline: "󰏪 Note Saved",
        },
        "schedule_event" => ToolNotificationInfo {
            glyph: "󰸗",
            headline: "󰸗 Calendar",
            description: "Scheduling calendar event...".to_string(),
            completion_headline: "󰸗 Event Scheduled",
        },
        "set_reminder" => ToolNotificationInfo {
            glyph: "󰔛",
            headline: "󰔛 Reminder",
            description: "Configuring countdown reminder...".to_string(),
            completion_headline: "󰔛 Reminder Configured",
        },
        "launch_workflow" => {
            let name = call_args["workflow_name"].as_str().unwrap_or("environment");
            ToolNotificationInfo {
                glyph: "󰌨",
                headline: "󰌨 Workflow",
                description: format!("Activating workflow '{name}'..."),
                completion_headline: "󰌨 Workflow Activated",
            }
        }
        "execute_command" => {
            let cmd = call_args["command"].as_str().unwrap_or("command");
            let short_cmd = if cmd.len() > 38 {
                format!("{}...", &cmd[..35])
            } else {
                cmd.to_string()
            };
            ToolNotificationInfo {
                glyph: "󰞷",
                headline: "󰞷 Shell Command",
                description: format!("Running: {short_cmd}"),
                completion_headline: "󰞷 Command Executed",
            }
        }
        "inspect_screen" => ToolNotificationInfo {
            glyph: "󰍹",
            headline: "󰍹 Screen Vision",
            description: "Analyzing desktop screen with vision model...".to_string(),
            completion_headline: "󰍹 Vision Analyzed",
        },
        "draft_email" => ToolNotificationInfo {
            glyph: "󰇮",
            headline: "󰇮 Email Draft",
            description: "Composing email draft...".to_string(),
            completion_headline: "󰇮 Email Drafted",
        },
        "set_clipboard" => ToolNotificationInfo {
            glyph: "󰅍",
            headline: "󰅍 Clipboard",
            description: "Copying text to clipboard...".to_string(),
            completion_headline: "󰅍 Copied to Clipboard",
        },
        "lock_screen" => ToolNotificationInfo {
            glyph: "󰌾",
            headline: "󰌾 Lock Screen",
            description: "Locking desktop session...".to_string(),
            completion_headline: "󰌾 Screen Locked",
        },
        "reboot_system" => ToolNotificationInfo {
            glyph: "󰐥",
            headline: "󰐥 Reboot Confirmation",
            description: "Initiating reboot confirmation...".to_string(),
            completion_headline: "󰐥 Rebooting System",
        },
        "shutdown_system" => ToolNotificationInfo {
            glyph: "󰐥",
            headline: "󰐥 Shutdown Confirmation",
            description: "Initiating shutdown confirmation...".to_string(),
            completion_headline: "󰐥 Shutting Down",
        },
        "switch_workspace" => {
            let ws = call_args["workspace"].as_i64().unwrap_or(1);
            ToolNotificationInfo {
                glyph: "󰚩",
                headline: "󰚩 Jarvis",
                description: format!("Switching to workspace {ws}..."),
                completion_headline: "󰚩 Workspace Changed",
            }
        }
        "focus_application" => {
            let app = call_args["app_name"].as_str().unwrap_or("app");
            ToolNotificationInfo {
                glyph: "󰚩",
                headline: "󰚩 Jarvis",
                description: format!("Focusing {app}..."),
                completion_headline: "󰚩 Application Focused",
            }
        }
        "dismiss_session" => ToolNotificationInfo {
            glyph: "󰚩",
            headline: "󰚩 Jarvis",
            description: "Ending voice session...".to_string(),
            completion_headline: "󰚩 Jarvis",
        },
        _ => ToolNotificationInfo {
            glyph: "󰚩",
            headline: "󰚩 Jarvis: Working...",
            description: format!("Executing {call_name}..."),
            completion_headline: "󰚩 Jarvis: Complete",
        },
    }
}

impl TaskNotifier {
    pub fn global() -> &'static Self {
        &NOTIFIER
    }

    /// Check if a working task notification is currently active on screen
    pub fn is_active(&self) -> bool {
        self.active_id.load(Ordering::SeqCst) > 0
    }

    /// Reset internal notification tracking state
    pub fn reset_id(&self) {
        self.active_id.store(0, Ordering::SeqCst);
    }

    /// Get current active notification id
    pub fn current_id(&self) -> u32 {
        self.active_id.load(Ordering::SeqCst)
    }

    /// Display initial working notification with default glyph
    pub async fn start(&self, headline: &str, task_description: &str) {
        self.start_with_glyph("󰚩", headline, task_description).await;
    }

    /// Display initial working notification with custom glyph and track notification ID for in-place updates
    pub async fn start_with_glyph(&self, glyph: &str, headline: &str, task_description: &str) {
        if let Ok(mut g) = self.last_glyph.write() {
            *g = glyph.to_string();
        }
        if let Ok(mut h) = self.last_headline.write() {
            *h = headline.to_string();
        }

        let clean_desc = if task_description.trim().is_empty() {
            "Processing your request..."
        } else {
            task_description.trim()
        };

        // 1. Try omarchy-notification-send (prints ID with -p)
        if let Ok(path) = which::which("omarchy-notification-send") {
            let output = Command::new(path)
                .arg("--app-name")
                .arg("Jarvis")
                .arg("-g")
                .arg(glyph)
                .arg("-u")
                .arg("normal")
                .arg("-p")
                .arg("-t")
                .arg("0")
                .arg(headline)
                .arg(clean_desc)
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .output()
                .await;

            if let Ok(out) = output {
                if out.status.success() {
                    let id_str = String::from_utf8_lossy(&out.stdout);
                    if let Ok(id) = id_str.trim().parse::<u32>() {
                        debug!("Omarchy notification started with ID: {id}");
                        self.active_id.store(id, Ordering::SeqCst);
                        return;
                    }
                }
            }
        }

        // 2. Fallback to notify-send with synchronous tag
        if let Ok(path) = which::which("notify-send") {
            let output = Command::new(path)
                .arg("-a")
                .arg("Jarvis")
                .arg("-p")
                .arg("-t")
                .arg("0")
                .arg("-h")
                .arg("string:x-canonical-private-synchronous:jarvis-task")
                .arg("-h")
                .arg("string:synchronous:jarvis-task")
                .arg(headline)
                .arg(clean_desc)
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .output()
                .await;

            if let Ok(out) = output {
                if out.status.success() {
                    let id_str = String::from_utf8_lossy(&out.stdout);
                    if let Ok(id) = id_str.trim().parse::<u32>() {
                        debug!("notify-send started with ID: {id}");
                        self.active_id.store(id, Ordering::SeqCst);
                    }
                }
            }
        }
    }

    /// Update existing toast in place with new action description
    pub async fn update(&self, headline: &str, task_description: &str) {
        self.update_with_glyph("󱑎", headline, task_description)
            .await;
    }

    /// Update existing toast in place with specific glyph and headline
    pub async fn update_with_glyph(&self, glyph: &str, headline: &str, task_description: &str) {
        let id = self.active_id.load(Ordering::SeqCst);
        let clean_desc = if task_description.trim().is_empty() {
            "Working..."
        } else {
            task_description.trim()
        };

        if id == 0 {
            self.start_with_glyph(glyph, headline, clean_desc).await;
            return;
        }

        if let Ok(mut g) = self.last_glyph.write() {
            *g = glyph.to_string();
        }
        if let Ok(mut h) = self.last_headline.write() {
            *h = headline.to_string();
        }

        if let Ok(path) = which::which("omarchy-notification-send") {
            let _ = Command::new(path)
                .arg("--app-name")
                .arg("Jarvis")
                .arg("-g")
                .arg(glyph)
                .arg("-u")
                .arg("normal")
                .arg("-r")
                .arg(id.to_string())
                .arg("-t")
                .arg("0")
                .arg(headline)
                .arg(clean_desc)
                .status()
                .await;
            return;
        }

        if let Ok(path) = which::which("notify-send") {
            let _ = Command::new(path)
                .arg("-a")
                .arg("Jarvis")
                .arg("-r")
                .arg(id.to_string())
                .arg("-t")
                .arg("0")
                .arg("-h")
                .arg("string:x-canonical-private-synchronous:jarvis-task")
                .arg("-h")
                .arg("string:synchronous:jarvis-task")
                .arg(headline)
                .arg(clean_desc)
                .status()
                .await;
        }
    }

    /// Start if not active, or update in-place if already showing
    pub async fn start_or_update(&self, glyph: &str, headline: &str, task_description: &str) {
        if self.is_active() {
            self.update_with_glyph(glyph, headline, task_description)
                .await;
        } else {
            self.start_with_glyph(glyph, headline, task_description)
                .await;
        }
    }

    /// Mark task as complete with default glyph and set auto-dismiss timeout (4 seconds)
    pub async fn finish(&self, headline: &str, summary: &str) {
        let glyph = self
            .last_glyph
            .read()
            .map(|g| g.clone())
            .unwrap_or_else(|_| "󰚩".to_string());
        self.finish_with_glyph(&glyph, headline, summary).await;
    }

    /// Mark task complete using remembered task glyph and completion title
    pub async fn finish_smart(&self, summary: &str) {
        let glyph = self
            .last_glyph
            .read()
            .map(|g| g.clone())
            .unwrap_or_else(|_| "󰚩".to_string());
        let headline = self
            .last_headline
            .read()
            .map(|h| h.clone())
            .unwrap_or_else(|_| "Jarvis: Complete".to_string());
        self.finish_with_glyph(&glyph, &headline, summary).await;
    }

    /// Mark task as complete with custom glyph and auto-dismiss
    pub async fn finish_with_glyph(&self, glyph: &str, headline: &str, summary: &str) {
        let id = self.active_id.swap(0, Ordering::SeqCst);
        let clean_summary = if summary.trim().is_empty() {
            "Operation complete."
        } else {
            summary.trim()
        };

        if let Ok(path) = which::which("omarchy-notification-send") {
            let mut cmd = Command::new(path);
            cmd.arg("--app-name")
                .arg("Jarvis")
                .arg("-g")
                .arg(glyph)
                .arg("-u")
                .arg("normal")
                .arg("-t")
                .arg("4000");

            if id > 0 {
                cmd.arg("-r").arg(id.to_string());
            }

            let _ = cmd.arg(headline).arg(clean_summary).status().await;
            return;
        }

        if let Ok(path) = which::which("notify-send") {
            let mut cmd = Command::new(path);
            cmd.arg("-a")
                .arg("Jarvis")
                .arg("-t")
                .arg("4000")
                .arg("-h")
                .arg("string:x-canonical-private-synchronous:jarvis-task")
                .arg("-h")
                .arg("string:synchronous:jarvis-task");

            if id > 0 {
                cmd.arg("-r").arg(id.to_string());
            }

            let _ = cmd.arg(headline).arg(clean_summary).status().await;
        }
    }

    /// Dismiss or mark cancelled
    pub async fn cancel(&self) {
        let id = self.active_id.swap(0, Ordering::SeqCst);
        if id > 0 {
            if let Ok(path) = which::which("omarchy-notification-send") {
                let _ = Command::new(path)
                    .arg("--app-name")
                    .arg("Jarvis")
                    .arg("-g")
                    .arg("󰚩")
                    .arg("-u")
                    .arg("low")
                    .arg("-r")
                    .arg(id.to_string())
                    .arg("-t")
                    .arg("2000")
                    .arg("Jarvis")
                    .arg("Task cancelled.")
                    .status()
                    .await;
            } else if let Ok(path) = which::which("notify-send") {
                let _ = Command::new(path)
                    .arg("-a")
                    .arg("Jarvis")
                    .arg("-r")
                    .arg(id.to_string())
                    .arg("-t")
                    .arg("2000")
                    .arg("-h")
                    .arg("string:x-canonical-private-synchronous:jarvis-task")
                    .arg("Jarvis")
                    .arg("Task cancelled.")
                    .status()
                    .await;
            }
        }
    }
}

/// Send a standalone one-shot desktop notification using omarchy-notification-send or notify-send.
pub async fn send_desktop_notification(
    glyph: &str,
    headline: &str,
    description: &str,
    timeout_ms: u32,
    urgency: &str,
) {
    let clean_glyph = if glyph.is_empty() { "󰚩" } else { glyph };
    let clean_urgency = match urgency.to_lowercase().as_str() {
        "low" => "low",
        "critical" => "critical",
        _ => "normal",
    };

    if let Ok(path) = which::which("omarchy-notification-send") {
        let mut cmd = Command::new(path);
        cmd.arg("--app-name")
            .arg("Jarvis")
            .arg("-g")
            .arg(clean_glyph)
            .arg("-u")
            .arg(clean_urgency)
            .arg("-t")
            .arg(timeout_ms.to_string())
            .arg(headline);

        if !description.trim().is_empty() {
            cmd.arg(description.trim());
        }

        cmd.stdout(Stdio::null()).stderr(Stdio::null());
        let _ = cmd.status().await;
        return;
    }

    if let Ok(path) = which::which("notify-send") {
        let mut cmd = Command::new(path);
        cmd.arg("-a")
            .arg("Jarvis")
            .arg("-u")
            .arg(clean_urgency)
            .arg("-t")
            .arg(timeout_ms.to_string())
            .arg(headline);

        if !description.trim().is_empty() {
            cmd.arg(description.trim());
        }

        cmd.stdout(Stdio::null()).stderr(Stdio::null());
        let _ = cmd.status().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_task_notifier_lifecycle() {
        let notifier = TaskNotifier::global();
        notifier.reset_id();
        assert_eq!(notifier.current_id(), 0);
        assert!(!notifier.is_active());

        // Setting an ID directly to test reset and swap
        notifier.active_id.store(999, Ordering::SeqCst);
        assert_eq!(notifier.current_id(), 999);
        assert!(notifier.is_active());

        notifier.reset_id();
        assert_eq!(notifier.current_id(), 0);
        assert!(!notifier.is_active());
    }

    #[test]
    fn test_tool_notification_info_mapping() {
        let empty_args = serde_json::json!({});
        let battery_info = tool_notification_info("get_battery", &empty_args);
        assert_eq!(battery_info.glyph, "󰁹");
        assert_eq!(battery_info.completion_headline, "󰁹 Battery Status");

        let speedtest_info = tool_notification_info("network_speedtest", &empty_args);
        assert_eq!(speedtest_info.glyph, "󰛳");
        assert_eq!(speedtest_info.completion_headline, "󰛳 Speedtest Results");

        let media_args = serde_json::json!({"query": "Prison Break S01E12"});
        let media_info = tool_notification_info("play_media", &media_args);
        assert_eq!(media_info.glyph, "󰐌");
        assert!(media_info.description.contains("Prison Break S01E12"));

        let note_info = tool_notification_info("create_note", &empty_args);
        assert_eq!(note_info.glyph, "󰏪");
        assert_eq!(note_info.completion_headline, "󰏪 Note Saved");
    }
}
