# 󰚩 Jarvis: Intelligent Linux Desktop Assistant

Jarvis is a voice-first, multi-turn AI desktop assistant built specifically for **Omarchy Linux** and the **Hyprland** tiling window manager. Powered by Google Gemini Flash and Groq Whisper, Jarvis brings native OS orchestration, desktop workflows, calendar scheduling, email composition, and note capture to your fingertips.

---

## 🌟 Key Capabilities

### 1. 🚀 Multi-Workspace Workflow Automation
Launch full-blown multi-window development and productivity environments across Hyprland workspaces with a single command:
* **Coding / Dev**: Neovim & Terminal on Workspace 1, Web Browser on Workspace 2.
* **Research**: Browser on Workspace 1, Obsidian notes on Workspace 2.
* **Writing**: Obsidian editor on Workspace 1, Browser on Workspace 2.
* **Communication**: Thunderbird email on Workspace 1, WhatsApp Web on Workspace 2.
* **Media / Chill**: YouTube on Workspace 1, music player on Workspace 2.
* *Custom Workflows*: Easily customized in `~/.config/jarvis/workflows.json`.

### 2. 📅 Calendar & Reminders
* **Natural Date/Time Scheduling**: "Schedule a meeting with Alex tomorrow at 3pm", "Set up a dentist appointment on Friday at 10am". Creates Google Calendar entries and RFC 5545 `.ics` files ready for Thunderbird.
* **Desktop Reminders**: "Remind me in 25 minutes to check the oven", "Remind me in an hour to take a break". Fully integrated with `omarchy-reminder` and user systemd timers.
* **Manage Reminders**: "List my reminders", "Clear my reminders".

### 3. ✉️ Email Drafting
* **Smart Drafts**: "Draft an email to john@example.com about the project kickoff", "Send an email to HR asking about vacation policy".
* **Immediate Review**: Opens Thunderbird (`thunderbird -compose`) or Gmail web compose window prefilled with recipient, subject, and polished message body for your review before sending.

### 4. 📝 Notes & Thought Capture
* **Instant Notes**: "Take a note: ideas for the next release", "Jot down grocery list: milk, coffee, apples".
* Automatically timestamps and formats markdown files into `~/Notes/*.md`, with optional auto-opening in Obsidian or Neovim.
* "List my notes" to see recent thoughts.

### 5. 🤖 Autonomous Project & Code Generation (Claude Code, Codex, Antigravity)
* **Pluggable CLI AI Engine**: Choose your preferred coding agent via `JARVIS_CLI_AI_TOOL` (`claude` [default], `agy`, or `codex`).
* **Autonomous Project Scaffolding**: "Create a python CLI tool called log_analyzer in my home directory", "Create a react dashboard project", "Build a Rust web scraper".
* **Full Autonomous Permissions**: Executes CLI tools with zero-friction approval flags (`--dangerously-skip-permissions` for Claude/Antigravity, `--dangerously-bypass-approvals-and-sandbox` for Codex) so tasks never block waiting for TTY inputs.
* **Interactive Floating Popup**: Say "in a popup window" or ask for visual review to launch a centered floating modal terminal (`TUI.float`) where you can watch the agent code live.
* **Background Execution & Desktop Alerts**: Dispatches detached background jobs and triggers Omarchy desktop notifications when project generation is finished.

### 6. 🖥️ Native Hyprland & Omarchy Control
* **Workspace & Windows**: "Switch to workspace 3", "Focus Kitty", "Close this window", "Toggle split orientation", "Toggle fullscreen".
* **System Hardware**: "Volume up 10%", "Mute audio", "Set brightness to 70%", "What's my battery level?".
* **Dynamic Theming**: "Change theme to catppuccin", "Switch to gruvbox". Synchronizes the Quickshell HUD with active Omarchy colors in real time.

### 7. ⌨️ Hands-Free Virtual Input & Navigation (Zero-Touch)
* **Virtual Typing**: "Type 'git commit -m \"fix: issue resolved\"' and press enter" (injects via Wayland `wtype` safely over stdin).
* **Keyboard Shortcuts**: "Save file" (`ctrl+s`), "Reopen tab" (`ctrl+shift+t`), "Switch tab".
* **Key Navigation & Scrolling**: "Press escape", "Press enter", "Scroll down", "Scroll up".

### 8. 🐚 Universal Linux Shell Execution
* **Safe Terminal Execution**: "Run git status", "Check available memory with free -h", "List files in Downloads".
* Built-in 15s timeout protection and intelligent buffer truncation.

### 9. 🎵 Media & Track Control (MPRIS D-Bus)
* **Playback Control**: "Pause music", "Next track", "Previous track", "Play", "Stop".
* **Live Query**: "What song is playing right now?" (introspects Spotify, Chromium/YouTube, mpv, Firefox, etc.).

### 10. 📋 Clipboard & System Power
* **Clipboard Management**: "What is on my clipboard?", "Copy this link to my clipboard".
* **Power & Diagnostics**: "Lock the screen", "Reboot system", "Toggle Bluetooth", "System status", "Run network speedtest".

### 11. 🌐 Web & Media Navigation
* **Instant Search**: "Search the web for Arch Linux PipeWire configuration".
* **YouTube Playback**: "Open MKBHD's latest video on YouTube".
* **Direct URLs**: "Open github.com".

### 12. 👁️ Screen Perception & Multimodal Vision
* "Take a look at my screen and help me debug this code error".
* Takes an instantaneous snapshot via `grim` and uses Gemini's multimodal vision to analyze active windows and errors.

