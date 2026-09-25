pub mod calendar;
pub mod clipboard;
pub mod coding_cli;
pub mod editor;
pub mod email;
pub mod hyprland;
pub mod localsend;
pub mod media;
pub mod meeting;
pub mod notes;
pub mod omarchy;
pub mod power;
pub mod research;
pub mod screen;
pub mod shell;
pub mod virtual_input;
pub mod web;
pub mod workflow;

use crate::core::config::Settings;
use crate::core::error::{JarvisError, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[async_trait]
pub trait Tool: Send + Sync {
    /// The unique identifier of the tool used by Gemini function calling
    fn name(&self) -> &'static str;

    /// Human- and model-readable description of what this tool accomplishes
    fn description(&self) -> &'static str;

    /// Gemini-compliant JSON schema for tool arguments
    fn parameters_schema(&self) -> Value;

    /// Execute the tool given the input arguments (JSON map).
    async fn execute(&self, args: Value) -> Result<String>;
}

// -----------------------------------------------------------------------------
// Dismiss Session Tool
// -----------------------------------------------------------------------------

pub struct DismissSessionTool {
    session_ended: Arc<AtomicBool>,
}

impl DismissSessionTool {
    pub fn new(session_ended: Arc<AtomicBool>) -> Self {
        Self { session_ended }
    }
}

#[async_trait]
impl Tool for DismissSessionTool {
    fn name(&self) -> &'static str {
        "dismiss_session"
    }

    fn description(&self) -> &'static str {
        "Dismiss Jarvis and conclude the ongoing conversation session when the user is done, says that's it, or goodbye."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "farewell": {
                    "type": "STRING",
                    "description": "Polite farewell phrase to speak before exiting. Default: 'Very well, sir. Have a wonderful day.'"
                }
            }
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        self.session_ended.store(true, Ordering::SeqCst);
        let farewell = args["farewell"]
            .as_str()
            .unwrap_or("Very well, sir. Have a wonderful day.");
        Ok(farewell.to_string())
    }
}

