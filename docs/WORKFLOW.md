# 󰚩 Jarvis Workflows & System Capabilities Guide

This guide documents the multi-workspace workflows, calendar scheduling, email composition, reminders, and note-taking systems built into **Jarvis** for **Omarchy Linux** and **Hyprland**.

---

## 1. Multi-Workspace Workflows

Workflows allow Jarvis to launch entire multi-application workspaces across Hyprland desktops with a single voice or CLI command. 

### Built-in Workflows

| Preset | Aliases | Workspace Layout | Description |
| :--- | :--- | :--- | :--- |
| **`coding`** | `dev`, `code`, `development` | **WS 1**: Editor (`omarchy-launch-editor`) & Terminal<br>**WS 2**: Web Browser (`omarchy-launch-browser`) | Complete programming environment with code and shell side-by-side, browser on adjacent workspace. |
| **`research`** | `study`, `reading` | **WS 1**: Web Browser<br>**WS 2**: Obsidian Notes (`obsidian`) | Web exploration on workspace 1, note-taking and knowledge base on workspace 2. |
| **`writing`** | `notes`, `write`, `drafting` | **WS 1**: Obsidian Notes<br>**WS 2**: Web Browser | Distraction-free writing in Obsidian with reference browser available on workspace 2. |
| **`communication`**| `social`, `mail`, `messaging` | **WS 1**: Thunderbird Email (`thunderbird`)<br>**WS 2**: WhatsApp Web (`omarchy-launch-webapp`) | Unified communication hub for email and messaging. |
| **`media`** | `chill`, `music`, `relax` | **WS 1**: YouTube (`omarchy-launch-browser youtube.com`)<br>**WS 2**: Music Player (`cliamp`) | Relaxed media setup for videos and background music playback. |

### Spoken Voice Prompts
* *"Jarvis, open my dev workflow"*
* *"Launch the coding setup"*
* *"Start research workflow"*
* *"What workflows do I have available?"*
* *"Show me the details for the coding workflow"*
* *"Save my current windows as a workflow called dev-review"* (auto-captures open Hyprland applications!)
* *"Create a workflow called trading with browser on workspace 1 and telegram on workspace 2"*
* *"Delete the trading workflow"*

### Dedicated Workflow CLI Commands
```bash
# List all configured workflows with workspaces and aliases
jarvis --workflow-list

# Show detailed launch steps of a specific workflow
jarvis --workflow-show coding

# Capture your currently open application windows as a new workflow preset
jarvis --workflow-capture dev-review

# Launch a workflow directly from the terminal
jarvis --workflow-launch coding

# Delete a custom workflow preset
jarvis --workflow-delete dev-review
```

---

## 2. Custom Workflows & Instant Window Capture

Jarvis enables two ways to create custom workflows:
1. **Automated Window Snapshotting (`capture_current_workflow`)**: Open whatever apps you need across workspaces 1, 2, 3, etc., then simply say *"Jarvis, save my current windows as a workflow called review"*. Jarvis queries Hyprland's socket (`clients`), identifies the open applications, generates the launch steps, and saves them to `workflows.json`.
2. **Natural Voice Construction (`save_custom_workflow`)**: Say *"Create a workflow called social with thunderbird on workspace 1 and slack on workspace 2"*.

All workflows are stored in human-readable JSON at:
```bash
~/.config/jarvis/workflows.json
```

You can also edit this file directly. Changes are loaded immediately without restarting background services.


### Workflow Configuration Schema

```json
{
  "trading": {
    "description": "Financial charts on workspace 1, news and portfolio on workspace 2",
    "aliases": ["stocks", "crypto", "finance"],
    "primary_workspace": 1,
    "steps": [
      {
        "workspace": 1,
        "launch": "omarchy-launch-browser https://tradingview.com",
        "delay": 0.25
      },
      {
        "workspace": 2,
        "launch": "omarchy-launch-browser https://news.ycombinator.com",
        "delay": 0.25
      }
    ]
  }
}
```

