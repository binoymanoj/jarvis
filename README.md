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

### 9. 🎬 Movie, TV Show & Music Automation
* **Episode & Movie Playback**: "Open Prison break ep12 from season 1", "Play Interstellar in full screen". Jarvis scans configured media directories, fuzzy-matches titles, seasons, and episodes, and launches playback in fullscreen via `mpv`.
* **Seamless Resume**: "Continue Prison break from where I left off". Jarvis tracks playback history (`~/.local/state/jarvis/media_history.json`) and leverages `mpv`'s position memory to resume the exact episode and timestamp.
* **Customizable Media Folders**: Configure custom directories (e.g. `["~/Videos", "~/Movies", "~/Downloads"]`) and player in `~/.config/jarvis/config.toml`.
* **MPRIS Playback Control**: "Pause music", "Next track", "Previous track", "Play", "Stop".
* **Live Query**: "What song is playing right now?" (introspects Spotify, Chromium/YouTube, mpv, Firefox, etc.).

### 10. 📋 Clipboard & System Power
* **Clipboard Management**: "What is on my clipboard?", "Copy this link to my clipboard".
* **Power & Diagnostics**: "Lock the screen", "Reboot system", "Shut down the computer", "Toggle Bluetooth", "System status", "Run network speedtest".
* **Themed TUI & Voice Confirmation**: Critical system operations (shutdown, reboot, logout, workflow deletion) automatically launch a centered, floating TUI modal (`TUI.float`) styled with active Omarchy palette colors, interactive pill buttons, and a 15-second countdown timer, while simultaneously listening for hands-free voice confirmation ("yes"/"confirm" vs "no"/"cancel").

### 11. 📲 LocalSend Synchronization & Seamless Sharing
* **Call Out and Share**: Say the name of any file, document, photo, or series episode (e.g. *"Share my resume on localsend"*, *"Send Prison break episode 12 via localsend"*), and Jarvis resolves it and queues it into LocalSend's sending window.
* **Clipboard & Screenshots**: *"Share my clipboard on localsend"*, *"Share this screenshot with localsend"*.
* **Hyprland Auto-Focus**: Dispatches window focus (`org.localsend.localsend_app`) so LocalSend immediately surfaces in the foreground.
* **CLI & Direct Transfer**: Compatible with GUI `localsend` and headless `localsend-cli` for targeting specific devices or IPs directly.

### 12. 🔬 Deep Research & Floating Neovim Markdown Viewer
* **Multi-Section Technical Reports**: *"Research how quantum computing error correction works"*, *"Deep dive into Linux epoll architecture"*. Jarvis generates structured Markdown reports complete with executive summaries, ASCII diagrams, equations, and code blocks.
* **Centered Floating Modal**: Opens instantly in a centered, floating Neovim window (`TUI.float`) styled with your active Omarchy theme colors.
* **Quick Dismissal**: Configured in read-only mode (`nvim -R`) with soft text wrapping. Press `q` or `Ctrl+C` to close and return to your work immediately.
* **Local Persistence**: Reports are cached at `~/.cache/jarvis/research/<slug>_<timestamp>.md`.

### 13. 🌐 Web & Media Navigation
* **Instant Search**: "Search the web for Arch Linux PipeWire configuration".
* **YouTube Playback**: "Open MKBHD's latest video on YouTube".
* **Direct URLs**: "Open github.com".

### 14. 👁️ Screen Perception & Multimodal Vision
* "Take a look at my screen and help me debug this code error".
* Takes an instantaneous snapshot via `grim` and uses Gemini's multimodal vision to analyze active windows and errors.

### 15. 🎙️ Hands-Free Wake Word ("Hey Jarvis")
* **Continuous Offline Detection**: Powered by `openWakeWord` and `hey_jarvis_v0.1.onnx` with zero cloud latency and <0.8% CPU usage.
* **Hands-Free Activation**: Speak *"Hey Jarvis"* (or custom *"Hey <name>"*) to immediately trigger the conversational HUD without touching the keyboard. Requiring the "Hey" prefix eliminates false activations from casual mentions of "Jarvis" or ambient room conversation.
* **Background Daemon & Systemd**: Managed seamlessly via user systemd service (`systemctl --user start/enable jarvis`).

