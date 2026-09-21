# Omarchy Jarvis: Architecture & Implementation Plan

> **Project Target**: Intelligent Desktop Assistant ("Jarvis") natively integrated with Omarchy Linux & Hyprland compositor.  
> **Host Environment**: Omarchy Linux (Arch Linux rolling base, Hyprland 0.56+, Quickshell, Intel Core i7-1185G7, 32 GB RAM, Iris Xe Graphics).  
> **Architecture**: **100% Native Rust** (Tokio Async, CPAL, `ort` ONNX Runtime, `mimalloc`).  
> **Author**: Binoy & Antigravity  
> **Date**: September 2026 (Updated for Native Rust v0.2.0)  

---

## 1. Executive Summary & Vision

Omarchy is an opinionated, developer-centric Linux distribution where every system capability is exposed as a composable command-line interface, and the display server is driven by **Hyprland**, a Wayland tiling compositor with high-speed bidirectional UNIX domain socket IPC.

In Omarchy:
1. **The OS is an API**: Omarchy exposes 350+ structured CLI commands (`omarchy commands --json`) controlling audio, themes, packages, display, capture, and app launching.
2. **Instant Window & Workspace Management**: Hyprland provides direct programmatic control over window creation, silent workspace relocation, tiling layout splits, floating state, and fullscreen.
3. **Event Stream Awareness**: Hyprland's `.socket2.sock` broadcasts real-time system events (workspace changes, active window focus, monitor connections), giving Jarvis continuous situational awareness.
4. **Scriptable Shell & Notifications**: Quickshell and Omarchy notifications (`omarchy-notification-send`) allow interactive notifications with custom glyphs and one-click `--exec` action buttons.

---

## 2. Native Rust Architecture

Jarvis operates as a persistent user-level service (`systemd --user`) split into four decoupled layers in pure Rust:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            USER INTERACTION LAYER                           │
│  • Voice: Wake word ("Hey Jarvis") or Push-to-Talk hotkey (Super+C)         │
│  • Omarchy Shell: Custom Quickshell top-bar widget / status visualizer      │
│  • CLI Client: `jarvis -c "organize my workspace"`                          │
│  • Wayland HUD: Transparent 15-bar glowing animated waveform                │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                          JARVIS CORE DAEMON (BRAIN)                         │
│  • Audio Pipeline: CPAL 0.18 capture → openWakeWord ONNX → Silero VAD       │
│  • Reasoning & Agent Loop: Function-calling LLM (Gemini 3.5 / Groq Fallback)│
│  • Context Memory: Tracks active workspace, focused window, system state    │
│  • Tool Dispatcher: 51 Native Rust tool implementations                     │
│  • Allocator: mimalloc for aggressive page reclamation and low idle RAM     │
└──────────────────────┬───────────────────────────────┬──────────────────────┘
                       │                               │
                       ▼                               ▼
┌──────────────────────────────────────┐  ┌───────────────────────────────────┐
                       │
                       ▼                               ▼
┌──────────────────────────────────────┐  ┌───────────────────────────────────┐
│         HYPRLAND IPC (HANDS)         │  │     OMARCHY TOOLING (ECOSYSTEM)   │
│ • Command Socket (`.socket.sock`)    │  │ • 356 `omarchy <group> <cmd>`     │
│ • Event Socket (`.socket2.sock`)     │  │ • Audio / Volume / Sink Switching │
│ • Workspace management & presets     │  │ • Brightness / Power / Bluetooth  │
│ • Silent window relocation           │  │ • Screenshot OCR & Screen Capture │
│ • Dwindle layout / split control     │  │ • Theme switching & App Launchers │
└──────────────────────────────────────┘  └───────────────────────────────────┘
```

---

## 3. Technology Stack & Language Selection

### 3.1 Why 100% Native Rust Won
* **Zero Idle CPU**: With ONNX Runtime thread spinning disabled (`with_spin_control(false)`) and single-threaded inference (`with_intra_threads(1)`), background CPU drops below 1%.
* **Drastic Memory Reduction**: Dropped idle RSS from ~410 MB to ~25–35 MB, freeing system resources for heavy development workloads.
* **Instant Start Time**: Native binary boots in ~2ms without interpreter initialization.
* **Single Statically-Linked Binary**: Eliminates virtual environments, wheel dependencies, or runtime conflicts.

---

## 4. Deep Integration with Omarchy & Hyprland

### 4.1 Hyprland Socket Architecture
Hyprland exposes two UNIX domain sockets located in `$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/`:
1. **`.socket.sock` (Command Dispatcher)**:
   - Evaluated via `tokio::net::UnixStream` for sub-millisecond execution.
2. **`.socket2.sock` (Event Stream)**:
   - Stream of workspace and active window events continuously monitored by the Jarvis agent.

### 4.2 Dynamic Omarchy Tool Discovery & Execution
Jarvis executes Omarchy commands directly via non-blocking async processes, capturing output and returning structured results to the AI reasoning loop.
