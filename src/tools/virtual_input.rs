use crate::core::error::{JarvisError, Result};
use crate::tools::Tool;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::{sleep, Duration};
use tracing::debug;

pub struct VirtualInputManager {
    wtype_bin: PathBuf,
}

impl Default for VirtualInputManager {
    fn default() -> Self {
        Self::new()
    }
}

impl VirtualInputManager {
    pub fn new() -> Self {
        let wtype_bin = which::which("wtype").unwrap_or_else(|_| PathBuf::from("/usr/bin/wtype"));
        Self { wtype_bin }
    }

    fn map_key<'a>(&self, raw: &'a str) -> std::borrow::Cow<'a, str> {
        let trimmed = raw.trim();
        match trimmed.to_lowercase().as_str() {
            "enter" | "return" => std::borrow::Cow::Borrowed("Return"),
            "tab" => std::borrow::Cow::Borrowed("Tab"),
            "esc" | "escape" => std::borrow::Cow::Borrowed("Escape"),
            "backspace" => std::borrow::Cow::Borrowed("BackSpace"),
            "space" | "spacebar" => std::borrow::Cow::Borrowed("space"),
            "up" => std::borrow::Cow::Borrowed("Up"),
            "down" => std::borrow::Cow::Borrowed("Down"),
            "left" => std::borrow::Cow::Borrowed("Left"),
            "right" => std::borrow::Cow::Borrowed("Right"),
            "page_up" | "pageup" => std::borrow::Cow::Borrowed("Page_Up"),
            "page_down" | "pagedown" => std::borrow::Cow::Borrowed("Page_Down"),
            "home" => std::borrow::Cow::Borrowed("Home"),
            "end" => std::borrow::Cow::Borrowed("End"),
            "delete" => std::borrow::Cow::Borrowed("Delete"),
            _ => std::borrow::Cow::Borrowed(trimmed),
        }
    }

    pub async fn type_text(&self, text: &str, enter_after: bool) -> Result<String> {
        if text.is_empty() {
            return Ok("No text provided to type.".to_string());
        }

        debug!("Virtual input typing: '{text}' (enter={enter_after})");
        let mut child = Command::new(&self.wtype_bin)
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes()).await?;
            stdin.flush().await?;
        }
        child.wait().await?;

        if enter_after {
            sleep(Duration::from_millis(50)).await;
            self.press_key("Return").await?;
        }

        Ok(format!("Typed: \"{text}\""))
    }

    pub async fn press_key(&self, key_name: &str) -> Result<String> {
        let key = self.map_key(key_name);
        debug!("Virtual input pressing key: '{key}'");

        let status = Command::new(&self.wtype_bin)
            .arg("-k")
            .arg(key.as_ref())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await?;

        if status.success() {
            Ok(format!("Pressed key '{key}'."))
        } else {
            Err(JarvisError::ToolExecution {
                tool: "press_key".to_string(),
                message: format!("wtype failed with exit code: {:?}", status.code()),
            })
        }
    }

    pub async fn send_shortcut(&self, modifiers: &str, key: &str) -> Result<String> {
        let mapped_key = self.map_key(key);
        let clean_mods = modifiers.replace('+', " ");
        let mod_parts: Vec<&str> = clean_mods
            .split_whitespace()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        let mut cmd = Command::new(&self.wtype_bin);
        for m in &mod_parts {
            cmd.arg("-M").arg(m);
        }
        cmd.arg("-k").arg(mapped_key.as_ref());
        for m in mod_parts.iter().rev() {
            cmd.arg("-m").arg(m);
        }

        cmd.stdout(Stdio::null()).stderr(Stdio::null());
        let status = cmd.status().await?;

        if status.success() {
            let mod_joined = mod_parts.join("+");
            Ok(format!("Sent shortcut {mod_joined}+{mapped_key}."))
        } else {
            Err(JarvisError::ToolExecution {
                tool: "send_shortcut".to_string(),
                message: format!("wtype shortcut failed with status {:?}", status.code()),
            })
        }
    }

    pub async fn scroll(&self, direction: &str, amount: u32) -> Result<String> {
        let clean_dir = direction.to_lowercase();
        let key = if clean_dir == "down" || clean_dir == "d" || clean_dir == "next" {
            "Page_Down"
        } else {
            "Page_Up"
        };
        let repeats = amount.clamp(1, 10);

        for i in 0..repeats {
            self.press_key(key).await?;
            if i < repeats - 1 {
                sleep(Duration::from_millis(80)).await;
            }
        }
        Ok(format!("Scrolled {clean_dir}."))
    }
}

