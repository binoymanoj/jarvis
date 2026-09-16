import asyncio
from pathlib import Path
import shutil
from typing import Optional
from jarvis.core.logger import log

UI_DIR = Path(__file__).resolve().parent


class JarvisHUD:
    """Controls the Quickshell floating Siri-like HUD via high-speed IPC."""

    def __init__(self, ui_path: Optional[Path] = None):
        self.ui_path = ui_path or UI_DIR
        self.qs_bin = shutil.which("qs") or "/usr/bin/qs"
        self._is_started = False

    async def ensure_running(self) -> bool:
        """Starts the Quickshell HUD daemon in the background if not already active."""
        if self._is_started:
            return True

        # Try pinging existing instance
        try:
            proc = await asyncio.create_subprocess_exec(
                self.qs_bin, "ipc", "-p", str(self.ui_path), "call", "jarvis", "ping",
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            stdout, _ = await proc.communicate()
            if "pong" in stdout.decode():
                self._is_started = True
                return True
        except Exception:
            pass

        # Launch detached instance
        try:
            launch_proc = await asyncio.create_subprocess_exec(
                self.qs_bin, "-d", "-n", "-p", str(self.ui_path),
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            await launch_proc.communicate()
            # Brief yield to let Wayland layer-shell map
            await asyncio.sleep(0.3)
            self._is_started = True
            return True
        except Exception as e:
            log.warning(f"Failed to launch Quickshell HUD: {e}")
            return False

    async def _call(self, method: str, *args: str) -> str:
        """Invoke an IPC method on the Quickshell Jarvis HUD."""
        await self.ensure_running()
        cmd = [self.qs_bin, "ipc", "-p", str(self.ui_path), "call", "jarvis", method] + list(args)
        try:
            proc = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            stdout, _ = await proc.communicate()
            return stdout.decode().strip()
        except Exception:
            self._is_started = False
            return ""

    async def sync_theme(self) -> None:
        """Push the currently active Omarchy theme colors into the HUD over IPC."""
        try:
            from jarvis.core.theme import load_omarchy_theme
            theme = load_omarchy_theme()
            await self._call(
                "setTheme",
                theme.get("accent", "#00f0ff"),
                theme.get("foreground", "#cacccc"),
                theme.get("background", "#101315"),
                theme.get("dark_background", "#080a0b"),
                theme.get("cyan", "#00f0ff"),
                theme.get("magenta", "#d946ef"),
                theme.get("blue", "#38bdf8"),
                theme.get("name", "Omarchy"),
            )
        except Exception as e:
            log.debug(f"Theme sync IPC warning: {e}")

    async def show_listening(self, caption: str = "Listening...") -> None:
        """Display the HUD in reactive listening state with dynamic theme colors."""
        await self.sync_theme()
        await self._call("setListening", caption)

    async def update_volume(self, volume: float) -> None:
        """Update real-time audio volume level (0.0 to 1.0)."""
        # Call IPC without blocking
        await self._call("setVolume", f"{volume:.2f}")

    async def show_thinking(self, caption: str = "Thinking...") -> None:
        """Display the HUD in swirling gradient thinking state."""
        await self._call("setThinking", caption)

    async def show_speaking(self, caption: str = "Speaking...") -> None:
        """Display the HUD in speaking state."""
        await self._call("setSpeaking", caption)

    async def hide(self) -> None:
        """Smoothly hide the HUD."""
        await self._call("hide")
