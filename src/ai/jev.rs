//! TypeSafe AI "Jev" System One Client & Fast-Path Intent Router
//!
//! Provides sub-100ms decision routing using TypeSafe AI's Jev model
//! (https://typesafe.ai/blog/introducing-system-one-models-and-jev)
//! combined with an instant (<0.1ms) zero-overhead local intent matcher.

use crate::core::config::Settings;
use crate::core::error::{JarvisError, Result};
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;
use tracing::{debug, info};

pub const JEV_API_ENDPOINT: &str = "https://api.typesafe.ai/v1/systemone";

#[derive(Debug, Clone, PartialEq)]
pub enum FastPathAction {
    MediaPlayPause,
    MediaNext,
    MediaPrevious,
    MediaStop,
    VolumeMute,
    VolumeAdjust(i32),
    LockScreen,
    SwitchWorkspace(i64),
    DismissSession,
}

impl FastPathAction {
    pub async fn execute(
        &self,
        registry: &crate::tools::ToolRegistry,
        hyprland: &crate::tools::hyprland::HyprlandController,
    ) -> Result<String> {
        match self {
            FastPathAction::MediaPlayPause => {
                registry.execute_tool("media_play_pause", json!({})).await
            }
            FastPathAction::MediaNext => {
                registry.execute_tool("media_next", json!({})).await
            }
            FastPathAction::MediaPrevious => {
                registry.execute_tool("media_previous", json!({})).await
            }
            FastPathAction::MediaStop => {
                registry.execute_tool("media_stop", json!({})).await
            }
            FastPathAction::VolumeMute => {
                registry
                    .execute_tool("adjust_volume", json!({"adjustment": "mute-toggle"}))
                    .await
            }
            FastPathAction::VolumeAdjust(delta) => {
                let adj = if *delta > 0 {
                    format!("+{delta}")
                } else {
                    format!("{delta}")
                };
                registry
                    .execute_tool("adjust_volume", json!({"adjustment": adj}))
                    .await
            }
            FastPathAction::LockScreen => {
                registry.execute_tool("lock_screen", json!({})).await
            }
            FastPathAction::SwitchWorkspace(id) => {
                hyprland.change_workspace(*id as i32).await?;
                Ok(format!("Switched to workspace {id}."))
            }
            FastPathAction::DismissSession => {
                Ok("Very well, sir. Have a wonderful day.".to_string())
            }
        }
    }

    pub fn spoken_confirmation(&self) -> &'static str {
        match self {
            FastPathAction::MediaPlayPause => "Toggled.",
            FastPathAction::MediaNext => "Next track.",
            FastPathAction::MediaPrevious => "Previous track.",
            FastPathAction::MediaStop => "Stopped.",
            FastPathAction::VolumeMute => "Muted.",
            FastPathAction::VolumeAdjust(d) if *d > 0 => "Volume up.",
            FastPathAction::VolumeAdjust(_) => "Volume down.",
            FastPathAction::LockScreen => "Locked.",
            FastPathAction::SwitchWorkspace(_) => "Switched.",
            FastPathAction::DismissSession => "Very well, sir. Have a wonderful day.",
        }
    }
}

/// Client for TypeSafe AI's Jev System One Model
#[derive(Clone)]
pub struct JevClient {
    client: Client,
    api_key: String,
    endpoint: String,
}

impl JevClient {
    pub fn new(api_key: &str) -> Self {
        let client = Client::builder()
            .connect_timeout(Duration::from_millis(500))
            .timeout(Duration::from_millis(1500))
            .build()
            .unwrap_or_default();

        Self {
            client,
            api_key: api_key.trim().to_string(),
            endpoint: JEV_API_ENDPOINT.to_string(),
        }
    }

    pub fn from_settings(settings: &Settings) -> Option<Self> {
        settings
            .typesafe_api_key
            .as_ref()
            .filter(|k| !k.trim().is_empty())
            .map(|k| Self::new(k))
    }

    /// Evaluates typed questions against state in parallel via Jev
    pub async fn evaluate(&self, state: &str, questions: Value) -> Result<Value> {
        let payload = json!({
            "state": state,
            "model": "jev-latest",
            "questions": questions
        });

        let resp = self
            .client
            .post(&self.endpoint)
            .bearer_auth(&self.api_key)
            .json(&payload)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(JarvisError::ApiError {
                status,
                body: text,
            });
        }

