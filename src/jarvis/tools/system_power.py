"""System power, hardware, and diagnostic management tool for Jarvis.

Enables hands-free control of lock screen, reboot, shutdown, bluetooth,
system hardware stats, and network speed testing via native Omarchy utilities.
"""

import asyncio
from typing import Optional
from jarvis.core.logger import log
from jarvis.tools.omarchy import OmarchyBridge


class SystemPowerManager:
    """Manages system session state, power controls, and hardware diagnostics."""

    def __init__(self, omarchy: Optional[OmarchyBridge] = None):
        self.omarchy = omarchy or OmarchyBridge()

    async def lock_screen(self) -> str:
        """Lock the computer screen and put display to sleep."""
        log.info("[cyan]Locking system screen...[/cyan]")
        out = await self.omarchy.run("system lock")
        return "Screen locked." if not out else f"Screen lock status: {out}"

    async def logout_system(self) -> str:
        """Log out of the current desktop user session."""
        log.info("[cyan]Logging out of user session...[/cyan]")
        out = await self.omarchy.run("system logout")
        return "Logging out..." if not out else f"Logout status: {out}"

    async def reboot_system(self) -> str:
        """Reboot the computer."""
        log.info("[cyan]Rebooting system...[/cyan]")
        out = await self.omarchy.run("system reboot")
        return "Rebooting system..." if not out else f"Reboot status: {out}"

    async def shutdown_system(self) -> str:
        """Shut down the computer."""
        log.info("[cyan]Shutting down system...[/cyan]")
        out = await self.omarchy.run("system shutdown")
        return "Shutting down system..." if not out else f"Shutdown status: {out}"

    async def get_system_stats(self) -> str:
        """Get live CPU, memory, and Wi-Fi load diagnostics."""
        log.info("[cyan]Fetching system hardware stats...[/cyan]")
        out = await self.omarchy.run("system stats")
        return out if out else "System stats currently unavailable."

    async def toggle_bluetooth(self, action: str = "toggle") -> str:
        """Control Bluetooth power state. Options: 'on', 'off', 'toggle', 'is-on'."""
        clean_action = action.lower().strip()
        if clean_action not in ("on", "off", "toggle", "is-on"):
            clean_action = "toggle"

        log.info(f"[cyan]Setting Bluetooth power state: {clean_action}[/cyan]")
        out = await self.omarchy.run("bluetooth power", clean_action)
        if clean_action == "is-on":
            return "Bluetooth is on." if not out else f"Bluetooth status: {out}"
        return f"Bluetooth power set to {clean_action}."

    async def network_speedtest(self, direction: str = "down") -> str:
        """Measure live internet connection speed. Direction: 'down' (download) or 'up' (upload)."""
        clean_dir = "up" if "up" in direction.lower() else "down"
        log.info(f"[cyan]Running network speedtest ({clean_dir})...[/cyan]")
        out = await self.omarchy.run("network speedtest", clean_dir)
        return f"Network speedtest ({clean_dir}): {out}" if out else "Speedtest completed."

    async def get_network_status(self) -> str:
        """Get active Wi-Fi connection and network details."""
        log.info("[cyan]Checking network status...[/cyan]")
        out = await self.omarchy.run("network status")
        return f"Network status: {out}" if out else "Network status unavailable."
