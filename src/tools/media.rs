use crate::core::error::Result;
use crate::tools::Tool;
use async_trait::async_trait;
use regex::Regex;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;

pub struct MediaManager {
    dbus_send_bin: PathBuf,
}

impl Default for MediaManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MediaManager {
    pub fn new() -> Self {
        let dbus_send_bin = which::which("dbus-send").unwrap_or_else(|_| PathBuf::from("/usr/bin/dbus-send"));
        Self { dbus_send_bin }
    }

    pub async fn get_active_players(&self) -> Vec<String> {
        let output = Command::new(&self.dbus_send_bin)
            .arg("--session")
            .arg("--dest=org.freedesktop.DBus")
            .arg("--type=method_call")
            .arg("--print-reply")
            .arg("/org/freedesktop/DBus")
            .arg("org.freedesktop.DBus.ListNames")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .await;

        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            let re = Regex::new(r#"string "(org\.mpris\.MediaPlayer2\.[^"]+)""#).unwrap();
            let mut players = Vec::new();
            for cap in re.captures_iter(&text) {
                players.push(cap[1].to_string());
            }
            return players;
        }
        Vec::new()
    }

    pub async fn call_player_method(&self, method: &str, target_player: Option<&str>) -> bool {
        let players = if let Some(p) = target_player {
            vec![p.to_string()]
        } else {
            self.get_active_players().await
        };

        if players.is_empty() {
            return false;
        }

        let mut success = false;
        for player in &players {
            let status = Command::new(&self.dbus_send_bin)
                .arg("--session")
                .arg(format!("--dest={player}"))
                .arg("--type=method_call")
                .arg("/org/mpris/MediaPlayer2")
                .arg(format!("org.mpris.MediaPlayer2.Player.{method}"))
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await;

            if let Ok(s) = status {
                if s.success() {
                    success = true;
                    if target_player.is_none() {
                        break;
                    }
                }
            }
        }
        success
    }

    pub async fn media_play_pause(&self, player: Option<&str>) -> Result<String> {
        let ok = self.call_player_method("PlayPause", player).await;
        if ok {
            Ok("Toggled media playback.".to_string())
        } else {
            Ok("No active media player detected.".to_string())
        }
    }

    pub async fn media_next(&self, player: Option<&str>) -> Result<String> {
        let ok = self.call_player_method("Next", player).await;
        if ok {
            Ok("Skipped to next track.".to_string())
        } else {
            Ok("No active media player detected.".to_string())
        }
    }

    pub async fn media_previous(&self, player: Option<&str>) -> Result<String> {
        let ok = self.call_player_method("Previous", player).await;
        if ok {
            Ok("Returned to previous track.".to_string())
        } else {
            Ok("No active media player detected.".to_string())
        }
    }

    pub async fn media_stop(&self, player: Option<&str>) -> Result<String> {
        let ok = self.call_player_method("Stop", player).await;
        if ok {
            Ok("Stopped media playback.".to_string())
        } else {
            Ok("No active media player detected.".to_string())
        }
    }

