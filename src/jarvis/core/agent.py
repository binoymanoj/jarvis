import asyncio
from pathlib import Path
import re
from typing import Any, Callable, Dict, List, Optional
from google import genai
from google.genai import types
from jarvis.core.config import settings
from jarvis.core.logger import log
from jarvis.tools.antigravity import AntigravityManager
from jarvis.tools.calendar import CalendarManager
from jarvis.tools.clipboard import ClipboardManager
from jarvis.tools.email import EmailManager
from jarvis.tools.hyprland import HyprlandController
from jarvis.tools.media import MediaManager
from jarvis.tools.notes import NoteManager
from jarvis.tools.omarchy import OmarchyBridge
from jarvis.tools.screen import ScreenPerception
from jarvis.tools.shell import ShellExecutor
from jarvis.tools.system_power import SystemPowerManager
from jarvis.tools.virtual_input import VirtualInputManager
from jarvis.tools.web import WebNavigator
from jarvis.tools.workflow import WorkflowManager


def is_exit_command(text: str) -> bool:
    """Check if the transcribed phrase indicates the user wants to end the conversation session."""
    if not text:
        return False
    # Normalize: strip punctuation, lowercase, collapse whitespace
    clean = re.sub(r"[^\w\s]", "", text.lower()).strip()
    clean = re.sub(r"\s+", " ", clean)

    exact_kills = {
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
    }
    if clean in exact_kills:
        return True

    # Regex patterns for natural variations
    patterns = [
        r"^(no\s+)?that('?s|\s+is)\s+(it|all)(\s+(for\s+now|thank\s+you|thanks))?$",
        r"^(thank\s+you|thanks)[,\s]+(that('?s|\s+is)\s+(it|all)|jarvis)$",
        r"^(im|i am|we are|were)?\s*all?\s*done(\s+now)?$",
        r"^(that('?ll|\s+will)\s+be\s+all)(\s+(for\s+now|thank\s+you|thanks))?$",
        r"^(nothing\s+else|no\s+more)(\s+(for\s+now|thank\s+you|thanks))?$",
    ]
    return any(re.match(p, clean) for p in patterns)


SYSTEM_INSTRUCTION = """
You are Jarvis, an intelligent, elegant, and efficient desktop AI assistant natively integrated with Omarchy Linux and the Hyprland tiling compositor.
You allow the user to operate their computer completely hands-free.

Core Guidelines:
1. Tone: Calm, sophisticated, polite, and efficient (reminiscent of the British assistant persona).
2. Spoken Answers: Keep responses concise and conversational (1 to 2 sentences max), as they will be spoken aloud to the user.
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
"""

FALLBACK_MODELS = [
    "gemini-3.5-flash",
    "gemini-3.5-flash-lite",
    "gemini-2.5-flash",
    "gemini-2.5-flash-lite",
    "gemini-flash-latest",
    "gemini-flash-lite-latest",
]


