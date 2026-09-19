use crate::core::error::Result;
use crate::tools::Tool;
use async_trait::async_trait;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;
use tracing::info;

pub const VIDEO_EXTENSIONS: &[&str] = &[
    "mkv", "mp4", "avi", "webm", "mov", "m4v", "flv", "wmv", "ts", "mpg", "mpeg",
];

// -----------------------------------------------------------------------------
// Media Watch History Persistence
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MediaHistory {
    pub shows: HashMap<String, ShowHistoryEntry>,
    pub recent: Vec<RecentMediaEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowHistoryEntry {
    pub title: String,
    pub file_path: String,
    pub season: Option<u32>,
    pub episode: Option<u32>,
    pub last_played_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentMediaEntry {
    pub title: String,
    pub file_path: String,
    pub season: Option<u32>,
    pub episode: Option<u32>,
    pub played_at: String,
}

impl MediaHistory {
    pub fn history_path() -> PathBuf {
        crate::core::logging::get_log_dir().join("media_history.json")
    }

    pub fn load() -> Self {
        let p = Self::history_path();
        if let Ok(content) = fs::read_to_string(&p) {
            if let Ok(hist) = serde_json::from_str::<MediaHistory>(&content) {
                return hist;
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<()> {
        let p = Self::history_path();
        if let Some(parent) = p.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let data = serde_json::to_string_pretty(self)?;
        fs::write(&p, data)?;
        Ok(())
    }

    pub fn record_play(
        &mut self,
        title: &str,
        file_path: &Path,
        season: Option<u32>,
        episode: Option<u32>,
    ) {
        let now = chrono::Utc::now().to_rfc3339();
        let norm_key = title.to_lowercase().trim().to_string();
        let path_str = file_path.to_string_lossy().to_string();

        let show_entry = ShowHistoryEntry {
            title: title.to_string(),
            file_path: path_str.clone(),
            season,
            episode,
            last_played_at: now.clone(),
        };
        self.shows.insert(norm_key, show_entry);

        let recent_entry = RecentMediaEntry {
            title: title.to_string(),
            file_path: path_str,
            season,
            episode,
            played_at: now,
        };
        self.recent
            .retain(|r| r.file_path != recent_entry.file_path);
        self.recent.push(recent_entry);
        if self.recent.len() > 30 {
            let excess = self.recent.len() - 30;
            self.recent.drain(0..excess);
        }
        let _ = self.save();
    }
}

// -----------------------------------------------------------------------------
// Media Match Result
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct MediaMatch {
    pub file_path: PathBuf,
    pub title: String,
    pub season: Option<u32>,
    pub episode: Option<u32>,
    pub display_name: String,
}

pub fn has_video_extension(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if VIDEO_EXTENSIONS
            .iter()
            .any(|&v| v.eq_ignore_ascii_case(ext))
        {
            return true;
        }
    }
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        let lower = name.to_lowercase();
        if VIDEO_EXTENSIONS
            .iter()
            .any(|&v| lower.ends_with(&format!(".{v}")) || lower.ends_with(&format!(" {v}")))
        {
            return true;
        }
    }
    false
}

pub fn is_video_file(path: &Path) -> bool {
    path.is_file() && has_video_extension(path)
}

pub fn extract_season_episode(path: &Path) -> (Option<u32>, Option<u32>) {
    let full_path = path.to_string_lossy().to_lowercase();
    let filename = path
        .file_name()
        .map(|f| f.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    // 1. Try combined SxxExx pattern (e.g. s01e12, s1e12, s01.e12, s1_e12)
    let re_s_e = Regex::new(r#"[sS]0*(\d+)[._\s-]*[eE]0*(\d+)"#).unwrap();
    if let Some(cap) = re_s_e
        .captures(&filename)
        .or_else(|| re_s_e.captures(&full_path))
    {
        let s: Option<u32> = cap[1].parse().ok();
        let e: Option<u32> = cap[2].parse().ok();
        if s.is_some() && e.is_some() {
            return (s, e);
        }
    }

    // 2. Try NxNN pattern (e.g. 1x12)
    let re_nxn = Regex::new(r#"\b0*(\d+)x0*(\d+)\b"#).unwrap();
    if let Some(cap) = re_nxn
        .captures(&filename)
        .or_else(|| re_nxn.captures(&full_path))
    {
        let s: Option<u32> = cap[1].parse().ok();
        let e: Option<u32> = cap[2].parse().ok();
        if s.is_some() && e.is_some() {
            return (s, e);
        }
    }

    // 3. Try separate Season and Episode:
    let re_season = Regex::new(r#"(?:season|series|s)[\s._-]*0*(\d+)"#).unwrap();
    let season: Option<u32> = re_season
        .captures(&full_path)
        .and_then(|c| c[1].parse().ok());

    let re_episode = Regex::new(r#"(?:episode|ep|e)[\s._-]*0*(\d+)"#).unwrap();
    let mut episode: Option<u32> = re_episode
        .captures(&filename)
        .and_then(|c| c[1].parse().ok());

    if episode.is_none() {
        let re_num = Regex::new(r#"\b0*(\d{1,3})\b"#).unwrap();
        for cap in re_num.captures_iter(&filename) {
            if let Ok(n) = cap[1].parse::<u32>() {
                if n > 0 && n <= 100 && Some(n) != season {
                    episode = Some(n);
                    break;
                }
            }
        }
    }

    (season, episode)
}

pub struct MediaManager {
    dbus_send_bin: PathBuf,
    media_dirs: Vec<PathBuf>,
    media_player: String,
}

impl Default for MediaManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MediaManager {
    pub fn new() -> Self {
        let dbus_send_bin =
            which::which("dbus-send").unwrap_or_else(|_| PathBuf::from("/usr/bin/dbus-send"));
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let home_path = PathBuf::from(&home);
        Self {
            dbus_send_bin,
            media_dirs: vec![
                home_path.join("Videos"),
                home_path.join("Movies"),
                home_path.join("Downloads"),
                home_path,
            ],
            media_player: "mpv".to_string(),
        }
    }

    pub fn with_settings(media_dirs: Vec<PathBuf>, media_player: &str) -> Self {
        let dbus_send_bin =
            which::which("dbus-send").unwrap_or_else(|_| PathBuf::from("/usr/bin/dbus-send"));
        Self {
            dbus_send_bin,
            media_dirs,
            media_player: media_player.to_string(),
        }
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
        let artist_re =
            Regex::new(r#"string\s+"xesam:artist"\s+variant\s+(?:array\s+\[\s+)?string\s+"(.*?)"#)
                .unwrap();

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
                let title = title_re
                    .captures(&text)
                    .map(|c| c[1].to_string())
                    .unwrap_or_default();
                let artist = artist_re
                    .captures(&text)
                    .map(|c| c[1].to_string())
                    .unwrap_or_default();

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

    // -------------------------------------------------------------------------
    // Video File Finding & Playback Automation
    // -------------------------------------------------------------------------

    pub fn find_media(
        &self,
        title: &str,
        target_season: Option<u32>,
        target_episode: Option<u32>,
    ) -> Option<MediaMatch> {
        let clean_title = title.trim();
        let query_words: Vec<String> = clean_title
            .to_lowercase()
            .split_whitespace()
            .map(|w| w.chars().filter(|c| c.is_alphanumeric()).collect())
            .filter(|w: &String| !w.is_empty())
            .collect();

        if query_words.is_empty() {
            return None;
        }

        let mut video_files = Vec::new();
        for dir in &self.media_dirs {
            if dir.exists() {
                scan_media_dir(dir, 0, 5, &mut video_files);
            }
        }

        if video_files.is_empty() {
            return None;
        }

        let mut best_match: Option<(i64, MediaMatch)> = None;

        for file_path in video_files {
            let path_str = file_path.to_string_lossy().to_lowercase();
            let filename = file_path
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_default();
            let filename_lower = filename.to_lowercase();

            // Check if all title words match anywhere in the file path
            let all_words_match = query_words.iter().all(|w| path_str.contains(w.as_str()));
            if !all_words_match {
                continue;
            }

            let (file_season, file_episode) = extract_season_episode(&file_path);

            let mut score: i64 = 100;

            if filename_lower.contains(&clean_title.to_lowercase()) {
                score += 80;
            }

            // Evaluate Season Match
            if let Some(req_s) = target_season {
                if file_season == Some(req_s) {
                    score += 200;
                } else if file_season.is_some() {
                    score -= 600; // Wrong season!
                }
            }

            // Evaluate Episode Match
            if let Some(req_e) = target_episode {
                if file_episode == Some(req_e) {
                    score += 400;
                } else if file_episode.is_some() {
                    score -= 600; // Wrong episode!
                }
            }

            // Extra bonus when both season & episode requested and matched
            if target_season.is_some() && target_episode.is_some() {
                if file_season == target_season && file_episode == target_episode {
                    score += 1000;
                }
            } else if target_season.is_none() && target_episode.is_none() {
                // If opening show without episode, prefer earliest episode
                let s_num = file_season.unwrap_or(99);
                let e_num = file_episode.unwrap_or(99);
                let order_penalty = (s_num as i64 * 100) + (e_num as i64);
                score += (2000 - order_penalty).max(0);
            }

            let display_name = match (file_season, file_episode) {
                (Some(s), Some(e)) => format!("{clean_title} Season {s} Episode {e}"),
                (_, Some(e)) => format!("{clean_title} Episode {e}"),
                _ => clean_title.to_string(),
            };

            let candidate = MediaMatch {
                file_path,
                title: clean_title.to_string(),
                season: file_season,
                episode: file_episode,
                display_name,
            };

            match &best_match {
                Some((best_score, _)) if score > *best_score => {
                    best_match = Some((score, candidate));
                }
                None => {
                    best_match = Some((score, candidate));
                }
                _ => {}
            }
        }

        best_match.map(|(_, m)| m)
    }

    pub async fn play_video_file(
        &self,
        file_path: &Path,
        title: &str,
        season: Option<u32>,
        episode: Option<u32>,
        fullscreen: bool,
    ) -> Result<String> {
        let player_bin = if which::which(&self.media_player).is_ok() {
            self.media_player.clone()
        } else if which::which("mpv").is_ok() {
            "mpv".to_string()
        } else if which::which("vlc").is_ok() {
            "vlc".to_string()
        } else {
            return Err(crate::core::error::JarvisError::Other(
                "No media player found. Please install mpv: sudo pacman -S mpv".to_string(),
            ));
        };

        let mut cmd = Command::new(&player_bin);
        if player_bin.contains("mpv") {
            if fullscreen {
                cmd.arg("--fs");
            }
            cmd.arg("--save-position-on-quit");
            cmd.arg("--keep-open=yes");
        } else {
            if fullscreen {
                cmd.arg("--fullscreen");
            }
            cmd.arg(file_path);
        }

        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| {
                crate::core::error::JarvisError::Other(format!(
                    "Failed to spawn media player '{player_bin}': {e}"
                ))
            })?;

        let mut hist = MediaHistory::load();
        hist.record_play(title, file_path, season, episode);

        info!(
            "Launched media player '{player_bin}' playing {:?} (fullscreen: {fullscreen})",
            file_path
        );

        let ep_detail = match (season, episode) {
            (Some(s), Some(e)) => format!(" Season {s} Episode {e}"),
            (_, Some(e)) => format!(" Episode {e}"),
            _ => String::new(),
        };

        if !crate::ui::TaskNotifier::global().is_active() {
            crate::ui::send_desktop_notification(
                "󰐌",
                "Now Playing",
                &format!("{title}{ep_detail} (Fullscreen)"),
                4000,
                "normal",
            )
            .await;
        }

        Ok(format!("Playing {title}{ep_detail} in fullscreen."))
    }

    pub async fn play_media(
        &self,
        title: &str,
        season: Option<u32>,
        episode: Option<u32>,
        resume: bool,
        fullscreen: bool,
    ) -> Result<String> {
        if resume && season.is_none() && episode.is_none() {
            return self.resume_media(Some(title), fullscreen).await;
        }

        if let Some(media_match) = self.find_media(title, season, episode) {
            self.play_video_file(
                &media_match.file_path,
                &media_match.title,
                media_match.season,
                media_match.episode,
                fullscreen,
            )
            .await
        } else {
            let dirs_display = self
                .media_dirs
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            let target_str = match (season, episode) {
                (Some(s), Some(e)) => format!("Season {s} Episode {e} of '{title}'"),
                (_, Some(e)) => format!("Episode {e} of '{title}'"),
                _ => format!("'{title}'"),
            };
            Ok(format!(
                "Could not find {target_str} in configured media folders ({dirs_display})."
            ))
        }
    }

    pub async fn resume_media(&self, title: Option<&str>, fullscreen: bool) -> Result<String> {
        let hist = MediaHistory::load();

        if let Some(t) = title {
            let norm_key = t.to_lowercase().trim().to_string();
            if let Some(show) = hist.shows.get(&norm_key) {
                let p = PathBuf::from(&show.file_path);
                if p.exists() {
                    let _ = self
                        .play_video_file(&p, &show.title, show.season, show.episode, fullscreen)
                        .await?;
                    let ep_detail = match (show.season, show.episode) {
                        (Some(s), Some(e)) => format!(" Season {s} Episode {e}"),
                        (_, Some(e)) => format!(" Episode {e}"),
                        _ => String::new(),
                    };
                    return Ok(format!(
                        "Continuing {}{ep_detail} in fullscreen from where you left off.",
                        show.title
                    ));
                }
            }

            // If not found in history, try finding first episode
            if let Some(media_match) = self.find_media(t, None, None) {
                let _ = self
                    .play_video_file(
                        &media_match.file_path,
                        &media_match.title,
                        media_match.season,
                        media_match.episode,
                        fullscreen,
                    )
                    .await?;
                return Ok(format!(
                    "Continuing {} in fullscreen.",
                    media_match.display_name
                ));
            }

            Ok(format!(
                "Could not find any media files for '{t}' in your media folders."
            ))
        } else {
            if let Some(rec) = hist.recent.last() {
                let p = PathBuf::from(&rec.file_path);
                if p.exists() {
                    let _ = self
                        .play_video_file(&p, &rec.title, rec.season, rec.episode, fullscreen)
                        .await?;
                    let ep_detail = match (rec.season, rec.episode) {
                        (Some(s), Some(e)) => format!(" Season {s} Episode {e}"),
                        (_, Some(e)) => format!(" Episode {e}"),
                        _ => String::new(),
                    };
                    return Ok(format!(
                        "Continuing {}{ep_detail} in fullscreen from where you left off.",
                        rec.title
                    ));
                }
            }
            Ok("No recently watched media found in history to resume.".to_string())
        }
    }
}

fn scan_media_dir(dir: &Path, depth: usize, max_depth: usize, results: &mut Vec<PathBuf>) {
    if depth > max_depth {
        return;
    }
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') || name == "node_modules" || name == "target" || name == ".git"
            {
                continue;
            }
        }
        if path.is_dir() {
            scan_media_dir(&path, depth + 1, max_depth, results);
        } else if is_video_file(&path) {
            results.push(path);
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

pub struct PlayMediaTool {
    media: Arc<MediaManager>,
}

impl PlayMediaTool {
    pub fn new(media: Arc<MediaManager>) -> Self {
        Self { media }
    }
}

#[async_trait]
impl Tool for PlayMediaTool {
    fn name(&self) -> &'static str {
        "play_media"
    }

    fn description(&self) -> &'static str {
        "Search for and play a movie, TV show, or video file in fullscreen using the media player (mpv). Supports finding specific episodes (e.g. 'Open Prison break ep12 from season 1') or continuing from where left off."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "title": {
                    "type": "STRING",
                    "description": "Title of the TV series, movie, or video file to search for and play (e.g. 'Prison Break', 'Inception')."
                },
                "season": {
                    "type": "INTEGER",
                    "description": "Optional TV show season number (e.g. 1)."
                },
                "episode": {
                    "type": "INTEGER",
                    "description": "Optional TV show episode number (e.g. 12)."
                },
                "resume": {
                    "type": "BOOLEAN",
                    "description": "True to continue or resume playback from where the user previously left off."
                },
                "fullscreen": {
                    "type": "BOOLEAN",
                    "description": "Whether to start the video in fullscreen mode. Defaults to true."
                }
            },
            "required": ["title"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let title = args["title"].as_str().unwrap_or("").trim();
        if title.is_empty() {
            return Ok(
                "Please specify the name of the show, movie, or video file to play.".to_string(),
            );
        }

        let season = args.get("season").and_then(|v| {
            if let Some(n) = v.as_u64() {
                Some(n as u32)
            } else if let Some(s) = v.as_str() {
                s.chars()
                    .filter(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse()
                    .ok()
            } else {
                None
            }
        });

        let episode = args.get("episode").and_then(|v| {
            if let Some(n) = v.as_u64() {
                Some(n as u32)
            } else if let Some(s) = v.as_str() {
                s.chars()
                    .filter(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse()
                    .ok()
            } else {
                None
            }
        });

        let resume = args
            .get("resume")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let fullscreen = args
            .get("fullscreen")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        self.media
            .play_media(title, season, episode, resume, fullscreen)
            .await
    }
}

pub struct ResumeMediaTool {
    media: Arc<MediaManager>,
}

impl ResumeMediaTool {
    pub fn new(media: Arc<MediaManager>) -> Self {
        Self { media }
    }
}

#[async_trait]
impl Tool for ResumeMediaTool {
    fn name(&self) -> &'static str {
        "resume_media"
    }

    fn description(&self) -> &'static str {
        "Resume or continue playing a TV show, movie, or recent video from where the user left off (e.g. 'continue prison break from where I left off'). Automatically restores the exact playback timestamp."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "title": {
                    "type": "STRING",
                    "description": "Optional title of the TV series or movie to continue (e.g. 'Prison Break'). If omitted, continues the most recently played media."
                },
                "fullscreen": {
                    "type": "BOOLEAN",
                    "description": "Whether to start the video in fullscreen mode. Defaults to true."
                }
            }
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let title = args.get("title").and_then(|v| v.as_str()).map(|s| s.trim());
        let fullscreen = args
            .get("fullscreen")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        self.media.resume_media(title, fullscreen).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_season_episode_formats() {
        let p1 = Path::new("/home/user/Movies/Prison Break/Season 1/Prison Break S01E12 720p English Esubs MoviesFlix mkv");
        assert_eq!(extract_season_episode(p1), (Some(1), Some(12)));

        let p2 = Path::new("/home/user/Movies/Prison Break/Season 1/Prison Break S01E01.mkv");
        assert_eq!(extract_season_episode(p2), (Some(1), Some(1)));

        let p3 = Path::new("/home/user/Downloads/Show.Name.2x05.mp4");
        assert_eq!(extract_season_episode(p3), (Some(2), Some(5)));

        let p4 = Path::new("/home/user/Movies/Inception (2010)/Inception.mkv");
        assert_eq!(extract_season_episode(p4), (None, None));
    }

    #[test]
    fn test_has_video_extension() {
        assert!(has_video_extension(Path::new("movie.mkv")));
        assert!(has_video_extension(Path::new("movie.mp4")));
        assert!(has_video_extension(Path::new("movie.avi")));
        assert!(has_video_extension(Path::new(
            "Prison Break S01E12 MoviesFlix mkv"
        )));
        assert!(!has_video_extension(Path::new("movie.txt")));
        assert!(!has_video_extension(Path::new("movie.pdf")));
    }

    #[test]
    fn test_find_prison_break_ep12() {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let movies_dir = PathBuf::from(home).join("Movies");
        if !movies_dir.join("Prison Break").exists() {
            return;
        }
        let manager = MediaManager::with_settings(vec![movies_dir], "mpv");
        let found = manager.find_media("Prison break", Some(1), Some(12));
        assert!(found.is_some());
        let m = found.unwrap();
        assert_eq!(m.season, Some(1));
        assert_eq!(m.episode, Some(12));
        assert!(m.file_path.to_str().unwrap().contains("S01E12"));
    }
}
