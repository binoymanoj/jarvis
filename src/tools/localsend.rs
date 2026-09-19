use crate::core::error::{JarvisError, Result};
use crate::tools::hyprland::HyprlandController;
use crate::tools::Tool;
use async_trait::async_trait;
use regex::Regex;
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::sleep;
use tracing::info;

#[derive(Debug, PartialEq, Eq)]
pub enum ShareTarget {
    File(PathBuf),
    Text(String),
}

pub struct LocalSendManager {
    localsend_bin: PathBuf,
    localsend_cli_bin: Option<PathBuf>,
    hyprland: Arc<HyprlandController>,
    media_dirs: Vec<PathBuf>,
    search_dirs: Vec<PathBuf>,
}

impl LocalSendManager {
    pub fn new(hyprland: Arc<HyprlandController>, media_dirs: Vec<PathBuf>) -> Self {
        let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let home_path = PathBuf::from(&home);

        let localsend_bin =
            which::which("localsend").unwrap_or_else(|_| PathBuf::from("/usr/bin/localsend"));
        let localsend_cli_bin = which::which("localsend-cli").ok();

        let mut search_dirs = Vec::new();
        // Priority directories for sharing
        for dir in [
            home_path.join("Downloads"),
            home_path.join("Documents"),
            home_path.join("Pictures"),
            home_path.join("Pictures").join("Screenshots"),
            home_path.join("Desktop"),
            home_path.join("Videos"),
            home_path.join("Codes").join("personal"),
            home_path.join("Codes"),
            home_path.join("Projects"),
            PathBuf::from("/tmp"),
        ] {
            if dir.is_dir() {
                search_dirs.push(dir);
            }
        }

        Self {
            localsend_bin,
            localsend_cli_bin,
            hyprland,
            media_dirs,
            search_dirs,
        }
    }

