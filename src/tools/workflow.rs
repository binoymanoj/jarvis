use crate::core::error::{JarvisError, Result};
use crate::tools::hyprland::HyprlandController;
use crate::tools::Tool;
use async_trait::async_trait;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::sleep;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub workspace: i32,
    pub launch: String,
    #[serde(default = "default_delay")]
    pub delay: f32,
}

fn default_delay() -> f32 {
    0.25
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDef {
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default = "default_primary_workspace")]
    pub primary_workspace: i32,
    pub steps: Vec<WorkflowStep>,
}

fn default_primary_workspace() -> i32 {
    1
}

pub struct WorkflowManager {
    hyprland: Arc<HyprlandController>,
    config_dir: PathBuf,
    workflows_file: PathBuf,
}

impl WorkflowManager {
    pub fn new(hyprland: Arc<HyprlandController>) -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let config_dir = PathBuf::from(home).join(".config").join("jarvis");
        let workflows_file = config_dir.join("workflows.json");
        let manager = Self {
            hyprland,
            config_dir,
            workflows_file,
        };
        manager.ensure_config();
        manager
    }

    fn default_workflows() -> HashMap<String, WorkflowDef> {
        let mut map = HashMap::new();
        map.insert(
            "coding".to_string(),
            WorkflowDef {
                description: "Development environment: Editor and terminal on workspace 1, browser on workspace 2".into(),
                aliases: vec!["dev".into(), "code".into(), "development".into()],
                primary_workspace: 1,
                steps: vec![
                    WorkflowStep { workspace: 1, launch: "omarchy-launch-editor".into(), delay: 0.25 },
                    WorkflowStep { workspace: 1, launch: "omarchy-launch-terminal".into(), delay: 0.25 },
                    WorkflowStep { workspace: 2, launch: "omarchy-launch-browser".into(), delay: 0.25 },
                ],
            },
        );
        map.insert(
            "research".to_string(),
            WorkflowDef {
                description: "Research environment: Browser on workspace 1, Obsidian notes on workspace 2".into(),
                aliases: vec!["study".into(), "reading".into()],
                primary_workspace: 1,
                steps: vec![
                    WorkflowStep { workspace: 1, launch: "omarchy-launch-browser".into(), delay: 0.25 },
                    WorkflowStep { workspace: 2, launch: "obsidian".into(), delay: 0.25 },
                ],
            },
        );
        map.insert(
            "writing".to_string(),
            WorkflowDef {
                description: "Writing environment: Obsidian notes on workspace 1, browser on workspace 2".into(),
                aliases: vec!["notes".into(), "write".into(), "drafting".into()],
                primary_workspace: 1,
                steps: vec![
                    WorkflowStep { workspace: 1, launch: "obsidian".into(), delay: 0.25 },
                    WorkflowStep { workspace: 2, launch: "omarchy-launch-browser".into(), delay: 0.25 },
                ],
            },
        );
        map.insert(
            "communication".to_string(),
            WorkflowDef {
                description: "Communication setup: Thunderbird email on workspace 1, web messaging on workspace 2".into(),
                aliases: vec!["social".into(), "mail".into(), "messaging".into()],
                primary_workspace: 1,
                steps: vec![
                    WorkflowStep { workspace: 1, launch: "thunderbird".into(), delay: 0.3 },
                    WorkflowStep { workspace: 2, launch: "omarchy-launch-webapp https://web.whatsapp.com".into(), delay: 0.25 },
                ],
            },
        );
        map.insert(
            "media".to_string(),
            WorkflowDef {
                description: "Entertainment setup: YouTube browser on workspace 1, Spotify/cliamp on workspace 2".into(),
                aliases: vec!["chill".into(), "music".into(), "relax".into()],
                primary_workspace: 1,
                steps: vec![
                    WorkflowStep { workspace: 1, launch: "omarchy-launch-browser https://www.youtube.com".into(), delay: 0.25 },
                    WorkflowStep { workspace: 2, launch: "cliamp".into(), delay: 0.25 },
                ],
            },
        );
        map
    }

    fn ensure_config(&self) {
        if !self.workflows_file.is_file() {
            let _ = fs::create_dir_all(&self.config_dir);
            let defaults = Self::default_workflows();
            if let Ok(json) = serde_json::to_string_pretty(&defaults) {
                let _ = fs::write(&self.workflows_file, json);
            }
        }
    }

    pub fn get_workflows(&self) -> HashMap<String, WorkflowDef> {
        self.ensure_config();
        if let Ok(content) = fs::read_to_string(&self.workflows_file) {
            if let Ok(map) = serde_json::from_str::<HashMap<String, WorkflowDef>>(&content) {
                return map;
            }
        }
        Self::default_workflows()
    }

    fn save_to_disk(&self, workflows: &HashMap<String, WorkflowDef>) -> Result<()> {
        let _ = fs::create_dir_all(&self.config_dir);
        let json = serde_json::to_string_pretty(workflows)?;
        fs::write(&self.workflows_file, json)?;
        Ok(())
    }

    pub fn find_workflow(&self, name: &str) -> Option<(String, WorkflowDef)> {
        let clean = name.trim().to_lowercase();
        let workflows = self.get_workflows();
        for (k, wf) in workflows {
            if k.to_lowercase() == clean {
                return Some((k, wf));
            }
            for alias in &wf.aliases {
                if alias.to_lowercase() == clean {
                    return Some((k, wf));
                }
            }
        }
        None
    }

    pub fn list_workflows(&self) -> Result<String> {
        let workflows = self.get_workflows();
        let mut keys: Vec<String> = workflows.keys().cloned().collect();
        keys.sort();

        let mut lines = vec!["Available workflows:".to_string()];
        for k in keys {
            if let Some(wf) = workflows.get(&k) {
                let aliases_str = if !wf.aliases.is_empty() {
                    format!(" (aliases: {})", wf.aliases.join(", "))
                } else {
                    String::new()
                };

                let mut ws_list: Vec<i32> = wf.steps.iter().map(|s| s.workspace).collect();
                ws_list.sort();
                ws_list.dedup();
                let ws_str = if !ws_list.is_empty() {
                    format!(" [Workspaces: {}]", ws_list.iter().map(|w| w.to_string()).collect::<Vec<_>>().join(", "))
                } else {
                    String::new()
                };

                lines.push(format!("• **{k}**{aliases_str}{ws_str}: {}", wf.description));
            }
        }
        Ok(lines.join("\n"))
    }

    pub fn get_workflow_details(&self, name: &str) -> Result<String> {
        match self.find_workflow(name) {
            Some((key, wf)) => {
                let mut lines = vec![
                    format!("### Workflow: {key}"),
                    format!("**Description**: {}", wf.description),
                    format!("**Aliases**: {}", if wf.aliases.is_empty() { "None".to_string() } else { wf.aliases.join(", ") }),
                    format!("**Primary Workspace**: {}", wf.primary_workspace),
                    "**Launch Steps**:".to_string(),
                ];
                for (idx, step) in wf.steps.iter().enumerate() {
                    lines.push(format!(
                        "  {}. Workspace {}: `{}` (delay: {}s)",
                        idx + 1,
                        step.workspace,
                        step.launch,
                        step.delay
                    ));
                }
                Ok(lines.join("\n"))
            }
            None => Ok(format!("Workflow '{name}' not found.")),
        }
    }

    pub async fn launch_workflow(&self, name: &str) -> Result<String> {
        let (key, wf) = match self.find_workflow(name) {
            Some(res) => res,
            None => return Ok(format!("Workflow '{name}' not found. Use list_workflows to see options.")),
        };

        info!("Launching workflow '{key}' with {} steps", wf.steps.len());

        for step in &wf.steps {
            let _ = self.hyprland.change_workspace(step.workspace).await;
            sleep(Duration::from_millis(100)).await;

            let _ = Command::new("/bin/bash")
                .arg("-c")
                .arg(&step.launch)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();

            let delay_ms = (step.delay * 1000.0) as u64;
            sleep(Duration::from_millis(delay_ms)).await;
        }

        // Return focus to primary workspace
        sleep(Duration::from_millis(150)).await;
        let _ = self.hyprland.change_workspace(wf.primary_workspace).await;

        Ok(format!("Workflow '{key}' launched successfully on primary workspace {}.", wf.primary_workspace))
    }

    pub fn save_workflow(
        &self,
        name: &str,
        description: &str,
        steps: Vec<WorkflowStep>,
        aliases: Option<Vec<String>>,
        primary_workspace: i32,
    ) -> Result<String> {
        let clean_slug = slugify(name);
        if clean_slug.is_empty() {
            return Ok("Invalid workflow name provided.".to_string());
        }
        if steps.is_empty() {
            return Ok("A workflow requires at least one launch step.".to_string());
        }

        let mut clean_aliases = Vec::new();
        if let Some(list) = aliases {
            for a in list {
                let trimmed = a.trim().to_lowercase();
                if !trimmed.is_empty() && trimmed != clean_slug && !clean_aliases.contains(&trimmed) {
                    clean_aliases.push(trimmed);
                }
            }
        }

        let mut workflows = self.get_workflows();
        let step_count = steps.len();
        workflows.insert(
            clean_slug.clone(),
            WorkflowDef {
                description: if description.trim().is_empty() {
                    format!("Custom workflow '{clean_slug}'")
                } else {
                    description.trim().to_string()
                },
                aliases: clean_aliases,
                primary_workspace,
                steps,
            },
        );

        self.save_to_disk(&workflows)?;
        Ok(format!("Successfully saved custom workflow '{clean_slug}' with {step_count} launch steps."))
    }

    pub fn delete_workflow(&self, name: &str) -> Result<String> {
        let clean = name.trim().to_lowercase();
        let mut workflows = self.get_workflows();

        let mut target_key = None;
        for (k, wf) in &workflows {
            if k.to_lowercase() == clean {
                target_key = Some(k.clone());
                break;
            }
            for alias in &wf.aliases {
                if alias.to_lowercase() == clean {
                    target_key = Some(k.clone());
                    break;
                }
            }
        }

        match target_key {
            Some(k) => {
                workflows.remove(&k);
                self.save_to_disk(&workflows)?;
                Ok(format!("Workflow '{k}' has been deleted."))
            }
            None => Ok(format!("Workflow '{name}' not found.")),
        }
    }

    pub async fn capture_current_setup(
        &self,
        name: &str,
        description: &str,
        aliases: Option<Vec<String>>,
    ) -> Result<String> {
        let clean_slug = slugify(name);
        if clean_slug.is_empty() {
            return Ok("Please provide a valid name for the workflow to capture.".to_string());
        }

        let clients = self.hyprland.get_clients().await?;
        let client_list = match clients.as_array() {
            Some(arr) if !arr.is_empty() => arr,
            _ => return Ok("No active application windows found to capture.".to_string()),
        };

        let mut seen_pairs = std::collections::HashSet::new();
        let mut steps = Vec::new();

        for c in client_list {
            let ws_id = c["workspace"]["id"].as_i64().unwrap_or(1) as i32;
            if ws_id <= 0 {
                continue; // Skip scratchpads
            }

            if let Some(launch_cmd) = map_client_to_launch_cmd(c) {
                let pair = (ws_id, launch_cmd.clone());
                if !seen_pairs.contains(&pair) {
                    seen_pairs.insert(pair);
                    steps.push(WorkflowStep {
                        workspace: ws_id,
                        launch: launch_cmd,
                        delay: 0.25,
                    });
                }
            }
        }

        if steps.is_empty() {
            return Ok("Could not resolve executable commands for the currently open windows.".to_string());
        }

        steps.sort_by_key(|s| s.workspace);
        let primary_ws = steps.first().map(|s| s.workspace).unwrap_or(1);
        self.save_workflow(&clean_slug, description, steps, aliases, primary_ws)
    }
}

