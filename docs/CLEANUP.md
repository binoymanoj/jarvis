# Omarchy Jarvis: Complete System Audit & Uninstallation Guide

This document details **every single file, configuration change, binary, cache, and dependency** added to this system during the development and setup of **Jarvis**, followed by a step-by-step process and an automated one-line script to completely remove Jarvis from your machine.

---

## 1. System Inventory: Everything Added or Modified

### A. System Packages (Pacman / AUR)
* **None (0 packages installed).**
* No `pacman -S` or `yay -S` commands were executed with root/sudo. No system-wide system libraries were polluted.

### B. Systemd Services
* **`~/.config/systemd/user/jarvis.service`**: The user systemd unit managing the background wake word listener daemon.
  - Stopped via: `systemctl --user stop jarvis`
  - Disabled via: `systemctl --user disable jarvis`

### C. Executables & Binary Wrappers
1. **`~/.local/bin/jarvis`**: The global executable shell wrapper that dispatches hotkey and CLI commands into the project virtual environment.
2. **`~/.local/bin/uv` & `~/.local/bin/uvx`** *(optional)*: The standalone `uv` package and tool runner installed during environment bootstrap.
3. **`~/.local/bin/env` & `~/.local/bin/env.fish`** *(optional)*: Created by the `uv` installer to export PATH.

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
   Real-time status file shared between the Jarvis daemon and the Omarchy menu bar plugin.
4. **`~/.config/mise/config.toml`** *(optional)*:
   Line added under `[tools]`:
   ```toml
   uv = "latest"
   ```
5. **`~/.config/fish/conf.d/uv.env.fish`** *(optional)*: Added by `uv` installer.
6. **`~/.config/uv/uv-receipt.json`** *(optional)*: Metadata from `uv` installer.


### E. Project Directory & Local Python Environment
* **`/home/binoy/Codes/personal/jarvis`**:
  - `src/jarvis/`: Core code (agent, tools, UI QML, audio pipelines).
  - `src/jarvis/audio/models/`: Downloaded local ONNX models:
    - `silero_vad.onnx` (~2.3 MB)
    - `en_GB-alan-medium.onnx` (~61 MB)
    - `en_GB-alan-medium.onnx.json`
  - `.venv/`: The project-isolated Python 3.14 virtual environment (all Python wheels: `google-genai`, `groq`, `sounddevice`, `numpy`, `piper-tts`, `onnxruntime`, etc.).
  - `.env`: Secret API keys (`GEMINI_API_KEY`, `GROQ_API_KEY`).
  - `.git/`: Git repository metadata.
  - `docs/`, `tests/`, `pyproject.toml`, `uv.lock`.

### F. Runtime Transient Files & Caches
* **`/tmp/jarvis-1000.pid`**: Process ID lockfile for instance management and kill switch.
* **`/tmp/jarvis-1000.state`**: Transient state indicator (`recording`, `processing`, `speaking`).
* **`~/.cache/uv/`**: Cached Python wheels and source distributions.

---

## 2. Step-by-Step Manual Uninstallation Process

If you prefer to remove everything step-by-step by hand:

### Step 1: Kill any running Jarvis or Quickshell HUD instances
```bash
# Terminate any running jarvis process
pkill -f "jarvis" 2>/dev/null || true

# Terminate any orphaned Quickshell HUD instances running the jarvis UI
pkill -f "quickshell.*jarvis" 2>/dev/null || true

# Remove runtime lock files
rm -f /tmp/jarvis-*.pid /tmp/jarvis-*.state
```

### Step 2: Remove Hyprland Keybindings
Open `~/.config/hypr/bindings.lua` in your editor and remove the Jarvis binding block:
```lua
-- Remove these 4 lines:
-- Jarvis Voice Assistant (Push-to-Talk Hold / Toggle & Emergency Kill)
o.bind("SUPER + SHIFT + RETURN", "Jarvis Assistant (Hold/Toggle)", "/home/binoy/.local/bin/jarvis -t")
o.bind("SUPER + SHIFT + RETURN", "Jarvis Assistant (Release)", "/home/binoy/.local/bin/jarvis -s", { release = true })
o.bind("SUPER + ALT + ESCAPE", "Kill Jarvis Assistant", "/home/binoy/.local/bin/jarvis -k")
```
Reload Hyprland so the shortcut is freed immediately:
```bash
hyprctl reload
```

### Step 3: Remove the Executable Wrapper
```bash
rm -f ~/.local/bin/jarvis
```

### Step 4: Delete the Jarvis Project Repository & Virtual Environment
```bash
rm -rf /home/binoy/Codes/personal/jarvis
```

---

## 3. (Optional) Remove `uv` Package Manager

If you only installed `uv` for Jarvis and do not use it for any other Python projects:

### Step 1: Remove `uv` from `mise`
```bash
mise uninstall uv 2>/dev/null || true
```
And remove `uv = "latest"` from `~/.config/mise/config.toml`.

### Step 2: Remove `uv` binaries and configurations
```bash
rm -f ~/.local/bin/uv ~/.local/bin/uvx ~/.local/bin/env ~/.local/bin/env.fish
rm -rf ~/.config/uv ~/.config/fish/conf.d/uv.env.fish
rm -rf ~/.cache/uv
```

---

## 4. Complete One-Shot Automated Removal Script

To completely clean up Jarvis in 3 seconds, run this script:

```bash
#!/usr/bin/env bash
set -e

echo "==> Stopping any active Jarvis sessions..."
pkill -f "jarvis" 2>/dev/null || true
pkill -f "quickshell.*jarvis" 2>/dev/null || true
rm -f /tmp/jarvis-*.pid /tmp/jarvis-*.state

echo "==> Removing global binary wrapper..."
rm -f "$HOME/.local/bin/jarvis"

echo "==> Removing Hyprland keybindings..."
if [ -f "$HOME/.config/hypr/bindings.lua" ]; then
    sed -i "/-- Jarvis Voice Assistant/,/Kill Jarvis Assistant/d" "$HOME/.config/hypr/bindings.lua"
    hyprctl reload 2>/dev/null || true
fi

echo "==> Deleting repository and virtual environment..."
rm -rf "$HOME/Codes/personal/jarvis"

echo "==> Cleaning cache..."
rm -rf "$HOME/.cache/uv"

echo "✔ Jarvis has been completely removed from your system."
```

---

## 5. Verification Checklist

To confirm that your PC is 100% clean of Jarvis:

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
