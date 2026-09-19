# 󰚩 Jarvis: Complete Setup & Configuration Guide

Welcome to **Jarvis**! This guide walks you through setting up, compiling, configuring, and running Jarvis on your Linux desktop from scratch.

Jarvis is an ultra-low-latency, zero-cost, multi-turn AI desktop assistant built natively in **Rust** for **Omarchy Linux** and the **Hyprland** tiling window manager.

---

## 📋 Table of Contents

1. [System Requirements & Dependencies](#1-system-requirements--dependencies)
2. [Obtaining Free API Keys](#2-obtaining-free-api-keys)
3. [Building & Installing from Source](#3-building--installing-from-source)
4. [Models & Assets Setup](#4-models--assets-setup)
5. [Configuration (`config.toml` & `.env`)](#5-configuration-configtoml--env)
6. [Hyprland Keybindings & Window Rules](#6-hyprland-keybindings--window-rules)
7. [Systemd Background Daemon](#7-systemd-background-daemon)
8. [Omarchy Top Bar Widget](#8-omarchy-top-bar-widget)
9. [Testing & Verification](#9-testing--verification)
10. [Troubleshooting & FAQs](#10-troubleshooting--faqs)

---

## 1. System Requirements & Dependencies

### Operating System & Compositor
* **OS**: Linux (optimized for Arch Linux / Omarchy Linux).
* **Compositor**: Hyprland (Wayland).
* **Audio Server**: PipeWire with WirePlumber session manager.

### Required Software Packages

Install the following standard desktop utilities:

#### On Arch Linux / Omarchy (`pacman` / `paru` / `yay`):
```bash
sudo pacman -S --needed \
    rustup base-devel \
    pipewire wireplumber \
    mpv \
    wl-clipboard \
    grim \
    libnotify \
    kitty \
    neovim \
    curl jq
```

#### AUR Packages:
```bash
# Quickshell for the floating HUD overlay and menu bar widget
paru -S quickshell-git

# Wayland virtual keyboard keystroke injector
paru -S wtype
```

### Optional Integrations
* **LocalSend** (`localsend-bin` or `localsend-cli`): Seamless cross-device file/clipboard sharing.
* **Autonomous Coding Agents**:
  * [Claude Code](https://github.com/anthropics/claude-code): `npm install -g @anthropic-ai/claude-code`
  * [Codex CLI](https://github.com/openai/codex): `npm install -g @openai/codex`
  * [Antigravity CLI (`agy`)](https://github.com/google-deepmind/antigravity-cli)

---

## 2. Obtaining Free API Keys

Jarvis is designed for **zero operating cost** by leveraging generous free-tier APIs:

### 1. Google Gemini (Brain & Reasoning) — *Required*
1. Visit [Google AI Studio](https://aistudio.google.com/).
2. Click **Get API key** and create a free key.
3. Provides generous free access to `gemini-3.5-flash-lite` and `gemini-2.5-flash` (up to 15 RPM).

### 2. Groq Cloud (Ultra-Fast Speech-to-Text) — *Required*
1. Visit [Groq Cloud Console](https://console.groq.com/).
2. Navigate to **API Keys** and generate an API key.
3. Provides free ultra-fast Whisper Large v3 Turbo transcription (~150ms latency).

### 3. Other Supported Providers (*Optional*)
Jarvis also supports OpenAI (`OPENAI_API_KEY`), Anthropic Claude (`ANTHROPIC_API_KEY`), and OpenRouter (`OPENROUTER_API_KEY`) if you prefer alternative models.

---

## 3. Building & Installing from Source

### Step 1: Ensure Rust is installed
```bash
rustup default stable
rustup update
```

### Step 2: Clone the Repository
```bash
git clone https://github.com/binoymanoj/jarvis.git
cd jarvis
```

### Step 3: Build Optimized Release Binary
```bash
cargo build --release
```
*The build compiles with Link-Time Optimization (`lto = "fat"`), stripped symbols, and `mimalloc` allocator for minimum memory footprint (~18-25 MB idle).*

### Step 4: Install Binary to User PATH
```bash
install -Dm755 target/release/jarvis ~/.local/bin/jarvis
```

Ensure `~/.local/bin` is in your `$PATH`. If not, add this to your `~/.bashrc` or `~/.zshrc`:
```bash
export PATH="$HOME/.local/bin:$PATH"
```

Verify installation:
```bash
jarvis --help
```

---

## 4. Models & Assets Setup

Jarvis uses local ONNX models for offline wake word detection, voice activity detection (VAD), and offline text-to-speech fallback. All required models are conveniently provided in the repository's `models/` folder.

### Step 1: Copy Models
```bash
mkdir -p ~/.config/jarvis/models
cp -r models/* ~/.config/jarvis/models/
```
*(Alternatively, Jarvis also searches `~/.local/share/jarvis/models` or `./models`).*

### Model Overview:
* **`silero_vad.onnx`**: Silero Voice Activity Detection (cuts recording dynamically on natural speech pause).
* **`hey_jarvis_v0.1.onnx`**: openWakeWord 80-feature neural model for "Hey Jarvis".
* **`melspectrogram.onnx` & `embedding_model.onnx`**: openWakeWord audio preprocessing models.
* **`alexa_v0.1.onnx`, `hey_mycroft_v0.1.onnx`, `hey_rhasspy_v0.1.onnx`**: Optional alternate wake words.
* **`en_GB-alan-medium.onnx`**: Offline Piper neural voice fallback.

### Step 2: Copy UI Assets (Quickshell HUD)
```bash
mkdir -p ~/.config/jarvis/ui
cp -r ui/* ~/.config/jarvis/ui/
```
*(Jarvis also checks `~/.local/share/jarvis/ui` and `./ui`).*

---

## 5. Configuration (`config.toml` & `.env`)

Jarvis configuration is located at `~/.config/jarvis/config.toml`. If the file does not exist, Jarvis automatically generates a documented default configuration template on its first run.

You can also copy the repository's template directly:
```bash
mkdir -p ~/.config/jarvis
cp config.toml ~/.config/jarvis/config.toml
```

### Complete Configuration Reference

Here is a full breakdown of `~/.config/jarvis/config.toml`:

```toml
# ==============================================================================
# Jarvis Configuration File (~/.config/jarvis/config.toml)
# ==============================================================================

[ai]
# AI Provider: "gemini" (free default), "openai", "anthropic" (claude), "groq", "openrouter"
provider = "gemini"

# Model name for chosen provider:
# - Gemini: "gemini-3.5-flash-lite" (fastest, default), "gemini-2.5-flash"
# - OpenAI: "gpt-4o", "gpt-4o-mini"
# - Anthropic: "claude-3-7-sonnet-latest", "claude-3-5-haiku-latest"
# - Groq: "llama-3.3-70b-versatile"
# - OpenRouter: "anthropic/claude-3.7-sonnet", "deepseek/deepseek-r1"
model = "gemini-3.5-flash-lite"

# Provider API Keys
# (You can place them here, or in ~/.config/jarvis/.env, or export them in your shell)
gemini_api_key = "AIzaSy..."
openai_api_key = ""
anthropic_api_key = ""
groq_api_key = "gsk_..."
openrouter_api_key = ""

# Autonomous coding CLI assistant ("claude", "codex", "agy")
cli_tool = "claude"

[wakeword]
# Continuous background wake word detection in daemon mode
enabled = true

# Wake word name:
# - "hey jarvis" (default, highly recommended)
# - "hey <name>" (e.g. "hey computer")
# - "alexa", "hey_mycroft", "hey_rhasspy"
name = "hey jarvis"

# Sensitivity threshold (0.30 - 0.70). Default: 0.50
# Requiring clear "Hey Jarvis" prevents accidental triggers from ambient room chatter.
threshold = 0.50

# Optional custom ONNX model path (relative to models dir or absolute)
# model_path = "models/hey_jarvis_v0.1.onnx"

[editor]
# Code editor to open project files with ("nvim", "neovim", "code", "helix")
default = "nvim"

# Terminal emulator used to host the editor & popups ("kitty", "foot")
terminal = "kitty"

# Project search directories (searched when asking e.g. "open main.rs from my project")
project_dirs = ["~/Projects", "~/Codes", "~/src", "~"]

[media]
# Directories searched when asking to play movies, TV series, or media files
# Supports tilde expansion (e.g. ~/Videos, ~/Movies, ~/Downloads)
media_dirs = ["~/Videos", "~/Movies", "~/Downloads"]

# Preferred media player executable ("mpv", "vlc")
player = "mpv"

[audio]
# Speech-to-Text engine ("groq" or "local")
stt_engine = "groq"
whisper_model = "whisper-large-v3-turbo"

# Text-to-Speech engine:
# - "edge" (pure Rust WebSocket, zero latency, natural neural voices)
# - "piper" (100% offline local neural TTS)
tts_engine = "edge"
edge_voice = "en-GB-RyanNeural"
piper_voice = "en_GB-alan-medium"

# VAD and Silence detection
silence_threshold_seconds = 1.3
initial_listen_timeout = 10.0
followup_listen_timeout = 8.0
max_recording_seconds = 25.0
```

### Environment File (`~/.config/jarvis/.env` or `./.env`)

If you prefer keeping API keys separate from `config.toml`, you can create `~/.config/jarvis/.env`:
```bash
GEMINI_API_KEY=AIzaSy...
GROQ_API_KEY=gsk_...
```
Jarvis automatically checks for `.env` in `~/.config/jarvis/.env`, `./.env`, and the shell environment.

---

## 6. Hyprland Keybindings & Window Rules

### 1. Keybindings

Add the following shortcuts to your Hyprland configuration.

If you are using **Omarchy** (`~/.config/hypr/bindings.lua`):
```lua
-- Push-to-Talk: Press and hold to speak, release to process immediately
o.bind("SUPER + SHIFT + RETURN", "Jarvis Voice Assistant (Hold/Toggle)", "jarvis -t")
o.bind("SUPER + SHIFT + RETURN", "Jarvis Voice Assistant (Release)", "jarvis -s", { release = true })

-- Emergency Kill Switch: Instantly silences speech, cancels commands, hides HUD
o.bind("SUPER + ALT + ESCAPE", "Kill Jarvis Assistant", "jarvis -k")
```

If you are using standard **`hyprland.conf`** (`~/.config/hypr/hyprland.conf`):
```conf
# Push-to-Talk (Hold / Release)
bind = SUPER SHIFT, Return, exec, jarvis -t
bindr = SUPER SHIFT, Return, exec, jarvis -s

# Emergency Kill Switch
bind = SUPER ALT, Escape, exec, jarvis -k
```

### 2. Window Rules for Floating Confirmation & Neovim Popups

Jarvis opens confirmation dialogs and deep research markdown viewers inside centered floating windows with window class `TUI.float`.

Ensure your `hyprland.conf` contains the floating rule:
```conf
windowrulev2 = float, class:^(TUI.float)$
windowrulev2 = center, class:^(TUI.float)$
windowrulev2 = size 760 480, class:^(TUI.float)$
windowrulev2 = dimaround, class:^(TUI.float)$
```

---

## 7. Systemd Background Daemon

Jarvis runs as a lightweight user systemd service. When running in daemon mode (`jarvis --daemon` or `jarvis -d`), it continuously listens for the offline wake word (*"Hey Jarvis"*) while consuming <0.8% CPU and ~18-25 MB of memory.

### Step 1: Create the User Systemd Service File

Create `~/.config/systemd/user/jarvis.service`:
```ini
[Unit]
Description=Jarvis AI Assistant Background Wake Word Daemon
PartOf=graphical-session.target
After=graphical-session.target pipewire.service

[Service]
Type=simple
ExecStart=%h/.local/bin/jarvis --daemon
Restart=on-failure
RestartSec=3
Environment=XDG_RUNTIME_DIR=%t

[Install]
WantedBy=graphical-session.target
```

### Step 2: Enable & Start the Service
```bash
systemctl --user daemon-reload
systemctl --user enable --now jarvis
```

### Step 3: Check Daemon Status
```bash
systemctl --user status jarvis
```

### Managing the Service:
```bash
# View live logs
jarvis -l -f

# Restart daemon
jarvis -r

# Completely stop and quit Jarvis
jarvis -q
```

---

## 8. Omarchy Top Bar Widget

If you use the Omarchy desktop shell or Quickshell top bar:

1. The widget files are located in `ui/bar/` (`BarWidget.qml` and `manifest.json`).
2. Copy them to your Omarchy bar widgets directory:
   ```bash
   mkdir -p ~/.config/omarchy/bar/widgets/jarvis
   cp -r ui/bar/* ~/.config/omarchy/bar/widgets/jarvis/
   ```
3. Restart Quickshell:
   ```bash
   omarchy-restart-shell
   ```
4. **Features**:
   - Robot icon (`󰚩`) matching your active Omarchy theme colors.
   - Pulsing red dot indicator when the microphone is recording.
   - Click to open interactive control popover (Microphone toggle, Wake Word toggle, Quick action launchers, Restart, and Quit).

---

## 9. Testing & Verification

Run these verification steps to confirm everything is working:

### 1. Test Single Command Execution via CLI
```bash
jarvis -c "what time is it"
```
*Expected: Jarvis prints the answer and speaks it aloud using Edge TTS neural voice.*

### 2. Test Silent Execution
```bash
jarvis -c "check free memory" --no-speech
```
*Expected: Jarvis runs `free -h` through its shell tool and outputs the text response without playing audio.*

### 3. Test Themed Confirmation TUI Modal
```bash
jarvis --confirm-tui --title "Installation Test" --prompt "Does the Jarvis confirmation dialog render properly?"
```
*Expected: A floating terminal opens with pill buttons, countdown timer, and theme colors. Press `Y` or `Enter` to confirm, or `N`/`Esc` to cancel.*

### 4. Test Deep Research in Neovim
```bash
jarvis -c "research the history of the linux kernel in neovim"
```
*Expected: Jarvis compiles a structured Markdown document and displays it in a centered floating Neovim window (`nvim -R`). Press `q` to dismiss.*

### 5. Test Media Automation
```bash
jarvis -c "open prison break ep12 from season 1"
```
*Expected: Jarvis scans your configured `media_dirs`, locates the file, launches `mpv` in fullscreen, and sends an in-place desktop progress notification.*

### 6. Test Wake Word ("Hey Jarvis")
Speak clearly into your microphone:
> *"Hey Jarvis, what is my battery level?"*

*Expected: You hear a subtle wake chime, the transparent Quickshell waveform HUD appears at the bottom, records your voice, displays the answer, and speaks it.*

---

## 10. Troubleshooting & FAQs

### 1. "Failed to find silero_vad.onnx" or "Model not found"
**Cause:** Model files were not copied to the search path.  
**Fix:** Ensure models exist in `~/.config/jarvis/models/` or `~/.local/share/jarvis/models/`:
```bash
ls -la ~/.config/jarvis/models/
```
If empty, re-copy from the repository: `cp -r models/* ~/.config/jarvis/models/`.

### 2. Microphone Not Recording / Audio Permissions
**Cause:** PipeWire recording device muted or blocked.  
**Fix:**
```bash
# Check recording devices
wpctl status
# Test recording a sample WAV
pw-record test.wav
```
Ensure your microphone is the default input device in `pavucontrol` or WirePlumber.

### 3. "API Key Missing: GEMINI_API_KEY"
**Cause:** No API key defined in `config.toml`, `.env`, or environment.  
**Fix:** Add `gemini_api_key = "YOUR_KEY"` to `~/.config/jarvis/config.toml`, or add `GEMINI_API_KEY=YOUR_KEY` to `~/.config/jarvis/.env`.

### 4. Wake Word Triggers Too Easily or Not Easily Enough
**Cause:** Wake word threshold needs tuning for your microphone sensitivity.  
**Fix:** In `~/.config/jarvis/config.toml`, adjust `[wakeword] threshold`:
* If triggering on ambient room noise: increase threshold to `0.55` or `0.60`.
* If hard to trigger: lower threshold to `0.40` or `0.45`.
* Default is `0.50`.

### 5. Another Instance Is Already Running / Socket Busy
**Cause:** A background daemon or previous run is holding `/tmp/jarvis.sock`.  
**Fix:**
```bash
jarvis -k
# Or kill all instances cleanly:
pkill -f jarvis
rm -f /tmp/jarvis.sock
```

### 6. Where are the logs?
View real-time logs:
```bash
jarvis -l -f
# Or inspect the log file directly:
cat ~/.local/state/jarvis/jarvis.log
```

---

*Enjoy your ultra-fast, native Rust Jarvis assistant! For questions, issue reports, or feature requests, visit the [Jarvis GitHub Repository](https://github.com/binoymanoj/jarvis).*
