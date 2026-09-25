# Omarchy Jarvis: Master Architecture, Free-Tier Strategy & Living Roadmap

> **Project Target**: AI Desktop Voice Assistant ("Jarvis") natively integrated with Omarchy Linux & Hyprland.  
> **Host Environment**: Omarchy Linux (Arch Linux base, Hyprland 0.56+, Quickshell 0.3.1, PipeWire, Intel Core i7-1185G7, 31 GB RAM).  
> **Core Language**: **100% Native Rust** (`jarvis v0.2.0`) with Tokio Async Runtime, CPAL, and `mimalloc`.  
> **Activation**: **Continuous Offline Wake Word ("Hey Jarvis")** + **Push-to-Talk Hotkey (`SUPER + C`)**.  
> **Emergency Kill Switch**: **`SUPER + ALT + ESCAPE`**  
> **Full Performance & Specs Reference**: See [docs/PERFORMANCE_AND_SPECS.md](file:///home/binoy/Codes/personal/jarvis/docs/PERFORMANCE_AND_SPECS.md)  
> **Document Status**: Master architectural blueprint and living engineering roadmap.

```
󰚩 Omarchy Jarvis v0.2.0 (100% Rust Native)
  Architecture:      x86_64-unknown-linux-gnu
  Memory Allocator:  mimalloc (Drop-in high performance allocator)
  AI Provider:       gemini (Google AI Studio)
  Reasoning Model:   gemini-3.5-flash-lite (with 5-tier multi-model fallback)
  Wake Word:         openWakeWord ONNX ('hey_jarvis_v0.1.onnx', threshold: 0.50)
  Editor / Terminal: nvim via kitty
  STT Engine:        groq (whisper-large-v3-turbo)
  TTS Engine:        edge (en-GB-RyanNeural) / local piper-tts
  Active API Key:    Configured
  Groq Whisper Key:  Configured
  Background Daemon: Active (systemd user unit `jarvis.service`)
```

---

## 1. Project Overview & Native Rust Architecture

```
                           ┌─────────────────────────────────────────────────────────┐
                           │               KEYBIND TRIGGER: SUPER + C                │
                           └────────────────────────────┬────────────────────────────┘
                                                        │
                                                        ▼
┌──────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│                                       JARVIS RUNTIME DAEMON (100% Pure Rust)                                     │
│                                                                                                                  │
│  ┌───────────────────────────────┐     ┌────────────────────────────────┐     ┌───────────────────────────────┐  │
│  │     AUDIO INPUT & VAD         │     │     FREE-TIER LLM & AGENT      │     │      SPEECH SYNTHESIS         │  │
│  │ • PipeWire (cpal 0.18)        │────▶│ • Google Gemini (reqwest 0.13) │────▶│ • msedge-tts (neural stream)  │  │
│  │ • openWakeWord ONNX (ort 2.0) │     │ • GroqCloud (Whisper & Llama)  │     │ • piper-tts (local fallback)  │  │
│  │ • Silero VAD (ort 2.0)        │     │ • Native Tool Registry (56)    │     │ • Direct PipeWire / pw-play   │  │
│  └───────────────────────────────┘     └───────────────┬────────────────┘     └───────────────────────────────┘  │
│                                                        │                                                         │
│                                                        ▼                                                         │
│                                        ┌────────────────────────────────┐                                        │
│                                        │   TOOL DISPATCHER & CONTEXT    │                                        │
│                                        │ • Hyprland Socket (`.socket`)  │                                        │
│                                        │ • Hyprland Events (`.socket2`) │                                        │
│                                        │ • Omarchy 350+ CLI Suite       │                                        │
│                                        │ • Screen Capture (`grim`)      │                                        │
│                                        └────────────────────────────────┘                                        │
└───────────────────────────────────────────────────────┬──────────────────────────────────────────────────────────┘
                                                        │ IPC State Stream (/run/user/1000/jarvis-status.json)
                                                        ▼
┌──────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│                                 FLOATING WAYLAND HUD (Quickshell QML Wave HUD)                                   │
│  • Located at bottom-center of screen via Wayland layer-shell                                                    │
│  • 15-bar symmetrical glowing gradient waveform reacting in real time to microphone volume                       │
│  • Rotates & pulses during LLM reasoning ("Thinking...")                                                         │
│  • Resonates smoothly during speech playback ("Speaking...")                                                     │
│  • Automatically collapses and fades out when idle                                                               │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Free-Tier API Strategy (Zero Cloud Cost)

| Role | Recommended Provider | Free Tier Allowance | Latency | Why it was chosen |
| :--- | :--- | :--- | :--- | :--- |
| **Reasoning & Tool Execution** | **Google AI Studio** (`gemini-3.5-flash-lite`, `gemini-2.5-flash`) | **15 RPM**, 1,000,000 TPM, **1,500 Requests/Day** free | ~400–700 ms | Massive context window, native parallel function calling, multimodal vision for screen diagnostics, zero credit card required. |
| **Fast Reasoning Fallback** | **GroqCloud** (`llama-3.3-70b-versatile`) | **30 RPM**, 6,000 TPM, 1,000 RPD free | ~150–250 ms (~300 t/s) | Ultra-fast LLM inference; instant response for simple commands. |
| **Speech-to-Text (STT)** | **Groq Audio API** (`whisper-large-v3-turbo`) | **20 RPM**, 2,000 audio seconds/min free | **~100–180 ms** | Transcribes full voice prompts 5x faster than real-time with zero local CPU load. |
| **Wake Word Engine** | **openWakeWord ONNX** (`hey_jarvis_v0.1.onnx`) | Unlimited / 100% offline local CPU | **~25 ms** | Sub-1% CPU usage via single-threaded, no-spinning ONNX Runtime (`ort`). |
| **Voice Activity Detection** | **Silero VAD ONNX** (`silero_vad.onnx`) | Unlimited / 100% offline local CPU | **~3 ms** | Detects human speech and silence pauses (0.85s cutoff) with zero cloud dependency. |
| **Text-to-Speech (TTS)** | **Microsoft Edge TTS** (`en-GB-RyanNeural`) | Unlimited free | **~180–220 ms** | Natural neural British voice synthesis over streaming HTTP. |
| **TTS (Local Offline Fallback)**| **`piper-tts`** (`en_GB-alan-medium.onnx`) | Unlimited / 100% offline | **< 50 ms** first chunk | High quality local voice synthesis directly to `pw-play`. |

---

## 3. Rust Engine & Systems Architecture

### 3.1 Core Components
* **Asynchronous Runtime**: [`tokio`](https://tokio.rs) v1.53 with multi-thread worker pool limited to 2 worker threads, conserving thread stacks and kernel contexts.
* **Global Memory Allocator**: [`mimalloc`](https://github.com/purpleprotocol/mimalloc_rust) v0.1.52, eliminating glibc multi-arena fragmentation and returning memory pages aggressively to Linux.
* **Audio Capture**: [`cpal`](https://crates.io/crates/cpal) v0.18 collecting 16 kHz 16-bit mono PCM chunks from PipeWire.
* **ONNX Runtime**: [`ort`](https://ort.pyke.io) v2.0.0-rc.13 with global thread pool configuration (`with_spin_control(false)`, `with_intra_threads(1)`, `with_inter_threads(1)`), eliminating CPU busy-waiting.
* **HTTP Client**: [`reqwest`](https://crates.io/crates/reqwest) v0.13 with native TLS and connection pooling.
* **Hyprland IPC**: Native async UNIX domain socket client (`tokio::net::UnixStream`) querying `$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket.sock`.

---

## 4. Keybinding & Floating Wave HUD Architecture

### 4.1 Activation Flow
1. User presses **`SUPER + C`** anywhere in Hyprland or speaks **"Hey Jarvis"**.
2. Keybind calls `jarvis -t` (toggles PTT listening mode).
3. **Floating Transparent Animated Wave HUD appears** at the bottom-center of the screen:
   - Microphone streams to memory buffer via non-blocking PipeWire CPAL stream.
   - Symmetrical 15-bar electric waveform ripples and scales in real-time to microphone volume.
4. User speaks their command:
   - **Silero VAD** cuts off after **0.85s** of silence.
5. HUD transitions to **"Thinking..."** state (traveling scanning wave across the bars).
6. Jarvis dispatches tool call or response via Gemini Flash Lite:
   - High-speed Groq Whisper transcription (`whisper-large-v3-turbo`).
   - Native Rust tool execution against Hyprland, applications, shell, or desktop.
7. HUD pulses smoothly in **"Speaking..."** state while TTS plays audio.
8. HUD smoothly collapses and fades out when complete.

---

## 5. Master Roadmap & Completed Milestones

### Phase 1: Native Rust Audio Engine & Wake Word ✅
- [x] **CPAL PipeWire Capture**: 16,000 Hz, 1 channel, 16-bit PCM streaming.
- [x] **Native ONNX Inference**: Single-threaded `ort` running `melspectrogram.onnx`, `embedding_model.onnx`, and `hey_jarvis_v0.1.onnx`.
- [x] **Silero VAD v4**: Integrated offline voice activity detection for natural conversational turn completion.
- [x] **Wake Chime**: Instant procedural WAV chime generation and playback.

### Phase 2: Hyprland & Omarchy IPC Handlers ✅
- [x] **Direct Hyprland Command Socket**: Async client for `.socket.sock`.
- [x] **Hyprland Event Stream Listener**: Async event consumer for `.socket2.sock`.
- [x] **Omarchy Command Bridge**: Wrapper to execute Omarchy CLI commands and menu bar integration.
- [x] **Screen Perception Tool**: Crop & capture active window or full screen via `grim` and multimodal vision.

### Phase 3: AI Agent & 56 Tool Palettes ✅
- [x] **Multi-Model Fallback Pool**: 5-tier Gemini & Groq fallback matrix with automatic failover on 429 quota limits.
- [x] **56 Native Desktop Tools**: Complete suite covering Hyprland workspace management, media automation & MPRIS, virtual typing (`wtype`), keyboard shortcuts, calendar scheduling, email composition, notes, autonomous CLI coding, shell execution, clipboard, LocalSend sharing, Neovim research viewing, instant Google Meet meetings, and system power.
- [x] **Quickshell QML Wave HUD**: Fluid 60fps GPU-accelerated layer-shell HUD.

### Phase 4: Systems Hardening & Memory Optimization ✅
- [x] **Zero-Spinning ONNX**: Eliminated thread pool spin loops (`with_spin_control(false)` and `with_intra_op_spinning(false)`).
- [x] **mimalloc Integration**: Reduced heap retention and eliminated glibc multi-arena memory growth.
- [x] **Dependency Deduplication**: Unified reqwest, hyper, and TLS stacks to v0.13.

### Phase 5: Advanced Desktop Tooling & Visual Safety ✅
- [x] **Themed Confirmation Modal TUI**: Interactive floating card (`TUI.float`) with active Omarchy colors, pill buttons, and auto-cancellation countdown for critical operations (shutdown, reboot, logout).
- [x] **LocalSend First-Class Integration**: Instant local file, episode, clipboard, and screenshot transfer via `localsend_share` with automatic window focus.
- [x] **Deep Research in Floating Neovim**: Comprehensive multi-section Markdown report viewer inside a centered floating Neovim modal (`display_research_in_neovim`).
- [x] **TV Show & Media Automation**: Fuzzy-matching video playback (`play_media`) and history-backed resume (`resume_media`) via `mpv`.
- [x] **Activity History & Logging**: Structured append-only execution log (`jarvis.log`) at `~/.local/state/jarvis/jarvis.log` with `jarvis -l`, `-l -f`, `--logs-path`.