fn slugify(name: &str) -> String {
    let re = Regex::new(r"[^a-zA-Z0-9_\-]").unwrap();
    let replaced = re.replace_all(name, "_");
    let re_multi = Regex::new(r"_+").unwrap();
    re_multi.replace_all(&replaced, "_").trim_matches('_').to_lowercase()
}

fn map_client_to_launch_cmd(client: &Value) -> Option<String> {
    let initial_cls = client["initialClass"].as_str().unwrap_or("");
    let cls = client["class"].as_str().unwrap_or("");
    let target_cls = if !initial_cls.is_empty() { initial_cls } else { cls }.trim();
    let lower = target_cls.to_lowercase();

    // 1. Web Browsers
    if ["helium", "chromium", "google-chrome", "firefox", "brave"].iter().any(|b| lower.contains(b)) {
        return Some("omarchy-launch-browser".to_string());
    }

    // 2. Terminals
    if ["foot", "kitty", "alacritty", "wezterm", "gnome-terminal"].iter().any(|t| lower.contains(t)) {
        return Some("omarchy-launch-terminal".to_string());
    }

    // 3. Editors & IDEs
    if lower.contains("code") || lower.contains("vscodium") {
        return Some("code".to_string());
    }
    if lower.contains("nvim") || lower.contains("neovim") {
        return Some("omarchy-launch-editor".to_string());
    }

    // 4. File Managers
    if lower.contains("nautilus") {
        return Some("nautilus".to_string());
    }
    if lower.contains("thunar") || lower.contains("dolphin") {
        return Some(lower);
    }

    // 5. Media Players
    if lower.contains("mpv") {
        return Some("mpv".to_string());
    }
    if lower.contains("spotify") {
        return Some("spotify".to_string());
    }
    if lower.contains("vlc") {
        return Some("vlc".to_string());
    }

    // 6. Notes & Knowledge Base
    if lower.contains("obsidian") {
        return Some("obsidian".to_string());
    }

    // 7. Communications
    if lower.contains("thunderbird") {
        return Some("thunderbird".to_string());
    }
    if lower.contains("slack") {
        return Some("slack".to_string());
    }
    if lower.contains("discord") {
        return Some("discord".to_string());
    }
    if lower.contains("telegram") {
        return Some("telegram-desktop".to_string());
    }

    // 8. Webapps
    if lower.starts_with("chrome-") && lower.contains("__") {
        let parts: Vec<&str> = lower.split('-').collect();
        if parts.len() > 1 {
            let domain = parts[1].split("__").next().unwrap_or("");
            if !domain.is_empty() {
                return Some(format!("omarchy-launch-webapp https://{domain}"));
            }
        }
        return Some("omarchy-launch-browser".to_string());
    }

    // 9. Generic binary check
    if !target_cls.is_empty() {
        let short_bin = target_cls.split('.').next_back().unwrap_or(target_cls).to_lowercase();
        if which::which(&short_bin).is_ok() {
            return Some(short_bin);
        }
        if which::which(&lower).is_ok() {
            return Some(lower);
        }
        return Some(short_bin);
    }

    None
}