// -----------------------------------------------------------------------------
// Tool Registry
// -----------------------------------------------------------------------------

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        self.tools.insert(tool.name().to_string(), Arc::new(tool));
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    pub fn all(&self) -> Vec<Arc<dyn Tool>> {
        self.tools.values().cloned().collect()
    }

    pub fn count(&self) -> usize {
        self.tools.len()
    }

    /// Generates Gemini 3.5 FunctionDeclarations payload
    pub fn gemini_function_declarations(&self) -> Value {
        let mut declarations = Vec::new();
        let mut sorted_keys: Vec<&String> = self.tools.keys().collect();
        sorted_keys.sort();

        for key in sorted_keys {
            if let Some(tool) = self.tools.get(key) {
                declarations.push(json!({
                    "name": tool.name(),
                    "description": tool.description(),
                    "parameters": tool.parameters_schema(),
                }));
            }
        }

        json!([{
            "functionDeclarations": declarations
        }])
    }

    pub async fn execute_tool(&self, name: &str, args: Value) -> Result<String> {
        let tool = self
            .get(name)
            .ok_or_else(|| JarvisError::ToolNotFound(name.to_string()))?;
        tool.execute(args).await
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Builds the default complete tool registry containing all 48+ desktop tools
pub fn build_tool_registry(
    settings: &Settings,
    session_ended_flag: Arc<AtomicBool>,
) -> ToolRegistry {
    let mut reg = ToolRegistry::new();

    // 1. Core controllers
    let hyprland = Arc::new(hyprland::HyprlandController::new());
    let omarchy = Arc::new(omarchy::OmarchyBridge::new());
    let virtual_input = Arc::new(virtual_input::VirtualInputManager::new());
    let shell = Arc::new(shell::ShellExecutor::default());
    let media = Arc::new(media::MediaManager::with_settings(
        settings.resolved_media_dirs(),
        &settings.media_player,
    ));
    let clipboard = Arc::new(clipboard::ClipboardManager::new());
    let power = Arc::new(power::SystemPowerManager::new(
        omarchy.clone(),
        settings.clone(),
    ));
    let workflow = Arc::new(workflow::WorkflowManager::with_settings(
        hyprland.clone(),
        Some(settings.clone()),
    ));
    let web = Arc::new(web::WebNavigator::new());
    let screen = Arc::new(screen::ScreenPerception::new(hyprland.clone()));
    let calendar = Arc::new(calendar::CalendarManager::new());
    let notes = Arc::new(notes::NoteManager::new());
    let email = Arc::new(email::EmailManager::new());
    let coding_cli = Arc::new(coding_cli::CodingCLIManager::new(
        &settings.cli_ai_tool,
        omarchy.clone(),
    ));

    // 2. Register Hyprland Tools (5)
    reg.register(hyprland::SwitchWorkspaceTool::new(hyprland.clone()));
    reg.register(hyprland::FocusApplicationTool::new(hyprland.clone()));
    reg.register(hyprland::CloseActiveWindowTool::new(hyprland.clone()));
    reg.register(hyprland::ToggleLayoutSplitTool::new(hyprland.clone()));
    reg.register(hyprland::ToggleFullscreenTool::new(hyprland.clone()));

    // 3. Register Omarchy Tools (6)
    reg.register(omarchy::AdjustVolumeTool::new(omarchy.clone()));
    reg.register(omarchy::SetBrightnessTool::new(omarchy.clone()));
    reg.register(omarchy::SetThemeTool::new(omarchy.clone()));
    reg.register(omarchy::GetBatteryTool::new(omarchy.clone()));
    reg.register(omarchy::LaunchApplicationTool::new(omarchy.clone()));
    reg.register(omarchy::NotifyTool::new(omarchy.clone()));

    // 4. Register Virtual Input Tools (4)
    reg.register(virtual_input::TypeTextTool::new(virtual_input.clone()));
    reg.register(virtual_input::PressKeyTool::new(virtual_input.clone()));
    reg.register(virtual_input::SendShortcutTool::new(virtual_input.clone()));
    reg.register(virtual_input::ScrollTool::new(virtual_input));

    // 5. Register Shell Executor Tool (1)
    reg.register(shell::ExecuteCommandTool::new(shell));

    // 6. Register Media Player & Video Tools (7)
    reg.register(media::MediaPlayPauseTool::new(media.clone()));
    reg.register(media::MediaNextTool::new(media.clone()));
    reg.register(media::MediaPreviousTool::new(media.clone()));
    reg.register(media::MediaStopTool::new(media.clone()));
    reg.register(media::GetNowPlayingTool::new(media.clone()));
    reg.register(media::PlayMediaTool::new(media.clone()));
    reg.register(media::ResumeMediaTool::new(media));

    // 7. Register Clipboard Tools (2)
    reg.register(clipboard::GetClipboardTool::new(clipboard.clone()));
    reg.register(clipboard::SetClipboardTool::new(clipboard.clone()));

    // 8. Register Power & Diagnostics Tools (7)
    reg.register(power::LockScreenTool::new(power.clone()));
    reg.register(power::LogoutSystemTool::new(power.clone()));
    reg.register(power::RebootSystemTool::new(power.clone()));
    reg.register(power::ShutdownSystemTool::new(power.clone()));
    reg.register(power::GetSystemStatsTool::new(power.clone()));
    reg.register(power::ToggleBluetoothTool::new(power.clone()));
    reg.register(power::NetworkSpeedtestTool::new(power));

    // 9. Register Workflow Orchestrator Tools (6)
    reg.register(workflow::LaunchWorkflowTool::new(workflow.clone()));
    reg.register(workflow::ListWorkflowsTool::new(workflow.clone()));
    reg.register(workflow::CaptureCurrentWorkflowTool::new(workflow.clone()));
    reg.register(workflow::SaveCustomWorkflowTool::new(workflow.clone()));
    reg.register(workflow::DeleteCustomWorkflowTool::new(workflow.clone()));
    reg.register(workflow::GetWorkflowDetailsTool::new(workflow));

    // 10. Register Web Navigator Tools (3)
    reg.register(web::OpenUrlTool::new(web.clone()));
    reg.register(web::SearchWebTool::new(web.clone()));
    reg.register(web::OpenYoutubeTool::new(web.clone()));

    // 11. Register Screen Perception Tool (1)
    reg.register(screen::InspectScreenTool::new(screen));

    // 12. Register Calendar & Reminder Tools (4)
    reg.register(calendar::ScheduleEventTool::new(calendar.clone()));
    reg.register(calendar::SetReminderTool::new(calendar.clone()));
    reg.register(calendar::ListRemindersTool::new(calendar.clone()));
    reg.register(calendar::ClearRemindersTool::new(calendar));

    // 13. Register Email Tool (1)
    reg.register(email::DraftEmailTool::new(email));

    // 14. Register Note Tools (2)
    reg.register(notes::CreateNoteTool::new(notes.clone()));
    reg.register(notes::ListNotesTool::new(notes));

    // 15. Register Autonomous Coding CLI Tools (2)
    reg.register(coding_cli::CreateProjectTool::new(coding_cli.clone()));
    reg.register(coding_cli::DelegateToAntigravityTool::new(coding_cli));

    // 16. Register Editor Tools (1)
    reg.register(editor::OpenFileInEditorTool::new(
        &settings.editor,
        &settings.terminal,
        &settings.project_dirs,
    ));

    // 17. Register LocalSend Tools (1)
    let localsend = Arc::new(localsend::LocalSendManager::new(
        hyprland.clone(),
        settings.resolved_media_dirs(),
    ));
    reg.register(localsend::LocalSendShareTool::new(localsend));

    // 18. Register Research & Neovim Markdown Tools (1)
    let research = Arc::new(research::ResearchManager::new());
    reg.register(research::DisplayResearchInNeovimTool::new(research));

    // 19. Register Quick Meeting Tool (1)
    let meeting = Arc::new(meeting::MeetingManager::new(
        hyprland,
        clipboard,
        web,
    ));
    reg.register(meeting::CreateQuickMeetingTool::new(meeting));

    // 20. Dismiss Session Tool (1)
    reg.register(DismissSessionTool::new(session_ended_flag));

    reg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_tool_registry_count_and_schemas() {
        let settings = Settings::default();
        let flag = Arc::new(AtomicBool::new(false));
        let registry = build_tool_registry(&settings, flag);

        // All 56 native tools must be successfully registered!
        assert_eq!(registry.count(), 56);

        // Ensure key tools are retrievable
        assert!(registry.get("switch_workspace").is_some());
        assert!(registry.get("focus_application").is_some());
        assert!(registry.get("execute_command").is_some());
        assert!(registry.get("create_quick_meeting").is_some());
        assert!(registry.get("media_play_pause").is_some());
        assert!(registry.get("play_media").is_some());
        assert!(registry.get("resume_media").is_some());
        assert!(registry.get("localsend_share").is_some());
        assert!(registry.get("display_research_in_neovim").is_some());
        assert!(registry.get("create_quick_meeting").is_some());
        assert!(registry.get("launch_workflow").is_some());
        assert!(registry.get("create_project").is_some());
        assert!(registry.get("open_file_in_editor").is_some());
        assert!(registry.get("dismiss_session").is_some());

        // Verify Gemini function declarations format
        let decls = registry.gemini_function_declarations();
        assert!(decls.is_array());
        let list = decls[0]["functionDeclarations"].as_array().unwrap();
        assert_eq!(list.len(), 56);
    }
}
