use crate::core::error::{JarvisError, Result};
use crate::tools::omarchy::OmarchyBridge;
use crate::tools::Tool;
use async_trait::async_trait;
use regex::Regex;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;
use tracing::{info, warn};

pub struct CodingCLIManager {
    cli_tool: String,
    kitty_bin: Option<PathBuf>,
    foot_bin: Option<PathBuf>,
    terminal_launcher: Option<PathBuf>,
    _omarchy: Arc<OmarchyBridge>,
}

impl CodingCLIManager {
    pub fn new(cli_tool: &str, omarchy: Arc<OmarchyBridge>) -> Self {
        let kitty_bin = which::which("kitty").ok();
        let foot_bin = which::which("foot").ok();
        let terminal_launcher = which::which("omarchy-launch-terminal").ok();

        Self {
            cli_tool: cli_tool.to_lowercase().trim().to_string(),
            kitty_bin,
            foot_bin,
            terminal_launcher,
            _omarchy: omarchy,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self.cli_tool.as_str() {
            "claude" => "Claude Code",
            "codex" => "Codex",
            _ => "Antigravity",
        }
    }

    pub fn get_binary_path(&self) -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        match self.cli_tool.as_str() {
            "claude" => which::which("claude")
                .unwrap_or_else(|_| PathBuf::from(&home).join(".local/bin/claude")),
            "codex" => which::which("codex")
                .unwrap_or_else(|_| PathBuf::from(&home).join(".local/share/mise/shims/codex")),
            _ => {
                which::which("agy").unwrap_or_else(|_| PathBuf::from(&home).join(".local/bin/agy"))
            }
        }
    }

    pub fn resolve_target_dir(&self, name: &str, requested_dir: Option<&str>) -> PathBuf {
        let home_str = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let home = PathBuf::from(&home_str);

        let re = Regex::new(r"[^\w.-]").unwrap();
        let clean_name = re.replace_all(name.trim(), "_").to_string();
        let final_name = if clean_name.is_empty() {
            "new_project".to_string()
        } else {
            clean_name
        };

        if let Some(raw) = requested_dir {
            let trimmed = raw.trim();
            if trimmed == "~" || trimmed.eq_ignore_ascii_case("home") {
                return home.join(&final_name);
            }
            let expanded = if let Some(stripped) = trimmed.strip_prefix("~/") {
                home.join(stripped)
            } else {
                PathBuf::from(trimmed)
            };
            if expanded.file_name().and_then(|s| s.to_str()) == Some(&final_name) {
                return expanded;
            }
            return expanded.join(&final_name);
        }

        let codes_personal = home.join("Codes").join("personal");
        if codes_personal.is_dir() {
            return codes_personal.join(&final_name);
        }

        let projects_dir = home.join("Projects");
        if projects_dir.is_dir() {
            return projects_dir.join(&final_name);
        }

        home.join(&final_name)
    }

    pub fn build_scaffold_prompt(
        &self,
        name: &str,
        description: &str,
        target_path: &Path,
        project_type: &str,
    ) -> String {
        let type_str = if project_type.trim().is_empty() {
            "project"
        } else {
            project_type.trim()
        };
        let desc_str = if description.trim().is_empty() {
            format!("Initialize a complete {type_str} named {name}.")
        } else {
            description.trim().to_string()
        };
        let path_str = target_path.to_string_lossy();

        format!(
            "Active Workspace Directory: {path_str}\n\
            You have full autonomous permissions. Please create and completely initialize the {type_str} named '{name}' directly inside the workspace directory `{path_str}`.\n\n\
            Specifications:\n{desc_str}\n\n\
            Requirements:\n\
            1. Generate all required directory structure, source files, and configuration files directly inside `{path_str}`.\n\
            2. Write complete, functional, production-ready code with proper error handling and documentation. Avoid placeholder code, stubs, or TODO comments.\n\
            3. Create a comprehensive README.md in `{path_str}` explaining what the project does, prerequisites, and how to run or test it.\n\
            4. Ensure all files are written directly to disk in `{path_str}` and formatted cleanly."
        )
    }