// -----------------------------------------------------------------------------
// Tool Implementations
// -----------------------------------------------------------------------------

pub struct LaunchWorkflowTool {
    workflow: Arc<WorkflowManager>,
}

impl LaunchWorkflowTool {
    pub fn new(workflow: Arc<WorkflowManager>) -> Self {
        Self { workflow }
    }
}

#[async_trait]
impl Tool for LaunchWorkflowTool {
    fn name(&self) -> &'static str {
        "launch_workflow"
    }

    fn description(&self) -> &'static str {
        "Launch a multi-workspace workflow setup (e.g. 'coding'/'dev', 'research', 'writing', 'communication', 'media')."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "name": {
                    "type": "STRING",
                    "description": "The name or alias of the workflow to trigger."
                }
            },
            "required": ["name"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let name = args["name"]
            .as_str()
            .ok_or_else(|| JarvisError::ToolParameter("launch_workflow".into(), "name string is required".into()))?;

        self.workflow.launch_workflow(name).await
    }
}

pub struct ListWorkflowsTool {
    workflow: Arc<WorkflowManager>,
}

impl ListWorkflowsTool {
    pub fn new(workflow: Arc<WorkflowManager>) -> Self {
        Self { workflow }
    }
}

#[async_trait]
impl Tool for ListWorkflowsTool {
    fn name(&self) -> &'static str {
        "list_workflows"
    }

    fn description(&self) -> &'static str {
        "List all configured workflow presets and their descriptions."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {}
        })
    }

    async fn execute(&self, _args: Value) -> Result<String> {
        self.workflow.list_workflows()
    }
}

