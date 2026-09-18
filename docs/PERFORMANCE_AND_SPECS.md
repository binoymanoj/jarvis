# 󰚩 Jarvis: Performance Benchmarks, Technical Specifications & Resource Profiles

**Document Version:** 3.0 (100% Native Rust Architecture)  
**Host Target:** Omarchy Linux (x86_64) / Hyprland Wayland Compositor  
**Last Updated:** September 18, 2026  

---

## 1. System Specifications & Host Environment

Jarvis is natively built and compiled in Rust for Omarchy Linux running on modern multi-core x86_64 hardware:

| Parameter | Specification | Notes |
| :--- | :--- | :--- |
| **Operating System** | Omarchy Linux (Arch Linux rolling release) | Kernel `7.2.3-arch1-3` (PREEMPT_DYNAMIC x86_64) |
| **Display Server & Compositor** | Wayland / Hyprland 0.56+ | Native UNIX command socket (`.socket`) and event socket (`.socket2`) |
| **Processor (CPU)** | 11th Gen Intel(R) Core(TM) i7-1185G7 @ 3.00GHz | 4 Physical Cores / 8 Threads (vCPUs) with Iris Xe Graphics |
| **System Memory (RAM)** | 31 GiB DDR4 | ~5.2 GiB free, ~21 GiB available buffer cache |
| **Memory Allocator** | `mimalloc` v0.1.52 | Drop-in allocator avoiding glibc arena growth and returning memory to the kernel |
| **Swap Space** | 62 GiB NVMe Swap | Zero swap usage by Jarvis |
| **Audio Server** | PipeWire with WirePlumber session manager | 16 kHz mono microphone capture via `cpal`, 22.05 kHz stereo output |
| **Compiled Language** | Rust 1.98+ (Edition 2021) | Native Tokio async runtime, statically linked LTO release |
| **UI Framework** | Quickshell (Qt6 QML) | Native Wayland layer-shell overlay (100% transparent, zero borders) |

---

## 2. AI Model & Engine Stack

Jarvis utilizes an edge-to-cloud split architecture: continuous microphone streaming, wake word detection, voice activity detection, and speech synthesis run **100% locally on CPU via ONNX Runtime (`ort`)**, while transcription and multi-step reasoning are delegated to high-speed cloud providers.

```
                      ┌─────────────────────────────────────────────────────────┐
                      │              PipeWire Audio Input Stream                │
                      └───────────────────────────┬─────────────────────────────┘
                                                  │
                                                  ▼
                      ┌─────────────────────────────────────────────────────────┐
                      │    Local CPU: openWakeWord (ort 2.0 single-threaded)    │
                      │               Model: hey_jarvis_v0.1.onnx               │
                      └───────────────────────────┬─────────────────────────────┘
                                                  │ (Wake word detected)
                                                  ▼
                      ┌─────────────────────────────────────────────────────────┐
                      │         Audio Recorder with Adaptive Silero VAD         │
                      │         Cuts audio on 0.85s natural speech pause        │
                      └───────────────────────────┬─────────────────────────────┘
                                                  │ (WAV audio payload)
                                                  ▼
                      ┌─────────────────────────────────────────────────────────┐
                      │       Cloud STT: Groq Whisper Large v3 Turbo (150ms)    │
                      └───────────────────────────┬─────────────────────────────┘
                                                  │ (Transcribed user prompt)
                                                  ▼
                      ┌─────────────────────────────────────────────────────────┐
                      │   Agent Brain: Google Gemini 3.5 Flash Lite (500ms)     │
                      │      Multi-tier quota fallback pool (5 models)          │
                      └─────────────┬─────────────────────────────┬─────────────┘
                                    │                             │
                 (Tool Calls)       │                             │ (Spoken Response)
                                    ▼                             ▼
        ┌───────────────────────────────────┐    ┌───────────────────────────────────┐
        │     Native Linux / Omarchy Tools  │    │ High-Fidelity Neural TTS:         │
        │  (Shell, Virtual Input, Media,    │    │  • Edge Neural: en-GB-RyanNeural  │
        │   Hyprland, Clipboard, Power...)  │    │  • Local Offline Piper ONNX       │
        └───────────────────────────────────┘    └───────────────────────────────────┘
```

### Model Specifications & Execution Roles

