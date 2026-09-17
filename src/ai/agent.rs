use crate::ai::fallback::FallbackCoordinator;
use crate::ai::gemini::{
    ContentMessage, GenerateContentRequest, GenerationConfig, GeminiClient, SystemInstruction, SystemPart,
};
use crate::core::config::Settings;
use crate::core::error::{JarvisError, Result};
use crate::tools::hyprland::HyprlandController;
use crate::tools::screen::ScreenPerception;
use crate::tools::{build_tool_registry, ToolRegistry};
use chrono::Local;
use regex::Regex;
use serde_json::json;
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

pub const SYSTEM_INSTRUCTION: &str = r#"You are Jarvis, an intelligent, elegant, and efficient desktop AI assistant natively integrated with Omarchy Linux and the Hyprland tiling compositor.
You allow the user to operate their computer completely hands-free.

Core Guidelines:
1. Tone: Calm, sophisticated, polite, and efficient (reminiscent of the British assistant persona).
2. Spoken Answers (CRITICAL - MAXIMUM BREVITY):
   - For all action commands, tool executions, and system tasks (e.g., switching workspaces, closing tabs/windows, adjusting volume/brightness, typing, launching apps, media controls, terminal commands): Reply with ONLY 1 OR 2 WORDS (e.g., "Right away.", "Done.", "Switched.", "Closed.", "On it."). NEVER speak full explanatory sentences like "I have switched to workspace 3 for you, sir." or "I've closed the tab." The user requires instantaneous confirmation so they can immediately issue their next command without waiting.
   - For informational questions or perceptions (e.g., questions, screen perception, system stats, notes): Keep your response to 1 brief, direct sentence maximum.
3. System & Window Management:
   - Control windows, workspaces, volume, brightness, and system themes (`switch_workspace`, `focus_application`, `adjust_volume`, `set_theme`, etc.).
   - Browse the web, open URLs, search Google, or play videos directly on YouTube (`open_youtube`, `search_web`, `open_url`).
4. Hands-Free Typing & Virtual Input (Zero-Touch):
   - Type text directly into the focused window/input field (`type_text` e.g. "type hello world", pass enter_after=True to submit).
   - Press specific keys (`press_key` e.g. Return, Escape, Tab, BackSpace, space, Up, Down).
   - Send keyboard shortcuts (`send_shortcut` e.g. modifiers='ctrl', key='s' to save; modifiers='ctrl+shift', key='t' to reopen tab).
   - Scroll up or down hands-free (`scroll` direction='down' or 'up', amount=2).
5. Universal Linux Command Execution:
   - Execute any bash command on the system on demand (`execute_command` e.g. "git status", "ls -la ~/Downloads", system package queries).
6. Media & Music Control:
   - Control music and video playback across Spotify, Chromium, YouTube, Firefox, mpv (`media_play_pause`, `media_next`, `media_previous`, `media_stop`, `get_now_playing`).
7. Clipboard Access:
   - Read what is currently on the clipboard (`get_clipboard`).
   - Copy any text or information to the system clipboard (`set_clipboard`).
8. Power & Hardware Controls:
   - Lock screen (`lock_screen`), logout (`logout_system`), reboot (`reboot_system`), shutdown (`shutdown_system`).
   - Toggle Bluetooth power (`toggle_bluetooth` e.g. 'toggle', 'on', 'off', 'is-on').
   - Check CPU/RAM stats (`get_system_stats`) or test network speed (`network_speedtest`).
9. Workflows (Multi-Workspace Automation):
   - Launch workflows (`launch_workflow` e.g. "open dev workflow", "coding setup", "research", "chill/media"), list available presets (`list_workflows`), or inspect launch steps (`get_workflow_details`).
   - Automatically capture currently open application windows across workspaces as a new workflow preset (`capture_current_workflow` e.g. "save my current setup as dev-review").
   - Create or save custom workflows (`save_custom_workflow` e.g. name, description, steps_json with workspace and launch commands).
   - Delete a custom workflow (`delete_custom_workflow`).
10. Calendar & Reminders:
   - Schedule meetings or calendar events (`schedule_event` e.g. "tomorrow 3pm", "Friday 10am").
   - Set countdown reminders with desktop notification alerts (`set_reminder` e.g. 15 minutes, 'Check the oven'), or view (`list_reminders`) and clear them (`clear_reminders`).
