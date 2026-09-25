use crate::core::error::Result;
use crate::core::is_test_environment;
use crate::tools::clipboard::ClipboardManager;
use crate::tools::hyprland::HyprlandController;
use crate::tools::web::WebNavigator;
use crate::tools::Tool;
use async_trait::async_trait;
use regex::Regex;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{debug, info};

pub struct MeetingManager {
    hyprland: Arc<HyprlandController>,
    clipboard: Arc<ClipboardManager>,
    web: Arc<WebNavigator>,
}

impl MeetingManager {
    pub fn new(
        hyprland: Arc<HyprlandController>,
        clipboard: Arc<ClipboardManager>,
        web: Arc<WebNavigator>,
    ) -> Self {
        Self {
            hyprland,
            clipboard,
            web,
        }
    }

    /// Extract Google Meet 10-char room code from a window title (e.g. "Meet - abc-defg-hij" or "abc-defg-hij - Google Meet")
    pub fn extract_meeting_code(title: &str) -> Option<String> {
        let re = Regex::new(r"(?i)\b([a-z]{3}-[a-z]{4}-[a-z]{3})\b").ok()?;
        re.captures(title)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_lowercase())
    }

    /// Creates an instant Google Meet meeting, opens it in the browser, and copies link to clipboard
    pub async fn create_quick_meeting(&self) -> Result<String> {
        let base_url = "https://meet.google.com/new";
        info!("Creating quick Google Meet meeting...");

        if is_test_environment() {
            let mock_link = "https://meet.google.com/abc-defg-hij";
            return Ok(format!("Meeting created: {mock_link}. Link copied to clipboard."));
        }

        // 1. Launch Google Meet instant meeting URL in the default browser
        self.web.open_url(base_url).await?;

        // 2. Pre-set clipboard to base URL
        let _ = self.clipboard.set_clipboard(base_url).await;

        // 3. Poll Hyprland window titles for up to 4.5s to capture the resolved meeting code
        let hyprland = self.hyprland.clone();
        let clipboard = self.clipboard.clone();

        let mut resolved_link = None;
        let start = Instant::now();
        while start.elapsed() < Duration::from_millis(4500) {
            sleep(Duration::from_millis(250)).await;
            if let Ok(clients) = hyprland.get_clients().await {
                if let Some(client_list) = clients.as_array() {
                    for c in client_list {
                        let title = c["title"].as_str().unwrap_or("");
                        if let Some(code) = Self::extract_meeting_code(title) {
                            let full_url = format!("https://meet.google.com/{code}");
                            resolved_link = Some(full_url);
                            break;
                        }
                    }
                }
            }
            if resolved_link.is_some() {
                break;
            }
        }

        if let Some(ref link) = resolved_link {
            let _ = clipboard.set_clipboard(link).await;
            info!("Google Meet room resolved: {link}");
            return Ok(format!("Meeting created: {link}. Link copied to clipboard."));
        }

        // 4. If the browser is still negotiating redirect, continue watching in background for another 12s
        tokio::spawn(async move {
            let bg_start = Instant::now();
            while bg_start.elapsed() < Duration::from_secs(12) {
                sleep(Duration::from_millis(400)).await;
                if let Ok(clients) = hyprland.get_clients().await {
                    if let Some(client_list) = clients.as_array() {
                        for c in client_list {
                            let title = c["title"].as_str().unwrap_or("");
                            if let Some(code) = Self::extract_meeting_code(title) {
                                let full_url = format!("https://meet.google.com/{code}");
                                debug!("Background meeting watcher resolved: {full_url}");
                                let _ = clipboard.set_clipboard(&full_url).await;
                                return;
                            }
                        }
                    }
                }
            }
        });

        Ok("Meeting created. Link copied to clipboard.".to_string())
    }
}

pub struct CreateQuickMeetingTool {
    manager: Arc<MeetingManager>,
}

impl CreateQuickMeetingTool {
    pub fn new(manager: Arc<MeetingManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for CreateQuickMeetingTool {
    fn name(&self) -> &'static str {
        "create_quick_meeting"
    }

    fn description(&self) -> &'static str {
        "Start an instant Google Meet meeting (https://meet.google.com/), open it in the default browser, and copy the meeting link to the system clipboard."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _args: Value) -> Result<String> {
        self.manager.create_quick_meeting().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_meeting_code() {
        assert_eq!(
            MeetingManager::extract_meeting_code("Meet - abc-defg-hij"),
            Some("abc-defg-hij".to_string())
        );
        assert_eq!(
            MeetingManager::extract_meeting_code("abc-defg-hij - Google Meet"),
            Some("abc-defg-hij".to_string())
        );
        assert_eq!(
            MeetingManager::extract_meeting_code("Google Meet: ABC-DEFG-HIJ - Helium"),
            Some("abc-defg-hij".to_string())
        );
        assert_eq!(
            MeetingManager::extract_meeting_code("https://meet.google.com/xyz-uvwx-rst"),
            Some("xyz-uvwx-rst".to_string())
        );
        assert_eq!(
            MeetingManager::extract_meeting_code("Just another tab - Google Chrome"),
            None
        );
    }
}
