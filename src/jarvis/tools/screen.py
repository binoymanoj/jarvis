import asyncio
from datetime import datetime
from pathlib import Path
import shutil
from typing import Optional
from jarvis.core.logger import log
from jarvis.tools.hyprland import HyprlandController


class ScreenPerception:
    """Wayland screen capture and visual perception tool via grim."""

    def __init__(self, storage_dir: Optional[Path] = None):
        self.grim_bin = shutil.which("grim") or "/usr/bin/grim"
        self.storage_dir = storage_dir or Path("/tmp/jarvis_captures")
        self.storage_dir.mkdir(parents=True, exist_ok=True)
        self.hyprland = HyprlandController()

    def _generate_capture_path(self, prefix: str = "screen") -> Path:
        """Generates a timestamped file path for a capture."""
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S_%f")
        return self.storage_dir / f"{prefix}_{timestamp}.png"

    async def capture_full_screen(self, output_path: Optional[Path] = None) -> Path:
        """Captures full display output."""
        target = output_path or self._generate_capture_path("fullscreen")
        proc = await asyncio.create_subprocess_exec(
            self.grim_bin, "-t", "png", str(target),
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        _, stderr = await proc.communicate()
        if proc.returncode != 0:
            raise RuntimeError(f"grim full screenshot failed: {stderr.decode()}")
        return target

    async def capture_active_window(self, output_path: Optional[Path] = None) -> Path:
        """Captures cropped area corresponding precisely to the currently focused window."""
        win = await self.hyprland.get_active_window()
        at = win.get("at", [0, 0])
        size = win.get("size", [0, 0])

        if not size or size[0] <= 0 or size[1] <= 0:
            log.warning("Active window has invalid geometry, falling back to fullscreen capture")
            return await self.capture_full_screen(output_path)

        x, y = at[0], at[1]
        w, h = size[0], size[1]
        geometry = f"{x},{y} {w}x{h}"

        target = output_path or self._generate_capture_path("activewindow")
        proc = await asyncio.create_subprocess_exec(
            self.grim_bin, "-g", geometry, "-t", "png", str(target),
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        _, stderr = await proc.communicate()
        if proc.returncode != 0:
            log.warning(f"grim geometry capture failed: {stderr.decode()}, trying fullscreen fallback")
            return await self.capture_full_screen(output_path)
        return target
