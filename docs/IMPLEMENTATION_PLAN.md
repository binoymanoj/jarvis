# Omarchy Jarvis: Architecture & Implementation Plan

> **Project Target**: Intelligent Desktop Assistant ("Jarvis") natively integrated with Omarchy Linux & Hyprland compositor.  
> **Host Environment**: Omarchy Linux (Arch Linux rolling base, Hyprland 0.56+, Quickshell, Intel Core i7-1185G7, 32 GB RAM, Iris Xe Graphics).  
> **Author**: Binoy & Antigravity  
> **Date**: September 2026  

---

## 1. Executive Summary & Vision

Omarchy is an opinionated, developer-centric Linux distribution that takes the Unix philosophy to its logical conclusion: **every system capability is exposed as a composable command-line interface**, and the display server is driven by **Hyprland**, a Wayland tiling compositor with high-speed bidirectional UNIX domain socket IPC.

Traditional desktop voice assistants fail because interacting with desktop GUIs requires brittle accessibility trees or inaccurate pixel clicking. In Omarchy:
1. **The OS is already an API**: Omarchy exposes 350+ structured CLI commands (`omarchy commands --json`) controlling audio, themes, packages, display, capture, and app launching.
2. **Instant Window & Workspace Management**: Hyprland provides direct programmatic control over window creation, silent workspace relocation, tiling layout splits, floating state, and fullscreen.
3. **Event Stream Awareness**: Hyprland's `.socket2.sock` broadcasts real-time system events (workspace changes, active window focus, monitor connections), giving Jarvis continuous situational awareness.
4. **Scriptable Shell & Notifications**: Quickshell and Omarchy notifications (`omarchy-notification-send`) allow interactive notifications with custom glyphs and one-click `--exec` action buttons.

This document defines the complete technical blueprint, component selection, programming language trade-offs, and phased roadmap to build **Jarvis for Omarchy**.

---

## 2. System Architecture

Jarvis operates as a persistent user-level service (`systemd --user`) split into four decoupled layers:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            USER INTERACTION LAYER                           │
│  • Voice: Wake word ("Hey Jarvis") or Push-to-Talk hotkey (Super + Grave)    │
│  • Omarchy Shell: Custom Quickshell top-bar widget / status visualizer      │
│  • CLI Client: `jarvis "organize my workspace"`                             │
│  • Wayland HUD: Optional floating input / feedback window                   │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                          JARVIS CORE DAEMON (BRAIN)                         │
│  • Audio Pipeline: Wake Word Detector → STT Transcriber → TTS Synthesizer   │
│  • Reasoning & Agent Loop: Function-calling LLM (Gemini / Groq / Local)     │
│  • Context Memory: Tracks active workspace, focused window, system state    │
│  • Tool Dispatcher: Translates model tool calls into OS-level commands      │
└──────────────────────┬───────────────────────────────┬──────────────────────┘
                       │                               │
                       ▼                               ▼