| Component | Model Name | Provider / Engine | Target | Latency | Key Attributes |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Wake Word Detection** | `hey_jarvis_v0.1.onnx` | `ort` (C API ONNX Runtime) | Local CPU | **~25 ms** | 16 kHz sliding window, Soft AGC normalization, threshold 0.22 |
| **Voice Activity Detection** | `silero_vad.onnx` | `ort` (C API ONNX Runtime) | Local CPU | **~3 ms** | Real-time speech probability analysis; trailing 0.85s silence cutoff |
| **Speech-to-Text (STT)** | `whisper-large-v3-turbo` | Groq Cloud LPU API | Cloud | **~150 - 200 ms** | Sub-second transcription, punctuation, and phrase formatting |
| **Primary Reasoning (LLM)** | `gemini-3.5-flash-lite` | Google AI Studio | Cloud | **~400 - 650 ms** | Zero thinking budget, native parallel tool calling, high rate limits |
| **Quota Fallback Pool** | `gemini-3.5-flash`, `gemini-2.5-flash`, `gemini-2.5-flash-lite`, `llama-3.3-70b-versatile` | Gemini / Groq | Cloud | ~500 - 900 ms | Automatic transparent failover on 429 quota exhaustion |
| **Speech Synthesis (TTS)** | `en-GB-RyanNeural` | Microsoft Edge TTS | Cloud | **~180 - 220 ms** | Natural neural British assistant persona |
| **TTS (Offline Fallback)** | `en_GB-alan-medium.onnx` | Piper TTS (ONNX) | Local CPU | **~150 ms** | Sub-50ms first-chunk generation; Realtime Factor (RTF) 0.053 |
| **Autonomous Coding CLI** | Claude Code / Codex / Antigravity (`agy`) | Local CLI tools | Local Process | Task-dependent | Driven with `--dangerously-skip-permissions` for zero-block execution |

---

## 2. Resource Consumption Profile (Idle vs. Working)

Jarvis was profiled under continuous operation using `ps`, `systemd-cgtop`, and PipeWire monitors with the native Rust engine:

### 2.1. Idle State (Background Listening Standby)
In idle state, the `jarvis.service` systemd daemon continuously reads the microphone stream from PipeWire and passes frames to `openWakeWord`:

| Metric | Rust Daemon (`jarvis --daemon`) | Quickshell HUD (`qs`) | Notes |
| :--- | :--- | :--- | :--- |
| **Resident Memory (RSS)** | **~25 – 35 MB** | **~220 MB** | Down from ~410 MB in the old prototype |
| **Virtual Memory (VSZ)** | ~600 MB | ~880 MB | Lean address space allocation |
| **CPU Utilization** | **< 1.0%** of 1 core | **0.0%** | Single-threaded ONNX with zero busy-spin |
| **Disk I/O** | 0 KB/s | 0 KB/s | Zero disk writes at idle |
| **Network Bandwidth** | 0 KB/s | 0 KB/s | 100% offline wake word listening |

### 2.2. Active Work State (Voice Turn, Reasoning & Execution)
During active conversational turns (wake word triggered → audio recorded → Groq transcription → Gemini tool reasoning → speech playback):

| Metric | Measured Peak | Duration of Peak |
| :--- | :--- | :--- |
| **Resident Memory (RSS)** | **~35 – 45 MB** | Sustained during conversation session |
| **CPU Utilization (Daemon)** | **2% - 5%** of 1 core | Audio normalization and HTTP client dispatch |
| **CPU Utilization (Quickshell)** | **1.5% - 3.0%** of 1 core | Smooth 60fps fluid 15-bar waveform animation |
| **Network Bandwidth (Upload)** | **~20 - 50 KB** | Compressed WAV audio chunk sent to Groq STT |
| **Network Bandwidth (Download)** | **~2 - 8 KB** | Streaming JSON responses from Gemini |

---

## 3. Comprehensive Hands-Free Tool Palette (51 Tools)

Jarvis registers **51 native tools** directly inside Gemini's tool declaration registry:

1. **Virtual Input & Typing**: `type_text`, `press_key`, `send_shortcut`, `scroll`.
2. **Linux Shell**: `execute_command`.
3. **Media Control (MPRIS)**: `media_play_pause`, `media_next`, `media_previous`, `media_stop`, `get_now_playing`.
4. **Power & Hardware**: `lock_screen`, `logout_system`, `reboot_system`, `shutdown_system`, `toggle_bluetooth`, `get_system_stats`, `network_speedtest`.
5. **Wayland Clipboard**: `get_clipboard`, `set_clipboard`.
6. **Hyprland Orchestration**: `switch_workspace`, `focus_application`, `close_active_window`, `toggle_fullscreen`, `toggle_layout_split`.
7. **System Audio, Display & Themes**: `adjust_volume`, `set_brightness`, `set_theme`, `get_battery`, `launch_application`, `notify`.
8. **Web Navigation**: `open_youtube`, `search_web`, `open_url`.
9. **Multi-Workspace Workflows**: `launch_workflow`, `list_workflows`, `capture_current_workflow`, `save_custom_workflow`, `delete_custom_workflow`, `get_workflow_details`.
10. **Calendar & Reminders**: `schedule_event`, `set_reminder`, `list_reminders`, `clear_reminders`.
11. **Email Drafting**: `draft_email`.
12. **Notes & Thought Capture**: `create_note`, `list_notes`.
13. **Autonomous Project & Code Generation**: `create_project`, `delegate_to_antigravity`.
14. **Screen Multimodal Vision**: `inspect_screen`.
15. **Conversational Lifecycle**: `dismiss_session`.