    pub async fn get_now_playing(&self) -> Result<String> {
        let players = self.get_active_players().await;
        if players.is_empty() {
            return Ok("No active media player found.".to_string());
        }

        let mut tracks = Vec::new();
        let title_re = Regex::new(r#"string\s+"xesam:title"\s+variant\s+string\s+"(.*?)"#).unwrap();
        let artist_re = Regex::new(r#"string\s+"xesam:artist"\s+variant\s+(?:array\s+\[\s+)?string\s+"(.*?)"#).unwrap();

        for player in &players {
            let output = Command::new(&self.dbus_send_bin)
                .arg("--session")
                .arg(format!("--dest={player}"))
                .arg("--type=method_call")
                .arg("--print-reply")
                .arg("/org/mpris/MediaPlayer2")
                .arg("org.freedesktop.DBus.Properties.Get")
                .arg("string:org.mpris.MediaPlayer2.Player")
                .arg("string:Metadata")
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .output()
                .await;

            if let Ok(out) = output {
                let text = String::from_utf8_lossy(&out.stdout);
                let title = title_re.captures(&text).map(|c| c[1].to_string()).unwrap_or_default();
                let artist = artist_re.captures(&text).map(|c| c[1].to_string()).unwrap_or_default();

                let mut short_player = player.split('.').next_back().unwrap_or("Media").to_string();
                let lower_p = player.to_lowercase();
                if lower_p.contains("chromium") {
                    short_player = "Chromium/Browser".to_string();
                } else if lower_p.contains("firefox") {
                    short_player = "Firefox".to_string();
                } else if lower_p.contains("spotify") {
                    short_player = "Spotify".to_string();
                }

                if !title.is_empty() {
                    let mut info = format!("'{title}'");
                    if !artist.is_empty() {
                        info.push_str(&format!(" by {artist}"));
                    }
                    info.push_str(&format!(" ({short_player})"));
                    tracks.push(info);
                }
            }
        }

        if !tracks.is_empty() {
            Ok(format!("Now playing: {}.", tracks.join("; ")))
        } else {
            Ok("Media player is open, but no active track info is available.".to_string())
        }
    }
}

// -----------------------------------------------------------------------------
// Tool Implementations
// -----------------------------------------------------------------------------

pub struct MediaPlayPauseTool {
    media: Arc<MediaManager>,
}

impl MediaPlayPauseTool {
    pub fn new(media: Arc<MediaManager>) -> Self {
        Self { media }
    }
}

#[async_trait]
impl Tool for MediaPlayPauseTool {
    fn name(&self) -> &'static str {
        "media_play_pause"
    }

    fn description(&self) -> &'static str {
        "Toggle media playback (play/pause) across active media players (Spotify, Chromium, YouTube, Firefox, mpv)."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {}
        })
    }

    async fn execute(&self, _args: Value) -> Result<String> {
        self.media.media_play_pause(None).await
    }
}

pub struct MediaNextTool {
    media: Arc<MediaManager>,
}

impl MediaNextTool {
    pub fn new(media: Arc<MediaManager>) -> Self {
        Self { media }
    }
}

#[async_trait]
impl Tool for MediaNextTool {
    fn name(&self) -> &'static str {
        "media_next"
    }

    fn description(&self) -> &'static str {
        "Skip to the next song or media track."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {}
        })
    }

    async fn execute(&self, _args: Value) -> Result<String> {
        self.media.media_next(None).await
    }
}

pub struct MediaPreviousTool {
    media: Arc<MediaManager>,
}

impl MediaPreviousTool {
    pub fn new(media: Arc<MediaManager>) -> Self {
        Self { media }
    }
}

#[async_trait]
impl Tool for MediaPreviousTool {
    fn name(&self) -> &'static str {
        "media_previous"
    }

    fn description(&self) -> &'static str {
        "Skip to the previous song or media track."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {}
        })
    }

    async fn execute(&self, _args: Value) -> Result<String> {
        self.media.media_previous(None).await
    }
}

pub struct MediaStopTool {
    media: Arc<MediaManager>,
}

impl MediaStopTool {
    pub fn new(media: Arc<MediaManager>) -> Self {
        Self { media }
    }
}

#[async_trait]
impl Tool for MediaStopTool {
    fn name(&self) -> &'static str {
        "media_stop"
    }

    fn description(&self) -> &'static str {
        "Stop current media playback."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {}
        })
    }

    async fn execute(&self, _args: Value) -> Result<String> {
        self.media.media_stop(None).await
    }
}

pub struct GetNowPlayingTool {
    media: Arc<MediaManager>,
}

impl GetNowPlayingTool {
    pub fn new(media: Arc<MediaManager>) -> Self {
        Self { media }
    }
}

#[async_trait]
impl Tool for GetNowPlayingTool {
    fn name(&self) -> &'static str {
        "get_now_playing"
    }

    fn description(&self) -> &'static str {
        "Get the title and artist of currently playing music or video."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {}
        })
    }

    async fn execute(&self, _args: Value) -> Result<String> {
        self.media.get_now_playing().await
    }
}