### 16. 📊 Omarchy Top Bar Widget & Popover UI
* **Live Menu Bar Icon**: Robot glyph (`󰚩`) in the top bar matching active Omarchy theme colors.
* **Microphone Active Dot**: A bright, pulsing red indicator dot appears strictly when the microphone is recording audio.
* **Interactive Control Popover**: Left-click to open a floating panel with instant controls:
  - Mic listening toggle & emergency kill switch.
  - Wake word on/off switch.
  - Quick action workflow launchers (Dev Setup, Screen Vision, Notes, Media Hub).
  - System controls: **Restart** (`󰑐`) to reload the daemon and widget, and **Quit Jarvis** (`󰗼`) to completely stop all processes (`systemctl --user stop jarvis`, background daemons, audio) and release all CPU & memory.
  - Active coding CLI indicator.

### 17. 🔔 Desktop Notifications & Active Progress Indicators
* **In-Place Progress Toasts**: Background and multi-step tasks (battery diagnostics, network speedtests, media playback, deep research generation, and project scaffolding) trigger dynamic in-place desktop toasts (`TaskNotifier`). The notification updates smoothly as steps complete and auto-dismisses after 4 seconds.
* **Targeted System Notifications**: Dedicated notification feedback for key events:
  - 󰚩 Wake Word toggled on/off (`jarvis -W`).
  - 󰚩 Kill Switch activated (`SUPER + ALT + ESCAPE`).
  - 󰐌 Media playback matched and launched in fullscreen.
  - 󰈙 Research compiled and opened in floating Neovim.
  - 󰄬 LocalSend share dispatched.
  - 󰏪 Notes saved to disk.
  - 󰔛 Reminders and calendar events scheduled.
  - 󰌨 Workflows activated.
* **Zero-Spam Filter**: Trivial conversational questions and OS-level volume/brightness changes never emit redundant desktop notifications.


---

## ⌨️ Hotkeys & Control

Configured in `~/.config/hypr/bindings.lua`:

| Keybinding | Action | Description |
| :--- | :--- | :--- |
| `SUPER + C` | **Wakeup / Push-to-Talk (Hold / Toggle)** | Tap to toggle listening, or hold to speak and release to process immediately. |
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

* **100% Native Rust Architecture**: Compiled binary (`jarvis v0.2.0`) built with Tokio async runtime, `mimalloc` global allocator, and CPAL audio streaming.
* **Ultra-Low Memory Footprint**: **~18–25 MB RSS idle memory** (<68 MB total including Quickshell HUD overlay).
* **LLM Engine**: Google Gemini 3.5 Flash / Flash-Lite with multi-tier automatic quota fallback (`gemini-3.5-flash-lite` → `gemini-2.5-flash` → `gemini-flash-latest`).
* **Speech-to-Text (STT)**: Groq Whisper Large v3 Turbo (~150ms ultra-fast transcription).
* **Text-to-Speech (TTS)**: Microsoft Edge Neural TTS (`en-GB-RyanNeural`) with local Piper neural TTS (`en_GB-alan-medium`) offline fallback.
* **Voice Activity Detection (VAD)**: Silero VAD ONNX model for real-time speech boundary detection and trailing silence cutoff.
* **HUD Overlay**: Transparent horizontal Quickshell interface with real-time waveform visualization matching Omarchy system themes.

---

## 🚀 Quick Start & Installation

Getting Jarvis running on your system takes less than 3 minutes:

```bash
# 1. Clone & compile release binary
git clone https://github.com/binoymanoj/jarvis.git && cd jarvis
cargo build --release
install -Dm755 target/release/jarvis ~/.local/bin/jarvis

# 2. Copy bundled offline models and UI assets
mkdir -p ~/.config/jarvis/models ~/.config/jarvis/ui
cp -r models/* ~/.config/jarvis/models/
cp -r ui/* ~/.config/jarvis/ui/

# 3. Configure API keys in ~/.config/jarvis/config.toml
cp config.toml ~/.config/jarvis/config.toml
# Edit ~/.config/jarvis/config.toml to add your GEMINI_API_KEY and GROQ_API_KEY

# 4. Start background wake word daemon
jarvis -d
```