    pub async fn launch_popup_terminal(
        &self,
        cwd: &Path,
        prompt: &str,
        title: &str,
    ) -> Result<bool> {
        let _ = fs::create_dir_all(cwd);
        let bin = self.get_binary_path();
        let bin_str = bin.to_string_lossy();
        let cwd_str = cwd.to_string_lossy();

        let core_cmd = match self.cli_tool.as_str() {
            "claude" => format!("\"{bin_str}\" \"{prompt}\" --dangerously-skip-permissions"),
            "codex" => format!("\"{bin_str}\" -C \"{cwd_str}\" --dangerously-bypass-approvals-and-sandbox \"{prompt}\""),
            _ => format!("\"{bin_str}\" --add-dir \"{cwd_str}\" -i \"{prompt}\" --dangerously-skip-permissions"),
        };

        let interactive_sh = format!(
            "{core_cmd}; echo; echo '══════════════════════════════════════════════════════════════════'; \
            echo '{} session complete. Press Enter to close this window...'; read",
            self.display_name()
        );

        if let Some(ref kitty) = self.kitty_bin {
            let status = Command::new(kitty)
                .arg("--class")
                .arg("TUI.float")
                .arg("-T")
                .arg(title)
                .arg("-d")
                .arg(cwd)
                .arg("bash")
                .arg("-c")
                .arg(&interactive_sh)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
            return Ok(status.is_ok());
        }

        if let Some(ref foot) = self.foot_bin {
            let status = Command::new(foot)
                .arg("--app-id")
                .arg("TUI.float")
                .arg("-T")
                .arg(title)
                .arg("-D")
                .arg(cwd)
                .arg("bash")
                .arg("-c")
                .arg(&interactive_sh)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
            return Ok(status.is_ok());
        }

        if let Some(ref launcher) = self.terminal_launcher {
            let status = Command::new(launcher)
                .arg("bash")
                .arg("-c")
                .arg(&interactive_sh)
                .current_dir(cwd)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
            return Ok(status.is_ok());
        }

        warn!("No terminal emulator found for popup window");
        Ok(false)
    }

    pub async fn create_project(
        &self,
        name: &str,
        description: &str,
        target_dir: Option<&str>,
        project_type: &str,
        open_terminal: bool,
    ) -> Result<String> {
        let target_path = self.resolve_target_dir(name, target_dir);
        let _ = fs::create_dir_all(&target_path);
        let prompt = self.build_scaffold_prompt(name, description, &target_path, project_type);
        let tool_label = self.display_name();

        info!(
            "Creating {project_type} '{name}' at {:?} via {tool_label}",
            target_path
        );

        if open_terminal {
            let launched = self
                .launch_popup_terminal(
                    &target_path,
                    &prompt,
                    &format!("Jarvis - {tool_label} ({name})"),
                )
                .await?;

            if launched {
                return Ok(format!(
                    "Opened interactive {tool_label} popup window to scaffold '{name}' inside {}.",
                    target_path.to_string_lossy()
                ));
            }
        }

        // Headless autonomous background execution
        let bin = self.get_binary_path();
        let bin_str = bin.to_string_lossy();
        let path_str = target_path.to_string_lossy();
        let log_file = PathBuf::from("/tmp").join(format!("jarvis_{name}_scaffold.log"));

        let head_cmd = match self.cli_tool.as_str() {
            "claude" => format!("\"{bin_str}\" -p \"{prompt}\" --dangerously-skip-permissions > \"{}\" 2>&1", log_file.to_string_lossy()),
            "codex" => format!("\"{bin_str}\" exec -C \"{path_str}\" --dangerously-bypass-approvals-and-sandbox -a never \"{prompt}\" > \"{}\" 2>&1", log_file.to_string_lossy()),
            _ => format!("\"{bin_str}\" --add-dir \"{path_str}\" -p \"{prompt}\" --dangerously-skip-permissions > \"{}\" 2>&1", log_file.to_string_lossy()),
        };

        let _ = Command::new("setsid")
            .arg("/bin/bash")
            .arg("-c")
            .arg(&head_cmd)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        Ok(format!(
            "Delegated project creation for '{name}' to {tool_label} in background at {path_str}. Output logging to {}.",
            log_file.to_string_lossy()
        ))
    }