        let body: Value = resp.json().await?;

        Ok(body)
    }

    /// Fast-routes intent using Jev Choice primitive in ~70ms
    pub async fn fast_route_intent(&self, transcript: &str) -> Option<FastPathAction> {
        let questions = json!({
            "intent": {
                "type": "choice",
                "instructions": "Determine which direct desktop action matches the user's command",
                "criteria": {
                    "media_play_pause": "Play, pause, resume, or toggle media playback",
                    "media_next": "Skip to next track or song",
                    "media_previous": "Go back to previous track or song",
                    "media_stop": "Stop playing media or music",
                    "volume_up": "Raise or increase audio volume, louder",
                    "volume_down": "Lower or decrease audio volume, softer",
                    "volume_mute": "Mute or silence audio output",
                    "lock_screen": "Lock the screen or computer",
                    "session_exit": "User is finished, saying goodbye, or dismissing the assistant",
                    "none": "Open-ended question, search, complex task, or non-fixed action"
                }
            }
        });

        match self.evaluate(transcript, questions).await {
            Ok(res) => {
                let choice_data = res.get("answers")?.get("intent")?;
                let choice = choice_data.get("choice")?.as_str()?;
                let confidence = choice_data.get("confidence")?.as_f64().unwrap_or(0.0);

                debug!("Jev classified intent: '{choice}' (confidence: {confidence:.2})");

                if confidence < 0.75 {
                    return None;
                }

                match choice {
                    "media_play_pause" => Some(FastPathAction::MediaPlayPause),
                    "media_next" => Some(FastPathAction::MediaNext),
                    "media_previous" => Some(FastPathAction::MediaPrevious),
                    "media_stop" => Some(FastPathAction::MediaStop),
                    "volume_up" => Some(FastPathAction::VolumeAdjust(10)),
                    "volume_down" => Some(FastPathAction::VolumeAdjust(-10)),
                    "volume_mute" => Some(FastPathAction::VolumeMute),
                    "lock_screen" => Some(FastPathAction::LockScreen),
                    "session_exit" => Some(FastPathAction::DismissSession),
                    _ => None,
                }
            }
            Err(e) => {
                debug!("Jev fast route failed: {e}");
                None
            }
        }
    }

    /// Checks if the user is dismissing the assistant using Jev Noul primitive in ~70ms
    pub async fn check_dismissal(&self, transcript: &str) -> bool {
        let questions = json!({
            "is_dismissal": {
                "type": "noul",
                "instructions": "The user is expressing that they are done, finished, dismissing the assistant, or saying goodbye."
            }
        });

        match self.evaluate(transcript, questions).await {
            Ok(res) => {
                let noul = res
                    .get("answers")
                    .and_then(|a| a.get("is_dismissal"))
                    .and_then(|d| d.get("noul"))
                    .and_then(|n| n.as_f64())
                    .unwrap_or(0.0);

                debug!("Jev dismissal check: noul={noul:.2}");
                noul >= 0.80
            }
            Err(_) => false,
        }
    }
}

/// Zero-latency Local Rule Matcher (<0.1ms)
pub fn local_match_fast_path(transcript: &str) -> Option<FastPathAction> {
    let clean: String = transcript
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .to_lowercase();
    let trimmed = clean.split_whitespace().collect::<Vec<&str>>().join(" ");

    if trimmed.is_empty() {
        return None;
    }

    // Media Play/Pause
    match trimmed.as_str() {
        "pause"
        | "resume"
        | "play"
        | "pause music"
        | "resume music"
        | "play music"
        | "pause video"
        | "resume video"
        | "play video"
        | "toggle playback"
        | "toggle music"
        | "play pause"
        | "playpause" => return Some(FastPathAction::MediaPlayPause),

        "next"
        | "next song"
        | "next track"
        | "skip"
        | "skip song"
        | "skip track" => return Some(FastPathAction::MediaNext),

        "previous"
        | "previous song"
        | "previous track"
        | "prev song"
        | "prev track"
        | "last song"
        | "last track" => return Some(FastPathAction::MediaPrevious),

        "stop music"
        | "stop playback"
        | "stop media"
        | "stop playing" => return Some(FastPathAction::MediaStop),

        "mute"
        | "unmute"
        | "mute audio"
        | "mute volume"
        | "silence"
        | "silence audio" => return Some(FastPathAction::VolumeMute),

        "volume up"
        | "turn up volume"
        | "raise volume"
        | "increase volume"
        | "louder"
        | "pump up volume" => return Some(FastPathAction::VolumeAdjust(10)),

        "volume down"
        | "turn down volume"
        | "lower volume"
        | "decrease volume"
        | "softer"
        | "quieter" => return Some(FastPathAction::VolumeAdjust(-10)),

        "lock"
        | "lock screen"
        | "lock system"
        | "lock pc"
        | "lock laptop" => return Some(FastPathAction::LockScreen),

        _ => {}
    }

    // Switch workspace pattern e.g. "switch to workspace 3", "go to workspace 2", "workspace 1"
    if let Some(id) = parse_workspace_command(&trimmed) {
        return Some(FastPathAction::SwitchWorkspace(id));
    }

    None
}

