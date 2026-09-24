use crate::ai::fallback::FallbackCoordinator;
use crate::ai::gemini::{
    ContentMessage, GeminiClient, GenerateContentRequest, GenerationConfig, SystemInstruction,
    SystemPart,
};
use crate::ai::jev::{FastPathAction, FastPathRouter};
use crate::ai::providers::AIClient;
use crate::core::config::Settings;
use crate::core::error::{JarvisError, Result};
use crate::tools::hyprland::HyprlandController;
use crate::tools::screen::ScreenPerception;
use crate::tools::{build_tool_registry, ToolRegistry};
use chrono::Local;
use regex::Regex;
use serde_json::{json, Value};
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
   - For all action commands, tool executions, and system tasks (e.g., switching workspaces, closing tabs/windows, adjusting volume/brightness, typing, launching apps, media controls, terminal commands, opening editor): Reply with ONLY 1 OR 2 WORDS (e.g., "Right away.", "Done.", "Switched.", "Closed.", "On it.", "Opened."). NEVER speak full explanatory sentences like "I have switched to workspace 3 for you, sir." or "I've closed the tab." The user requires instantaneous confirmation so they can immediately issue their next command without waiting.
   - If a tool execution result indicates cancellation, refusal, or failure (e.g., "cancelled", "aborted", "failed", "not found"): Reply with ONLY 1 OR 2 WORDS reflecting the outcome (e.g., "Cancelled.", "Aborted."). NEVER claim an action succeeded (like saying "Shutting down.") if the tool was cancelled.
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
6. Media, Movie & Music Automation:
   - Play TV shows, movies, or video files (`play_media` e.g. "Open Prison break ep12 from season 1" -> query="prison break", season=1, episode=12, fullscreen=true). Jarvis searches configured media directories and opens it in fullscreen.
   - Resume media playback from where the user left off (`resume_media` e.g. "continue prison break from where I left off" -> query="prison break"). Jarvis resumes the last watched episode and timestamp.
   - Control music and video playback across Spotify, Chromium, YouTube, Firefox, mpv (`media_play_pause`, `media_next`, `media_previous`, `media_stop`, `get_now_playing`).
7. Clipboard Access:
   - Read what is currently on the clipboard (`get_clipboard`).
   - Copy any text or information to the system clipboard (`set_clipboard`).
8. Power & Hardware Controls:
   - Lock screen (`lock_screen`), logout (`logout_system`), reboot (`reboot_system`), shutdown (`shutdown_system`).
   - Note: Major system commands (shutdown, reboot, logout) automatically prompt the user with a visual Yes/No popup and accept simultaneous voice confirmation before performing the action.
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
14. Project File Navigation & Code Editing:
   - When asked to open, edit, or view any file from a project or codebase (e.g., "open models.go file from the project tracky researcher tui project in a new neovim instance", "open main.rs in nvim"), immediately call `open_file_in_editor(project_name="...", file_path="...", editor="nvim")`.
   - Jarvis will automatically locate the project directory in ~/Codes/personal, ~/Codes, or ~/Projects, find the file, and launch the editor in a new terminal window.
15. LocalSend File & Content Sharing:
   - When the user asks to share, send, or transfer any file, photo, media, video, text, link, or clipboard item via LocalSend (e.g., "share resume.pdf with localsend", "send prison break ep 12 via localsend", "share this photo", "send clipboard to localsend", "share screenshot"):
     Call `localsend_share(item="...")`.
     Jarvis searches Downloads, media folders, Documents, Pictures, and Desktop, resolves the file, opens LocalSend directly into its Send window with the item queued, and brings the window to the foreground.
16. Deep Research & Markdown Viewer (Floating Neovim Popup):
   - When the user asks for research, an explanation, deep dive, technical analysis, comparison, study, or notes on ANY topic (e.g., "research quantum computing", "look up rust async runtimes", "research wireguard vs openvpn", "research and show me..."):
     Synthesize a comprehensive, in-depth, well-structured research report in Markdown (including title, executive summary, key concepts, technical breakdowns, code examples or tables, and takeaways), and immediately call `display_research_in_neovim(title="...", content="...")`.
     This opens the report in a sleek floating Neovim popup window on the user's desktop.
     Keep your spoken reply to 1 or 2 words (e.g., "Research ready.", "Opened.").
