use crate::core::error::{JarvisError, Result};
use crate::tools::Tool;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::debug;

pub struct ClipboardManager {
    wl_paste_bin: PathBuf,
    wl_copy_bin: PathBuf,
}

impl Default for ClipboardManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ClipboardManager {
    pub fn new() -> Self {
        let wl_paste_bin =
            which::which("wl-paste").unwrap_or_else(|_| PathBuf::from("/usr/bin/wl-paste"));
        let wl_copy_bin =
            which::which("wl-copy").unwrap_or_else(|_| PathBuf::from("/usr/bin/wl-copy"));
        Self {
            wl_paste_bin,
            wl_copy_bin,
        }
    }

    pub async fn get_clipboard(&self) -> Result<String> {
        let output = Command::new(&self.wl_paste_bin)
            .arg("-n")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            debug!("wl-paste returned non-zero: {err}");
            return Ok("Clipboard is empty or contains non-text data.".to_string());
        }

        let mut text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if text.is_empty() {
            return Ok("Clipboard is currently empty.".to_string());
        }

        if text.len() > 1500 {
            text = format!("{}... [truncated]", &text[..1500]);
        }

        debug!("Read clipboard content ({} chars)", text.len());
        Ok(format!("Clipboard content: \"{text}\""))
    }

    pub async fn set_clipboard(&self, text: &str) -> Result<String> {
        if text.is_empty() {
            return Ok("No text provided to copy to clipboard.".to_string());
        }

        if text.len() <= 65536 {
            let status = Command::new(&self.wl_copy_bin)
                .arg("--")
                .arg(text)
                .status()
                .await?;
            if !status.success() {
                debug!("wl-copy exited with status {:?}", status.code());
            }
        } else {
            let mut child = Command::new(&self.wl_copy_bin)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;

            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(text.as_bytes()).await?;
                stdin.flush().await?;
            }

            child.wait().await?;
        }
        debug!("Copied text to clipboard ({} chars)", text.len());
        let preview = if text.len() > 60 {
            format!("{}...", &text[..60])
        } else {
            text.to_string()
        };

        if !crate::ui::TaskNotifier::global().is_active() {
            crate::ui::send_desktop_notification("󰅍", "Copied to Clipboard", &preview, 2500, "low")
                .await;
        }

        Ok(format!("Copied to clipboard: \"{preview}\""))
    }
}

// -----------------------------------------------------------------------------
// Tool Implementations
// -----------------------------------------------------------------------------

pub struct GetClipboardTool {
    clipboard: Arc<ClipboardManager>,
}

impl GetClipboardTool {
    pub fn new(clipboard: Arc<ClipboardManager>) -> Self {
        Self { clipboard }
    }
}

#[async_trait]
impl Tool for GetClipboardTool {
    fn name(&self) -> &'static str {
        "get_clipboard"
    }

    fn description(&self) -> &'static str {
        "Read the current text content stored in the system clipboard."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {}
        })
    }

    async fn execute(&self, _args: Value) -> Result<String> {
        self.clipboard.get_clipboard().await
    }
}

pub struct SetClipboardTool {
    clipboard: Arc<ClipboardManager>,
}

impl SetClipboardTool {
    pub fn new(clipboard: Arc<ClipboardManager>) -> Self {
        Self { clipboard }
    }
}

#[async_trait]
impl Tool for SetClipboardTool {
    fn name(&self) -> &'static str {
        "set_clipboard"
    }

    fn description(&self) -> &'static str {
        "Copy specified text to the system clipboard."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "text": {
                    "type": "STRING",
                    "description": "The exact text to place onto the system clipboard."
                }
            },
            "required": ["text"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let text = args["text"].as_str().ok_or_else(|| {
            JarvisError::ToolParameter("set_clipboard".into(), "text string is required".into())
        })?;

        self.clipboard.set_clipboard(text).await
    }
}