### Schema Properties
* **`description`** *(string)*: Short description used when Jarvis lists workflows.
* **`aliases`** *(array of strings)*: Natural speech variations and synonyms for triggering the workflow.
* **`primary_workspace`** *(integer)*: The workspace Hyprland will focus on after all applications are launched.
* **`steps`** *(array of objects)*:
  * **`workspace`**: Target Hyprland workspace number (e.g. `1`, `2`, `3`).
  * **`launch`**: Shell command or executable binary with arguments (e.g. `thunderbird`, `omarchy-launch-editor`, `obsidian`).
  * **`delay`**: Delay in seconds (e.g. `0.25`) between launches to let window rules and tiling apply smoothly.

---

## 3. Calendar & Event Scheduling

Jarvis provides natural date and time parsing for scheduling meetings and events without needing rigid syntax.

### How It Works
1. **Natural Language Parser**: Parses relative times (*"tomorrow 3pm"*, *"Friday 10am"*, *"day after tomorrow at 2:30pm"*, *"today 18:00"*, *"next Monday 9am"*).
2. **Google Calendar Web**: Automatically constructs and opens a Google Calendar template URL with title, description, and time window prefilled.
3. **Local RFC 5545 `.ics` Files**: Generates standard iCalendar `.ics` files saved in:
   ```bash
   ~/.local/share/jarvis/events/
   ```
   Compatible with Thunderbird, KOrganizer, Google Calendar, and Apple Calendar.
4. **Thunderbird Local Calendar**: When requested with local calendar provider, directly launches `thunderbird -file <event.ics>` to import into your local calendar.

### Spoken Voice Examples
* *"Schedule a meeting with Alex tomorrow at 3pm"*
* *"Schedule dentist appointment on Friday at 10am"*
* *"Create an event for project retrospective next Monday at 2pm"*

### CLI Example
```bash
jarvis -c "schedule team sync for tomorrow at 4pm" --no-speech
```

---

## 4. Systemd Countdown Reminders

Jarvis interfaces directly with the native **`omarchy-reminder`** system and user systemd timers.

### Features
* **Zero Background Overhead**: Uses systemd user timer units rather than sleeping background threads.
* **Desktop Notifications**: Triggers native desktop notifications with sound and visual badges when the countdown completes.
* **Status Bar Integration**: Displays active countdowns in the Omarchy top bar tray indicator.

### Spoken Voice Examples
* *"Remind me in 30 minutes to drink water"*
* *"Remind me in 15 minutes to check the oven"*
* *"Remind me in 2 hours to submit the PR"*
* *"List my reminders"*
* *"Clear my reminders"*

### CLI Examples & Verification
```bash
# Set a reminder via Jarvis
jarvis -c "remind me in 45 minutes to stretch" --no-speech

# Check active timers directly with systemd/omarchy-reminder
omarchy-reminder show --json

# Clear active reminders
jarvis -c "clear my reminders" --no-speech
```

---

## 5. Email Drafting & Review

Jarvis drafts professional email correspondence and immediately brings up compose windows with all fields prefilled for your review and final approval.

### Safe by Design
Jarvis **never sends emails autonomously without user review**. It composes the draft and opens your compose client so you can review, edit, and click "Send".

### Supported Email Clients
1. **Mozilla Thunderbird** (`thunderbird -compose`): Opens native desktop compose window prefilling `to`, `subject`, and formatted `body`.
2. **Gmail Web Compose**: Opens Google Chrome / default browser at `mail.google.com/mail/?view=cm...` with all parameters encoded.
3. **Standard Mailto**: Falls back to `xdg-open mailto:...` if other clients are not detected.

### Spoken Voice Examples
* *"Draft an email to bob@example.com about our project update"*
* *"Compose an email to HR asking about vacation rollover policy"*
* *"Draft an email in Gmail to team@company.org about tomorrow's agenda"*

### CLI Example
```bash
jarvis -c "draft an email to client@example.com about quarterly deliverable" --no-speech
```

---

## 6. Quick Notes & Scratchpad

Jarvis captures thoughts, meeting minutes, and scratchpad items directly to local markdown files.

### Storage & Structure
* All notes are saved as Markdown (`.md`) files in:
  ```bash
  ~/Notes/
  ```
* **Automatic Timestamps**: Every note includes the full date and timestamp of creation or update.
* **Append on Existing**: If a note with the same title already exists, Jarvis appends the new text under an updated timestamp separator rather than overwriting.
* **Obsidian Integration**: If requested or when `open_editor=True`, Jarvis immediately opens the note in Obsidian or your configured `$EDITOR`.

