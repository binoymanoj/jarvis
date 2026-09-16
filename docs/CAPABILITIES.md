# 󰚩 Jarvis: Complete Capabilities & Feature Reference

Jarvis is a voice-first, multi-turn AI desktop assistant built specifically for **Omarchy Linux** and the **Hyprland** tiling window compositor. It natively bridges conversational intelligence with operating system orchestration, developer tooling, workflow management, and day-to-day productivity.

---

## 📑 Table of Contents
1. [Autonomous Project & Code Generation (Claude Code, Codex, Antigravity)](#1-autonomous-project--code-generation-claude-code-codex-antigravity)
2. [Multi-Workspace Workflow Orchestration](#2-multi-workspace-workflow-orchestration)
3. [Calendar Scheduling & Systemd Countdown Reminders](#3-calendar-scheduling--systemd-countdown-reminders)
4. [Email Composition & Safe Review](#4-email-composition--safe-review)
5. [Markdown Notes & Scratchpad](#5-markdown-notes--scratchpad)
6. [Desktop, Window & Hardware Control (Hyprland & Omarchy)](#6-desktop-window--hardware-control-hyprland--omarchy)
7. [Hands-Free Virtual Input & Navigation (Zero-Touch)](#7-hands-free-virtual-input--navigation-zero-touch)
8. [Universal Linux Shell Execution](#8-universal-linux-shell-execution)
9. [Media Playback & Track Control (MPRIS D-Bus)](#9-media-playback--track-control-mpris-d-bus)
10. [Wayland Clipboard Management](#10-wayland-clipboard-management)
11. [System Power & Hardware Diagnostics](#11-system-power--hardware-diagnostics)
12. [Multimodal Screen Perception & Vision](#12-multimodal-screen-perception--vision)
13. [Web & Media Navigation](#13-web--media-navigation)
14. [Quickshell HUD Overlay & Dynamic Theming](#14-quickshell-hud-overlay--dynamic-theming)
15. [Audio Pipeline & Voice Engine](#15-audio-pipeline--voice-engine)
16. [Hands-Free Wake Word Detection ("Hey Jarvis")](#16-hands-free-wake-word-detection-hey-jarvis)
17. [Omarchy Menu Bar Icon & Popover Controls](#17-omarchy-menu-bar-icon--popover-controls)
18. [Push-to-Talk, Hotkeys & Emergency Kill Switch](#18-push-to-talk-hotkeys--emergency-kill-switch)
19. [Systemd Background Service & Daemon](#19-systemd-background-service--daemon)
20. [Continuous Conversation & Kill Words](#20-continuous-conversation--kill-words)
21. [Quota Resiliency & Multi-Tier Fallback](#21-quota-resiliency--multi-tier-fallback)
22. [Performance Benchmarks & Resource Profiles](#22-performance-benchmarks--resource-profiles)
23. [CLI Commands Reference](#23-cli-commands-reference)


---

## 1. Autonomous Project & Code Generation (Claude Code, Codex, Antigravity)

Jarvis features a modular, pluggable coding engine that drives your choice of autonomous CLI AI tool: **Claude Code (`claude`)**, **Codex CLI (`codex`)**, or **Antigravity CLI (`agy`)**.

### Configurable Engine (`JARVIS_CLI_AI_TOOL`)
Configure your preference in `.env`:
* `JARVIS_CLI_AI_TOOL="claude"` (Default) — Uses Anthropic's Claude Code CLI.
* `JARVIS_CLI_AI_TOOL="agy"` — Uses Google's Antigravity CLI.
* `JARVIS_CLI_AI_TOOL="codex"` — Uses OpenAI Codex CLI.

### Key Features
* **Zero-Friction Permissions**: 
  - Claude Code: uses `--dangerously-skip-permissions` with print flag `-p`.
  - Antigravity: uses `--dangerously-skip-permissions` with `--add-dir <target_path>`.
  - Codex: uses `--dangerously-bypass-approvals-and-sandbox` with `-a never` and `-C <target_path>`.
  Eliminates blocking TTY confirmation prompts so projects scaffold end-to-end autonomously.
* **Workspace Path Binding**: Automatically targets home (`~/<project>`), `~/Codes/personal/<project>`, `~/Projects/<project>`, or any custom path.
* **Dual Execution Modes**:
  1. **Background Autonomous Mode (Default)**: Jarvis confirms immediately via spoken voice (~2 seconds) and continues listening. A detached background session (`setsid`) builds the project independently, logs to `.<tool>_scaffold.log`, and triggers a desktop notification when finished.
  2. **Interactive Floating Popup Modal (`open_terminal=True`)**: Saying *"in a popup window"* or asking for visual review launches a centered floating modal terminal (`TUI.float`, sized `875x600` via Omarchy window rules) running the tool interactively (`claude`, `agy -i`, or `codex`) so you can watch code being generated live.

### Voice Commands
* *"Jarvis, create a python CLI tool called log_analyzer in my home directory"*
* *"Create a React dashboard project called analytics-portal in my Projects folder"*
* *"Build a Rust web scraper in a popup window"*
* *"Scaffold a project called task-tracker with unit tests and a README"*

---

## 2. Multi-Workspace Workflow Orchestration

Launch complete multi-application desktop environments across Hyprland workspaces with a single command.

### Preset Workflows
* **`coding`** (Aliases: `dev`, `code`, `development`):
  - Workspace 1: Code editor (`omarchy-launch-editor` / Neovim) & Terminal.
  - Workspace 2: Web browser (`omarchy-launch-browser`).
* **`research`** (Aliases: `study`, `reading`):
  - Workspace 1: Web browser.
  - Workspace 2: Obsidian knowledge base (`obsidian`).
* **`writing`** (Aliases: `notes`, `write`, `drafting`):
  - Workspace 1: Obsidian notes.
  - Workspace 2: Reference browser.
* **`communication`** (Aliases: `social`, `mail`, `messaging`):
  - Workspace 1: Thunderbird email (`thunderbird`).
  - Workspace 2: WhatsApp Web (`omarchy-launch-webapp`).
* **`media`** (Aliases: `chill`, `music`, `relax`):
  - Workspace 1: YouTube browser.
  - Workspace 2: Music player (`cliamp`).

### Custom Workflows & Window Capture
* **Instant Window Snapshotting (`capture_current_workflow`)**: Ask Jarvis to capture your currently open applications across workspaces into a named preset without manually writing JSON.
* **Natural Voice Creation (`save_custom_workflow`)**: Build workflows via conversational commands specifying target workspaces and application binaries.
* **Human-Readable Storage**: Stored at `~/.config/jarvis/workflows.json` with step delays, aliases, and primary workspace focus.
* **Inspect & Delete**: Ask Jarvis to show details for any workflow or delete obsolete presets.

### Voice Commands
* *"Jarvis, open my dev workflow"*
* *"Launch the coding setup"*
* *"Start research workflow"*
* *"What workflows do I have available?"*
* *"Show me the details for the coding workflow"*
* *"Save my current windows as a workflow called dev-review"*
* *"Create a workflow called trading with browser on workspace 1 and telegram on workspace 2"*
* *"Delete the trading workflow"*

### CLI Commands
```bash
jarvis --workflow-list
jarvis --workflow-show coding
jarvis --workflow-capture dev-review
jarvis --workflow-launch coding
jarvis --workflow-delete dev-review
```


---

## 3. Calendar Scheduling & Systemd Countdown Reminders

Manage schedule events and active desktop reminders using natural language.

### Natural Language Event Scheduling
* Parses relative and absolute dates and times: *"tomorrow 3pm"*, *"Friday 10am"*, *"day after tomorrow at 2:30pm"*, *"today 18:00"*, *"next Monday 9am"*.
* **Google Calendar**: Generates and opens prefilled Google Calendar template URLs with event title, description, and time window.
* **Local iCalendar (`.ics`)**: Generates standard RFC 5545 `.ics` event files in `~/.local/share/jarvis/events/` and imports into Thunderbird via `thunderbird -file <path.ics>`.

### Systemd Countdown Reminders
* Interfaces directly with native **`omarchy-reminder`** and user systemd timers.
* Consumes **zero background CPU** while waiting.
* Triggers native desktop notification banners and updates the Omarchy top bar indicator.

### Voice Commands
* *"Schedule a meeting with Alex tomorrow at 3pm"*
* *"Schedule dentist appointment on Friday at 10am"*
* *"Remind me in 30 minutes to drink water"*
* *"Remind me in 15 minutes to check the oven"*
* *"List my reminders"*
* *"Clear my reminders"*

---

## 4. Email Composition & Safe Review

Jarvis drafts professional email correspondence while enforcing a safe-by-design human-in-the-loop workflow.

### How It Works
* Jarvis formulates polished subject lines and message bodies tailored to your prompt.
* **Never sends emails autonomously without review**: Immediately opens your preferred compose window with all fields prefilled so you can review, edit, and click "Send".
* Supported clients:
  - **Mozilla Thunderbird**: Native desktop compose via `thunderbird -compose "to=...,subject=...,body=..."`.
  - **Gmail Web**: Webmail compose via `mail.google.com/mail/?view=cm...`.
  - **Standard Mailto**: Standard `xdg-open mailto:...` fallback.

### Voice Commands
* *"Draft an email to bob@example.com about our project update"*
* *"Compose an email to HR asking about vacation rollover policy"*
* *"Draft an email in Gmail to team@company.org about tomorrow's agenda"*

---

## 5. Markdown Notes & Scratchpad

Capture thoughts, meeting minutes, and scratchpad items directly to local markdown files.

### Features
* All notes are saved to `~/Notes/<title>.md`.
* Automatically includes full creation timestamps.
* **Append on Existing**: If a note with the same title exists, new thoughts are appended under an updated timestamp separator rather than overwritten.
* **Obsidian & Editor Integration**: Opens automatically in Obsidian or your default `$EDITOR` upon request.

### Voice Commands
* *"Take a note: ideas for the next release"*
* *"Jot down grocery list: milk, sourdough, dark chocolate, coffee beans"*
* *"Note down meeting discussion: deploy migration scripts before midnight"*
* *"List my recent notes"*

---

## 6. Desktop, Window & Hardware Control (Hyprland & Omarchy)

Control your desktop environment natively without touching the mouse or keyboard.

### Window & Workspace Management
* Switch workspaces: *"Switch to workspace 2"*, *"Go to workspace 4"*.
* Focus apps: *"Focus Kitty"*, *"Focus browser"*, *"Focus Neovim"*.
* Window controls: *"Close this window"*, *"Toggle fullscreen"*, *"Toggle split orientation"*.

### Hardware & System Controls
* Volume: *"Volume up 10%"*, *"Volume down 5%"*, *"Mute audio"*.
* Screen brightness: *"Set brightness to 70%"*, *"Increase brightness 10%"*.
* Battery: *"What is my battery level?"*, *"Battery status"*.
* Application launcher: *"Launch terminal"*, *"Launch browser"*, *"Open Nautilus"*.

### Dynamic System Theming
* Switch themes: *"Change theme to catppuccin"*, *"Switch to tokyo-night"*, *"Set theme to solitude"*.
* Real-time HUD color synchronization with Omarchy palettes (`colors.toml`, `theme.name`).

---

## 7. Hands-Free Virtual Input & Navigation (Zero-Touch)

Operate focused applications without touching the keyboard or mouse using Wayland's native `wtype` integration.

### Keystroke Injection & Typing
* **Arbitrary Text Typing**: Injects text safely into the active input field or application window via stdin.
* **Return/Submit Flag**: Supports automatic enter/submission after typing (`enter_after=True`).
* **Key Pressing**: Press specific keyboard keys hands-free (`Return`, `Tab`, `Escape`, `BackSpace`, `space`, `Up`, `Down`, `Page_Down`, `Home`, `End`).
* **Keyboard Shortcuts**: Synthesizes multi-modifier shortcuts (e.g. `ctrl+s`, `ctrl+shift+t`, `alt+tab`, `ctrl+c`, `ctrl+v`).
* **Document Scrolling**: Scrolls active windows up or down smoothly.

### Voice Commands
* *"Jarvis, type 'git commit -m \"feat: initial release\"' and press enter"*
* *"Type my email address binoy@example.com"*
* *"Press enter"*
* *"Save this file"* (sends `ctrl+s`)
* *"Reopen my last closed tab"* (sends `ctrl+shift+t`)
* *"Scroll down"* / *"Scroll up"*

---

## 8. Universal Linux Shell Execution

Execute arbitrary bash commands and scripts on demand with built-in safety controls.

### Features
* Executes commands directly in your active shell environment (`/bin/bash`).
* **Safety Timeout**: Enforces a strict 15.0-second execution timeout to prevent runaway processes.
* **Smart Output Truncation**: Automatically buffers and limits terminal output (up to 2,000 characters) to prevent overflowing LLM context limits while preserving head and tail diagnostics.
* **Error Code Inspection**: Returns stderr and exit codes to the agent for self-correction.

### Voice Commands
* *"Jarvis, run git status in this repository"*
* *"Check available disk space with df -h"*
* *"List the files in my Downloads directory"*
* *"How many Docker containers are currently running?"*

---

## 9. Media Playback & Track Control (MPRIS D-Bus)

Control multimedia playback across any MPRIS-compliant media player (Chromium, YouTube, Spotify, Firefox, mpv, VLC) with zero extra dependencies using native D-Bus calls.

### Features
* **Full Transport Control**: Play/Pause toggle, Next track, Previous track, Stop.
* **Live Metadata Query**: Reads real-time track title, artist name, and player application name.
* **Universal Compatibility**: Works out of the box with web browsers and native desktop media players.

### Voice Commands
* *"Jarvis, pause the music"* / *"Play"*
* *"Skip to the next song"*
* *"Go back to the previous track"*
* *"What song is playing right now?"*
* *"Stop media playback"*

---

## 10. Wayland Clipboard Management

Access and modify the Wayland system clipboard hands-free via `wl-clipboard` (`wl-paste` and `wl-copy`).

### Features
* **Read Clipboard**: Inspects active text on the clipboard without clicking.
* **Write to Clipboard**: Copies synthesized text, code snippets, formatted links, or answers directly into the system clipboard.

### Voice Commands
* *"What is currently copied to my clipboard?"*
* *"Copy that last command to my clipboard"*
* *"Put my SSH public key onto the clipboard"*

---

## 11. System Power & Hardware Diagnostics

Perform system power operations, Bluetooth management, and network throughput testing.

### Features
* **Lock & Power State**: Lock screen (`omarchy system lock`), logout (`omarchy system logout`), reboot (`omarchy system reboot`), and shutdown (`omarchy system shutdown`).
* **Bluetooth Controls**: Toggle Bluetooth adapter power or check power state.
* **Hardware Diagnostics**: Real-time CPU utilization, RAM usage, and active Wi-Fi connection info.
* **Network Speedtest**: Measures live download or upload throughput via `omarchy network speedtest`.

### Voice Commands
* *"Lock my computer"*
* *"Reboot the system"*
* *"Turn Bluetooth off"* / *"Toggle Bluetooth"*
* *"How is system performance doing?"* (queries CPU, memory, network load)
* *"Run an internet speed test"*

---

## 12. Multimodal Screen Perception & Vision

Ask Jarvis to visually inspect what is currently on your screen.

### Features
* Takes an instantaneous snapshot of the active window or entire screen via `grim`.
* Transmits the image directly to Gemini's multimodal vision engine.
* Diagnoses compiler errors, layout bugs, terminal traces, and GUI states.
* Image snapshots are securely purged from disk immediately after inspection.

### Voice Commands
* *"Take a look at my screen and tell me why this build failed"*
* *"Inspect this error on my screen"*
* *"What is currently displayed in this window?"*

---

## 13. Web & Media Navigation

Instantly access search engines, online videos, and websites.

### Features
* **Web Search**: Opens your default browser with direct search queries.
* **YouTube Playback**: Directly searches and plays videos on YouTube.
* **URL Navigation**: Opens any web domain or URL in your default browser.

### Voice Commands
* *"Search the web for Arch Linux PipeWire configuration"*
* *"Open MKBHD's latest video on YouTube"*
* *"Open github.com"*


---

---

## 14. Quickshell HUD Overlay & Dynamic Theming

A custom floating Heads-Up Display built using **Quickshell** matching the **Omarchy Linux** aesthetic.

### Visual Design
* **Floating Glass Capsule Dock**: Centered at the bottom with rounded corners (`radius: 18`) and translucent obsidian glass (`#101315` at 92% opacity).
* **Specular Top Highlight**: Subtle inner glass reflection matching Omarchy system popups.
* **Responsive Width**: Dynamically expands and contracts (340px to 620px) with smooth cubic easing.
* **Futuristic AI Resonator Orb (48x48px)**:
  - Rotating precision telemetry reticle with micro-ticks.
  - Energy halo that expands and breathes in sync with microphone volume.
  - Cosmic glass lens with 3 layered sinusoidal waveforms using additive light blending.
  - State color changes: Listening (Theme accent), Reasoning (Violet quantum surge with orbiting sparks), Speaking (Bright cyan/accent breathing rhythm).
* **Bouncing Audio Equalizer**: 4 vertical equalizer bars that bounce live with speech volume.
* **System Telemetry**: Displays the active Omarchy theme (e.g. `// SOLITUDE • GEMINI`) in `JetBrainsMono Nerd Font`.
* **Crisp Antialiased Typography**: Antialiased system sans-serif text with clean readability and zero clunky text outlines.

---

## 15. Audio Pipeline & Voice Engine

Optimized for near-instantaneous speech recognition and natural local voice synthesis.

* **Speech-to-Text (STT)**: Groq Whisper Large v3 Turbo (~420ms ultra-low latency transcription) with local faster-whisper fallback.
* **Text-to-Speech (TTS)**: Piper local neural TTS (`en_GB-alan-medium`, sophisticated British persona, ~157ms synthesis) with Edge TTS online fallback.
* **Voice Activity Detection (VAD)**: Silero VAD ONNX model for real-time speech probability analysis and 0.85s trailing silence termination.

---

## 16. Hands-Free Wake Word Detection ("Hey Jarvis")

Jarvis features continuous, zero-latency offline wake word detection powered by `openWakeWord` and the pre-trained `hey_jarvis_v0.1.onnx` neural model.
* **Hands-Free Activation**: Speak *"Hey Jarvis"* or *"Jarvis"* anytime to awaken the assistant without needing to touch the keyboard.
* **Zero Cloud Latency**: Processes 16kHz audio in 80ms chunks on CPU with minimal resource usage (~1-2% total system CPU).
* **Configurable Sensitivity**: Tuned by default to `0.28` in `.env` via `JARVIS_WAKEWORD_THRESHOLD=0.28` to reliably trigger on both *"Hey Jarvis"* and *"Jarvis"*.
* **Dynamic Soft AGC**: Automatically boosts low-volume microphone streams (such as Bluetooth headsets or laptop internal mics) so wake word embeddings remain clear and accurate.
* **Instant Audio Chime**: Plays a subtle two-tone wake chime the moment the wake word triggers to confirm that Jarvis has awakened and is listening.

---

## 17. Omarchy Menu Bar Icon & Popover Controls

Jarvis integrates natively into the **Omarchy Top Bar** (Quickshell) as a first-class bar-widget plugin (`~/.config/omarchy/plugins/jarvis/`).

### Top Bar Indicator & Microphone Dot
* **Native Bar Icon**: Displays the Jarvis robot glyph (`󰚩`) in your top bar, fully integrated with Omarchy theme colors.
* **Microphone Active Indicator Dot**: Whenever Jarvis is actively recording audio (`mic_active: true`), a bright pulsing red dot appears on the top-right corner of the icon. When the microphone is idle or muted, the dot vanishes.
* **Interaction**:
  - **Left-Click**: Toggles the rich Popover Control Panel.
  - **Right-Click / Middle-Click**: Instantly triggers / toggles voice listening (`jarvis -t`).

### Popover Control Panel
Clicking the menu bar icon reveals a floating `KeyboardPanel` popover offering quick controls:
1. **Hero Header**: Status pill showing real-time state (`Listening`, `Thinking`, `Speaking`, `Standby`).
2. **Instant Actions**: One-click **Listen Now** (mic toggle) and **Kill Switch** (emergency silence).
3. **Voice Settings**: Toggle hands-free Wake Word detection on/off with an interactive switch.
4. **Quick Workflows**: Launch Dev Setup, Screen Vision snapshot, Take Note, or Media Hub directly.
5. **System Controls**:
   - **Restart** (`󰑐`): Executes `jarvis --restart` to reload the user systemd service and rescan Omarchy menubar plugins.
   - **Quit Jarvis** (`󰗼`): Executes `jarvis --quit` to cleanly stop `systemctl --user stop jarvis`, kill running background daemon processes and Quickshell HUD overlays, stop audio streams, and free up all CPU and memory.
6. **Telemetry Footer**: Displays the active coding CLI engine (`AGY`, `CLAUDE`, `CODEX`) and hotkey tips.

---

## 18. Push-to-Talk, Hotkeys & Emergency Kill Switch

Configured natively in `~/.config/hypr/bindings.lua`:

| Shortcut | Action | Description |
| :--- | :--- | :--- |
| **`SUPER + SHIFT + ENTER`** | **Push-to-Talk (Hold / Toggle)** | **Hold**: Hold while speaking, release to submit immediately.<br>**Tap**: Tap once to toggle continuous listening mode. |
| **`SUPER + ALT + ESCAPE`** | **Emergency Kill Switch** | Instantly silences audio, terminates active LLM thinking, cancels background tasks, and dismisses the HUD. |

---

## 19. Systemd Background Service & Daemon

Jarvis can run continuously in the background listening for wake words via user systemd service:

```bash
# Start background daemon immediately
systemctl --user start jarvis

# Enable daemon to run automatically on login
systemctl --user enable jarvis

# Restart background daemon and rescan menubar
systemctl --user restart jarvis
# Or via Jarvis CLI helper:
jarvis --restart

# Completely stop background daemon and release all CPU/RAM
systemctl --user stop jarvis
# Or via Jarvis CLI helper:
jarvis --quit

# Check service status and logs
systemctl --user status jarvis
journalctl --user -u jarvis -f
```

---

## 20. Continuous Conversation & Kill Words

Jarvis maintains context across sequential commands within the same session. After executing an action, Jarvis remains active and attentive for follow-up requests.

### Dismissal Phrases
When you are done, speak any natural dismissal phrase:
* *"That's it, thank you"*
* *"Done"*
* *"That'll be all, Jarvis"*
* *"Goodbye"*
* *"Dismiss"*

Jarvis will acknowledge with a polite farewell and cleanly hide the HUD overlay.

---

## 21. Quota Resiliency & Multi-Tier Fallback

Eliminates API quota lockouts and delays through automatic cascading fallback:
1. **`gemini-3.5-flash-lite`** *(Primary)*: Sub-second response time with high free-tier rate limits.
2. **`gemini-3.5-flash`** *(Fallback Tier 1)*
3. **`gemini-2.5-flash`** *(Fallback Tier 2)*
4. **`gemini-2.5-flash-lite`** *(Fallback Tier 3)*
5. **`gemini-flash-latest`** *(Fallback Tier 4)*
6. **`gemini-flash-lite-latest`** *(Fallback Tier 5)*

If any model encounters a 429 quota exhaustion or rate limit, Jarvis seamlessly falls back to the next available model in real time without dropping the conversation.

---

## 22. Performance Benchmarks & Resource Profiles

For granular micro-benchmarks, hardware profiling, latency timelines, and idle/active memory graphs, refer to the full report in:
**[Performance & Specifications Document](file:///home/binoy/Codes/personal/jarvis/docs/PERFORMANCE_AND_SPECS.md)**

### Quick Resource Summary:
* **Background Idle Memory (RSS)**: ~373 MB (Daemon) + ~224 MB (Quickshell HUD) = **~597 MB total** (<1.9% of system RAM).
* **Background Idle CPU**: **~10-12% of 1 core** (~1.3% total CPU capacity).
* **Active Turn Memory**: ~408 MB peak.
* **Active Turn CPU**: ~35-55% of 1 core burst during Piper ONNX synthesis (~150ms).
* **Network Footprint**: 0 KB/s idle (100% offline listening); ~25-80 KB per active voice turn.
* **End-to-End Perceived Turnaround**: **~1.25 - 1.55 seconds**.

---

## 23. CLI Commands Reference

Jarvis can be invoked directly from the terminal or scripted into custom Hyprland hotkeys:

```bash
# Start background daemon listening for 'Hey Jarvis' wake word
jarvis -d

# Restart Jarvis systemd daemon and refresh Omarchy menubar plugin
jarvis -r
jarvis --restart

# Completely stop and quit all Jarvis processes, daemons, and systemd service
jarvis -q
jarvis --quit

# Restart Omarchy Quickshell top bar to cleanly reload QML widgets/popups
omarchy-restart-shell

# Check wake word detection and background daemon status
jarvis --wakeword-status

# Toggle wake word detection on or off
jarvis --wakeword-toggle

# Trigger push-to-talk toggle / start voice session
jarvis -t

# Trigger push-to-talk release (stops recording and processes audio)
jarvis -s

# Emergency kill switch (silences active speech, interrupts LLM, and hides HUD)
jarvis -k

# Execute a single command via voice response
jarvis -c "what is my battery level"

# Execute a command silently (text output only, no TTS speech)
jarvis -c "open my dev workflow" --no-speech

# Scaffold a project autonomously via configured CLI AI tool (agy, claude, or codex)
jarvis -c "create a python CLI called csv_tool in ~/Codes/personal/csv_tool" --no-speech

# Scaffold a project in an interactive floating popup modal
jarvis -c "create a rust app called fast_grep in a popup window" --no-speech
```

---

## 📄 Related Documentation
* [Performance Benchmarks & Technical Specifications](file:///home/binoy/Codes/personal/jarvis/docs/PERFORMANCE_AND_SPECS.md)
* [Rust Migration Roadmap (Future Memory Optimization)](file:///home/binoy/Codes/personal/jarvis/docs/RUST_MIGRATION_TODO.md)
* [Multi-Workspace Workflows Guide](file:///home/binoy/Codes/personal/jarvis/docs/WORKFLOW.md)
* [System Audit & Complete Uninstallation Guide](file:///home/binoy/Codes/personal/jarvis/docs/CLEANUP.md)
* [Architecture & Todo Roadmap](file:///home/binoy/Codes/personal/jarvis/docs/ARCHITECTURE_AND_TODO.md)


