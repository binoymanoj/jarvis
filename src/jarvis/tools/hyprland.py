import asyncio
import json
import os
from pathlib import Path
from typing import Any, AsyncGenerator, Dict, List, Optional
from jarvis.core.logger import log


class HyprlandController:
    """Direct UNIX domain socket controller for Hyprland compositor."""

    def __init__(self, instance_sig: Optional[str] = None):
        self.sig = instance_sig or os.environ.get("HYPRLAND_INSTANCE_SIGNATURE")
        runtime_dir = os.environ.get("XDG_RUNTIME_DIR", f"/run/user/{os.getuid()}")
        
        if self.sig:
            self.cmd_socket_path = Path(runtime_dir) / "hypr" / self.sig / ".socket.sock"
            self.event_socket_path = Path(runtime_dir) / "hypr" / self.sig / ".socket2.sock"
        else:
            self.cmd_socket_path = None
            self.event_socket_path = None

    async def _send_socket_request(self, payload: bytes) -> str:
        """Sends raw bytes to Hyprland's .socket.sock and reads response."""
        if not self.cmd_socket_path or not self.cmd_socket_path.exists():
            raise FileNotFoundError("Hyprland command socket not found. Is Hyprland running?")

        reader, writer = await asyncio.open_unix_connection(str(self.cmd_socket_path))
        try:
            writer.write(payload)
            await writer.drain()
            response = bytearray()
            while True:
                chunk = await reader.read(4096)
                if not chunk:
                    break
                response.extend(chunk)
            return response.decode("utf-8", errors="replace").strip()
        finally:
            writer.close()
            await writer.wait_closed()

    async def get_json(self, command: str) -> Any:
        """Fetch structured JSON data directly via socket using 'j/<command>'."""
        payload = f"j/{command}".encode("utf-8")
        raw = await self._send_socket_request(payload)
        if not raw:
            return {}
        try:
            return json.loads(raw)
        except json.JSONDecodeError:
            log.warning(f"Failed to parse JSON for command 'j/{command}': {raw[:100]}")
            return {}

    async def get_active_window(self) -> Dict[str, Any]:
        """Fetch current focused window information."""
        return await self.get_json("activewindow")

    async def get_workspaces(self) -> List[Dict[str, Any]]:
        """List all active workspaces."""
        return await self.get_json("workspaces")

    async def get_clients(self) -> List[Dict[str, Any]]:
        """List all open application windows."""
        return await self.get_json("clients")

    async def get_monitors(self) -> List[Dict[str, Any]]:
        """List all connected displays/monitors."""
        return await self.get_json("monitors")

    async def dispatch(self, lua_or_raw_cmd: str) -> str:
        """Send a dispatch action to Hyprland."""
        payload = f"dispatch {lua_or_raw_cmd}".encode("utf-8")
        try:
            res = await self._send_socket_request(payload)
            if "error:" not in res.lower() and "unknown request" not in res.lower():
                return res
        except Exception as e:
            log.warning(f"Socket dispatch failed: {e}, attempting hyprctl fallback")

        # Fallback to hyprctl if Lua socket dispatch fails
        proc = await asyncio.create_subprocess_exec(
            "hyprctl", "dispatch", lua_or_raw_cmd,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        stdout, stderr = await proc.communicate()
        return stdout.decode().strip() or stderr.decode().strip()

    async def change_workspace(self, workspace_id: int) -> str:
        """Switch active workspace."""
        return await self.dispatch(f'hl.dsp.focus({{ workspace = "{workspace_id}" }})')

    async def focus_window(self, address: str) -> str:
        """Focus a specific window by memory address."""
        clean_addr = address if address.startswith("address:") else f"address:{address}"
        return await self.dispatch(f'hl.dsp.focus({{ window = "{clean_addr}" }})')

    async def focus_app(self, app_name: str) -> bool:
        """Find and focus a window matching the app name/class."""
        clients = await self.get_clients()
        query = app_name.lower()
        for client in clients:
            c_class = str(client.get("class", "")).lower()
            c_title = str(client.get("title", "")).lower()
            c_initial = str(client.get("initialClass", "")).lower()
            if query in c_class or query in c_title or query in c_initial:
                addr = client.get("address")
                if addr:
                    await self.focus_window(addr)
                    return True
        return False

    async def close_active_window(self) -> str:
        """Close the currently focused window."""
        return await self.dispatch("hl.dsp.window.close()")

    async def toggle_split(self) -> str:
        """Toggle split orientation in dwindle layout."""
        return await self.dispatch('hl.dsp.layout("togglesplit")')

    async def toggle_fullscreen(self) -> str:
        """Toggle fullscreen state of active window."""
        return await self.dispatch("hl.dsp.window.fullscreen()")

    async def move_window_to_workspace(self, workspace_id: int, silent: bool = False) -> str:
        """Move active window to target workspace."""
        follow = "false" if silent else "true"
        return await self.dispatch(f'hl.dsp.window.move({{ workspace = "{workspace_id}", follow = {follow} }})')


class HyprlandEventListener:
    """Continuous async stream listener on Hyprland's .socket2.sock."""

    def __init__(self, instance_sig: Optional[str] = None):
        self.sig = instance_sig or os.environ.get("HYPRLAND_INSTANCE_SIGNATURE")
        runtime_dir = os.environ.get("XDG_RUNTIME_DIR", f"/run/user/{os.getuid()}")
        self.event_socket_path = Path(runtime_dir) / "hypr" / self.sig / ".socket2.sock" if self.sig else None

    async def listen(self) -> AsyncGenerator[Dict[str, str], None]:
        """Yields parsed Hyprland events: {'event': name, 'data': raw_str}."""
        if not self.event_socket_path or not self.event_socket_path.exists():
            raise FileNotFoundError("Hyprland event socket not found.")

        reader, writer = await asyncio.open_unix_connection(str(self.event_socket_path))
        try:
            while True:
                line = await reader.readline()
                if not line:
                    break
                decoded = line.decode("utf-8", errors="replace").strip()
                if ">>" in decoded:
                    event_name, data = decoded.split(">>", 1)
                    yield {"event": event_name.strip(), "data": data.strip()}
        finally:
            writer.close()
            await writer.wait_closed()
