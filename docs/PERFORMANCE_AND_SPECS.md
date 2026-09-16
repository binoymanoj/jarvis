# 󰚩 Jarvis: Performance Benchmarks, Technical Specifications & Resource Profiles

**Document Version:** 2.0  
**Host Target:** Omarchy Linux (x86_64) / Hyprland Wayland Compositor  
**Last Updated:** September 16, 2026  

---

## 1. System Specifications & Host Environment

Jarvis is natively built and optimized for Omarchy Linux running on modern multi-core x86_64 hardware:

| Parameter | Specification | Notes |
| :--- | :--- | :--- |
| **Operating System** | Omarchy Linux (Arch Linux rolling release) | Kernel `7.2.3-arch1-3` (PREEMPT_DYNAMIC x86_64) |
| **Display Server & Compositor** | Wayland / Hyprland 0.56+ | Native UNIX command socket (`.socket`) and event socket (`.socket2`) |
| **Processor (CPU)** | 11th Gen Intel(R) Core(TM) i7-1185G7 @ 3.00GHz | 4 Physical Cores / 8 Threads (vCPUs) with Iris Xe Graphics |
| **System Memory (RAM)** | 31 GiB DDR4 | ~5.2 GiB free, ~21 GiB available buffer cache |
| **Swap Space** | 62 GiB NVMe Swap | Zero swap usage by Jarvis |
| **Audio Server** | PipeWire with WirePlumber session manager | 16 kHz mono microphone capture, 22.05 kHz stereo output |
| **Python Runtime** | Python 3.14.7 | Managed via `uv` package manager with native Rust virtualenv |
| **UI Framework** | Quickshell (Qt6 QML) | Native Wayland layer-shell overlay (100% transparent, zero borders) |

---

## 2. AI Model & Engine Stack

Jarvis utilizes an edge-to-cloud split architecture: continuous microphone streaming, wake word detection, voice activity detection, and speech synthesis run **100% locally on CPU via ONNX Runtime**, while transcription and multi-step reasoning are delegated to high-speed cloud providers.

```
                      ┌─────────────────────────────────────────────────────────┐
                      │              PipeWire Audio Input Stream                │
                      └───────────────────────────┬─────────────────────────────┘
                                                  │
                                                  ▼
                      ┌─────────────────────────────────────────────────────────┐
                      │        Local CPU: openWakeWord + Silero VAD (ONNX)       │
                      │               Model: hey_jarvis_v0.1.onnx               │
                      └───────────────────────────┬─────────────────────────────┘
                                                  │ (Wake word detected)
                                                  ▼
                      ┌─────────────────────────────────────────────────────────┐
                      │          Audio Recorder with Adaptive Speech VAD         │
                      │         Cuts audio on 0.85s natural speech pause        │
                      └───────────────────────────┬─────────────────────────────┘
                                                  │ (WAV audio payload)
                                                  ▼
                      ┌─────────────────────────────────────────────────────────┐
                      │       Cloud STT: Groq Whisper Large v3 Turbo (427ms)    │
                      └───────────────────────────┬─────────────────────────────┘
                                                  │ (Transcribed user prompt)
                                                  ▼
                      ┌─────────────────────────────────────────────────────────┐
                      │   Agent Brain: Google Gemini 3.5 Flash Lite (750ms)     │
                      │      Multi-tier quota fallback pool (6 models)          │
                      └─────────────┬─────────────────────────────┬─────────────┘
                                    │                             │
                 (Tool Calls)       │                             │ (Spoken Response)
                                    ▼                             ▼
        ┌───────────────────────────────────┐    ┌───────────────────────────────────┐
        │     Native Linux / Omarchy Tools  │    │ Local Offline TTS: Piper Alan ONNX│
        │  (Shell, Virtual Input, Media,    │    │   Model: en_GB-alan-medium (157ms)│
        │   Hyprland, Clipboard, Power...)  │    │     PipeWire raw audio playback   │
        └───────────────────────────────────┘    └───────────────────────────────────┘
```

### Model Specifications & Execution Roles

| Component | Model Name | Provider / Engine | Target | Latency | Key Attributes |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Wake Word Detection** | `hey_jarvis_v0.1.onnx` | `openwakeword` | Local CPU | **~32 ms** | 16 kHz sliding window, Soft AGC normalization, threshold 0.28 |
| **Voice Activity Detection** | Silero VAD v4 | `openwakeword.vad.VAD` (ONNX) | Local CPU | **~5 ms** | Real-time speech probability analysis; trailing 0.85s silence cutoff |
| **Speech-to-Text (STT)** | `whisper-large-v3-turbo` | Groq Cloud LPU API | Cloud | **~427 ms** | Sub-second transcription, punctuation, and phrase formatting |
| **Primary Reasoning (LLM)** | `gemini-3.5-flash-lite` | Google GenAI SDK | Cloud | **~650 - 950 ms** | Zero thinking budget, native parallel tool calling, high rate limits |
| **Quota Fallback Pool** | `gemini-3.5-flash`, `gemini-2.5-flash`, `gemini-2.5-flash-lite`, `gemini-flash-latest` | Google GenAI SDK | Cloud | ~700 - 1200 ms | Automatic transparent failover on 429 quota exhaustion |
| **Speech Synthesis (TTS)** | `en_GB-alan-medium.onnx` | Piper TTS (ONNX) | Local CPU | **~157 ms** | Natural British assistant persona; Realtime Factor (RTF) 0.053 |
| **TTS Cloud Fallback** | `en-GB-RyanNeural` | Microsoft Edge TTS | Cloud | ~250 ms | High-fidelity cloud TTS when offline voice files are missing |
| **Autonomous Coding CLI** | Claude Code / Codex / Antigravity (`agy`) | Local CLI tools | Local Process | Task-dependent | Driven with `--dangerously-skip-permissions` for zero-block execution |

