use crate::core::error::{JarvisError, Result};
use crate::tools::Tool;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, info};

pub struct OpenFileInEditorTool {
    default_editor: String,
    terminal: String,
    project_dirs: Vec<PathBuf>,
}

impl OpenFileInEditorTool {
    pub fn new(default_editor: &str, terminal: &str, project_dirs: &[String]) -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let home_path = PathBuf::from(&home);

        let resolved_dirs: Vec<PathBuf> = if project_dirs.is_empty() {
            vec![
                home_path.join("Codes").join("personal"),
                home_path.join("Codes"),
                home_path.join("Projects"),
                home_path.clone(),
            ]
        } else {
            project_dirs
                .iter()
                .map(|p| {
                    if let Some(stripped) = p.strip_prefix("~/") {
                        home_path.join(stripped)
                    } else if p == "~" {
                        home_path.clone()
                    } else {
                        PathBuf::from(p)
                    }
                })
                .collect()
        };

        Self {
            default_editor: default_editor.to_string(),
            terminal: terminal.to_string(),
            project_dirs: resolved_dirs,
        }
    }

    /// Resolve a fuzzy or natural language project name to an existing directory
    pub fn find_project_dir(&self, project_name: &str) -> Option<PathBuf> {
        let cleaned = project_name
            .to_lowercase()
            .replace("project", "")
            .replace("repo", "")
            .replace("repository", "")
            .trim()
            .to_string();

        let candidates = vec![
            cleaned.replace(' ', "-"),
            cleaned.replace(' ', "_"),
            cleaned.replace(' ', ""),
            cleaned.clone(),
        ];

        // 1. Direct check in candidate root directories
        for base in &self.project_dirs {
            if !base.is_dir() {
                continue;
            }
            for c in &candidates {
                let p = base.join(c);
                if p.is_dir() {
                    return Some(p);
                }
            }
        }

        // 2. Scan immediate subdirectories of each project root
        for base in &self.project_dirs {
            if !base.is_dir() {
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(base) {
                for entry in entries.flatten() {
                    if let Ok(ft) = entry.file_type() {
                        if ft.is_dir() {
                            let dir_name = entry.file_name().to_string_lossy().to_lowercase();
                            let norm_dir = dir_name.replace(['-', '_', ' '], "");
                            for c in &candidates {
                                let norm_c = c.replace(['-', '_', ' '], "");
                                if norm_dir == norm_c
                                    || norm_dir.contains(&norm_c)
                                    || norm_c.contains(&norm_dir)
                                {
                                    return Some(entry.path());
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Search recursively for target file within project root
    pub fn find_file_in_dir(&self, root: &Path, file_path_str: &str) -> Option<PathBuf> {
        let direct = root.join(file_path_str);
        if direct.is_file() {
            return Some(direct);
        }

        // Extract filename to match against
        let target_name = Path::new(file_path_str)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(file_path_str)
            .to_lowercase();

        let mut stack = vec![root.to_path_buf()];

        while let Some(current) = stack.pop() {
            if let Ok(entries) = std::fs::read_dir(&current) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().to_string();

                    // Skip common ignored directories
                    if name == ".git"
                        || name == "node_modules"
                        || name == "target"
                        || name == "vendor"
                        || name == ".venv"
                        || name == "__pycache__"
                    {
                        continue;
                    }

                    if path.is_dir() {
                        stack.push(path);
                    } else if path.is_file() {
                        let fname = name.to_lowercase();
                        if fname == target_name {
                            return Some(path);
                        }
                        // Also check if ends with relative path
                        if path.to_string_lossy().ends_with(file_path_str) {
                            return Some(path);
                        }
                    }
                }
            }
        }

        None
    }

    /// Launch the editor in terminal or GUI
    pub async fn launch_editor(
        &self,
        project_dir: &Path,
        file_path: &Path,
        editor: &str,
    ) -> Result<String> {
        let editor_cmd = editor.to_lowercase();
        let file_str = file_path.to_string_lossy().to_string();
        let project_str = project_dir.to_string_lossy().to_string();

        info!(
            "Opening '{}' in editor '{}' (cwd: {})",
            file_str, editor_cmd, project_str
        );

        match editor_cmd.as_str() {
            "code" | "vscode" | "cursor" => {
                let mut cmd = Command::new(&editor_cmd);
                cmd.arg(&file_str);
                cmd.stdout(Stdio::null()).stderr(Stdio::null());
                cmd.spawn().map_err(|e| {
                    JarvisError::Other(format!("Failed to spawn {editor_cmd}: {e}"))
                })?;
            }
            _ => {
                let bin = match editor_cmd.as_str() {
                    "neovim" => "nvim",
                    "helix" => "hx",
                    other => other,
                };

                // Use hyprctl dispatch exec or direct terminal
                let terminal_bin = if which::which(&self.terminal).is_ok() {
                    self.terminal.as_str()
                } else if which::which("kitty").is_ok() {
                    "kitty"
                } else if which::which("foot").is_ok() {
                    "foot"
                } else {
                    "kitty"
                };

                // Try launching through hyprctl dispatch exec so Hyprland places it naturally
                let hyprctl_status = Command::new("hyprctl")
                    .args([
                        "dispatch",
                        "exec",
                        &format!(
                            "{} -d {} -e {} {}",
                            terminal_bin, project_str, bin, file_str
                        ),
                    ])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn();

                if hyprctl_status.is_err() {
                    // Fallback to direct terminal execution
                    let mut cmd = Command::new(terminal_bin);
                    cmd.arg("-d")
                        .arg(&project_str)
                        .arg("-e")
                        .arg(bin)
                        .arg(&file_str);
                    cmd.stdout(Stdio::null()).stderr(Stdio::null());
                    cmd.spawn().map_err(|e| {
                        JarvisError::Other(format!("Failed to launch terminal editor: {e}"))
                    })?;
                }
            }
        }

        Ok(format!(
            "Opened {} in {}.",
            file_path.file_name().unwrap_or_default().to_string_lossy(),
            editor_cmd
        ))
    }
}

#[async_trait]
impl Tool for OpenFileInEditorTool {
    fn name(&self) -> &'static str {
        "open_file_in_editor"
    }

    fn description(&self) -> &'static str {
        "Locate and open any source code file from a project or codebase (e.g. in ~/Codes, ~/Codes/personal, ~/Projects) in a code editor (Neovim, VSCode, Helix) in a new terminal window."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "project_name": {
                    "type": "STRING",
                    "description": "The name of the project, repository, or directory (e.g. 'tracky researcher tui', 'tracky-researcher-tui', 'jarvis')."
                },
                "file_path": {
                    "type": "STRING",
                    "description": "The file name or relative path to open (e.g. 'models.go', 'internal/models/models.go', 'main.rs')."
                },
                "editor": {
                    "type": "STRING",
                    "description": "The editor to use ('nvim', 'neovim', 'code', 'helix'). Defaults to 'nvim'."
                }
            },
            "required": ["project_name", "file_path"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let project_name = args["project_name"].as_str().ok_or_else(|| {
            JarvisError::ToolParameter(
                "open_file_in_editor".into(),
                "project_name is required".into(),
            )
        })?;
        let file_path = args["file_path"].as_str().ok_or_else(|| {
            JarvisError::ToolParameter("open_file_in_editor".into(), "file_path is required".into())
        })?;
        let editor = args["editor"].as_str().unwrap_or(&self.default_editor);

        debug!(
            "Resolving project '{}' for file '{}'...",
            project_name, file_path
        );

        let project_dir =
            self.find_project_dir(project_name)
                .ok_or_else(|| JarvisError::ToolExecution {
                    tool: "open_file_in_editor".to_string(),
                    message: format!(
                        "Could not find project directory for '{}' in configured code paths.",
                        project_name
                    ),
                })?;

        let resolved_file = self
            .find_file_in_dir(&project_dir, file_path)
            .ok_or_else(|| JarvisError::ToolExecution {
                tool: "open_file_in_editor".to_string(),
                message: format!(
                    "Could not find file '{}' inside project '{}' at {:?}.",
                    file_path, project_name, project_dir
                ),
            })?;

        self.launch_editor(&project_dir, &resolved_file, editor)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_project_dir_and_file() {
        let temp_dir = tempfile::tempdir().expect("Failed to create tempdir");
        let proj_dir = temp_dir.path().join("tracky-researcher-tui");
        let nested_dir = proj_dir.join("internal").join("models");
        std::fs::create_dir_all(&nested_dir).expect("Failed to create nested test dirs");
        let model_file = nested_dir.join("models.go");
        std::fs::write(&model_file, "package models").expect("Failed to write test file");

        let tool = OpenFileInEditorTool::new(
            "nvim",
            "kitty",
            &[temp_dir.path().to_string_lossy().to_string()],
        );
        let proj = tool.find_project_dir("tracky researcher tui");
        assert!(
            proj.is_some(),
            "Should find tracky-researcher-tui directory"
        );
        let proj_path = proj.unwrap();
        assert!(proj_path.exists());

        let file = tool.find_file_in_dir(&proj_path, "models.go");
        assert!(
            file.is_some(),
            "Should find models.go recursively inside project"
        );
        let found_path = file.unwrap();
        assert!(found_path.ends_with("internal/models/models.go"));
    }
}