// -----------------------------------------------------------------------------
// Tool Implementations
// -----------------------------------------------------------------------------

pub struct TypeTextTool {
    input: Arc<VirtualInputManager>,
}

impl TypeTextTool {
    pub fn new(input: Arc<VirtualInputManager>) -> Self {
        Self { input }
    }
}

#[async_trait]
impl Tool for TypeTextTool {
    fn name(&self) -> &'static str {
        "type_text"
    }

    fn description(&self) -> &'static str {
        "Type text directly into the currently focused window hands-free. Set enter_after=True to submit."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "text": {
                    "type": "STRING",
                    "description": "The exact text string to type into the active window."
                },
                "enter_after": {
                    "type": "BOOLEAN",
                    "description": "Whether to press Return/Enter immediately after typing."
                }
            },
            "required": ["text"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let text = args["text"].as_str().ok_or_else(|| {
            JarvisError::ToolParameter("type_text".into(), "text string is required".into())
        })?;
        let enter_after = args["enter_after"].as_bool().unwrap_or(false);

        self.input.type_text(text, enter_after).await
    }
}

pub struct PressKeyTool {
    input: Arc<VirtualInputManager>,
}

impl PressKeyTool {
    pub fn new(input: Arc<VirtualInputManager>) -> Self {
        Self { input }
    }
}

#[async_trait]
impl Tool for PressKeyTool {
    fn name(&self) -> &'static str {
        "press_key"
    }

    fn description(&self) -> &'static str {
        "Press a keyboard key hands-free (e.g. 'Return', 'Escape', 'Tab', 'BackSpace', 'space', 'Up', 'Down', 'Page_Down')."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "key_name": {
                    "type": "STRING",
                    "description": "Name of the key to press (e.g. Return, Escape, Tab, BackSpace, space, Up, Down)."
                }
            },
            "required": ["key_name"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let key_name = args["key_name"].as_str().ok_or_else(|| {
            JarvisError::ToolParameter("press_key".into(), "key_name string is required".into())
        })?;

        self.input.press_key(key_name).await
    }
}

pub struct SendShortcutTool {
    input: Arc<VirtualInputManager>,
}

impl SendShortcutTool {
    pub fn new(input: Arc<VirtualInputManager>) -> Self {
        Self { input }
    }
}

#[async_trait]
impl Tool for SendShortcutTool {
    fn name(&self) -> &'static str {
        "send_shortcut"
    }

    fn description(&self) -> &'static str {
        "Send a keyboard shortcut hands-free (e.g. modifiers='ctrl', key='s' or modifiers='ctrl+shift', key='t')."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "modifiers": {
                    "type": "STRING",
                    "description": "Modifier keys separated by + or space (e.g. 'ctrl', 'ctrl+shift', 'alt', 'super')."
                },
                "key": {
                    "type": "STRING",
                    "description": "Primary key to press with modifiers (e.g. 's', 'c', 'v', 'Return')."
                }
            },
            "required": ["modifiers", "key"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let modifiers = args["modifiers"].as_str().ok_or_else(|| {
            JarvisError::ToolParameter(
                "send_shortcut".into(),
                "modifiers string is required".into(),
            )
        })?;
        let key = args["key"].as_str().ok_or_else(|| {
            JarvisError::ToolParameter("send_shortcut".into(), "key string is required".into())
        })?;

        self.input.send_shortcut(modifiers, key).await
    }
}

pub struct ScrollTool {
    input: Arc<VirtualInputManager>,
}

impl ScrollTool {
    pub fn new(input: Arc<VirtualInputManager>) -> Self {
        Self { input }
    }
}

#[async_trait]
impl Tool for ScrollTool {
    fn name(&self) -> &'static str {
        "scroll"
    }

    fn description(&self) -> &'static str {
        "Scroll the active window up or down hands-free (direction: 'up' or 'down', amount: 1-10)."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "direction": {
                    "type": "STRING",
                    "description": "Scroll direction ('up' or 'down'). Default is 'down'."
                },
                "amount": {
                    "type": "INTEGER",
                    "description": "Amount of page jumps to scroll (1 to 10). Default is 2."
                }
            }
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let direction = args["direction"].as_str().unwrap_or("down");
        let amount = args["amount"].as_u64().unwrap_or(2) as u32;

        self.input.scroll(direction, amount).await
    }
}