pub struct CaptureCurrentWorkflowTool {
    workflow: Arc<WorkflowManager>,
}

impl CaptureCurrentWorkflowTool {
    pub fn new(workflow: Arc<WorkflowManager>) -> Self {
        Self { workflow }
    }
}

#[async_trait]
impl Tool for CaptureCurrentWorkflowTool {
    fn name(&self) -> &'static str {
        "capture_current_workflow"
    }

    fn description(&self) -> &'static str {
        "Capture currently open application windows across Hyprland workspaces into a new saved workflow preset."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "name": {
                    "type": "STRING",
                    "description": "Identifier name for the captured workflow."
                },
                "description": {
                    "type": "STRING",
                    "description": "Optional human-readable description of what this workflow is for."
                },
                "aliases": {
                    "type": "STRING",
                    "description": "Optional comma-separated aliases for the workflow."
                }
            },
            "required": ["name"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let name = args["name"]
            .as_str()
            .ok_or_else(|| JarvisError::ToolParameter("capture_current_workflow".into(), "name string is required".into()))?;
        let description = args["description"].as_str().unwrap_or("");
        let aliases = args["aliases"].as_str().map(|a| {
            a.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        });

        self.workflow.capture_current_setup(name, description, aliases).await
    }
}

pub struct SaveCustomWorkflowTool {
    workflow: Arc<WorkflowManager>,
}