### 13. 🎙️ Hands-Free Wake Word ("Hey Jarvis")
* **Continuous Offline Detection**: Powered by `openWakeWord` and `hey_jarvis_v0.1.onnx` with zero cloud latency and ~1% CPU usage.
* **Hands-Free Activation**: Speak *"Hey Jarvis"* or *"Jarvis"* to immediately trigger the conversational HUD without touching the keyboard.
* **Background Daemon & Systemd**: Managed seamlessly via user systemd service (`systemctl --user start/enable jarvis`).

### 14. 📊 Omarchy Top Bar Widget & Popover UI
* **Live Menu Bar Icon**: Robot glyph (`󰚩`) in the top bar matching active Omarchy theme colors.
* **Microphone Active Dot**: A bright, pulsing red indicator dot appears strictly when the microphone is recording audio.
* **Interactive Control Popover**: Left-click to open a floating panel with instant controls:
  - Mic listening toggle & emergency kill switch.
  - Wake word on/off switch.
  - Quick action workflow launchers (Dev Setup, Screen Vision, Notes, Media Hub).
  - System controls: **Restart** (`󰑐`) to reload the daemon and widget, and **Quit Jarvis** (`󰗼`) to completely stop all processes (`systemctl --user stop jarvis`, background daemons, audio) and release all CPU & memory.
  - Active coding CLI indicator.


---

## ⌨️ Hotkeys & Control

Configured in `~/.config/hypr/bindings.lua`:

| Keybinding | Action | Description |
| :--- | :--- | :--- |
| `SUPER + SHIFT + ENTER` | **Push-to-Talk (Hold / Toggle)** | Press and hold to speak, release to send immediately. Or tap once to toggle continuous listening. |
| `SUPER + ALT + ESCAPE` | **Emergency Kill Switch** | Instantly silences audio, terminates active thinking, and hides the HUD. |

---

## 💬 Continuous Conversation & Kill Words

Jarvis supports natural ongoing multi-turn conversations. After completing a request, Jarvis stays attentive for follow-up commands.

When you're finished, simply say:
* *"That's it, thank you"*
* *"Done"*
* *"That'll be all, Jarvis"*
* *"Goodbye"*

Jarvis will politely acknowledge and dismiss the HUD.

---

## ⚙️ Architecture & Tech Stack

* **LLM Engine**: Google Gemini 3.5 Flash / Flash-Lite with multi-tier automatic quota fallback (`gemini-3.5-flash-lite` → `gemini-2.5-flash` → `gemini-flash-latest`).
* **Speech-to-Text (STT)**: Groq Whisper Large v3 (~100ms ultra-fast transcription).
* **Text-to-Speech (TTS)**: Local Piper neural TTS (`en_GB-alan-medium`) with Edge TTS fallback.
* **Voice Activity Detection (VAD)**: Silero VAD ONNX model for voice boundary detection.
* **HUD Overlay**: Transparent horizontal Quickshell interface with real-time waveform visualization matching Omarchy system themes.

---

## 🔧 Configuration (`.env`)

All core options are configurable via `.env` (see [.env.example](file:///home/binoy/Codes/personal/jarvis/.env.example)):

```bash
cp .env.example .env
```

Key environment variables:
* **`JARVIS_CLI_AI_TOOL`**: Preferred autonomous CLI coding agent (`claude` [default], `agy`, or `codex`).
* **`GEMINI_API_KEY`**: Google Gemini API key for fast reasoning and vision.
* **`GROQ_API_KEY`**: Groq API key for ultra-fast Whisper speech-to-text.
* **`JARVIS_TTS_ENGINE`**: TTS engine (`piper` for local neural synthesis, `edge` for cloud fallback).
* **`JARVIS_DEFAULT_WORKSPACE_ROOT`**: Default path for generated projects (`~/Codes/personal`).

---

## 🛠️ CLI Usage

```bash
# Run continuous background daemon (listening for "Hey Jarvis")
jarvis -d

# Restart background daemon and refresh Omarchy menubar plugin
jarvis -r
jarvis --restart

# Completely stop and quit all Jarvis processes (stops systemd service, frees CPU & RAM)
jarvis -q
jarvis --quit

# Restart Omarchy Quickshell top bar to cleanly reload QML widgets/popups
omarchy-restart-shell

# Check wake word detection and daemon status
jarvis --wakeword-status

# Toggle wake word detection on or off
jarvis --wakeword-toggle

# Trigger voice assistant (push-to-talk toggle)
jarvis -t

# Trigger push-to-talk release (stop recording and process)
jarvis -s

# Kill active voice session, silence audio, and hide HUD
jarvis -k

# Execute a single command via CLI (with voice response)
jarvis -c "what is my battery level"

# Execute a command silently (text output only)
jarvis -c "open my dev workflow" --no-speech

# Manage user systemd service
systemctl --user start jarvis
systemctl --user restart jarvis
systemctl --user stop jarvis
systemctl --user status jarvis
```

---

## 📄 Documentation

* [Performance Benchmarks & Technical Specifications](file:///home/binoy/Codes/personal/jarvis/docs/PERFORMANCE_AND_SPECS.md)
* [Rust Migration Roadmap (Future Memory Optimization)](file:///home/binoy/Codes/personal/jarvis/docs/RUST_MIGRATION_TODO.md)
* [Complete Capabilities & Feature Reference](file:///home/binoy/Codes/personal/jarvis/docs/CAPABILITIES.md)
* [Multi-Workspace Workflows Guide](file:///home/binoy/Codes/personal/jarvis/docs/WORKFLOW.md)
* [System Audit & Complete Uninstallation Guide](file:///home/binoy/Codes/personal/jarvis/docs/CLEANUP.md)
* [Architecture & Todo Roadmap](file:///home/binoy/Codes/personal/jarvis/docs/ARCHITECTURE_AND_TODO.md)