    /// Read active clipboard content using wl-paste or xclip
    pub async fn get_clipboard_text(&self) -> Option<String> {
        let output = Command::new("wl-paste")
            .args(["--no-newline"])
            .output()
            .await
            .ok()
            .or_else(|| {
                std::process::Command::new("xclip")
                    .args(["-selection", "clipboard", "-o"])
                    .output()
                    .ok()
            })?;

        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !text.is_empty() {
                return Some(text);
            }
        }
        None
    }

    /// Find newest screenshot file in standard screenshot locations
    pub fn find_latest_screenshot(&self) -> Option<PathBuf> {
        let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let home_path = PathBuf::from(home);

        let screenshot_dirs = [
            home_path.join("Pictures").join("Screenshots"),
            home_path.join("Pictures"),
            home_path.join("Desktop"),
            home_path.join("Downloads"),
        ];

        let mut newest_file: Option<(PathBuf, std::time::SystemTime)> = None;

        for dir in screenshot_dirs {
            if !dir.is_dir() {
                continue;
            }
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let name = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or_default()
                            .to_lowercase();
                        if (name.contains("screenshot")
                            || name.starts_with("shot_")
                            || name.starts_with("screen_"))
                            && (name.ends_with(".png")
                                || name.ends_with(".jpg")
                                || name.ends_with(".jpeg"))
                        {
                            if let Ok(meta) = entry.metadata() {
                                if let Ok(mtime) = meta.modified() {
                                    if newest_file.as_ref().is_none_or(|(_, t)| mtime > *t) {
                                        newest_file = Some((path, mtime));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        newest_file.map(|(p, _)| p)
    }

    /// Search for files matching season/episode patterns (e.g. Prison Break S01E12)
    fn find_media_episode(&self, query: &str) -> Option<PathBuf> {
        let ep_re1 = Regex::new(r"(?i)s(?:eason)?\s*0?(\d+).*?e(?:p(?:isode)?)?\s*0?(\d+)").ok()?;
        let ep_re2 = Regex::new(r"(?i)ep(?:isode)?\s*0?(\d+)").ok()?;

        let mut season_num: Option<u32> = None;
        let mut episode_num: Option<u32> = None;

        if let Some(caps) = ep_re1.captures(query) {
            season_num = caps.get(1).and_then(|m| m.as_str().parse().ok());
            episode_num = caps.get(2).and_then(|m| m.as_str().parse().ok());
        } else if let Some(caps) = ep_re2.captures(query) {
            episode_num = caps.get(1).and_then(|m| m.as_str().parse().ok());
        }

        let title_words: Vec<String> = query
            .split_whitespace()
            .filter(|w| {
                let l = w.to_lowercase();
                !l.starts_with("season")
                    && !l.starts_with("ep")
                    && !l.starts_with("s0")
                    && !l.starts_with("e0")
                    && !l.starts_with("from")
                    && !l.starts_with("in")
            })
            .map(|w| w.to_lowercase())
            .collect();

        // Search in media directories and Downloads
        let mut search_roots = self.media_dirs.clone();
        let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let downloads = PathBuf::from(home).join("Downloads");
        if !search_roots.contains(&downloads) && downloads.is_dir() {
            search_roots.push(downloads);
        }

        for root in &search_roots {
            if !root.is_dir() {
                continue;
            }
            if let Some(found) = self.walk_for_episode(root, &title_words, season_num, episode_num)
            {
                return Some(found);
            }
        }

        None
    }

    fn walk_for_episode(
        &self,
        dir: &Path,
        title_words: &[String],
        target_season: Option<u32>,
        target_episode: Option<u32>,
    ) -> Option<PathBuf> {
        let ep_re = Regex::new(r"(?i)s(?:eason)?\s*0?(\d+).*?e(?:p(?:isode)?)?\s*0?(\d+)").ok()?;
        let ep_only_re = Regex::new(r"(?i)(?:ep|e|episode)\s*0?(\d+)").ok()?;

        let mut stack = vec![dir.to_path_buf()];
        while let Some(current) = stack.pop() {
            if let Ok(entries) = fs::read_dir(&current) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().to_string();

                    if path.is_dir() {
                        if !name.starts_with('.') {
                            stack.push(path);
                        }
                    } else if path.is_file() {
                        let lower_name = name.to_lowercase();
                        let is_video = [".mp4", ".mkv", ".avi", ".mov", ".webm"]
                            .iter()
                            .any(|ext| lower_name.ends_with(ext));
                        if !is_video {
                            continue;
                        }

                        // Check if title words match
                        let matches_title = title_words.is_empty()
                            || title_words.iter().all(|w| lower_name.contains(w));
                        if !matches_title {
                            continue;
                        }

                        if let Some(target_ep) = target_episode {
                            if let Some(caps) = ep_re.captures(&name) {
                                let s: u32 = caps
                                    .get(1)
                                    .and_then(|m| m.as_str().parse().ok())
                                    .unwrap_or(0);
                                let e: u32 = caps
                                    .get(2)
                                    .and_then(|m| m.as_str().parse().ok())
                                    .unwrap_or(0);
                                if e == target_ep
                                    && (target_season.is_none() || target_season == Some(s))
                                {
                                    return Some(path);
                                }
                            } else if let Some(caps) = ep_only_re.captures(&name) {
                                let e: u32 = caps
                                    .get(1)
                                    .and_then(|m| m.as_str().parse().ok())
                                    .unwrap_or(0);
                                if e == target_ep {
                                    return Some(path);
                                }
                            }
                        } else if matches_title {
                            return Some(path);
                        }
                    }
                }
            }
        }
        None
    }

    /// Resolve an item name or search query to a local file path
    pub fn resolve_file(&self, query: &str) -> Option<PathBuf> {
        let clean = query.trim();
        if clean.is_empty() {
            return None;
        }

        // 1. Direct path check (absolute or home relative)
        let direct_path = if let Some(stripped) = clean.strip_prefix("~/") {
            let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(stripped)
        } else {
            PathBuf::from(clean)
        };

        if direct_path.is_file() {
            return Some(direct_path);
        }

        // 2. Check if asking for screenshot
        let lower = clean.to_lowercase();
        if lower == "screenshot" || lower == "screen" || lower == "latest screenshot" {
            if let Some(p) = self.find_latest_screenshot() {
                return Some(p);
            }
        }

        // 3. Media episode pattern check (e.g. Prison Break Season 1 Episode 12)
        if let Some(media_file) = self.find_media_episode(clean) {
            return Some(media_file);
        }

        // 4. Exact filename search across search directories
        let target_name = Path::new(clean)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(clean)
            .to_lowercase();

        let mut all_dirs = self.search_dirs.clone();
        for md in &self.media_dirs {
            if !all_dirs.contains(md) && md.is_dir() {
                all_dirs.push(md.clone());
            }
        }

        for base in &all_dirs {
            if !base.is_dir() {
                continue;
            }
            if let Ok(entries) = fs::read_dir(base) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let fname = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or_default()
                            .to_lowercase();
                        if fname == target_name {
                            return Some(path);
                        }
                    }
                }
            }
        }

        // 5. Partial / fuzzy search in search directories
        let keywords: Vec<&str> = clean.split_whitespace().collect();
        for base in &all_dirs {
            if !base.is_dir() {
                continue;
            }
            if let Ok(entries) = fs::read_dir(base) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let fname = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or_default()
                            .to_lowercase();
                        if keywords.iter().all(|k| fname.contains(&k.to_lowercase())) {
                            return Some(path);
                        }
                    }
                }
            }
        }

        None
    }

    /// Resolve what should be shared (file or text) based on input item and type
    pub async fn resolve_target(&self, item: &str, item_type: Option<&str>) -> Result<ShareTarget> {
        let t_type = item_type.unwrap_or("auto").to_lowercase();

        // 1. Explicit clipboard
        if t_type == "clipboard" || item.eq_ignore_ascii_case("clipboard") {
            if let Some(text) = self.get_clipboard_text().await {
                // If clipboard text points to an existing file, share that file!
                if let Some(path) = self.resolve_file(&text) {
                    return Ok(ShareTarget::File(path));
                }
                return Ok(ShareTarget::Text(text));
            } else {
                return Err(JarvisError::ToolExecution {
                    tool: "localsend_share".into(),
                    message: "Clipboard is currently empty.".into(),
                });
            }
        }

        // 2. Explicit screenshot
        if t_type == "screenshot" || item.eq_ignore_ascii_case("screenshot") {
            if let Some(path) = self.find_latest_screenshot() {
                return Ok(ShareTarget::File(path));
            } else {
                return Err(JarvisError::ToolExecution {
                    tool: "localsend_share".into(),
                    message: "No recent screenshot found in ~/Pictures/Screenshots.".into(),
                });
            }
        }

        // 3. URLs are always text shares
        if item.starts_with("http://") || item.starts_with("https://") {
            return Ok(ShareTarget::Text(item.to_string()));
        }

        // 4. If explicit text requested
        if t_type == "text" {
            return Ok(ShareTarget::Text(item.to_string()));
        }

        // 5. Try file resolution
        if let Some(file_path) = self.resolve_file(item) {
            return Ok(ShareTarget::File(file_path));
        }

        // 6. If explicit file requested and not found
        if t_type == "file" {
            return Err(JarvisError::ToolExecution {
                tool: "localsend_share".into(),
                message: format!(
                    "Could not find file '{item}' in common folders or media directories."
                ),
            });
        }

        // 7. Auto fallback: if it looks like a message or note, share as text
        if item.contains(' ') && !item.contains('/') && !item.contains('.') {
            return Ok(ShareTarget::Text(item.to_string()));
        }

        Err(JarvisError::ToolExecution {
            tool: "localsend_share".into(),
            message: format!("Could not locate file or item '{item}' to share with LocalSend."),
        })
    }

    /// Share the resolved item to LocalSend and focus the sending window
    pub async fn share_item(
        &self,
        item: &str,
        item_type: Option<&str>,
        target_device: Option<&str>,
    ) -> Result<String> {
        let target = self.resolve_target(item, item_type).await?;

        info!("LocalSend sharing target: {:?}", target);

        // If target_device is specified and localsend-cli is available, we can invoke CLI directly
        if let Some(dev) = target_device {
            if let Some(ref cli) = self.localsend_cli_bin {
                match target {
                    ShareTarget::File(ref path) => {
                        let mut cmd = Command::new(cli);
                        cmd.process_group(0);
                        cmd.arg("-f")
                            .arg(path)
                            .arg("--alias")
                            .arg(dev)
                            .stdout(Stdio::null())
                            .stderr(Stdio::null());
                        let _ = cmd.spawn();
                        return Ok(format!(
                            "Sending '{}' to device '{}' via LocalSend.",
                            path.file_name().unwrap_or_default().to_string_lossy(),
                            dev
                        ));
                    }
                    ShareTarget::Text(ref txt) => {
                        let mut cmd = Command::new(&self.localsend_bin);
                        cmd.process_group(0);
                        cmd.arg("--text")
                            .arg(txt)
                            .stdout(Stdio::null())
                            .stderr(Stdio::null());
                        let _ = cmd.spawn();
                        self.focus_localsend_window().await;
                        return Ok(format!("Sent text to LocalSend for device '{dev}'."));
                    }
                }
            }
        }

        // Standard LocalSend GUI workflow: load into sending window
        let result_message = match target {
            ShareTarget::File(ref path) => {
                let mut cmd = Command::new(&self.localsend_bin);
                cmd.process_group(0);
                cmd.arg(path).stdout(Stdio::null()).stderr(Stdio::null());
                cmd.spawn()
                    .map_err(|e| JarvisError::Other(format!("Failed to launch localsend: {e}")))?;

                let fname = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                format!("Shared '{fname}' to LocalSend sending window.")
            }
            ShareTarget::Text(ref text) => {
                let mut cmd = Command::new(&self.localsend_bin);
                cmd.process_group(0);
                cmd.arg("--text")
                    .arg(text)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                cmd.spawn()
                    .map_err(|e| JarvisError::Other(format!("Failed to launch localsend: {e}")))?;

                "Shared text to LocalSend sending window.".to_string()
            }
        };

        // Smoothly focus the LocalSend window on Hyprland
        self.focus_localsend_window().await;

        if !crate::ui::TaskNotifier::global().is_active() {
            crate::ui::send_desktop_notification(
                "󰄬",
                "LocalSend Share",
                &result_message,
                4000,
                "normal",
            )
            .await;
        }

        Ok(result_message)
    }

    /// Focus LocalSend window in Hyprland
    async fn focus_localsend_window(&self) {
        sleep(Duration::from_millis(250)).await;
        let _ = self
            .hyprland
            .dispatch("focuswindow class:org.localsend.localsend_app")
            .await;
    }
}

