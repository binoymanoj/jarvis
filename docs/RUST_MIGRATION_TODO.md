# 󰚩 Jarvis: Rust Migration Blueprint & Architecture Roadmap

**Document Status:** Future Engineering Plan & Backlog  
**Document Path:** `docs/RUST_MIGRATION_TODO.md`  
**Target Architecture:** Rust Micro-Daemon (24/7 Idle) + Decoupled Python AI Worker (On-Demand)  
**Target Memory Footprint:** **~15 – 25 MB Idle RAM** (down from ~370–410 MB; a **94% reduction**)  

---

## 1. Executive Summary & Motivation

Jarvis currently operates as a monolithic Python daemon (`jarvis.service`). While its end-to-end responsiveness (~1.3s) and tool capabilities (48 native functions) are best-in-class, its background idle footprint is **~370 to 410 MB of resident RAM (RSS)**.

Step-by-step memory profiling revealed that:
* **The tools and agent logic are NOT the problem**: Hyprland IPC, workflows, shell, clipboard, power controls, Google GenAI SDK, and Groq SDK consume **less than 15 MB combined**.
* **Local ML model graphs and scientific libraries consume ~380 MB**:
  - `openwakeword`: **~180 MB** (unconditionally imports `scipy`, `scipy.stats`, and `scikit-learn`).
  - `piper-tts`: **~145 MB** (preloads phonemizer, `espeak-ng`, and the 61 MB ONNX voice graph in resident RAM 24/7).
  - C-shared runtimes (`onnxruntime`, `numpy`, `sounddevice`): **~58 MB**.

By bifurcating the architecture—moving the 24/7 background audio listener, Hyprland sockets, and top-bar IPC to a **compiled Rust micro-daemon**, and keeping the AI reasoning, multimodal vision, and tool suite in **Python**—Jarvis will idle at **under 25 MB RAM** with instantaneous wake times.

---

## 2. Target Bifurcated Architecture

