use crate::core::error::{JarvisError, Result};
use crate::tools::Tool;
use async_trait::async_trait;
use regex::Regex;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::debug;

pub struct WebNavigator {
    browser_bin: PathBuf,
}

impl Default for WebNavigator {
    fn default() -> Self {
        Self::new()
    }
}

impl WebNavigator {
    pub fn new() -> Self {
        let browser_bin = which::which("omarchy-launch-browser")
            .or_else(|_| which::which("xdg-open"))
            .unwrap_or_else(|_| PathBuf::from("xdg-open"));
        Self { browser_bin }
    }

    pub async fn open_url(&self, url: &str) -> Result<String> {
        let trimmed = url.trim();
        let target_url = if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
            format!("https://{trimmed}")
        } else {
            trimmed.to_string()
        };

        debug!("Opening URL in browser: {target_url}");
        let mut child = Command::new(&self.browser_bin)
            .arg(&target_url)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        let _ = timeout(Duration::from_millis(200), child.wait()).await;
        Ok(format!("Opened {target_url} in browser."))
    }

    pub async fn search_web(&self, query: &str) -> Result<String> {
        let encoded = urlencoding::encode(query.trim());
        let url = format!("https://www.google.com/search?q={encoded}");
        self.open_url(&url).await
    }

    pub async fn open_youtube(&self, query: &str) -> Result<String> {
        let clean = query.trim();
        if clean.contains("youtube.com") || clean.contains("youtu.be") {
            return self.open_url(clean).await;
        }

        // Try to scrape video ID with quick timeout
        let resolve_res = timeout(
            Duration::from_millis(1500),
            Self::resolve_youtube_video(clean),
        )
        .await;

        match resolve_res {
            Ok(Some((video_id, title))) => {
                let watch_url = format!("https://www.youtube.com/watch?v={video_id}");
                self.open_url(&watch_url).await?;
                Ok(format!("Playing '{title}' on YouTube."))
            }
            _ => {
                // Fallback to YouTube search query
                let encoded = urlencoding::encode(clean);
                let search_url = format!("https://www.youtube.com/results?search_query={encoded}");
                self.open_url(&search_url).await?;
                Ok(format!("Opened YouTube search for '{clean}'."))
            }
        }
    }

    async fn resolve_youtube_video(query: &str) -> Option<(String, String)> {
        let encoded = urlencoding::encode(query);
        let url = format!("https://www.youtube.com/results?search_query={encoded}");

        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(1200))
            .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0")
            .build()
            .ok()?;

        let resp = client.get(&url).send().await.ok()?;
        let html = resp.text().await.ok()?;

        let re = Regex::new(
            r#""videoRenderer":\{"videoId":"([a-zA-Z0-9_-]{11})".*?"title":\{"runs":\[\{"text":"(.*?)"\}"#,
        )
        .ok()?;

        if let Some(caps) = re.captures(&html) {
            let id = caps.get(1).map(|m| m.as_str().to_string())?;
            let title = caps
                .get(2)
                .map(|m| m.as_str().to_string())
                .unwrap_or_else(|| query.to_string());
            return Some((id, title));
        }

        let id_re = Regex::new(r#""videoId":"([a-zA-Z0-9_-]{11})""#).ok()?;
        if let Some(caps) = id_re.captures(&html) {
            let id = caps.get(1).map(|m| m.as_str().to_string())?;
            return Some((id, query.to_string()));
        }

        None
    }
}

// -----------------------------------------------------------------------------
// Tool Implementations
// -----------------------------------------------------------------------------

pub struct OpenUrlTool {
    web: Arc<WebNavigator>,
}

impl OpenUrlTool {
    pub fn new(web: Arc<WebNavigator>) -> Self {
        Self { web }
    }
}

#[async_trait]
impl Tool for OpenUrlTool {
    fn name(&self) -> &'static str {
        "open_url"
    }

    fn description(&self) -> &'static str {
        "Open any website or URL in the default browser."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "url": {
                    "type": "STRING",
                    "description": "Website URL to open (e.g. 'github.com', 'https://reddit.com')."
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let url = args["url"].as_str().ok_or_else(|| {
            JarvisError::ToolParameter("open_url".into(), "url string is required".into())
        })?;

        self.web.open_url(url).await
    }
}

pub struct SearchWebTool {
    web: Arc<WebNavigator>,
}

impl SearchWebTool {
    pub fn new(web: Arc<WebNavigator>) -> Self {
        Self { web }
    }
}

#[async_trait]
impl Tool for SearchWebTool {
    fn name(&self) -> &'static str {
        "search_web"
    }

    fn description(&self) -> &'static str {
        "Search Google for information or websites."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "query": {
                    "type": "STRING",
                    "description": "The search query string."
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let query = args["query"].as_str().ok_or_else(|| {
            JarvisError::ToolParameter("search_web".into(), "query string is required".into())
        })?;

        self.web.search_web(query).await
    }
}

pub struct OpenYoutubeTool {
    web: Arc<WebNavigator>,
}

impl OpenYoutubeTool {
    pub fn new(web: Arc<WebNavigator>) -> Self {
        Self { web }
    }
}

#[async_trait]
impl Tool for OpenYoutubeTool {
    fn name(&self) -> &'static str {
        "open_youtube"
    }

    fn description(&self) -> &'static str {
        "Search and play a video on YouTube (e.g. 'mkbhd latest video', 'lofi beats')."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "query": {
                    "type": "STRING",
                    "description": "Search term or YouTube query."
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let query = args["query"].as_str().ok_or_else(|| {
            JarvisError::ToolParameter("open_youtube".into(), "query string is required".into())
        })?;

        self.web.open_youtube(query).await
    }
}
