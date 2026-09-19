use crate::core::error::{JarvisError, Result};
use crate::tools::Tool;
use async_trait::async_trait;
use chrono::Local;
use regex::Regex;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;
use tracing::debug;

pub struct NoteManager {
    notes_dir: PathBuf,
    editor_bin: Option<PathBuf>,
}

impl Default for NoteManager {
    fn default() -> Self {
        Self::new()
    }
}

impl NoteManager {
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let notes_dir = PathBuf::from(home).join("Notes");
        let _ = fs::create_dir_all(&notes_dir);

        let editor_bin = which::which("obsidian")
            .or_else(|_| which::which("omarchy-launch-editor"))
            .or_else(|_| which::which("nvim"))
            .ok();

        Self {
            notes_dir,
            editor_bin,
        }
    }

    pub async fn create_note(
        &self,
        title: &str,
        content: &str,
        open_in_editor: bool,
    ) -> Result<String> {
        let re = Regex::new(r"[^\w\s-]").unwrap();
        let clean = re.replace_all(title, "").trim().to_string();
        let safe_title = if clean.is_empty() {
            "Untitled_Note"
        } else {
            &clean
        };
        let filename = format!("{}.md", safe_title.replace(' ', "_"));
        let file_path = self.notes_dir.join(&filename);

        let timestamp = Local::now().format("%A, %B %d, %Y at %I:%M %p").to_string();

        let action = if file_path.is_file() {
            let existing = fs::read_to_string(&file_path).unwrap_or_default();
            let updated = format!("{existing}\n---\n*Updated on {timestamp}*\n\n{content}\n");
            fs::write(&file_path, updated)?;
            "Appended to"
        } else {
            let body = format!("# {title}\n\n*Created on {timestamp}*\n\n{content}\n");
            fs::write(&file_path, body)?;
            "Created"
        };

        debug!("{action} note: {:?}", file_path);

        if open_in_editor {
            if let Some(ref editor) = self.editor_bin {
                let _ = Command::new(editor)
                    .arg(&file_path)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn();
            }
        }

        if !crate::ui::TaskNotifier::global().is_active() {
            crate::ui::send_desktop_notification(
                "󰏪",
                "Note Saved",
                &format!("'{title}' saved to {filename}"),
                3500,
                "normal",
            )
            .await;
        }

        Ok(format!("{action} note '{title}' in {filename}, sir."))
    }

    pub fn list_recent_notes(&self, limit: usize) -> Result<String> {
        if !self.notes_dir.is_dir() {
            return Ok("No notes directory found.".to_string());
        }

        let mut entries = Vec::new();
        if let Ok(read_dir) = fs::read_dir(&self.notes_dir) {
            for entry in read_dir.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("md") {
                    if let Ok(meta) = path.metadata() {
                        let mtime = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                        entries.push((path, mtime));
                    }
                }
            }
        }

        if entries.is_empty() {
            return Ok("You have no saved notes yet, sir.".to_string());
        }

        entries.sort_by_key(|a| std::cmp::Reverse(a.1));
        let selected = entries.into_iter().take(limit);

        let mut lines = vec!["Recent notes:".to_string()];
        for (path, mtime) in selected {
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Note");
            let dt: chrono::DateTime<Local> = mtime.into();
            lines.push(format!(
                "- {} (modified {})",
                stem,
                dt.format("%b %d, %H:%M")
            ));
        }

        Ok(lines.join("\n"))
    }
}

// -----------------------------------------------------------------------------
// Tool Implementations
// -----------------------------------------------------------------------------

pub struct CreateNoteTool {
    notes: Arc<NoteManager>,
}

impl CreateNoteTool {
    pub fn new(notes: Arc<NoteManager>) -> Self {
        Self { notes }
    }
}

#[async_trait]
impl Tool for CreateNoteTool {
    fn name(&self) -> &'static str {
        "create_note"
    }

    fn description(&self) -> &'static str {
        "Create or append to a markdown note in ~/Notes and optionally open it in Obsidian or editor."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "title": {
                    "type": "STRING",
                    "description": "Note title or file name."
                },
                "content": {
                    "type": "STRING",
                    "description": "The markdown body text to write."
                },
                "open_editor": {
                    "type": "BOOLEAN",
                    "description": "Whether to immediately launch Obsidian / text editor. Default is false."
                }
            },
            "required": ["title", "content"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let title = args["title"].as_str().ok_or_else(|| {
            JarvisError::ToolParameter("create_note".into(), "title string is required".into())
        })?;
        let content = args["content"].as_str().ok_or_else(|| {
            JarvisError::ToolParameter("create_note".into(), "content string is required".into())
        })?;
        let open_editor = args["open_editor"].as_bool().unwrap_or(false);

        self.notes.create_note(title, content, open_editor).await
    }
}

pub struct ListNotesTool {
    notes: Arc<NoteManager>,
}

impl ListNotesTool {
    pub fn new(notes: Arc<NoteManager>) -> Self {
        Self { notes }
    }
}

#[async_trait]
impl Tool for ListNotesTool {
    fn name(&self) -> &'static str {
        "list_notes"
    }

    fn description(&self) -> &'static str {
        "List recent notes saved in the Notes directory."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {}
        })
    }

    async fn execute(&self, _args: Value) -> Result<String> {
        self.notes.list_recent_notes(5)
    }
}
