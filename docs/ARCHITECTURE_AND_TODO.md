# Omarchy Jarvis: Architecture, Free-Tier Strategy & Living Roadmap

> **Project Target**: AI Desktop Assistant ("Jarvis") natively integrated with Omarchy Linux & Hyprland.  
> **Host Environment**: Omarchy Linux (Arch Linux base, Hyprland 0.56+, Quickshell 0.3.1, PipeWire, Intel Core i7-1185G7, 31 GB RAM).  
> **Core Language**: **Python 3.14+** (managed via `uv`)  
> **Activation**: **Continuous Offline Wake Word ("Hey Jarvis")** + **Push-to-Talk Hotkey (`SUPER + SHIFT + ENTER`)**.  
> **Emergency Kill Switch**: **`SUPER + ALT + ESCAPE`**  
> **Full Performance & Specs Reference**: See [docs/PERFORMANCE_AND_SPECS.md](file:///home/binoy/Codes/personal/jarvis/docs/PERFORMANCE_AND_SPECS.md)  
> **Document Status**: Living roadmap and master architectural blueprint.

󰚩 Omarchy Jarvis v0.2.0 (100% Rust Native)
  Architecture:     x86_64-unknown-linux-gnu
  AI Provider:      gemini
  Reasoning Model:  gemini-3.5-flash-lite
  Wake Word:        'jarvis' (threshold: 0.22)
  Editor / Terminal: nvim via kitty
  STT Engine:       groq (whisper-large-v3-turbo)
  TTS Engine:       edge (en-GB-RyanNeural)
  Active API Key:   Configured
  Groq Whisper Key: Configured
  Background Daemon: Active


---

## 1. Project Overview & Finalized Design Decisions

```
                           ┌─────────────────────────────────────────────────────────┐
                           │          KEYBIND TRIGGER: SUPER + SHIFT + ENTER         │
                           └────────────────────────────┬────────────────────────────┘
                                                        │
                                                        ▼
┌──────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│                                           JARVIS RUNTIME DAEMON (Python)                                         │
│                                                                                                                  │
│  ┌───────────────────────────────┐     ┌────────────────────────────────┐     ┌───────────────────────────────┐  │
│  │     AUDIO INPUT & VAD         │     │     FREE-TIER LLM & AGENT      │     │      SPEECH SYNTHESIS         │  │
│  │ • PipeWire (sounddevice)      │────▶│ • Google AI Studio (Gemini)    │────▶│ • piper-tts (local, <50ms)    │  │
│  │ • Silero VAD (end-of-speech)  │     │ • GroqCloud (Llama 3.3 70B)    │     │ • or edge-tts (neural cloud)  │  │
│  │ • Groq Whisper / Local int8   │     │ • Native Tool / Function Calls │     │ • Direct PipeWire playback    │  │
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
                                                        │ IPC State Stream (LISTENING / THINKING / SPEAKING)
                                                        ▼
┌──────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│                                 FLOATING WAYLAND HUD (Siri-like Animated Orb)                                    │
│  • Located at bottom-center of screen                                                                            │
│  • Smooth fluid multi-color pulsing orb reacting to microphone level                                             │
│  • Rotates/swirls during LLM reasoning ("Thinking")                                                             │
│  • Resonates smoothly during speech playback                                                                     │
│  • Automatically slides down / fades out when idle                                                               │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Free-Tier API Strategy (Zero Cloud Cost)

Since the Antigravity Pro subscription provides model access strictly inside the Antigravity developer environment and does not provide an external standalone API key for Python scripts, we utilize the **best free-tier providers**:

| Role | Recommended Provider | Free Tier Allowance | Latency | Why it was chosen |
| :--- | :--- | :--- | :--- | :--- |
| **Reasoning & Tool Execution** | **Google AI Studio** (`Gemini 2.5 Flash` or `2.0 Flash`) | **15 RPM**, 1,000,000 TPM, **1,500 Requests/Day** for free | ~400–700 ms | Massive context window, native function calling, multimodal vision for screen diagnostics, zero credit card required at `aistudio.google.com`. |
| **Fast Reasoning Fallback** | **GroqCloud** (`Llama 3.3 70B Versatile`) | **30 RPM**, 6,000 TPM, 1,000 RPD free | ~150–250 ms (~300 t/s) | Fastest LLM inference available. Instantaneous response for quick commands. |
| **Speech-to-Text (STT)** | **Groq Audio API** (`Whisper Large v3`) | **20 RPM**, 2,000 audio seconds/min free | **~100–180 ms** | Transcribes full voice prompts 5x faster than real-time. |
| **STT (Offline Fallback)** | **`faster-whisper`** (Local CPU) | Unlimited / 100% offline | ~250–350 ms on i7-1185G7 | Zero cloud dependency; uses `base.en` or `small.en` with int8 quantization. |
| **Text-to-Speech (TTS)** | **`piper-tts`** (Local CPU) | Unlimited / 100% offline | **< 50 ms** first chunk | High quality British (`en_GB-alan-medium`) or American voices, instant playback without cloud delays. |
| **TTS (Online Alternative)** | **`edge-tts`** (Microsoft Edge neural) | Unlimited free | ~200 ms | Extremely natural voices without API keys. |

---

## 3. Python Package Manager Evaluation

| Package Manager | Installation / Speed | Lockfile & Standards | System Integration (Arch/Omarchy) | Merits | Demerits |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **`uv`** ⭐ *(Recommended)* | Written in Rust. **10x–100x faster** than pip. | Native `uv.lock` + standard `pyproject.toml`. | Available in `extra/uv` via pacman or `mise use -g uv`. | • Single binary replaces `pip`, `pip-tools`, `venv`, `poetry`, and `pyenv`.<br>• Fully respects Arch PEP 668.<br>• Instant environment creation (<50ms). | Newer tool (rapidly becoming industry standard). |
| **`poetry`** | Python-based. Slower dependency solver. | `poetry.lock` + `pyproject.toml`. | Needs separate install (`pipx install poetry`). | Mature, rich plugin ecosystem. | Noticeably slow dependency resolution, heavy footprint. |
| **Standard `venv` + `pip`** | Built into Python stdlib. | Requires manual `pip-compile` for lockfiles. | Built-in (`python -m venv .venv`). | Zero external tools required; universal. | Manual virtualenv activation required; pip installs are slow; no unified lockfile. |
| **`pixi` / `conda`** | Fast (pixi is Rust-based). | Binary environment lockfile. | Installs non-Python libraries. | Excellent for complex C/CUDA builds. | Overkill for this project; non-standard Python environment. |

---

## 4. Keybinding & Floating Animated Wave HUD Architecture

### 4.1 Activation Flow
1. User presses **`SUPER + SHIFT + ENTER`** anywhere in Hyprland.
2. The keybind invokes `/home/binoy/.local/bin/jarvis -t`.
3. **Floating Transparent Animated Wave HUD appears** at the bottom-center of the screen:
   - Microphone streams to memory buffer via non-blocking PipeWire.
   - Symmetrical 15-bar electric waveform ripples and scales in real-time to microphone volume.
4. User speaks their command:
   - **Silero VAD** cuts off after **0.55s** of silence (optimized for instant response).
5. HUD transitions to **"Thinking" state** (traveling scanning wave across the bars).
6. Jarvis dispatches tool call or response via Gemini 2.5 Flash (`thinking_budget=0` for ~1.0s turnaround):
   - Fast Groq Whisper transcription (`whisper-large-v3-turbo`).
   - Native Hyprland Lua dispatch (`hl.dsp.focus({ workspace = "..." })`).
7. HUD pulses smoothly in **"Speaking" state** while `piper-tts` plays audio.
8. HUD smoothly collapses and fades out when done.

### 4.2 HUD Visual Design (Transparent Fluid Waves)
- **Zero background container**: No rectangular cards, no borders, no frosted boxes; 100% transparent layer-shell.
- **Wave Spectrum**: 15 vertical rounded gradient bars (`#38bdf8` cyan to `#c084fc` purple) with Gaussian bell envelope.
- **Centered Text**: Typography positioned directly below the waves with subtle outline shadow for legibility across any wallpaper.

### 4.2 HUD Implementation Options

| Approach | Technology | Pros | Cons |
| :--- | :--- | :--- | :--- |
| **Option A: Quickshell QML** ⭐ *(Recommended)* | Native Wayland layer-shell via `quickshell` (already installed on Omarchy) | • Native to Omarchy design system.<br>• True Wayland layer-shell: anchored to bottom, transparent, zero window frame.<br>• GPU-accelerated fluid particle/wave shaders at 60fps. | Requires QML script communicating with Python daemon via socket/IPC. |
| **Option B: PySide6 / Qt6 Window** | Python Qt6 with QPainter / QML | • Single codebase in Python.<br>• Rich graphics capabilities. | Requires Hyprland floating window rules (`float`, `pin`, `noborder`) and extra process memory (~70MB). |
| **Option C: Omarchy Notification Toast** | `omarchy-notification-send` | • Zero UI code needed.<br>• Fast and simple. | Not a Siri-like glowing orb; purely textual notifications. |

---

## 5. Master Roadmap & Living TODO List

### Phase 1: Environment & Architecture Setup
- [x] **1.1 Package Manager Initialization**: Initialize project with `uv` (`pyproject.toml`, virtual environment `.venv`).
- [x] **1.2 Free API Key Configuration**: Create `.env` template and loader for `GEMINI_API_KEY` and `GROQ_API_KEY`.
- [x] **1.3 Project Directory Structure**: Establish `src/audio`, `src/core`, `src/tools`, `src/ui`, and `tests`.
- [x] **1.4 Logging & Config Manager**: Set up structured logging with clean Omarchy-friendly console output.

### Phase 2: Hyprland & Omarchy IPC Handlers
- [x] **2.1 Direct Hyprland Command Socket**: Async client for `$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket.sock`.
- [x] **2.2 Hyprland State Querying**: High-speed JSON fetching for `activewindow`, `workspaces`, and `clients`.
- [x] **2.3 Hyprland Event Stream Listener**: Background async listener for `.socket2.sock` (tracking workspace & active window changes).
- [x] **2.4 Omarchy Command Bridge**: Wrapper to execute any of Omarchy's 350+ CLI commands (`omarchy audio`, `omarchy theme`, `omarchy brightness`, etc.).
- [x] **2.5 Screen Perception Tool**: Crop & capture active window or full screen via `grim` + `hyprctl activewindow -j`.

### Phase 3: Push-to-Talk Audio & Speech Pipeline
- [x] **3.1 Audio Capture via PipeWire**: Non-blocking audio capture using `sounddevice` / `numpy`.
- [x] **3.2 Voice Activity Detection (VAD)**: Lightweight Silero VAD (ONNX) detecting end of speech in <1ms without PyTorch/CUDA bloat.
- [x] **3.3 Cloud STT (Groq Whisper)**: Ultra-fast cloud transcription client (~100ms verified with live key).
- [x] **3.4 Local STT Fallback (`faster-whisper`)**: Offline transcription on CPU if cloud is unavailable or disabled.
- [x] **3.5 TTS Voice Output (`piper-tts` & `edge-tts`)**: Sub-50ms local speech generation with natural British Alan voice.

### Phase 4: Free-Tier LLM Agent & Function Calling
- [x] **4.1 Gemini 2.5 Flash Agent**: Client using `google-genai` SDK with verified free Google AI Studio key.
- [x] **4.2 Tool Schema Definitions**: Expose Hyprland and Omarchy capabilities as native threadsafe sync tools (`switch_workspace`, `focus_application`, `close_active_window`, `toggle_layout_split`, `toggle_fullscreen`, `adjust_volume`, `set_brightness`, `set_theme`, `get_battery`, `launch_application`, `inspect_screen`, `notify`).
- [x] **4.3 Multimodal Screen Analysis**: Send screenshot + user voice query to Gemini Flash for instant UI error diagnosis.
- [x] **4.4 Conversational Context Memory**: Maintain short-term conversational context and window focus state.

### Phase 5: Floating Siri-like Animated HUD
- [x] **5.1 HUD Architecture**: Standalone Quickshell QML overlay anchored at bottom-center with transparent background and zero window frame (`shell.qml`).
- [x] **5.2 State Machine**: `IDLE` ➔ `LISTENING` (reactive multi-layer glowing orb scaling with audio RMS) ➔ `THINKING` (spinning gradient swirl) ➔ `SPEAKING` (harmonic wave pulses) ➔ `EXIT` (smooth fade-out).
- [x] **5.3 IPC Channel**: Python controller (`hud.py`) drives states and throttles 20fps volume metering over Quickshell IPC.

### Phase 6: Hotkey Integration & Systemd Service
- [x] **6.1 Hyprland Keybinding**: Mapped `SUPER + SHIFT + RETURN` in `~/.config/hypr/bindings.lua` to `/home/binoy/.local/bin/jarvis -t` (reloaded and active in Hyprland).
- [x] **6.2 CLI Entrypoint**: Complete support for interactive voice HUD mode (`jarvis -t`), headless text commands (`jarvis -c "..."`), and status diagnostics (`jarvis --status`).
- [x] **6.3 System-Wide Availability**: Installed executable wrapper in `~/.local/bin/jarvis` accessible from any directory.

---

## 6. Current Task Tracking Dashboard

| Milestone / Task | Status | Target Completion | Notes |
| :--- | :--- | :--- | :--- |
| Architecture Document & Living Plan | **DONE** ✅ | Milestone 0 | Finalized in `docs/ARCHITECTURE_AND_TODO.md` |
| Keybinding Reservation (`SUPER + SHIFT + ENTER`) | **DONE** ✅ | Milestone 0 | Confirmed and wired in `bindings.lua` |
| Package Manager Choice (`uv`) | **DONE** ✅ | Milestone 0 | `uv 0.12.12` installed and verified |
| STT Strategy Choice (Hybrid Groq + Faster-Whisper) | **DONE** ✅ | Milestone 0 | Ultra-low latency cloud + offline fallback |
| UI Framework Choice (Quickshell QML) | **DONE** ✅ | Milestone 0 | Native Wayland layer-shell round orbit HUD |
| Hotkey Interaction Choice (Push-to-Talk Hold / Toggle + Conversational VAD) | **DONE** ✅ | Milestone 0 | Hold key or tap once; release or ~700ms natural pause submits |
| **Phase 1: Environment & Project Scaffolding** | **DONE** ✅ | Phase 1 | `pyproject.toml`, `.venv`, Pydantic config, Rich logger, CLI |
| **Phase 2: Hyprland & Omarchy IPC Handlers** | **DONE** ✅ | Phase 2 | Lua dispatchers, sockets, events, Omarchy commands, screen capture |
| **Free Tier API Key Setup (Gemini + Groq)** | **DONE** ✅ | Phase 3 | Both keys verified and configured in `.env` |
| **Phase 3: Push-to-Talk Audio & Speech Pipeline** | **DONE** ✅ | Phase 3 | PipeWire capture, conversational ~700ms Silero VAD, Groq Whisper STT, Piper TTS |
| **Phase 4: Free-Tier LLM Agent & Function Calling** | **DONE** ✅ | Phase 4 | Instant context snapshot, multi-model fallback pool (`gemini-flash-latest`, `gemini-3.5-flash`) with zero quota lockouts |
| **Phase 5: Minimal Futuristic Horizontal HUD** | **DONE** ✅ | Phase 5 | 56px minimal AI orbit, unclipped left-to-right text, dynamic Omarchy theme sync, 100% transparent |
| **Phase 6: Hotkey Integration & Kill Switch** | **DONE** ✅ | Phase 6 | `SUPER + SHIFT + ENTER` (Hold/Toggle PTT), `SUPER + ALT + ESCAPE` (Instant Kill Switch) |
| **Phase 7: Hands-Free Operating Suite & Zero-Touch Tools** | **DONE** ✅ | Phase 7 | Wayland `wtype` typing/shortcuts, bash execution, MPRIS media control, clipboard, and system power |
| **Phase 8: System Hardening & Performance Benchmarking** | **DONE** ✅ | Phase 8 | VAD room-noise lock eliminated, HUD IPC optimized, benchmarks & specs documented in `docs/PERFORMANCE_AND_SPECS.md` |