fn parse_workspace_command(clean: &str) -> Option<i64> {
    let parts: Vec<&str> = clean.split_whitespace().collect();
    if parts.len() == 4
        && (parts[0] == "switch" || parts[0] == "go")
        && parts[1] == "to"
        && parts[2] == "workspace"
    {
        return parts[3].parse::<i64>().ok();
    }
    if parts.len() == 3 && (parts[0] == "switch" || parts[0] == "go") && parts[1] == "workspace" {
        return parts[2].parse::<i64>().ok();
    }
    if parts.len() == 2 && parts[0] == "workspace" {
        return parts[1].parse::<i64>().ok();
    }
    None
}

/// Hybrid Fast-Path Router: Local matcher (<0.1ms) + TypeSafe Jev (<100ms)
#[derive(Clone)]
pub struct FastPathRouter {
    jev: Option<JevClient>,
}

impl FastPathRouter {
    pub fn new(settings: &Settings) -> Self {
        Self {
            jev: JevClient::from_settings(settings),
        }
    }

    pub async fn route(&self, transcript: &str) -> Option<FastPathAction> {
        // Stage 1: Zero-latency local matcher (<0.1ms)
        if let Some(action) = local_match_fast_path(transcript) {
            info!("Local fast-path matched action for \"{transcript}\"");
            return Some(action);
        }

        // Stage 2: TypeSafe AI Jev Fast-Path (~70-150ms)
        if let Some(ref jev) = self.jev {
            if let Some(action) = jev.fast_route_intent(transcript).await {
                info!("Jev System One fast-path routed action for \"{transcript}\"");
                return Some(action);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_match_fast_path_media() {
        assert_eq!(
            local_match_fast_path("pause"),
            Some(FastPathAction::MediaPlayPause)
        );
        assert_eq!(
            local_match_fast_path("resume"),
            Some(FastPathAction::MediaPlayPause)
        );
        assert_eq!(
            local_match_fast_path("play music"),
            Some(FastPathAction::MediaPlayPause)
        );
        assert_eq!(
            local_match_fast_path("next track"),
            Some(FastPathAction::MediaNext)
        );
        assert_eq!(
            local_match_fast_path("previous song"),
            Some(FastPathAction::MediaPrevious)
        );
        assert_eq!(
            local_match_fast_path("stop music"),
            Some(FastPathAction::MediaStop)
        );
    }

    #[test]
    fn test_local_match_fast_path_volume_and_system() {
        assert_eq!(
            local_match_fast_path("mute"),
            Some(FastPathAction::VolumeMute)
        );
        assert_eq!(
            local_match_fast_path("volume up"),
            Some(FastPathAction::VolumeAdjust(10))
        );
        assert_eq!(
            local_match_fast_path("quieter"),
            Some(FastPathAction::VolumeAdjust(-10))
        );
        assert_eq!(
            local_match_fast_path("lock screen"),
            Some(FastPathAction::LockScreen)
        );
        assert_eq!(
            local_match_fast_path("switch to workspace 3"),
            Some(FastPathAction::SwitchWorkspace(3))
        );
        assert_eq!(
            local_match_fast_path("workspace 2"),
            Some(FastPathAction::SwitchWorkspace(2))
        );
    }

    #[test]
    fn test_local_match_non_fast_path() {
        assert_eq!(local_match_fast_path("open youtube"), None);
        assert_eq!(local_match_fast_path("what is the weather today"), None);
        assert_eq!(
            local_match_fast_path("create a basic cli weather app"),
            None
        );
        assert_eq!(
            local_match_fast_path("play episode 12 from prison break season 1"),
            None
        );
    }
}