---

## 3. Resource Consumption Profile (Idle vs. Working)

Jarvis was profiled under continuous operation using `ps`, `systemd-cgtop`, and PipeWire monitors:

### 3.1. Idle State (Background Listening Standby)
In idle state, the `jarvis.service` systemd daemon continuously reads the microphone stream from PipeWire and passes frames to `openWakeWord`:

| Metric | Background Daemon (`jarvis.service`) | Quickshell HUD (`qs`) | Total Jarvis Footprint |
| :--- | :--- | :--- | :--- |
| **Resident Memory (RSS)** | **373.7 MB** | **224.1 MB** | **~597.8 MB** (< 1.9% of 31 GiB system RAM) |
| **Virtual Memory (VSZ)** | 1.98 GB | 884.9 MB | ~2.86 GB |
| **CPU Utilization** | **10.2% - 12.4%** of 1 core | **0.0%** | **~1.3%** of total 8-thread CPU capacity |
| **Disk I/O** | 0 KB/s | 0 KB/s | **0 KB/s** |
| **Network Bandwidth** | 0 KB/s | 0 KB/s | **0 KB/s** (100% offline listening) |

### 3.2. Active Work State (Voice Turn, Reasoning & Execution)
During active conversational turns (wake word triggered → audio recorded → Groq transcription → Gemini tool reasoning → Piper speech synthesis):

| Metric | Measured Peak | Duration of Peak |
| :--- | :--- | :--- |
| **Resident Memory (RSS)** | **408.2 MB** | Sustained during conversation session |
| **CPU Utilization (Daemon)** | **35% - 55%** of 1 core | Only during Piper ONNX synthesis (~150 ms burst) |
| **CPU Utilization (Quickshell)** | **1.5% - 3.2%** of 1 core | Only while animating the 15-bar voice waveform |
| **Network Bandwidth (Upload)** | **~25 - 80 KB** | Single HTTPS POST of compressed WAV audio to Groq API |
| **Network Bandwidth (Download)** | **~2 - 6 KB** | JSON responses from Groq STT and Gemini Flash |

---

## 4. Performance Benchmarks & Latency Breakdown

Measured across live benchmark runs on the Intel Core i7-1185G7:

```
[User stops speaking]
       │
       ├─ (0.71s - 0.85s) ── Adaptive VAD Silence Detection
       │
       ├─ (0.42s) ────────── Groq Whisper Large v3 Turbo STT
       │
       ├─ (0.75s) ────────── Gemini 3.5 Flash Lite Reasoning & Tool Execution
       │
       ├─ (0.16s) ────────── Piper British Alan ONNX Synthesis (3.0s audio)
       │
       ▼
[Jarvis begins speaking] -> Total turnaround: ~1.33s (processing) + silence threshold
```

### Granular Latency Measurements

| Pipeline Stage | Engine / Model | Measured Time | Notes |
| :--- | :--- | :--- | :--- |
| **Wake Word Detection** | openWakeWord ONNX | **~32 ms** | Evaluated on 80ms audio frames; instantaneous detection |
| **Speech Pause Cutoff** | Silero VAD (ONNX) | **~710 - 850 ms** | Automatically stops recording when speech ends; no fixed timeout |
| **Speech-to-Text (STT)** | Groq Whisper Large v3 Turbo | **~427 ms** | Cloud HTTPS round-trip transcribing audio to text |
| **Agent Reasoning & Tools** | Gemini 3.5 Flash Lite | **~650 - 950 ms** | Zero thinking budget, parallel tool schema execution |
| **Speech Synthesis (TTS)** | Piper Alan (`en_GB-alan`) | **~157 ms** | Synthesizes 3.0 seconds of audio; Realtime Factor (RTF): **0.053** |
| **Virtual Input Typing** | `wtype` via stdin | **~8 - 15 ms** | Direct Wayland compositor keystroke injection |
| **Shell Command Execution** | Bash subprocess | **~10 - 45 ms** | Standard Linux commands (`git`, `ls`, system queries) |
| **End-to-End Turn Time** | Total Processing Loop | **~1.25 - 1.55 s** | Time from end of speech until Jarvis begins speaking |

---

## 5. Comprehensive Hands-Free Tool Palette (48 Functions)

Jarvis registers **48 native tools** directly inside Gemini's function calling schema:


