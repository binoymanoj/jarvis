import asyncio
import json
import shutil
from typing import Any, Dict, List, Optional
from jarvis.core.logger import log


class OmarchyBridge:
    """Async bridge to Omarchy Linux CLI tools and system notifications."""

    def __init__(self):
        self.omarchy_bin = shutil.which("omarchy") or "/usr/bin/omarchy"
        self.notify_bin = shutil.which("omarchy-notification-send") or "/usr/share/omarchy/bin/omarchy-notification-send"

    async def run(self, subcommand: str, *args: str) -> str:
        """Execute an arbitrary Omarchy CLI command."""
        cmd = [self.omarchy_bin] + subcommand.split() + list(args)
        proc = await asyncio.create_subprocess_exec(
            *cmd,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        stdout, stderr = await proc.communicate()
        out = stdout.decode("utf-8", errors="replace").strip()
        err = stderr.decode("utf-8", errors="replace").strip()
        if proc.returncode != 0 and err:
            log.warning(f"Omarchy command '{' '.join(cmd)}' failed: {err}")
            return err
        return out

    async def notify(
        self,
        headline: str,
        description: str = "",
        glyph: str = "󰚩",
        urgency: str = "normal",
        exec_cmd: Optional[List[str]] = None,
    ) -> bool:
        """Send a native Omarchy desktop notification with custom glyph and optional click action."""
        cmd = [self.notify_bin, "-g", glyph, "-u", urgency, headline]
        if description:
            cmd.append(description)
        if exec_cmd:
            cmd.extend(["--exec"] + exec_cmd)

        proc = await asyncio.create_subprocess_exec(
            *cmd,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        await proc.communicate()
        return proc.returncode == 0

    # Audio Controls
    async def set_volume(self, adjustment: str) -> str:
        """Adjust volume: 'raise', 'lower', 'mute-toggle', '+5', '-10'."""
        return await self.run("audio output volume", adjustment)

    # Brightness Controls
    async def get_brightness(self) -> str:
        """Get current screen brightness."""
        return await self.run("brightness display")

    async def set_brightness(self, level_or_delta: str) -> str:
        """Set screen brightness (e.g., '50', '+10', '-10')."""
        return await self.run("brightness display", str(level_or_delta))

    # Theme Controls
    async def get_theme(self) -> str:
        """Get currently active Omarchy theme."""
        return await self.run("theme current")

    async def set_theme(self, theme_name: str) -> str:
        """Apply an Omarchy theme by name."""
        return await self.run("theme set", theme_name)

    async def list_themes(self) -> List[str]:
        """List all available Omarchy themes."""
        output = await self.run("theme list")
        return [line.strip() for line in output.splitlines() if line.strip()]

    # Battery & Hardware
    async def get_battery_status(self) -> str:
        """Get battery state, percentage, and estimated time remaining."""
        return await self.run("battery status")

    # Application Launching
    async def launch_app(self, app_name: str, *args: str) -> str:
        """Launch an application through Omarchy launcher."""
        return await self.run(f"launch {app_name}", *args)