> 📖 **Looking for full step-by-step instructions?**  
> Check out the in-depth **[Complete Setup & Configuration Guide](file:///home/binoy/Codes/personal/jarvis/docs/SETUP_GUIDE.md)** covering system dependencies, Hyprland keybindings, systemd service setup, top bar widget, and troubleshooting.

---

## 🔧 Configuration (`config.toml` & `.env`)

Jarvis is configured via `~/.config/jarvis/config.toml` (auto-created on first run if missing). You can also provide environment variables or define them in `~/.config/jarvis/.env`:

```toml
# ~/.config/jarvis/config.toml
[ai]
provider = "gemini"               # "gemini", "openai", "anthropic", "groq", "openrouter"
model = "gemini-3.5-flash-lite"   # Fast default reasoning model
gemini_api_key = "AIzaSy..."      # Or set in ~/.config/jarvis/.env
groq_api_key = "gsk_..."          # Ultra-fast STT
cli_tool = "claude"               # Autonomous coding CLI: "claude", "codex", "agy"

[wakeword]
enabled = true                    # Continuous background wake word detection
name = "hey jarvis"               # "hey jarvis", "alexa", "hey_mycroft", "hey_rhasspy"
threshold = 0.50                  # Sensitivity threshold (0.30 - 0.70)

[editor]
default = "nvim"                  # "nvim", "code", "helix"
terminal = "kitty"                # "kitty", "foot"
project_dirs = ["~/Projects", "~/Codes", "~/src", "~"]

[media]
media_dirs = ["~/Videos", "~/Movies", "~/Downloads"]
player = "mpv"

[audio]
stt_engine = "groq"
whisper_model = "whisper-large-v3-turbo"
tts_engine = "edge"               # "edge" (zero-latency neural) or "piper" (offline)
edge_voice = "en-GB-RyanNeural"
```

Key environment variables:
* **`GEMINI_API_KEY`**: Google Gemini API key for reasoning, conversation, and screen vision.
* **`GROQ_API_KEY`**: Groq API key for ultra-fast Whisper speech-to-text (~150ms).
* **`JARVIS_CLI_AI_TOOL`**: Preferred autonomous CLI coding agent (`claude` [default], `agy`, or `codex`).
* **`JARVIS_TTS_ENGINE`**: TTS engine (`edge` for cloud neural synthesis, `piper` for local neural synthesis).
* **`JARVIS_MEDIA_DIRS`**: Comma-delimited list of directories to scan for movies/series.
* **`JARVIS_MEDIA_PLAYER`**: Default video player (`mpv`).

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
jarvis -W
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

# Launch the themed confirmation TUI modal directly (for testing or scripting)
jarvis --confirm-tui --title "System Reboot" --prompt "Are you sure you want to reboot the system now?"

# View recent Jarvis activity history and logs
jarvis -l
jarvis --logs

# Follow live activity logs in real time
jarvis -l -f
jarvis --logs --follow

# Print absolute path to jarvis.log
jarvis --logs-path

# Manage user systemd service
systemctl --user start jarvis
systemctl --user restart jarvis
systemctl --user stop jarvis
systemctl --user status jarvis
```

---

## 📄 Documentation

* [Complete Setup & Configuration Guide](file:///home/binoy/Codes/personal/jarvis/docs/SETUP_GUIDE.md)
* [Performance Benchmarks & Technical Specifications](file:///home/binoy/Codes/personal/jarvis/docs/PERFORMANCE_AND_SPECS.md)
* [Complete Capabilities & Feature Reference](file:///home/binoy/Codes/personal/jarvis/docs/CAPABILITIES.md)
* [Multi-Workspace Workflows Guide](file:///home/binoy/Codes/personal/jarvis/docs/WORKFLOW.md)
* [Rust Migration Roadmap & Completion Report](file:///home/binoy/Codes/personal/jarvis/docs/RUST_MIGRATION_TODO.md)
* [System Audit & Complete Uninstallation Guide](file:///home/binoy/Codes/personal/jarvis/docs/CLEANUP.md)
* [Architecture & Living Roadmap](file:///home/binoy/Codes/personal/jarvis/docs/ARCHITECTURE_AND_TODO.md)