### 1. Hands-Free Virtual Input & Navigation (Zero-Touch)
- `type_text(text: str, enter_after: bool = False)`: Injects text into focused window via `wtype` stdin.
- `press_key(key_name: str)`: Press Return, Escape, Tab, BackSpace, space, Up, Down, Page_Down, Home, End.
- `send_shortcut(modifiers: str, key: str)`: Sends keyboard shortcuts (`ctrl+s`, `ctrl+shift+t`, `alt+tab`).
- `scroll(direction: str = "down", amount: int = 2)`: Scrolls active window up or down hands-free.

### 2. Universal Linux Shell Execution
- `execute_command(command: str, timeout: float = 15.0)`: Runs arbitrary bash commands with safety timeouts and output truncation.

### 3. Media Playback Control (MPRIS D-Bus)
- `media_play_pause()`: Toggles media playback across Spotify, Chromium/YouTube, Firefox, mpv, VLC.
- `media_next()`: Skips to next song/video.
- `media_previous()`: Returns to previous track.
- `media_stop()`: Stops active playback.
- `get_now_playing()`: Retrieves active track title, artist, and player application name.

### 4. System Power & Hardware Diagnostics
- `lock_screen()`: Invokes `omarchy system lock` to lock workstation and sleep monitors.
- `logout_system()`: Safely logs out of the desktop session.
- `reboot_system()`: Reboots the workstation.
- `shutdown_system()`: Powers down the computer.
- `toggle_bluetooth(action: str = "toggle")`: Controls Bluetooth power (`on`, `off`, `toggle`, `is-on`).
- `get_system_stats()`: Reads CPU load, memory utilization, and active Wi-Fi connection info.
- `network_speedtest(direction: str = "down")`: Measures live download or upload throughput.

### 5. Wayland System Clipboard
- `get_clipboard()`: Inspects current text on clipboard via `wl-paste`.
- `set_clipboard(text: str)`: Copies text to clipboard via `wl-copy`.

### 6. Window & Workspace Orchestration (Hyprland IPC)
- `switch_workspace(workspace_id: int)`: Navigates to any Hyprland workspace.
- `focus_application(app_name: str)`: Brings running application window to foreground.
- `close_active_window()`: Closes currently focused window.
- `toggle_fullscreen()`: Toggles fullscreen state.
- `toggle_layout_split()`: Toggles dwindle layout horizontal/vertical split.

### 7. System Audio, Display & Themes
- `adjust_volume(adjustment: str)`: Adjusts volume (`raise`, `lower`, `mute-toggle`, `+5`, `-10`).
- `set_brightness(level: str)`: Adjusts display backlight (`+10`, `-10`, `75`).
- `set_theme(theme_name: str)`: Switches Omarchy theme (`tokyo-night`, `catppuccin`, `solitude`).
- `get_battery()`: Reports battery state, percentage, and remaining runtime.
- `launch_application(app_name: str)`: Launches apps via Omarchy launcher.
- `notify(headline: str, description: str)`: Sends Omarchy desktop notification.

### 8. Web Navigation & Online Media
- `open_youtube(query: str)`: Searches and opens video directly on YouTube non-blockingly.
- `search_web(query: str)`: Performs Google search in browser.
- `open_url(url: str)`: Opens any website URL.

### 9. Multi-Workspace Workflows
- `launch_workflow(name: str)`: Launches setups for `coding`, `research`, `writing`, `communication`, or `media`.
- `list_workflows()`: Lists configured workflow setups.
- `capture_current_workflow(name: str, description: str, aliases: str)`: Captures currently open applications across Hyprland workspaces into a new workflow.
- `save_custom_workflow(name: str, description: str, steps_json: str, aliases: str, primary_workspace: int)`: Saves or updates a custom workflow preset.
- `delete_custom_workflow(name: str)`: Deletes a workflow preset by name.
- `get_workflow_details(name: str)`: Retrieves detailed launch steps and workspace targets for a workflow.


### 10. Productivity & Personal Assistant
- `schedule_event(title: str, start_time: str, ...)`: Schedules calendar events.
- `set_reminder(minutes: int, message: str)`: Starts systemd countdown reminders.
- `list_reminders()`, `clear_reminders()`: Manages desktop countdown timers.
- `draft_email(recipient: str, subject: str, body: str)`: Composes emails in Thunderbird or Gmail.
- `create_note(title: str, content: str)`, `list_notes()`: Manages markdown notes in `~/Notes`.

### 11. Autonomous Project & Code Generation
- `create_project(name: str, description: str, ...)`: Scaffolds complete applications using `claude`, `agy`, or `codex`.
- `delegate_to_antigravity(task_description: str, ...)`: Delegates complex coding tasks to CLI AI tool.

### 12. Screen Perception & Multimodal Vision
- `inspect_screen(query: str, target: str = "active_window")`: Captures active window or full screen with `grim` and analyzes visually with Gemini.

### 13. Conversational Session Lifecycle
- `dismiss_session(farewell: str)`: Concludes conversational session when user says "done", "that's it", or "goodbye".