┌──────────────────────────────────────┐  ┌───────────────────────────────────┐
│         HYPRLAND IPC (HANDS)         │  │     OMARCHY TOOLING (ECOSYSTEM)   │
│ • Command Socket (`.socket.sock`)    │  │ • 356 `omarchy <group> <cmd>`     │
│ • Event Socket (`.socket2.sock`)     │  │ • Audio / Volume / Sink Switching │
│ • Workspace management & presets     │  │ • Brightness / Power / Bluetooth  │
│ • Silent window relocation           │  │ • Screenshot OCR & Screen Recording│
│ • Dwindle layout / split control     │  │ • Theme switching & App Launchers │
└──────────────────────────────────────┘  └───────────────────────────────────┘
```

---

## 3. Technology Stack & Language Evaluation

### 3.1 Programming Language Comparison

| Metric / Language | **Python 3.14** ⭐ *(Recommended)* | **Rust 1.98** | **Go 1.27** | **TypeScript (Bun/Node)** |
| :--- | :--- | :--- | :--- | :--- |
| **AI Ecosystem** | **Unrivaled** (Official Google GenAI, Anthropic, LiteLLM, LangChain) | Limited to raw HTTP API wrappers | Medium | Excellent (Vercel AI SDK, MCP SDK) |
| **Audio / Speech ML** | **Native** (`faster-whisper`, `openWakeWord`, `piper-tts`) | Bindings require C/C++ builds | Weak | External process spawning required |
| **Async UNIX Sockets** | First-class (`asyncio.open_unix_connection`) | First-class (`tokio::net::UnixStream`) | First-class (`net.Dial("unix")`) | First-class (`node:net`) |
| **Hyprland Integration** | Socket-based or `hyprctl` JSON | Official `hyprland-rs` crate | Community packages | Socket / CLI |
| **Memory Footprint** | ~50–80 MB daemon | ~15 MB daemon | ~25 MB daemon | ~60 MB daemon |
| **Development Speed** | **Fastest** (instant iteration on tools/prompts) | Slowest (strict types, long compile cycles) | Fast | Very Fast |

> **Conclusion**: **Python** is the optimal choice for the Jarvis daemon. It eliminates friction when integrating leading LLM APIs, speech models, and multimodal tools, while its `asyncio` event loop is more than fast enough to communicate with Hyprland sockets with sub-millisecond overhead.

---

### 3.2 Component & Service Recommendations

Targeted for the system's **Intel Core i7-1185G7 (4 cores / 8 threads), 32 GB RAM, and Iris Xe Graphics**:

| Function | Solution | Implementation Details |
| :--- | :--- | :--- |
| **Wake Word Detection** | `openWakeWord` or Push-to-Talk | Runs 100% locally on CPU. Supports custom models ("Hey Jarvis", "Computer"). Push-to-talk mapped to `SUPER + GRAVE` via `~/.config/hypr/bindings.lua`. |
| **Speech-to-Text (STT)** | **Groq Audio API** (Cloud) or **`faster-whisper`** (Local) | • Cloud: Groq Whisper-large-v3 provides ~100ms transcription latency.<br>• Local: `faster-whisper` (`base.en` or `small.en` int8) transcribes in ~180ms on 32GB RAM. |
| **LLM Reasoning & Agent** | **Google Gemini 2.5 Flash** / **Groq Llama 3.3 70B** | Sub-second response time, native JSON function calling, multimodal vision for screen diagnosis, and a large context window. |
| **Text-to-Speech (TTS)** | **`piper-tts`** (Local) | Zero cloud latency (<50ms time-to-first-chunk on CPU). The `en_GB-alan-medium` model gives a clean British assistant tone. |
| **Screen Perception** | `grim` + `hyprctl activewindow -j` | Takes instantaneous cropped screenshots of the active window or region and passes them to Gemini for visual troubleshooting. |
| **Desktop Notifications** | `omarchy-notification-send` | Rich notifications with glyphs (`-g`), urgency levels, and click actions (`--exec`). |

---

## 4. Deep Integration with Omarchy & Hyprland

### 4.1 Hyprland Socket Architecture

Hyprland exposes two UNIX domain sockets located in `$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/`:

1. **`.socket.sock` (Command Dispatcher)**:
   - Accepts raw text commands (e.g., `dispatch workspace 2`, `dispatch movetoworkspacesilent 3,address:0x...`).
   - Bypasses `hyprctl` process spawning overhead for sub-millisecond execution.

2. **`.socket2.sock` (Event Stream)**:
   - Emits continuous newline-delimited event streams:
     ```text
     workspace>>2
     focusedmon>>eDP-1,2
     activewindow>>helium,POV: Valorant on RTX 3070 Ti...
     openwindow>>0x5567642834a0,1,foot,foot
     closewindow>>0x5567642834a0
     ```
   - Jarvis maintains live in-memory state of what application the user is working in.

---

### 4.2 Dynamic Omarchy Tool Discovery

Instead of manually maintaining code for hundreds of OS utilities, Jarvis queries Omarchy's machine-readable command database at startup:

```bash
omarchy commands --json
```

Each entry contains:
```json
{
  "route": "omarchy audio output volume",
  "summary": "Adjust output volume and show the Omarchy OSD",
  "args": "[+|-]<percent>",
  "examples": ["omarchy audio output volume +5%"]
}
```

Jarvis automatically converts these definitions into LLM tool schemas:
- *"Turn down the volume by 15 percent"* → runs `omarchy audio output volume -15%`
- *"Switch to tokyo night theme"* → runs `omarchy theme set tokyo-night`
- *"Capture this text from my screen"* → runs `omarchy capture text`

---

### 4.3 Workspace Presets & Automation Scenarios

#### Preset 1: "Jarvis, setup my coding workspace"
1. Switch to Workspace 1 (`hyprctl dispatch workspace 1`).
2. Launch code editor on the left pane (`hyprctl dispatch exec zed`).
3. Launch terminal with tmux on the right pane (`hyprctl dispatch exec "omarchy launch terminal tmux"`).
4. Silently open project documentation on Workspace 2 (`hyprctl dispatch exec "[workspace 2 silent] helium https://docs.rs"`).
5. Send Omarchy confirmation toast:
   ```bash
   omarchy-notification-send -g "󰚩" "Jarvis" "Development workspace active on Workspace 1 & 2."
   ```

#### Preset 2: "Jarvis, look at my screen and fix this error"
1. Inspect active window geometry from `hyprctl activewindow -j`.
2. Capture screenshot with `grim -g "<x>,<y> <w>x<h>" /tmp/jarvis_active.png`.
3. Dispatch image to Gemini 2.5 Flash with the prompt: *"Explain this terminal / editor error and provide the command to resolve it."*
4. Present the solution via speech (TTS) and notification with an executable click action:
   ```bash
   omarchy-notification-send -g "󰚩" "Jarvis Analysis" "Run cargo check to resolve dependency error" \
     --exec "omarchy launch terminal -e cargo check"
   ```

---

## 5. Phased Implementation Roadmap

### Phase 1: Foundation & Tool Dispatcher (Milestone 1)
- [ ] Create project skeleton in Python (`uv` or `pip` virtualenv).
- [ ] Implement `HyprlandController` to communicate directly with `.socket.sock` and query `hyprctl clients -j` / `activewindow -j`.
- [ ] Implement `OmarchyBridge` to run `omarchy <group> <command>`.
- [ ] Build a CLI entrypoint: `jarvis "command string"` to test tool execution without voice.

### Phase 2: LLM Brain & Function Calling (Milestone 2)
- [ ] Integrate Google Gemini 2.5 Flash via `google-genai` SDK.
- [ ] Register core tools:
  - `workspace_switch(id)`
  - `window_move(workspace_id, window_title)`
  - `layout_toggle()`
  - `launch_app(app_name, workspace_id)`
  - `set_system_setting(setting, value)` (volume, brightness, theme)
  - `screen_inspect(prompt)` (multimodal screenshot analysis)
- [ ] Implement short-term conversational context memory.

### Phase 3: Speech Pipeline (Hearing & Voice) (Milestone 3)
- [ ] Set up `piper-tts` with `en_GB-alan-medium` for sub-50ms voice playback.
- [ ] Set up STT:
  - Fast cloud path: Groq Whisper API for ultra-low latency.
  - Local path: `faster-whisper` (int8 quantized) for offline use.
- [ ] Bind Push-to-Talk shortcut in `~/.config/hypr/bindings.lua`:
  ```lua
  o.bind("SUPER + GRAVE", "Jarvis Push-to-Talk", "jarvis --record-toggle")
  ```

### Phase 4: Event Stream & Proactive Behavior (Milestone 4)
- [ ] Create an async background task listening to `~/.socket2.sock`.
- [ ] Emit notifications or adjust settings on state transitions (e.g., monitor connect/disconnect, battery warnings via `omarchy battery`).

### Phase 5: Quickshell UI Integration (Milestone 5)
- [ ] Create an Omarchy shell plugin in `~/.config/omarchy/plugins/jarvis/`.
- [ ] Display an animated microphone / waveform widget in the top bar using Quickshell QML.
- [ ] Connect the widget to the Jarvis daemon via local IPC.

---

## 6. Reference Implementation (Starter Code)

### `src/hyprland.py` (Direct Socket Controller)

```python
import asyncio
import json
import os
from typing import Any, Dict