### Spoken Voice Examples
* *"Take a note: ideas for the next release"*
* *"Jot down grocery list: milk, sourdough, dark chocolate, coffee beans"*
* *"Note down meeting discussion: deploy migration scripts before midnight"*
* *"List my recent notes"*

### CLI Examples
```bash
# Take a note silently
jarvis -c "take a note: buy groceries and check system logs" --no-speech

# List recent notes
jarvis -c "list my notes" --no-speech
```

---

## 7. Autonomous Project & Code Generation (Claude Code, Codex, Antigravity)

Jarvis features an extensible autonomous coding engine that drives your preferred AI CLI coding assistant: **Claude Code (`claude`)**, **Codex CLI (`codex`)**, or **Antigravity CLI (`agy`)**, configured via `JARVIS_CLI_AI_TOOL` in `.env`.

### Supported CLI Engines
* **Claude Code (`claude`)**: Default option (`JARVIS_CLI_AI_TOOL="claude"`). Invoked with `--dangerously-skip-permissions` and headless print mode (`-p`) or interactive terminal mode.
* **Antigravity CLI (`agy`)**: Invoked with `--dangerously-skip-permissions`, workspace directory flag (`--add-dir <dir>`), and headless prompt (`-p`) or interactive UI (`-i`).
* **OpenAI Codex CLI (`codex`)**: Invoked with `--dangerously-bypass-approvals-and-sandbox`, non-interactive auto-execution (`exec -a never`), and directory binding (`-C <dir>`).

### Autonomous Permissions & Never Blocking
When invoked through Jarvis, whichever tool is active is launched with its respective non-interactive permission flags. This allows the AI agent to write files, create directories, format code, and initialize repositories without hanging or blocking on manual TTY confirmation prompts.

### Dual Execution Modes

1. **Background Autonomous Mode (Default)**:
   - Jarvis immediately confirms with a spoken voice reply (~2 seconds) and continues listening.
   - Spawns a detached background session (`setsid`) that continues running even if Jarvis's conversational turn concludes.
   - Outputs logs to `<project_dir>/.<tool>_scaffold.log`.
   - Sends an **Omarchy desktop notification** when project scaffolding is complete.

2. **Interactive Floating Popup Modal (`open_terminal=True`)**:
   - When asked to "show me", "open in a popup", or for higher-level permissions / visual review, Jarvis opens a centered floating modal terminal (`TUI.float`) running the tool interactively.
   - Matched by Omarchy Hyprland's `floating-window` rules: automatically centered at 875x600px.
   - Allows watching code being written live, reviewing diffs, and continuing interactive conversational development.

### Spoken Voice Examples
* *"Jarvis, create a python CLI tool called log_analyzer in my home directory"*
* *"Create a React dashboard project called admin-portal in my Projects folder"*
* *"Build a Rust web scraper in a popup window"*
* *"Create a complete project called task-tracker with unit tests and a README"*

### CLI Examples
```bash
# Create a project in the background
jarvis -c "create a python CLI called csv_parser in ~/Codes/personal/csv_parser" --no-speech

# Create a project with an interactive popup terminal
jarvis -c "create a rust app called fast_grep in a popup window" --no-speech
```

---

## 8. Conversational Flow & Control

### Multi-Turn Conversation
After executing an action, Jarvis remains active and listening for follow-up instructions (e.g. *"Now open YouTube"* or *"Also set a reminder for 10 minutes"*).

### Dismissal / Kill Words
To dismiss Jarvis and conclude the conversation session, simply say:
* *"That's it, thank you"*
* *"Done"*
* *"That'll be all, Jarvis"*
* *"Goodbye"*
* *"Dismiss"*

Jarvis will acknowledge with a polite farewell and cleanly hide the HUD overlay.

### Hardware Hotkeys & Emergency Stop
* **`SUPER + SHIFT + RETURN`**: Push-to-talk key.
  * Hold while speaking and release to submit immediately.
  * Or tap once to toggle continuous conversational listening.
* **`SUPER + ALT + ESCAPE`**: Emergency kill switch.
  * Immediately silences audio output, cancels ongoing LLM requests, terminates active processes, and dismisses the HUD.