17. Screen Perception: If the user asks you to look at their screen, inspect a window, or diagnose an error, call `inspect_screen`.
18. Ongoing Conversation & Dismissal:
   - Jarvis maintains conversational context across sequential commands within the same session.
   - When the user indicates they are finished, done, or dismisses you (e.g., "that's it", "done", "that's all", "goodbye"), acknowledge politely and call `dismiss_session`.
19. Multi-Action Requests (CRITICAL):
   - If the user's prompt asks you to perform multiple operations or an action followed by diagnostics, analysis, or research (e.g., "Open a new tmux window and coding editor, search for reasons for battery degradation, analyze apps and create a document and show"):
     You MUST execute ALL necessary tool calls sequentially across your tool turns until EVERY part of the user's request is completely fulfilled.
     NEVER return text like "On it." or "Working on it." midway through an uncompleted multi-step task.
     Only emit your final 1-2 word response (e.g., "Done.", "Research ready.") once ALL requested actions, tools, and research documents have been fully executed.
"#;

static EXIT_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"^(no\s+)?that('?s|\s+is)\s+(it|all)(\s+(for\s+now|thank\s+you|thanks))?$")
            .unwrap(),
        Regex::new(r"^(thank\s+you|thanks)[,\s]+(that('?s|\s+is)\s+(it|all)|jarvis)$").unwrap(),
        Regex::new(r"^(im|i am|we are|were)?\s*all?\s*done(\s+now)?$").unwrap(),
        Regex::new(r"^(that('?ll|\s+will)\s+be\s+all)(\s+(for\s+now|thank\s+you|thanks))?$")
            .unwrap(),
        Regex::new(r"^(nothing\s+else|no\s+more)(\s+(for\s+now|thank\s+you|thanks))?$").unwrap(),
    ]
});

const EXACT_EXIT_COMMANDS: &[&str] = &[
    "done",
    "im done",
    "i am done",
    "all done",
    "we are done",
    "were done",
    "thats it",
    "that is it",
    "thats all",
    "that is all",
    "that will be all",
    "thatll be all",
    "thats all for now",
    "that is all for now",
    "thats it for now",
    "that is it for now",
    "thank you thats it",
    "thanks thats it",
    "thank you thats all",
    "thanks thats all",
    "thats it thank you",
    "thats it thanks",
    "thats all thank you",
    "thats all thanks",
    "no thats it",
    "no that is it",
    "no thats all",
    "no that is all",
    "nothing else",
    "nothing else thanks",
    "nothing else thank you",
    "nothing",
    "no thanks",
    "no thank you",
    "bye",
    "goodbye",
    "bye bye",
    "bye jarvis",
    "goodbye jarvis",
    "stop",
    "stop listening",
    "exit",
    "quit",
    "dismiss",
    "never mind",
    "nevermind",
    "cancel",
    "shut down",
    "go to sleep",
    "close",
    "close jarvis",
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
    ai_client: Option<AIClient>,
    gemini_client: Option<GeminiClient>,
    fallback: Arc<Mutex<FallbackCoordinator>>,
    fast_router: FastPathRouter,
    registry: Arc<ToolRegistry>,
    hyprland: Arc<HyprlandController>,
    screen: Arc<ScreenPerception>,
    hud: Arc<crate::ui::JarvisHUD>,
    gemini_history: Arc<Mutex<Vec<ContentMessage>>>,
    openai_history: Arc<Mutex<Vec<Value>>>,
    anthropic_history: Arc<Mutex<Vec<Value>>>,
    session_ended: Arc<AtomicBool>,
    provider_name: String,
    model_name: String,
}

impl JarvisAgent {
    pub fn new(settings: &Settings) -> Self {
        Self::with_hud(settings, Arc::new(crate::ui::JarvisHUD::new()))
    }