class HyprlandController:
    def __init__(self):
        self.sig = os.environ.get("HYPRLAND_INSTANCE_SIGNATURE")
        runtime_dir = os.environ.get("XDG_RUNTIME_DIR", f"/run/user/{os.getuid()}")
        self.cmd_socket = f"{runtime_dir}/hypr/{self.sig}/.socket.sock"
        self.event_socket = f"{runtime_dir}/hypr/{self.sig}/.socket2.sock"

    async def dispatch(self, command: str) -> str:
        """Send a dispatch command directly over the Hyprland UNIX socket."""
        reader, writer = await asyncio.open_unix_connection(self.cmd_socket)
        writer.write(f"dispatch {command}\n".encode())
        await writer.drain()
        response = await reader.read(4096)
        writer.close()
        await writer.wait_closed()
        return response.decode().strip()

    async def get_active_window(self) -> Dict[str, Any]:
        """Fetch current active window metadata as JSON."""
        proc = await asyncio.create_subprocess_exec(
            "hyprctl", "activewindow", "-j",
            stdout=asyncio.subprocess.PIPE
        )
        stdout, _ = await proc.communicate()
        return json.loads(stdout.decode())

    async def get_workspaces(self) -> list:
        """List active workspaces."""
        proc = await asyncio.create_subprocess_exec(
            "hyprctl", "workspaces", "-j",
            stdout=asyncio.subprocess.PIPE
        )
        stdout, _ = await proc.communicate()
        return json.loads(stdout.decode())
```

### `src/omarchy.py` (Omarchy CLI Bridge)

```python
import asyncio
from typing import List, Optional

class OmarchyBridge:
    @staticmethod
    async def run(subcommand: str, *args: str) -> str:
        """Execute any command in the Omarchy command suite."""
        cmd = ["omarchy"] + subcommand.split() + list(args)
        proc = await asyncio.create_subprocess_exec(
            *cmd,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE
        )
        stdout, stderr = await proc.communicate()
        return stdout.decode().strip() if proc.returncode == 0 else stderr.decode().strip()

    @staticmethod
    async def notify(headline: str, body: str = "", glyph: str = "󰚩", exec_cmd: Optional[List[str]] = None):
        """Send an Omarchy native desktop notification with optional click action."""
        cmd = ["omarchy-notification-send", "-g", glyph, headline]
        if body:
            cmd.append(body)
        if exec_cmd:
            cmd.extend(["--exec"] + exec_cmd)
        await asyncio.create_subprocess_exec(*cmd)
```

---

## 7. Next Steps

1. Initialize project repository structure:
   ```bash
   mkdir -p src/tools src/audio tests
   ```
2. Set up Python virtual environment and dependencies (`google-genai`, `piper-tts`, `sounddevice`, `numpy`).
3. Test Hyprland socket communication and Omarchy dispatch in a live session.