impl SaveCustomWorkflowTool {
    pub fn new(workflow: Arc<WorkflowManager>) -> Self {
        Self { workflow }
    }
}

#[async_trait]
impl Tool for SaveCustomWorkflowTool {
    fn name(&self) -> &'static str {
        "save_custom_workflow"
    }

    fn description(&self) -> &'static str {
        "Create or update a custom workflow preset. steps_json should be a JSON array of objects with 'workspace' (int) and 'launch' (command string)."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "name": {
                    "type": "STRING",
                    "description": "Name of the workflow."
                },
                "description": {
                    "type": "STRING",
                    "description": "Description of the workflow."
                },
                "steps_json": {
                    "type": "STRING",
                    "description": "JSON array string containing objects with 'workspace' (int) and 'launch' (cmd string)."
                },
                "aliases": {
                    "type": "STRING",
                    "description": "Optional comma-separated aliases."
                },
                "primary_workspace": {
                    "type": "INTEGER",
                    "description": "Primary workspace number (default: 1)."
                }
            },
            "required": ["name", "description", "steps_json"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let name = args["name"]
            .as_str()
            .ok_or_else(|| JarvisError::ToolParameter("save_custom_workflow".into(), "name is required".into()))?;
        let description = args["description"].as_str().unwrap_or("");
        let primary_workspace = args["primary_workspace"].as_i64().unwrap_or(1) as i32;

        let steps: Vec<WorkflowStep> = if let Some(arr) = args["steps_json"].as_array() {
            serde_json::from_value(Value::Array(arr.clone()))
                .map_err(|e| JarvisError::ToolParameter("save_custom_workflow".into(), format!("Invalid steps format: {e}")))?
        } else if let Some(str_val) = args["steps_json"].as_str() {
            serde_json::from_str(str_val)
                .map_err(|e| JarvisError::ToolParameter("save_custom_workflow".into(), format!("Invalid steps JSON: {e}")))?
        } else {
            return Err(JarvisError::ToolParameter("save_custom_workflow".into(), "steps_json must be a JSON array or string".into()));
        };

        let aliases = args["aliases"].as_str().map(|a| {
            a.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        });

        self.workflow.save_workflow(name, description, steps, aliases, primary_workspace)
    }
}

pub struct DeleteCustomWorkflowTool {
    workflow: Arc<WorkflowManager>,
}

impl DeleteCustomWorkflowTool {
    pub fn new(workflow: Arc<WorkflowManager>) -> Self {
        Self { workflow }
    }
}

#[async_trait]
impl Tool for DeleteCustomWorkflowTool {
    fn name(&self) -> &'static str {
        "delete_custom_workflow"
    }

    fn description(&self) -> &'static str {
        "Delete a custom workflow preset by name or alias."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "name": {
                    "type": "STRING",
                    "description": "Name or alias of the workflow to delete."
                }
            },
            "required": ["name"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let name = args["name"]
            .as_str()
            .ok_or_else(|| JarvisError::ToolParameter("delete_custom_workflow".into(), "name string is required".into()))?;

        self.workflow.delete_workflow(name)
    }
}

pub struct GetWorkflowDetailsTool {
    workflow: Arc<WorkflowManager>,
}

impl GetWorkflowDetailsTool {
    pub fn new(workflow: Arc<WorkflowManager>) -> Self {
        Self { workflow }
    }
}

#[async_trait]
impl Tool for GetWorkflowDetailsTool {
    fn name(&self) -> &'static str {
        "get_workflow_details"
    }

    fn description(&self) -> &'static str {
        "Get the detailed launch steps, workspaces, and aliases of a specific workflow."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "name": {
                    "type": "STRING",
                    "description": "Name or alias of the workflow."
                }
            },
            "required": ["name"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let name = args["name"]
            .as_str()
            .ok_or_else(|| JarvisError::ToolParameter("get_workflow_details".into(), "name string is required".into()))?;

        self.workflow.get_workflow_details(name)
    }
}