    pub async fn create_anything(
        &self,
        task_description: &str,
        target_dir: Option<&str>,
        open_terminal: bool,
    ) -> Result<String> {
        self.create_project(
            "autonomous_task",
            task_description,
            target_dir,
            "task",
            open_terminal,
        )
        .await
    }
}

// -----------------------------------------------------------------------------
// Tool Implementations
// -----------------------------------------------------------------------------

pub struct CreateProjectTool {
    manager: Arc<CodingCLIManager>,
}

impl CreateProjectTool {
    pub fn new(manager: Arc<CodingCLIManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for CreateProjectTool {
    fn name(&self) -> &'static str {
        "create_project"
    }

    fn description(&self) -> &'static str {
        "Create and scaffold a complete project using the configured CLI AI tool (claude, codex, agy) with full autonomous permissions or floating popup window."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "name": {
                    "type": "STRING",
                    "description": "The project folder name (e.g., 'weather-cli', 'portfolio-website')."
                },
                "description": {
                    "type": "STRING",
                    "description": "Detailed specifications, features, and requirements for the project."
                },
                "target_dir": {
                    "type": "STRING",
                    "description": "Optional parent directory where the project folder should be created."
                },
                "project_type": {
                    "type": "STRING",
                    "description": "Type of project (e.g., 'Rust CLI', 'Python API', 'React app', 'project')."
                },
                "open_terminal": {
                    "type": "BOOLEAN",
                    "description": "Whether to launch a floating popup terminal window to watch progress. Default is false."
                }
            },
            "required": ["name"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let name = args["name"].as_str().ok_or_else(|| {
            JarvisError::ToolParameter("create_project".into(), "name string is required".into())
        })?;
        let description = args["description"].as_str().unwrap_or("");
        let target_dir = args["target_dir"].as_str();
        let project_type = args["project_type"].as_str().unwrap_or("project");
        let open_terminal = args["open_terminal"].as_bool().unwrap_or(false);

        self.manager
            .create_project(name, description, target_dir, project_type, open_terminal)
            .await
    }
}

pub struct DelegateToAntigravityTool {
    manager: Arc<CodingCLIManager>,
}

impl DelegateToAntigravityTool {
    pub fn new(manager: Arc<CodingCLIManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for DelegateToAntigravityTool {
    fn name(&self) -> &'static str {
        "delegate_to_antigravity"
    }

    fn description(&self) -> &'static str {
        "Delegate any code generation, document creation, or complex task to the CLI AI tool with full autonomous permissions."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "task_description": {
                    "type": "STRING",
                    "description": "Full detailed description of the task or project to generate."
                },
                "target_dir": {
                    "type": "STRING",
                    "description": "Optional target workspace folder."
                },
                "open_terminal": {
                    "type": "BOOLEAN",
                    "description": "Whether to launch a floating popup window. Default is false."
                }
            },
            "required": ["task_description"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let task_description = args["task_description"].as_str().ok_or_else(|| {
            JarvisError::ToolParameter(
                "delegate_to_antigravity".into(),
                "task_description is required".into(),
            )
        })?;
        let target_dir = args["target_dir"].as_str();
        let open_terminal = args["open_terminal"].as_bool().unwrap_or(false);

        self.manager
            .create_anything(task_description, target_dir, open_terminal)
            .await
    }
}