    pub fn with_hud(settings: &Settings, hud: Arc<crate::ui::JarvisHUD>) -> Self {
        let session_ended = Arc::new(AtomicBool::new(false));
        let registry = Arc::new(build_tool_registry(settings, session_ended.clone()));
        let hyprland = Arc::new(HyprlandController::new());
        let screen = Arc::new(ScreenPerception::new(hyprland.clone()));
        let fast_router = FastPathRouter::new(settings);

        let ai_client = AIClient::from_settings(settings).ok();
        let gemini_client = settings
            .gemini_api_key
            .as_ref()
            .map(|k| GeminiClient::new(k));
        let fallback = Arc::new(Mutex::new(FallbackCoordinator::new(
            Some(&settings.model_name),
            None,
        )));

        Self {
            ai_client,
            gemini_client,
            fallback,
            fast_router,
            registry,
            hyprland,
            screen,
            hud,
            gemini_history: Arc::new(Mutex::new(Vec::new())),
            openai_history: Arc::new(Mutex::new(Vec::new())),
            anthropic_history: Arc::new(Mutex::new(Vec::new())),
            session_ended,
            provider_name: settings.ai_provider.clone(),
            model_name: settings.model_name.clone(),
        }
    }

    /// Reset chat history and session termination state
    pub async fn reset_session(&self) {
        {
            let mut hist = self.gemini_history.lock().await;
            hist.clear();
        }
        {
            let mut hist = self.openai_history.lock().await;
            hist.clear();
        }
        {
            let mut hist = self.anthropic_history.lock().await;
            hist.clear();
        }
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

    pub fn registry(&self) -> &Arc<ToolRegistry> {
        &self.registry
    }

    pub fn hyprland(&self) -> &Arc<HyprlandController> {
        &self.hyprland
    }

    pub fn fast_router(&self) -> &FastPathRouter {
        &self.fast_router
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

    /// Process user prompt through active AI provider reasoning loop with automatic tool execution
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

        // Fast-path router check (Zero-latency regex or sub-100ms TypeSafe Jev System One)
        if let Some(action) = self.fast_router.route(trimmed).await {
            info!("Fast-path action matched in agent: {:?}", action);
            if matches!(action, FastPathAction::DismissSession) {
                self.session_ended.store(true, Ordering::SeqCst);
                return Ok(action.spoken_confirmation().to_string());
            }

            if let Err(e) = action.execute(&self.registry, &self.hyprland).await {
                warn!("Fast-path action execution error: {e}");
            }
            let conf = action.spoken_confirmation().to_string();

            // Record turn into histories so future conversational turns preserve context
            {
                let mut hist = self.gemini_history.lock().await;
                hist.push(ContentMessage {
                    role: "user".to_string(),
                    parts: vec![json!({ "text": trimmed })],
                });
                hist.push(ContentMessage {
                    role: "model".to_string(),
                    parts: vec![json!({ "text": conf.clone() })],
                });
            }
            {
                let mut o_hist = self.openai_history.lock().await;
                o_hist.push(json!({"role": "user", "content": trimmed}));
                o_hist.push(json!({"role": "assistant", "content": conf.clone()}));
            }
            {
                let mut a_hist = self.anthropic_history.lock().await;
                a_hist.push(json!({"role": "user", "content": trimmed}));
                a_hist.push(json!({"role": "assistant", "content": conf.clone()}));
            }

            return Ok(conf);
        }

        let ai_client = match &self.ai_client {
            Some(c) => c,
            None => {
                warn!(
                    "API key for provider '{}' not configured.",
                    self.provider_name
                );
                return Ok(format!(
                    "I require an API key for provider '{}', sir. Please configure it in ~/.config/jarvis/config.toml or your environment.",
                    self.provider_name
                ));
            }
        };

        let context = self.get_system_context().await;
        let turn_content = format!("{context}\n{trimmed}");

        match ai_client {
            AIClient::Gemini(client) => self.process_gemini(client, &turn_content).await,
            AIClient::OpenAI(client) => self.process_openai(client, &turn_content).await,
            AIClient::Anthropic(client) => self.process_anthropic(client, &turn_content).await,
        }
    }

    // -------------------------------------------------------------------------
    // Google Gemini Loop
    // -------------------------------------------------------------------------
    async fn process_gemini(&self, client: &GeminiClient, turn_content: &str) -> Result<String> {
        {
            let mut hist = self.gemini_history.lock().await;
            hist.push(ContentMessage {
                role: "user".to_string(),
                parts: vec![json!({ "text": turn_content })],
            });
        }

        const MAX_TOOL_TURNS: usize = 5;

        loop {
            let active_model = {
                let fb = self.fallback.lock().await;
                fb.current_model().to_string()
            };

            info!(
                "Reasoning with Gemini [{active_model}]: \"{}\"",
                turn_content.lines().last().unwrap_or_default()
            );

            let mut tool_turn = 0;
            let mut model_succeeded = false;
            let mut quota_exceeded = false;
            let mut final_text_reply = String::new();

            while tool_turn < MAX_TOOL_TURNS {
                tool_turn += 1;

                let current_history = {
                    let hist = self.gemini_history.lock().await;
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
                        return Ok(format!(
                            "I encountered an issue processing your request: {e}"
                        ));
                    }
                };

                let candidate = match response.candidates.and_then(|mut c| {
                    if c.is_empty() {
                        None
                    } else {
                        Some(c.remove(0))
                    }
                }) {
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

                let mut function_calls = Vec::new();
                for part in &content.parts {
                    if let Some(call) = part.get("functionCall") {
                        let name = call["name"].as_str().unwrap_or_default().to_string();
                        let args = call.get("args").cloned().unwrap_or_else(|| json!({}));
                        function_calls.push((name, args));
                    }
                }

                if function_calls.is_empty() {
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

                    {
                        let mut hist = self.gemini_history.lock().await;
                        hist.push(content);
                    }
                    model_succeeded = true;
                    break;
                }

                {
                    let mut hist = self.gemini_history.lock().await;
                    hist.push(content);
                }

                let is_single_call = function_calls.len() == 1;
                let mut direct_action_reply = None;

                for (call_name, call_args) in function_calls {
                    let result_str = self
                        .execute_tool_action(&call_name, call_args, &active_model)
                        .await;

                    if is_single_call && is_terminal_action_tool(&call_name) {
                        direct_action_reply = Some(action_confirmation_reply(&call_name, &result_str));
                    }

                    {
                        let mut hist = self.gemini_history.lock().await;
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

                if let Some(reply) = direct_action_reply {
                    final_text_reply = reply;
                    model_succeeded = true;
                    break;
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

    // -------------------------------------------------------------------------
    // OpenAI-Compatible Loop (OpenAI, Groq, OpenRouter)
    // -------------------------------------------------------------------------
    async fn process_openai(
        &self,
        client: &crate::ai::providers::OpenAICompatibleClient,
        turn_content: &str,
    ) -> Result<String> {
        let tools_val = {
            let mut tools = Vec::new();
            for tool in self.registry.all() {
                tools.push(json!({
                    "type": "function",
                    "function": {
                        "name": tool.name(),
                        "description": tool.description(),
                        "parameters": tool.parameters_schema()
                    }
                }));
            }
            json!(tools)
        };

        {
            let mut hist = self.openai_history.lock().await;
            if hist.is_empty() {
                hist.push(json!({
                    "role": "system",
                    "content": SYSTEM_INSTRUCTION
                }));
            }
            hist.push(json!({
                "role": "user",
                "content": turn_content
            }));
        }

        const MAX_TOOL_TURNS: usize = 5;
        let mut tool_turn = 0;

        while tool_turn < MAX_TOOL_TURNS {
            tool_turn += 1;

            let current_messages = {
                let hist = self.openai_history.lock().await;
                hist.clone()
            };

            let response = client
                .chat_completion(&self.model_name, &current_messages, Some(tools_val.clone()))
                .await?;

            if response.tool_calls.is_empty() {
                let reply = response.text.unwrap_or_else(|| "Done, sir.".to_string());
                {
                    let mut hist = self.openai_history.lock().await;
                    hist.push(json!({
                        "role": "assistant",
                        "content": reply
                    }));
                }
                info!("Jarvis Response: \"{reply}\"");
                return Ok(reply);
            }

            // Append assistant message with tool calls
            let mut raw_calls = Vec::new();
            for tc in &response.tool_calls {
                raw_calls.push(json!({
                    "id": tc.id,
                    "type": "function",
                    "function": {
                        "name": tc.name,
                        "arguments": tc.args.to_string()
                    }
                }));
            }

            {
                let mut hist = self.openai_history.lock().await;
                hist.push(json!({
                    "role": "assistant",
                    "content": response.text,
                    "tool_calls": raw_calls
                }));
            }

            let is_single_call = response.tool_calls.len() == 1;
            let mut direct_action_reply = None;

            // Execute each tool call and push result
            for tc in response.tool_calls {
                let result_str = self
                    .execute_tool_action(&tc.name, tc.args, &self.model_name)
                    .await;

                if is_single_call && is_terminal_action_tool(&tc.name) {
                    direct_action_reply = Some(action_confirmation_reply(&tc.name, &result_str));
                }

                {
                    let mut hist = self.openai_history.lock().await;
                    hist.push(json!({
                        "role": "tool",
                        "tool_call_id": tc.id,
                        "content": result_str
                    }));
                }
            }

            if let Some(reply) = direct_action_reply {
                info!("Jarvis Response: \"{reply}\"");
                return Ok(reply);
            }
        }

        Ok("Done, sir.".to_string())
    }

    // -------------------------------------------------------------------------
    // Anthropic Claude Loop
    // -------------------------------------------------------------------------
    async fn process_anthropic(
        &self,
        client: &crate::ai::providers::AnthropicClient,
        turn_content: &str,
    ) -> Result<String> {
        let tools_val = {
            let mut tools = Vec::new();
            for tool in self.registry.all() {
                tools.push(json!({
                    "name": tool.name(),
                    "description": tool.description(),
                    "input_schema": tool.parameters_schema()
                }));
            }
            json!(tools)
        };

        {
            let mut hist = self.anthropic_history.lock().await;
            hist.push(json!({
                "role": "user",
                "content": turn_content
            }));
        }

        const MAX_TOOL_TURNS: usize = 5;
        let mut tool_turn = 0;

        while tool_turn < MAX_TOOL_TURNS {
            tool_turn += 1;

            let current_messages = {
                let hist = self.anthropic_history.lock().await;
                hist.clone()
            };

            let response = client
                .create_message(
                    &self.model_name,
                    SYSTEM_INSTRUCTION,
                    &current_messages,
                    Some(tools_val.clone()),
                )
                .await?;

            if response.tool_calls.is_empty() {
                let reply = response.text.unwrap_or_else(|| "Done, sir.".to_string());
                {
                    let mut hist = self.anthropic_history.lock().await;
                    hist.push(json!({
                        "role": "assistant",
                        "content": reply
                    }));
                }
                info!("Jarvis Response: \"{reply}\"");
                return Ok(reply);
            }

            // Append assistant message with tool_use blocks
            let mut assistant_content = Vec::new();
            if let Some(ref txt) = response.text {
                assistant_content.push(json!({
                    "type": "text",
                    "text": txt
                }));
            }
            for tc in &response.tool_calls {
                assistant_content.push(json!({
                    "type": "tool_use",
                    "id": tc.id,
                    "name": tc.name,
                    "input": tc.args
                }));
            }

            {
                let mut hist = self.anthropic_history.lock().await;
                hist.push(json!({
                    "role": "assistant",
                    "content": assistant_content
                }));
            }

            // Execute tool calls and push tool_result blocks in a single user message
            let mut tool_results = Vec::new();
            for tc in response.tool_calls {
                let result_str = self
                    .execute_tool_action(&tc.name, tc.args, &self.model_name)
                    .await;
                tool_results.push(json!({
                    "type": "tool_result",
                    "tool_use_id": tc.id,
                    "content": result_str
                }));
            }

            {
                let mut hist = self.anthropic_history.lock().await;
                hist.push(json!({
                    "role": "user",
                    "content": tool_results
                }));
            }
        }

        Ok("Done, sir.".to_string())
    }

    /// Common tool execution logic across all AI providers
    async fn execute_tool_action(
        &self,
        call_name: &str,
        call_args: Value,
        active_model: &str,
    ) -> String {
        let notif_info = crate::ui::tool_notification_info(call_name, &call_args);
        info!(
            "Executing tool call: {call_name}({call_args:?}) - {}",
            notif_info.description
        );

        // Update menubar status, desktop notification, and HUD indicator
        crate::core::state::set_busy(&notif_info.description);
        crate::ui::TaskNotifier::global()
            .start_or_update(
                notif_info.glyph,
                notif_info.headline,
                &notif_info.description,
            )
            .await;
        self.hud.show_thinking(Some(&notif_info.description)).await;

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
                    let vision_res = if let Some(ref client) = self.gemini_client {
                        match fs::read(&path) {
                            Ok(bytes) => client
                                .analyze_image(active_model, &bytes, query)
                                .await
                                .unwrap_or_else(|e| format!("Vision analysis failed: {e}")),
                            Err(e) => format!("Failed to read capture file: {e}"),
                        }
                    } else {
                        "Screenshot captured, but Gemini API key is required for visual analysis."
                            .to_string()
                    };
                    let _ = fs::remove_file(&path);
                    vision_res
                }
                Err(e) => format!("Screenshot capture failed: {e}"),
            }
        } else {
            match self.registry.execute_tool(call_name, call_args).await {
                Ok(out) => out,
                Err(e) => format!("Tool execution failed: {e}"),
            }
        };

        info!("Tool result: {call_name} -> {result_str}");
        result_str
    }
}

pub fn friendly_tool_description(call_name: &str, call_args: &Value) -> String {
    crate::ui::tool_notification_info(call_name, call_args).description
}

pub fn is_terminal_action_tool(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "play_media"
            | "resume_media"
            | "media_play_pause"
            | "media_next"
            | "media_previous"
            | "media_stop"
            | "adjust_volume"
            | "switch_workspace"
            | "launch_application"
            | "focus_application"
            | "close_window"
            | "open_youtube"
            | "send_shortcut"
            | "type_text"
            | "press_key"
            | "scroll"
            | "lock_screen"
            | "logout_system"
            | "reboot_system"
            | "shutdown_system"
            | "set_reminder"
            | "clear_reminders"
            | "create_note"
            | "create_project"
            | "display_research_in_neovim"
            | "localsend_share"
            | "dismiss_session"
            | "set_theme"
            | "toggle_bluetooth"
    )
}

pub fn action_confirmation_reply(tool_name: &str, result_str: &str) -> String {
    let lower = result_str.to_lowercase();
    if lower.contains("cancelled") || lower.contains("aborted") {
        return "Cancelled.".to_string();
    }
    if lower.contains("failed") || lower.contains("error") {
        return "Action failed.".to_string();
    }
    if lower.contains("could not find") {
        if result_str.len() <= 60 {
            return result_str.to_string();
        }
        return "Could not find requested target.".to_string();
    }

    match tool_name {
        "play_media" | "resume_media" | "open_youtube" => "Playing.".to_string(),
        "media_play_pause" => "Toggled.".to_string(),
        "media_next" => "Next track.".to_string(),
        "media_previous" => "Previous track.".to_string(),
        "media_stop" => "Stopped.".to_string(),
        "adjust_volume" => "Adjusted.".to_string(),
        "switch_workspace" => "Switched.".to_string(),
        "launch_application" => "Opened.".to_string(),
        "focus_application" => "Focused.".to_string(),
        "close_window" => "Closed.".to_string(),
        "send_shortcut" | "type_text" | "press_key" | "scroll" => "Done.".to_string(),
        "lock_screen" => "Locked.".to_string(),
        "logout_system" => "Logging out.".to_string(),
        "reboot_system" => "Rebooting.".to_string(),
        "shutdown_system" => "Shutting down.".to_string(),
        "set_reminder" => "Reminder set.".to_string(),
        "clear_reminders" => "Reminders cleared.".to_string(),
        "create_note" => "Note saved.".to_string(),
        "create_project" => "Scaffolding.".to_string(),
        "display_research_in_neovim" => "Research ready.".to_string(),
        "localsend_share" => "Shared.".to_string(),
        "set_theme" => "Theme updated.".to_string(),
        "toggle_bluetooth" => "Bluetooth updated.".to_string(),
        "dismiss_session" => "Very well, sir. Have a wonderful day.".to_string(),
        _ => "Done.".to_string(),
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
        assert!(!is_exit_command("open models.go file from the project tracky researcher tui project in a new neovim instance"));
    }
}
