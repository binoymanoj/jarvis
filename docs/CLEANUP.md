# Omarchy Jarvis: Complete System Audit & Uninstallation Guide

This document details **every single file, configuration change, binary, cache, and dependency** associated with **Jarvis** (100% Native Rust Architecture v0.2.0), followed by a step-by-step process and an automated script to completely uninstall Jarvis from your machine.

---

## 1. System Inventory: Everything Added or Modified

### A. System Packages (Pacman / AUR)
* **None (0 packages installed).**
* No `pacman -S` or `yay -S` commands were executed with root/sudo. No system-wide system libraries were polluted.

### B. Systemd Services
* **`~/.config/systemd/user/jarvis.service`**: The user systemd unit managing the background wake word listener daemon (`jarvis --daemon`).
  - Stopped via: `systemctl --user stop jarvis`
  - Disabled via: `systemctl --user disable jarvis`

### C. Executables & Compiled Binaries
1. **`~/.local/bin/jarvis`**: The compiled native Rust binary installed from `target/release/jarvis`.
   - 100% Rust executable (linked with `mimalloc`, `ort` ONNX Runtime, and `cpal`).

### D. Configuration & Plugin Files Modified
1. **`~/.config/hypr/bindings.lua`**:
   Hyprland hotkeys:
   ```lua
   -- Jarvis Voice Assistant (Push-to-Talk Hold / Toggle & Emergency Kill)
   o.bind("SUPER + SHIFT + RETURN", "Jarvis Assistant (Hold/Toggle)", "/home/binoy/.local/bin/jarvis -t")
   o.bind("SUPER + SHIFT + RETURN", "Jarvis Assistant (Release)", "/home/binoy/.local/bin/jarvis -s", { release = true })
   o.bind("SUPER + ALT + ESCAPE", "Kill Jarvis Assistant", "/home/binoy/.local/bin/jarvis -k")
   ```
2. **`~/.config/omarchy/plugins/top-bar/jarvis.qml`**:
   The native Omarchy top bar widget plugin providing the menu bar icon, active mic indicator dot, and popover control panel.
3. **`/run/user/1000/jarvis-status.json`**:
   Real-time status file shared between the Jarvis Rust daemon and the Omarchy menu bar plugin.
4. **`~/.config/jarvis/config.toml`**:
   Optional user configuration file overriding default settings.

### E. Project Directory & Rust Workspace
* **`/home/binoy/Codes/personal/jarvis`**:
  - `src/`: 100% Native Rust source code (audio capture, wake word, VAD, Gemini AI agent, 51 desktop tools, CLI, HUD IPC).
  - `models/`: Local ONNX models:
    - `hey_jarvis_v0.1.onnx` (~1.3 MB)
    - `melspectrogram.onnx` (~1.1 MB)
    - `embedding_model.onnx` (~1.3 MB)
    - `silero_vad.onnx` (~1.8 MB)
    - `en_GB-alan-medium.onnx` (~61 MB) + `.json`
  - `Cargo.toml` & `Cargo.lock`: Rust manifest and locked dependency graph.
  - `target/`: Rust build artifacts (`cargo clean` deletes this directory).
  - `.env`: Secret API keys (`GEMINI_API_KEY`, `GROQ_API_KEY`).

### F. Runtime Transient Files & Caches
* **`/tmp/jarvis-1000.pid`**: Process ID lockfile for instance management and kill switch.
* **`/run/user/1000/jarvis-status.json`**: Transient state indicator (`standby`, `listening`, `processing`, `speaking`).

---

## 2. Step-by-Step Manual Uninstallation Process

### Step 1: Kill any running Jarvis or Quickshell HUD instances
```bash
# Stop and disable systemd user daemon
systemctl --user stop jarvis 2>/dev/null || true
systemctl --user disable jarvis 2>/dev/null || true

# Terminate any running jarvis process
pkill -f "jarvis" 2>/dev/null || true

# Terminate any Quickshell HUD instances running the jarvis UI
pkill -f "quickshell.*jarvis" 2>/dev/null || true

# Remove runtime lock files
rm -f /tmp/jarvis-*.pid /run/user/1000/jarvis-*.json
```

### Step 2: Remove Hyprland Keybindings
Open `~/.config/hypr/bindings.lua` in your editor and remove the Jarvis binding block:
```lua
-- Remove these lines:
o.bind("SUPER + SHIFT + RETURN", "Jarvis Assistant (Hold/Toggle)", "/home/binoy/.local/bin/jarvis -t")
o.bind("SUPER + SHIFT + RETURN", "Jarvis Assistant (Release)", "/home/binoy/.local/bin/jarvis -s", { release = true })
o.bind("SUPER + ALT + ESCAPE", "Kill Jarvis Assistant", "/home/binoy/.local/bin/jarvis -k")
```
Reload Hyprland so the shortcut is freed immediately:
```bash
hyprctl reload
```

### Step 3: Remove the Native Executable and Systemd Service
```bash
rm -f ~/.local/bin/jarvis
rm -f ~/.config/systemd/user/jarvis.service
systemctl --user daemon-reload
```

### Step 4: Remove Menu Bar Plugin (Optional)
```bash
rm -rf ~/.config/omarchy/plugins/top-bar/jarvis.qml
```

### Step 5: Delete the Jarvis Project Repository & Build Target
```bash
rm -rf /home/binoy/Codes/personal/jarvis
```

---

## 3. Complete One-Shot Automated Removal Script

```bash
#!/usr/bin/env bash
set -e

echo "==> Stopping systemd daemon and active Jarvis sessions..."
systemctl --user stop jarvis 2>/dev/null || true
systemctl --user disable jarvis 2>/dev/null || true
pkill -f "jarvis" 2>/dev/null || true
pkill -f "quickshell.*jarvis" 2>/dev/null || true
rm -f /tmp/jarvis-*.pid /run/user/1000/jarvis-*.json

echo "==> Removing native compiled binary and service..."
rm -f "$HOME/.local/bin/jarvis"
rm -f "$HOME/.config/systemd/user/jarvis.service"
systemctl --user daemon-reload 2>/dev/null || true

echo "==> Removing Hyprland keybindings..."
if [ -f "$HOME/.config/hypr/bindings.lua" ]; then
    sed -i "/-- Jarvis Voice Assistant/,/Kill Jarvis Assistant/d" "$HOME/.config/hypr/bindings.lua"
    hyprctl reload 2>/dev/null || true
fi

echo "==> Deleting repository..."
rm -rf "$HOME/Codes/personal/jarvis"

echo "✔ Jarvis (Rust Native) has been completely removed from your system."
```

---

## 4. Verification Checklist

1. **Check process list**:
   ```bash
   pgrep -a -f jarvis
   ```
   *(Should return nothing)*

2. **Check command availability**:
   ```bash
   which jarvis
   ```
   *(Should return `jarvis not found`)*

3. **Check Hyprland bindings**:
   ```bash
   grep -i "jarvis" ~/.config/hypr/bindings.lua
   ```
   *(Should return nothing)*

4. **Check filesystem**:
   ```bash
   ls -d /home/binoy/Codes/personal/jarvis 2>/dev/null || echo "Directory cleanly deleted"
   ```