11. Email Drafting:
   - When asked to draft, compose, or write an email, compose a polished, professional subject and body and call `draft_email`.
12. Notes & Scratchpad:
   - Capture quick thoughts, task lists, or meeting notes (`create_note`, `list_notes`).
13. Autonomous Project & Code Generation (Coding CLI Assistant):
   - When asked to create, scaffold, or generate a project, codebase, application, or complex code task (e.g., "create a project called...", "create a react app"), call `create_project`.
   - If the user asks for a popup, interactive terminal, pass `open_terminal=True`.
   - For general tasks, documents, notes, or scripts to delegate to the CLI AI tool, call `delegate_to_antigravity`.
14. Screen Perception: If the user asks you to look at their screen, inspect a window, or diagnose an error, call `inspect_screen`.
15. Ongoing Conversation & Dismissal:
   - Jarvis maintains conversational context across sequential commands within the same session.
   - When the user indicates they are finished, done, or dismisses you (e.g., "that's it", "done", "that's all", "goodbye"), acknowledge politely and call `dismiss_session`.
"#;

static EXIT_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"^(no\s+)?that('?s|\s+is)\s+(it|all)(\s+(for\s+now|thank\s+you|thanks))?$").unwrap(),
        Regex::new(r"^(thank\s+you|thanks)[,\s]+(that('?s|\s+is)\s+(it|all)|jarvis)$").unwrap(),
        Regex::new(r"^(im|i am|we are|were)?\s*all?\s*done(\s+now)?$").unwrap(),
        Regex::new(r"^(that('?ll|\s+will)\s+be\s+all)(\s+(for\s+now|thank\s+you|thanks))?$").unwrap(),
        Regex::new(r"^(nothing\s+else|no\s+more)(\s+(for\s+now|thank\s+you|thanks))?$").unwrap(),
    ]
});

const EXACT_EXIT_COMMANDS: &[&str] = &[
    "done", "im done", "i am done", "all done", "we are done", "were done",
    "thats it", "that is it", "thats all", "that is all", "that will be all", "thatll be all",
    "thats all for now", "that is all for now", "thats it for now", "that is it for now",
    "thank you thats it", "thanks thats it", "thank you thats all", "thanks thats all",
    "thats it thank you", "thats it thanks", "thats all thank you", "thats all thanks",
    "no thats it", "no that is it", "no thats all", "no that is all",
    "nothing else", "nothing else thanks", "nothing else thank you", "nothing",
    "no thanks", "no thank you",
    "bye", "goodbye", "bye bye", "bye jarvis", "goodbye jarvis",
    "stop", "stop listening", "exit", "quit", "dismiss",
    "never mind", "nevermind", "cancel", "shut down", "go to sleep",
    "close", "close jarvis",
];

pub fn is_exit_command(text: &str) -> bool {
    if text.trim().is_empty() {
        return false;
    }
    let clean: String = text
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .to_lowercase();
    let clean = clean.split_whitespace().collect::<Vec<&str>>().join(" ");

    if EXACT_EXIT_COMMANDS.contains(&clean.as_str()) {
        return true;
    }

    EXIT_PATTERNS.iter().any(|pattern| pattern.is_match(&clean))
}

pub struct JarvisAgent {
    client: Option<GeminiClient>,
    fallback: Arc<Mutex<FallbackCoordinator>>,
    registry: Arc<ToolRegistry>,
    hyprland: Arc<HyprlandController>,
    screen: Arc<ScreenPerception>,
    history: Arc<Mutex<Vec<ContentMessage>>>,
    session_ended: Arc<AtomicBool>,
}

impl JarvisAgent {
    pub fn new(settings: &Settings) -> Self {
        let session_ended = Arc::new(AtomicBool::new(false));
        let registry = Arc::new(build_tool_registry(settings, session_ended.clone()));
        let hyprland = Arc::new(HyprlandController::new());
        let screen = Arc::new(ScreenPerception::new(hyprland.clone()));

        let client = settings.gemini_api_key.as_ref().map(|k| GeminiClient::new(k));
        let fallback = Arc::new(Mutex::new(FallbackCoordinator::new(
            Some(&settings.model_name),
            None,
        )));

        Self {
            client,
            fallback,
            registry,
            hyprland,
            screen,
            history: Arc::new(Mutex::new(Vec::new())),
            session_ended,
        }
    }