class JarvisAgent:
    """Reasoning and tool-orchestration agent with automatic quota fallback."""

    def __init__(self, api_key: Optional[str] = None):
        self.api_key = api_key or settings.gemini_api_key
        self.hyprland = HyprlandController()
        self.omarchy = OmarchyBridge()
        self.screen = ScreenPerception()
        self.web = WebNavigator()
        self.workflow = WorkflowManager(hyprland=self.hyprland)
        self.calendar = CalendarManager()
        self.email = EmailManager()
        self.notes = NoteManager()
        self.antigravity = AntigravityManager()
        self.virtual_input = VirtualInputManager()
        self.shell = ShellExecutor()
        self.media = MediaManager()
        self.clipboard = ClipboardManager()
        self.system_power = SystemPowerManager(omarchy=self.omarchy)
        self._client: Optional[genai.Client] = None
        self._chat = None
        self._main_loop: Optional[asyncio.AbstractEventLoop] = None
        self.current_model = settings.model_name
        self.session_ended = False

        if self.api_key:
            self._client = genai.Client(api_key=self.api_key)


    def reset_session(self) -> None:
        """Reset the active chat session history and termination flag."""
        self._chat = None
        self.session_ended = False

    def _run_async(self, coro):
        """Dispatches an async coroutine to the main event loop from inside a sync tool callback."""
        if not self._main_loop or self._main_loop.is_closed():
            raise RuntimeError("Main asyncio event loop is not active.")
        future = asyncio.run_coroutine_threadsafe(coro, self._main_loop)
        return future.result()

    def _init_chat_session(self, model_name: Optional[str] = None) -> None:
        """Initializes the persistent Gemini chat session with synchronous tool wrappers."""
        if not self._client:
            return

        active_model = model_name or self.current_model

        def switch_workspace(workspace_id: int) -> str:
            """Switch to a specific Hyprland workspace (e.g., 1, 2, 3)."""
            log.info(f"[cyan]Executing tool: switch_workspace({workspace_id})[/cyan]")
            self._run_async(self.hyprland.change_workspace(workspace_id))
            return f"Switched to workspace {workspace_id}."

        def focus_application(app_name: str) -> str:
            """Focus an open application window by name or class (e.g., 'terminal', 'browser', 'foot', 'slack')."""
            log.info(f"[cyan]Executing tool: focus_application('{app_name}')[/cyan]")
            success = self._run_async(self.hyprland.focus_app(app_name))
            return f"Focused {app_name}." if success else f"Could not find open application matching '{app_name}'."

        def close_active_window() -> str:
            """Close the currently focused window."""
            log.info("[cyan]Executing tool: close_active_window()[/cyan]")
            self._run_async(self.hyprland.close_active_window())
            return "Closed active window."

        def toggle_layout_split() -> str:
            """Toggle the split orientation between vertical and horizontal in dwindle layout."""
            log.info("[cyan]Executing tool: toggle_layout_split()[/cyan]")
            self._run_async(self.hyprland.toggle_split())
            return "Toggled window split orientation."

        def toggle_fullscreen() -> str:
            """Toggle fullscreen state of the currently active window."""
            log.info("[cyan]Executing tool: toggle_fullscreen()[/cyan]")
            self._run_async(self.hyprland.toggle_fullscreen())
            return "Toggled fullscreen."

        def adjust_volume(adjustment: str) -> str:
            """Adjust audio volume. Options: 'raise', 'lower', '+5', '-10', 'mute-toggle'."""
            log.info(f"[cyan]Executing tool: adjust_volume('{adjustment}')[/cyan]")
            out = self._run_async(self.omarchy.set_volume(adjustment))
            return f"Volume adjusted: {out}"

        def set_brightness(level: str) -> str:
            """Set display brightness. Options: '+10', '-10', or absolute '50'."""
            log.info(f"[cyan]Executing tool: set_brightness('{level}')[/cyan]")
            out = self._run_async(self.omarchy.set_brightness(level))
            return f"Brightness updated: {out}"

        def set_theme(theme_name: str) -> str:
            """Apply an Omarchy system theme (e.g., 'tokyo-night', 'catppuccin', 'solitude', 'gruvbox')."""
            log.info(f"[cyan]Executing tool: set_theme('{theme_name}')[/cyan]")
            out = self._run_async(self.omarchy.set_theme(theme_name))
            return f"Theme changed: {out}"

        def get_battery() -> str:
            """Check system battery percentage and remaining run time."""
            log.info("[cyan]Executing tool: get_battery()[/cyan]")
            return self._run_async(self.omarchy.get_battery_status())

        def launch_application(app_name: str) -> str:
            """Launch an app via Omarchy launcher (e.g. 'terminal', 'browser', 'nautilus')."""
            log.info(f"[cyan]Executing tool: launch_application('{app_name}')[/cyan]")
            self._run_async(self.omarchy.launch_app(app_name))
            return f"Launched {app_name}."

        def inspect_screen(query: str, target: str = "active_window") -> str:
            """Capture the screen or active window and analyze it visually."""
            log.info(f"[cyan]Executing tool: inspect_screen('{query}', target='{target}')[/cyan]")
            if target == "fullscreen":
                img_path = self._run_async(self.screen.capture_full_screen())
            else:
                img_path = self._run_async(self.screen.capture_active_window())

            try:
                with open(img_path, "rb") as f:
                    img_bytes = f.read()

                part = types.Part.from_bytes(data=img_bytes, mime_type="image/png")
                prompt = f"The user asked: '{query}'. Based on this screenshot, give a direct, concise diagnosis or answer."
                res = self._client.models.generate_content(
                    model=active_model,
                    contents=[part, prompt],
                )
                return res.text.strip()
            finally:
                if img_path.exists():
                    img_path.unlink()

        def notify(headline: str, description: str = "") -> str:
            """Send an Omarchy desktop notification."""
            log.info(f"[cyan]Executing tool: notify('{headline}')[/cyan]")
            self._run_async(self.omarchy.notify(headline, description))
            return "Notification displayed."

        def open_url(url: str) -> str:
            """Open any website or URL in the default browser."""
            log.info(f"[cyan]Executing tool: open_url('{url}')[/cyan]")
            return self._run_async(self.web.open_url(url))

        def open_youtube(query: str) -> str:
            """Search and play a video on YouTube (e.g. 'mkbhd latest video', 'lofi beats')."""
            log.info(f"[cyan]Executing tool: open_youtube('{query}')[/cyan]")
            return self._run_async(self.web.open_youtube(query))

        def search_web(query: str) -> str:
            """Search Google for information or websites."""
            log.info(f"[cyan]Executing tool: search_web('{query}')[/cyan]")
            return self._run_async(self.web.search_web(query))

        def launch_workflow(name: str) -> str:
            """Launch a multi-workspace workflow setup (e.g. 'coding'/'dev', 'research', 'writing', 'communication', 'media')."""
            log.info(f"[cyan]Executing tool: launch_workflow('{name}')[/cyan]")
            return self._run_async(self.workflow.launch_workflow(name))

        def list_workflows() -> str:
            """List all configured workflow presets and their descriptions."""
            log.info("[cyan]Executing tool: list_workflows()[/cyan]")
            return self.workflow.list_workflows()

        def capture_current_workflow(name: str, description: str = "", aliases: str = "") -> str:
            """Capture currently open application windows across Hyprland workspaces into a new saved workflow preset."""
            log.info(f"[cyan]Executing tool: capture_current_workflow('{name}')[/cyan]")
            alias_list = [a.strip() for a in aliases.split(",") if a.strip()] if aliases else None
            return self._run_async(self.workflow.capture_current_setup(name, description, alias_list))

        def save_custom_workflow(
            name: str,
            description: str,
            steps_json: str,
            aliases: str = "",
            primary_workspace: int = 1,
        ) -> str:
            """Create or update a custom workflow preset. steps_json should be a JSON array of objects with 'workspace' (int) and 'launch' (command string)."""
            log.info(f"[cyan]Executing tool: save_custom_workflow('{name}')[/cyan]")
            import json
            try:
                if isinstance(steps_json, list):
                    steps = steps_json
                else:
                    steps = json.loads(steps_json)
            except Exception as e:
                return f"Invalid steps format: {e}. Provide a JSON list of workspace and launch objects."

            alias_list = [a.strip() for a in aliases.split(",") if a.strip()] if aliases else None
            return self.workflow.save_workflow(name, description, steps, alias_list, primary_workspace)

        def delete_custom_workflow(name: str) -> str:
            """Delete a custom workflow preset by name or alias."""
            log.info(f"[cyan]Executing tool: delete_custom_workflow('{name}')[/cyan]")
            return self.workflow.delete_workflow(name)

        def get_workflow_details(name: str) -> str:
            """Get the detailed launch steps, workspaces, and aliases of a specific workflow."""
            log.info(f"[cyan]Executing tool: get_workflow_details('{name}')[/cyan]")
            return self.workflow.get_workflow_details(name)


        def schedule_event(
            title: str,
            start_time: str,
            end_time: Optional[str] = None,
            description: str = "",
            provider: str = "auto",
        ) -> str:
            """Schedule an event in Google Calendar or Thunderbird (e.g. title='Team Sync', start_time='tomorrow 3pm')."""
            log.info(f"[cyan]Executing tool: schedule_event('{title}', '{start_time}')[/cyan]")
            return self._run_async(self.calendar.schedule_event(title, start_time, end_time, description, provider))

        def set_reminder(minutes: int, message: str) -> str:
            """Set a desktop timer reminder with system notification and alert (e.g. 15 minutes, 'Check the oven')."""
            log.info(f"[cyan]Executing tool: set_reminder({minutes}, '{message}')[/cyan]")
            return self._run_async(self.calendar.set_reminder(minutes, message))

        def list_reminders() -> str:
            """List all active upcoming desktop reminders and remaining times."""
            log.info("[cyan]Executing tool: list_reminders()[/cyan]")
            return self._run_async(self.calendar.list_reminders())

        def clear_reminders() -> str:
            """Clear all active desktop reminders."""
            log.info("[cyan]Executing tool: clear_reminders()[/cyan]")
            return self._run_async(self.calendar.clear_reminders())

        def draft_email(
            recipient: str,
            subject: str,
            body: str,
            client: str = "auto",
        ) -> str:
            """Draft an email and open the compose window in Thunderbird or Gmail with prefilled subject and body for user review."""
            log.info(f"[cyan]Executing tool: draft_email('{recipient}', '{subject}')[/cyan]")
            return self._run_async(self.email.draft_email(recipient, subject, body, client))

        def create_note(title: str, content: str, open_editor: bool = False) -> str:
            """Create or append to a markdown note in ~/Notes and optionally open it in Obsidian or editor."""
            log.info(f"[cyan]Executing tool: create_note('{title}')[/cyan]")
            return self._run_async(self.notes.create_note(title, content, open_editor))

        def list_notes() -> str:
            """List recent notes saved in the Notes directory."""
            log.info("[cyan]Executing tool: list_notes()[/cyan]")
            return self.notes.list_recent_notes()

        def create_project(
            name: str,
            description: str = "",
            target_dir: str = "",
            project_type: str = "project",
            open_terminal: bool = False,
        ) -> str:
            """Create and scaffold a complete project using Antigravity CLI (agy) with full autonomous permissions or floating popup window."""
            log.info(f"[cyan]Executing tool: create_project('{name}', type='{project_type}', popup={open_terminal})[/cyan]")
            return self._run_async(self.antigravity.create_project(name, description, target_dir or None, project_type, open_terminal))

        def delegate_to_antigravity(
            task_description: str,
            target_dir: str = "",
            open_terminal: bool = False,
        ) -> str:
            """Delegate any code generation, document creation, or complex task to Antigravity CLI (agy) with full autonomous permissions."""
            log.info(f"[cyan]Executing tool: delegate_to_antigravity('{task_description[:50]}')[/cyan]")
            return self._run_async(self.antigravity.create_anything(task_description, target_dir or None, open_terminal))

        def type_text(text: str, enter_after: bool = False) -> str:
            """Type text directly into the currently focused window hands-free. Set enter_after=True to submit."""
            log.info(f"[cyan]Executing tool: type_text('{text}', enter={enter_after})[/cyan]")
            return self._run_async(self.virtual_input.type_text(text, enter_after))

        def press_key(key_name: str) -> str:
            """Press a keyboard key hands-free (e.g. 'Return', 'Escape', 'Tab', 'BackSpace', 'space', 'Up', 'Down', 'Page_Down')."""
            log.info(f"[cyan]Executing tool: press_key('{key_name}')[/cyan]")
            return self._run_async(self.virtual_input.press_key(key_name))

        def send_shortcut(modifiers: str, key: str) -> str:
            """Send a keyboard shortcut hands-free (e.g. modifiers='ctrl', key='s' or modifiers='ctrl+shift', key='t')."""
            log.info(f"[cyan]Executing tool: send_shortcut('{modifiers}', '{key}')[/cyan]")
            return self._run_async(self.virtual_input.send_shortcut(modifiers, key))

        def scroll(direction: str = "down", amount: int = 2) -> str:
            """Scroll the active window up or down hands-free (direction: 'up' or 'down', amount: 1-10)."""
            log.info(f"[cyan]Executing tool: scroll(direction='{direction}', amount={amount})[/cyan]")
            return self._run_async(self.virtual_input.scroll(direction, amount))

        def execute_command(command: str, timeout: float = 15.0) -> str:
            """Execute any bash command directly on the Linux system and return its terminal output."""
            log.info(f"[cyan]Executing tool: execute_command('{command}')[/cyan]")
            return self._run_async(self.shell.execute_command(command, timeout))

        def media_play_pause() -> str:
            """Toggle media playback (play/pause) across active media players (Spotify, Chromium, YouTube, Firefox, mpv)."""
            log.info("[cyan]Executing tool: media_play_pause()[/cyan]")
            return self._run_async(self.media.media_play_pause())

        def media_next() -> str:
            """Skip to the next song or media track."""
            log.info("[cyan]Executing tool: media_next()[/cyan]")
            return self._run_async(self.media.media_next())

        def media_previous() -> str:
            """Skip to the previous song or media track."""
            log.info("[cyan]Executing tool: media_previous()[/cyan]")
            return self._run_async(self.media.media_previous())

        def media_stop() -> str:
            """Stop current media playback."""
            log.info("[cyan]Executing tool: media_stop()[/cyan]")
            return self._run_async(self.media.media_stop())

        def get_now_playing() -> str:
            """Get the title and artist of currently playing music or video."""
            log.info("[cyan]Executing tool: get_now_playing()[/cyan]")
            return self._run_async(self.media.get_now_playing())

        def get_clipboard() -> str:
            """Read the current text content stored in the system clipboard."""
            log.info("[cyan]Executing tool: get_clipboard()[/cyan]")
            return self._run_async(self.clipboard.get_clipboard())

        def set_clipboard(text: str) -> str:
            """Copy specified text to the system clipboard."""
            log.info(f"[cyan]Executing tool: set_clipboard('{text[:40]}')[/cyan]")
            return self._run_async(self.clipboard.set_clipboard(text))

        def lock_screen() -> str:
            """Lock the workstation and sleep displays."""
            log.info("[cyan]Executing tool: lock_screen()[/cyan]")
            return self._run_async(self.system_power.lock_screen())

        def logout_system() -> str:
            """Log out of the desktop session."""
            log.info("[cyan]Executing tool: logout_system()[/cyan]")
            return self._run_async(self.system_power.logout_system())

        def reboot_system() -> str:
            """Reboot the computer."""
            log.info("[cyan]Executing tool: reboot_system()[/cyan]")
            return self._run_async(self.system_power.reboot_system())

        def shutdown_system() -> str:
            """Shut down the computer."""
            log.info("[cyan]Executing tool: shutdown_system()[/cyan]")
            return self._run_async(self.system_power.shutdown_system())

        def toggle_bluetooth(action: str = "toggle") -> str:
            """Control Bluetooth power state ('on', 'off', 'toggle', 'is-on')."""
            log.info(f"[cyan]Executing tool: toggle_bluetooth('{action}')[/cyan]")
            return self._run_async(self.system_power.toggle_bluetooth(action))

        def get_system_stats() -> str:
            """Get live system stats (CPU usage, memory usage, network interface)."""
            log.info("[cyan]Executing tool: get_system_stats()[/cyan]")
            return self._run_async(self.system_power.get_system_stats())

        def network_speedtest(direction: str = "down") -> str:
            """Measure live internet speed ('down' or 'up')."""
            log.info(f"[cyan]Executing tool: network_speedtest('{direction}')[/cyan]")
            return self._run_async(self.system_power.network_speedtest(direction))

        def dismiss_session(farewell: str = "Very well, sir. Have a wonderful day.") -> str:
            """Dismiss Jarvis and conclude the ongoing conversation session when the user is done, says that's it, or goodbye."""
            log.info(f"[cyan]Executing tool: dismiss_session('{farewell}')[/cyan]")
            self.session_ended = True
            return farewell

        tools = [
            switch_workspace,
            focus_application,
            close_active_window,
            toggle_layout_split,
            toggle_fullscreen,
            adjust_volume,
            set_brightness,
            set_theme,
            get_battery,
            launch_application,
            open_url,
            open_youtube,
            search_web,
            launch_workflow,
            list_workflows,
            capture_current_workflow,
            save_custom_workflow,
            delete_custom_workflow,
            get_workflow_details,
            schedule_event,
            set_reminder,
            list_reminders,
            clear_reminders,
            draft_email,
            create_note,
            list_notes,
            create_project,
            delegate_to_antigravity,
            inspect_screen,
            notify,
            type_text,
            press_key,
            send_shortcut,
            scroll,
            execute_command,
            media_play_pause,
            media_next,
            media_previous,
            media_stop,
            get_now_playing,
            get_clipboard,
            set_clipboard,
            lock_screen,
            logout_system,
            reboot_system,
            shutdown_system,
            toggle_bluetooth,
            get_system_stats,
            network_speedtest,
            dismiss_session,
        ]


        config_kwargs = {
            "system_instruction": SYSTEM_INSTRUCTION,
            "tools": tools,
            "temperature": 0.2,
        }
        if "3.8" in active_model or active_model == "gemini-flash-latest":
            config_kwargs["thinking_config"] = types.ThinkingConfig(thinking_budget=0)

        self._chat = self._client.chats.create(
            model=active_model,
            config=types.GenerateContentConfig(**config_kwargs),
        )
        self.current_model = active_model

    async def get_system_context(self) -> str:
        """Fetch situational context snapshot with minimal overhead."""
        try:
            import datetime
            now_str = datetime.datetime.now().strftime("%A, %B %d, %Y at %I:%M %p")
            active_win = await self.hyprland.get_active_window()
            win_title = active_win.get("title", "Unknown")
            win_class = active_win.get("class", "Unknown")
            workspace_id = active_win.get("workspace", {}).get("id", 1)
            return f"[Context: CurrentTime='{now_str}', Focused='{win_title}' ({win_class}), Workspace={workspace_id}]"
        except Exception:
            return ""

    async def process_prompt(self, user_prompt: str) -> str:
        """Processes user voice/text query through Gemini Flash agent with tool execution and auto-fallback."""
        if not user_prompt or not user_prompt.strip():
            return ""

        if not self._client:
            log.warning("Gemini client not initialized (GEMINI_API_KEY missing).")
            return "I require a Google Gemini API key to operate, sir. Please configure it in your environment."

        self._main_loop = asyncio.get_running_loop()
        if not self._chat:
            self._init_chat_session()

        context = await self.get_system_context()
        full_message = f"{context}\n{user_prompt}" if context else user_prompt

        log.info(f"[cyan]Jarvis Thinking ({self.current_model}):[/cyan] \"{user_prompt}\"")

        # Attempt with current model, falling back if quota/rate limit is encountered
        models_to_try = [self.current_model] + [m for m in FALLBACK_MODELS if m != self.current_model]

        for model_candidate in models_to_try:
            try:
                if model_candidate != self.current_model or not self._chat:
                    log.info(f"[yellow]Switching to model {model_candidate}...[/yellow]")
                    self._init_chat_session(model_candidate)

                response = await self._main_loop.run_in_executor(
                    None,
                    lambda: self._chat.send_message(full_message),
                )
                reply = response.text.strip() if response.text else "Done, sir."
                log.info(f"[bold green]Jarvis Response ({self.current_model}):[/bold green] \"{reply}\"")
                return reply
            except Exception as e:
                err_str = str(e).lower()
                if "quota" in err_str or "resource_exhausted" in err_str or "429" in err_str or "not_found" in err_str:
                    log.warning(f"Model {model_candidate} unavailable ({e}). Attempting next fallback...")
                    continue
                else:
                    log.error(f"Error during agent reasoning with {model_candidate}: {e}")
                    return f"I encountered an issue processing your request: {e}"

        return "All model quota limits have been temporarily exceeded, sir. Please retry in a few moments."