```
┌──────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│                                     RUST MICRO-DAEMON: `jarvis-daemon`                                   │
│                                    (24/7 Background User Service in Rust)                                │
│                                                                                                          │
│  • Memory Footprint: ~15 – 25 MB RSS                                                                     │
│  • Audio Stream: Direct PipeWire / ALSA capture via `cpal` (16 kHz mono)                                 │
│  • Wake Word Engine: Native C/C++ ONNX Runtime (`ort` crate) running `hey_jarvis_v0.1.onnx`              │
│  • Soft AGC: Lightweight SIMD-accelerated RMS gain booster                                               │
│  • Hyprland IPC: Async Unix domain socket client (`.socket` & `.socket2`)                                │
│  • System Status: Writes `/run/user/1000/jarvis-status.json` for Omarchy Top Bar                          │
│  • Hotkey Coordination: Listens for `SUPER + SHIFT + ENTER` and `SUPER + ALT + ESC`                      │
└──────────────────────────────────────────────────┬───────────────────────────────────────────────────────┘
                                                   │
                                                   │ UNIX Domain Socket IPC / Shared Memory Buffer
                                                   │ (Dispatched on "Hey Jarvis" or Hotkey Press)
                                                   ▼
┌──────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│                                   PYTHON AI WORKER: `jarvis-worker`                                      │
│                                   (On-Demand / Event-Driven Python Agent)                                │
│                                                                                                          │
│  • Speech-to-Text: Groq Whisper Large v3 Turbo (~420ms)                                                  │
│  • Agent Reasoning: Google Gemini 3.5 Flash Lite + 5-tier quota fallback pool                            │
│  • 48 Native Tools: Workflows, Virtual Typing (`wtype`), Shell, Media (MPRIS), Clipboard, Power, etc.   │
│  • Multimodal Vision: Screen inspection via `grim` + Gemini Vision                                       │
│  • Speech Synthesis: Transient Piper TTS or cloud neural Edge TTS (exits after speech)                   │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Component Breakdown & Language Responsibilities

### 3.1. What Moves to Rust (`jarvis-daemon`)
| Component | Rust Crate / Technology | Role & Advantage |
| :--- | :--- | :--- |
| **Audio Capture** | `cpal` / `pipewire-rs` | Zero-allocation real-time 16kHz audio buffer collection from PipeWire. |
| **Wake Word Detection** | `ort` (ONNX Runtime bindings) | Runs `hey_jarvis_v0.1.onnx` directly via C API without Python, SciPy, or Scikit-learn overhead. |
| **Audio Gating / VAD** | `silero-vad` ONNX | Detects start and end of speech with <1ms CPU latency and ~2MB memory. |
| **Chime Synthesis** | `rodio` / raw PCM | Synthesizes and plays the two-tone wake confirmation chime instantly. |
| **Hyprland Event Stream** | `tokio::net::UnixStream` | Consumes `.socket2.sock` events to track workspace and active window changes. |
| **Omarchy Top Bar IPC** | `serde_json` + atomic file write | Updates `/run/user/1000/jarvis-status.json` with zero disk latency. |
| **Process Supervisor** | `tokio::process::Command` | Dispatches or signals the Python AI worker when user input is ready. |

### 3.2. What Stays in Python (`jarvis-worker`)
| Component | Python Library | Reason to Keep in Python |
| :--- | :--- | :--- |
| **LLM Reasoning & Agent Loop** | `google-genai` (Official SDK) | Google's Python SDK offers first-class support for Gemini 3.5 Flash, structured function calling, and streaming tools. |
| **Speech-to-Text (STT)** | `groq` (AsyncGroq SDK) | Simple, robust async HTTPS client for Groq Whisper Large v3 Turbo. |
| **Extensible Tool Palette** | Python functions | Rapid prototyping of new desktop tools, custom workflows, and bash automation without recompilation. |
| **Multimodal Screen Perception** | `google.genai.types.Part` | Fast image encoding and multimodal payload assembly. |

---

## 4. Phase-by-Phase Migration Plan

### Phase 1: Standalone Rust Wake Word & Audio Engine
- [ ] **1.1 Crate Workspace**: Initialize a Cargo workspace under `crates/jarvis-daemon/`.
- [ ] **1.2 PipeWire Capture**: Implement audio recording stream using `cpal` (16,000 Hz, 1 channel, 16-bit PCM).
- [ ] **1.3 Native ONNX Inference**: Bind `ort` (ONNX Runtime) to evaluate `hey_jarvis_v0.1.onnx` in 80ms chunk windows.
- [ ] **1.4 Soft AGC & Audio Normalization**: Port Python's dynamic volume multiplier to Rust to ensure sensitivity with laptop/Bluetooth microphones.
- [ ] **1.5 Wake Chime**: Implement instant PCM chime playback upon wake word trigger.
- [ ] **Benchmark Target**: Memory usage `< 20 MB RSS`, idle CPU `< 1.0%`.

### Phase 2: Hyprland & IPC Integration in Rust
- [ ] **2.1 Hyprland Socket Client**: Implement non-blocking async UNIX socket client for Hyprland's `.socket.sock` and `.socket2.sock`.
- [ ] **2.2 Status State Machine**: Manage states (`standby`, `listening`, `thinking`, `speaking`, `muted`) and update `/run/user/1000/jarvis-status.json`.
- [ ] **2.3 Hotkey & Signal Handling**: Handle `SIGUSR1` (force submit), `SIGUSR2` (PTT toggle), and `SIGHUP` (emergency kill).
- [ ] **2.4 Quickshell HUD IPC**: Forward audio RMS volume levels and state transitions to the Quickshell HUD over IPC.

### Phase 3: Rust-to-Python IPC Bridge Protocol
- [ ] **3.1 IPC Socket Protocol**: Establish a local UNIX domain socket at `/run/user/1000/jarvis-ai.sock`.
- [ ] **3.2 Message Schema**:
  ```json
  {
    "event": "voice_input_ready",
    "wav_path": "/run/user/1000/jarvis-audio.wav",
    "trigger_type": "wakeword",
    "focused_window": { "title": "...", "class": "...", "workspace": 1 }
  }
  ```
- [ ] **3.3 Worker Lifecycle**: Support two execution modes:
  - *Mode A (Persistent Resident Worker)*: Python worker sleeps on UNIX socket; active only during turns.
  - *Mode B (On-Demand Ephemeral Worker)*: Rust spawns Python worker per session and terminates it when dismissed.

### Phase 4: Decoupled Speech Synthesis (TTS Optimization)
- [ ] **4.1 Piper Standalone Binary**: Replace in-process `from piper import PiperVoice` with execution of the standalone `piper` binary (`echo "text" | piper ... | pw-play`).
- [ ] **4.2 Edge-TTS Cloud Alternative**: Default to `edge-tts` for cloud-based neural synthesis (zero resident model memory).
- [ ] **4.3 Resident Voice Unloading**: If local Piper is required, lazily allocate the model only when speaking and unmap after a 30-second inactivity timeout.

### Phase 5: Packaging & Systemd Migration
- [ ] **5.1 Unified Build Script**: Setup `justfile` or `cargo-make` to build the Rust binary and prepare the Python environment.
- [ ] **5.2 Systemd Unit Update**: Update `~/.config/systemd/user/jarvis.service` to run the compiled Rust binary:
  ```ini
  [Service]
  ExecStart=%h/.local/bin/jarvis-daemon
  Restart=always
  ```
- [ ] **5.3 Backward Compatibility**: Ensure all existing CLI flags (`jarvis -t`, `jarvis -k`, `jarvis -c`, `jarvis --workflow-list`) continue to operate transparently.

---

## 5. Phase 0: Immediate Python Trimming (Prior to Rust Migration)

Before executing the full Rust rewrite, the following immediate steps can be applied to the existing Python codebase to lower memory from **~400 MB to ~110 MB**:

1. **Lazy-Load Piper TTS (`-145 MB`)**:
   - Do not load `PiperVoice` during daemon initialization.
   - Load it on-demand only when Jarvis speaks, or invoke the standalone `piper` CLI binary.
2. **Eliminate SciPy/Sklearn from Wake Word Path (`-120 MB`)**:
   - `openwakeword`'s top-level imports load Scikit-Learn and SciPy.
   - Refactor `src/jarvis/audio/wakeword.py` to directly instantiate `onnxruntime.InferenceSession` for the three ONNX models (`melspectrogram.onnx`, `embedding_model.onnx`, `hey_jarvis_v0.1.onnx`), bypassing the Python package's training dependencies.
3. **Run Push-to-Talk Mode on Demand**:
   - In environments where persistent ~100MB is undesired, running in hotkey-only mode (`jarvis --wakeword-toggle`) yields **0 MB idle RAM**.

---

## 6. Expected Resource Benchmarks: Before vs. After

| Metric | Current Monolithic Python | Phase 0: Optimized Python | Phase 1–5: Rust + Python Hybrid |
| :--- | :--- | :--- | :--- |
| **Idle Memory (RSS)** | **~373 MB** | **~90 – 120 MB** | **~15 – 25 MB** |
| **Active Turn Memory** | ~408 MB | ~250 MB | ~180 MB |
| **Idle CPU Load** | ~10–12% of 1 core | ~8–10% of 1 core | **< 1.0% of 1 core** |
| **Daemon Startup Time** | ~2.5 seconds | ~1.2 seconds | **< 50 milliseconds** |
| **End-to-End Turn Latency** | ~1.33 seconds | ~1.33 seconds | **~1.05 – 1.20 seconds** |
| **Zero-Interruption Quota Fallback**| Supported (Gemini) | Supported (Gemini) | Supported (Gemini) |

---

## 7. Next Actions

When we are ready to commence this migration:
1. Create the `crates/jarvis-daemon` directory.
2. Build the prototype `cpal` + `ort` audio listener in Rust.
3. Benchmark the standalone binary on Omarchy Linux.