    /// Reset chat history and session termination state
    pub async fn reset_session(&self) {
        let mut hist = self.history.lock().await;
        hist.clear();
        self.session_ended.store(false, Ordering::SeqCst);
        let mut fb = self.fallback.lock().await;
        fb.reset();
        debug!("Jarvis session reset: history cleared, model reset to primary.");
    }

    pub fn is_session_ended(&self) -> bool {
        self.session_ended.load(Ordering::SeqCst)
    }

    pub fn mark_session_ended(&self) {
        self.session_ended.store(true, Ordering::SeqCst);
    }

    /// Situational context snapshot (focused window, active workspace, current time)
    pub async fn get_system_context(&self) -> String {
        let now = Local::now().format("%A, %B %d, %Y at %I:%M %p").to_string();
        let win = self.hyprland.get_active_window().await.unwrap_or(json!({}));
        let win_title = win["title"].as_str().unwrap_or("Unknown");
        let win_class = win["class"].as_str().unwrap_or("Unknown");
        let workspace_id = win["workspace"]["id"].as_i64().unwrap_or(1);

        format!("[Context: CurrentTime='{now}', Focused='{win_title}' ({win_class}), Workspace={workspace_id}]")
    }

    /// Process user prompt through Gemini reasoning loop with automatic tool execution and quota fallback
    pub async fn process_prompt(&self, prompt: &str) -> Result<String> {
        let trimmed = prompt.trim();
        if trimmed.is_empty() {
            return Ok(String::new());
        }

        // Fast path dismissal check
        if is_exit_command(trimmed) {
            info!("User dismissal command detected: '{}'", trimmed);
            self.session_ended.store(true, Ordering::SeqCst);
            return Ok("Very well, sir. Have a wonderful day.".to_string());
        }

        let client = match &self.client {
            Some(c) => c,
            None => {
                warn!("GEMINI_API_KEY not configured.");
                return Ok("I require a Google Gemini API key to operate, sir. Please configure it in your environment.".to_string());
            }
        };

        let context = self.get_system_context().await;
        let turn_content = format!("{context}\n{trimmed}");

        // Append user turn to conversation history
        {
            let mut hist = self.history.lock().await;
            hist.push(ContentMessage {
                role: "user".to_string(),
                parts: vec![json!({ "text": turn_content })],
            });
        }

        // Model reasoning and tool call execution loop with fallback
        const MAX_TOOL_TURNS: usize = 5;

        loop {
            let active_model = {
                let fb = self.fallback.lock().await;
                fb.current_model().to_string()
            };

            info!("Reasoning with model [{active_model}]: \"{trimmed}\"");

            let mut tool_turn = 0;
            let mut model_succeeded = false;
            let mut quota_exceeded = false;
            let mut final_text_reply = String::new();

            while tool_turn < MAX_TOOL_TURNS {
                tool_turn += 1;

                let current_history = {
                    let hist = self.history.lock().await;
                    hist.clone()
                };

                let request = GenerateContentRequest {
                    contents: current_history,
                    system_instruction: Some(SystemInstruction {
                        parts: vec![SystemPart {
                            text: SYSTEM_INSTRUCTION.to_string(),
                        }],
                    }),
                    tools: Some(self.registry.gemini_function_declarations()),
                    generation_config: Some(GenerationConfig { temperature: 0.2 }),
                };

                let response = match client.generate_content(&active_model, &request).await {
                    Ok(resp) => resp,
                    Err(JarvisError::QuotaExceeded(m)) => {
                        warn!("Quota limit hit on model {m}");
                        quota_exceeded = true;
                        break;
                    }
                    Err(e) => {
                        error!("Error during agent reasoning with {active_model}: {e}");
                        return Ok(format!("I encountered an issue processing your request: {e}"));
                    }
                };

                // Parse candidate
                let candidate = match response.candidates.and_then(|mut c| if c.is_empty() { None } else { Some(c.remove(0)) }) {
                    Some(c) => c,
                    None => {
                        final_text_reply = "Done, sir.".to_string();
                        model_succeeded = true;
                        break;
                    }
                };

                let content = match candidate.content {
                    Some(c) => c,
                    None => {
                        final_text_reply = "Done, sir.".to_string();
                        model_succeeded = true;
                        break;
                    }
                };

                // Extract function calls from content parts
                let mut function_calls = Vec::new();
                for part in &content.parts {
                    if let Some(call) = part.get("functionCall") {
                        let name = call["name"].as_str().unwrap_or_default().to_string();
                        let args = call.get("args").cloned().unwrap_or_else(|| json!({}));
                        function_calls.push((name, args));
                    }
                }

                if function_calls.is_empty() {
                    // Final text response
                    for part in &content.parts {
                        if let Some(txt) = part.get("text").and_then(|t| t.as_str()) {
                            if !txt.trim().is_empty() {
                                final_text_reply = txt.trim().to_string();
                                break;
                            }
                        }
                    }
                    if final_text_reply.is_empty() {
                        final_text_reply = "Done, sir.".to_string();
                    }

                    // Record model answer in history
                    {
                        let mut hist = self.history.lock().await;
                        hist.push(content);
                    }
                    model_succeeded = true;
                    break;
                }

                // Append model message (with exact function calls and thought signatures) to history
                {
                    let mut hist = self.history.lock().await;
                    hist.push(content);
                }

                // Execute function calls
                for (call_name, call_args) in function_calls {
                    info!("Executing tool call: {call_name}({call_args:?})");

                    let result_str = if call_name == "dismiss_session" {
                        self.session_ended.store(true, Ordering::SeqCst);
                        let farewell = call_args["farewell"]
                            .as_str()
                            .unwrap_or("Very well, sir. Have a wonderful day.");
                        farewell.to_string()
                    } else if call_name == "inspect_screen" {
                        let query = call_args["query"].as_str().unwrap_or("analyze this screen");
                        let target = call_args["target"].as_str().unwrap_or("active_window");

                        let capture_result = if target == "fullscreen" {
                            self.screen.capture_full_screen(None).await
                        } else {
                            self.screen.capture_active_window(None).await
                        };

                        match capture_result {
                            Ok(path) => {
                                let vision_res = match fs::read(&path) {
                                    Ok(bytes) => {
                                        client.analyze_image(&active_model, &bytes, query).await
                                            .unwrap_or_else(|e| format!("Vision analysis failed: {e}"))
                                    }
                                    Err(e) => format!("Failed to read capture file: {e}"),
                                };
                                let _ = fs::remove_file(&path);
                                vision_res
                            }
                            Err(e) => format!("Screenshot capture failed: {e}"),
                        }
                    } else {
                        match self.registry.execute_tool(&call_name, call_args).await {
                            Ok(out) => out,
                            Err(e) => format!("Tool execution failed: {e}"),
                        }
                    };

                    debug!("Tool result for {call_name}: {result_str}");

                    // Append functionResponse to history (with role "user" per Gemini v1beta specification)
                    {
                        let mut hist = self.history.lock().await;
                        hist.push(ContentMessage {
                            role: "user".to_string(),
                            parts: vec![json!({
                                "functionResponse": {
                                    "name": call_name,
                                    "response": {
                                        "output": result_str
                                    }
                                }
                            })],
                        });
                    }
                }
            }

            if model_succeeded {
                info!("Jarvis Response: \"{final_text_reply}\"");
                return Ok(final_text_reply);
            }

            if quota_exceeded {
                let mut fb = self.fallback.lock().await;
                if let Some(next_model) = fb.next_fallback() {
                    warn!("Failing over to next fallback model: {next_model}");
                    continue;
                } else {
                    error!("All fallback model quotas have been exhausted.");
                    return Ok("All model quota limits have been temporarily exceeded, sir. Please retry in a few moments.".to_string());
                }
            }

            return Ok("I have completed the requested operations, sir.".to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_commands() {
        assert!(is_exit_command("done"));
        assert!(is_exit_command("I am done"));
        assert!(is_exit_command("That's it"));
        assert!(is_exit_command("that is all for now"));
        assert!(is_exit_command("goodbye jarvis"));
        assert!(is_exit_command("nothing else thanks"));
        assert!(is_exit_command("dismiss"));

        assert!(!is_exit_command("open youtube"));
        assert!(!is_exit_command("type hello world"));
        assert!(!is_exit_command("schedule a meeting"));
    }
}