// -----------------------------------------------------------------------------
// Tool Implementations
// -----------------------------------------------------------------------------

pub struct LocalSendShareTool {
    manager: Arc<LocalSendManager>,
}

impl LocalSendShareTool {
    pub fn new(manager: Arc<LocalSendManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for LocalSendShareTool {
    fn name(&self) -> &'static str {
        "localsend_share"
    }

    fn description(&self) -> &'static str {
        "Share any file, photo, video, document, text note, URL, or clipboard content to nearby devices using LocalSend (opens directly into LocalSend's sending window)."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "item": {
                    "type": "STRING",
                    "description": "Name of the file, path, media episode (e.g. 'Prison Break ep12', 'resume.pdf', 'photo.jpg'), text message, URL, or 'clipboard' / 'screenshot'."
                },
                "item_type": {
                    "type": "STRING",
                    "description": "Type of item to share: 'auto' (default), 'file', 'text', 'clipboard', 'screenshot'."
                },
                "target_device": {
                    "type": "STRING",
                    "description": "Optional name, alias, or IP of the destination device."
                }
            },
            "required": ["item"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let item = args["item"].as_str().ok_or_else(|| {
            JarvisError::ToolParameter("localsend_share".into(), "item is required".into())
        })?;
        let item_type = args["item_type"].as_str();
        let target_device = args["target_device"].as_str();

        self.manager
            .share_item(item, item_type, target_device)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_localsend_resolve_target_url() {
        let hypr = Arc::new(HyprlandController::new());
        let mgr = LocalSendManager::new(hypr, vec![]);
        let res = mgr
            .resolve_target("https://github.com/localsend", None)
            .await;
        assert!(res.is_ok());
        assert_eq!(
            res.unwrap(),
            ShareTarget::Text("https://github.com/localsend".to_string())
        );
    }

    #[tokio::test]
    async fn test_localsend_resolve_existing_file() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("jarvis_test_localsend_sample.pdf");
        let _ = fs::write(&test_file, b"sample test pdf content");

        let hypr = Arc::new(HyprlandController::new());
        let mgr = LocalSendManager::new(hypr, vec![temp_dir]);
        let res = mgr
            .resolve_target("jarvis_test_localsend_sample.pdf", None)
            .await;
        assert!(res.is_ok());
        if let ShareTarget::File(p) = res.unwrap() {
            assert_eq!(p, test_file);
        } else {
            panic!("Expected file target");
        }

        let _ = fs::remove_file(&test_file);
    }
}
