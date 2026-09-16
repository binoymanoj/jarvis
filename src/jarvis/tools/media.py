"""MPRIS Media playback controller tool for Jarvis.

Enables hands-free media control (play/pause, next, previous, stop, now playing)
across all MPRIS-compliant media players (Spotify, Chromium/Chrome, Firefox, mpv, VLC, etc.)
using native DBus calls without extra package dependencies.
"""

import asyncio
import re
import shutil
from typing import List, Optional
from jarvis.core.logger import log


class MediaManager:
    """Controls media playback via standard MPRIS D-Bus interfaces."""

    def __init__(self):
        self._dbus_send_bin = shutil.which("dbus-send") or "/usr/bin/dbus-send"

    async def get_active_players(self) -> List[str]:
        """Discover all currently active MPRIS media players on the session bus."""
        try:
            proc = await asyncio.create_subprocess_exec(
                self._dbus_send_bin,
                "--session",
                "--dest=org.freedesktop.DBus",
                "--type=method_call",
                "--print-reply",
                "/org/freedesktop/DBus",
                "org.freedesktop.DBus.ListNames",
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.DEVNULL,
            )
            stdout, _ = await proc.communicate()
            text = stdout.decode("utf-8", errors="replace")

            # Extract names starting with org.mpris.MediaPlayer2.
            players = re.findall(r'string "(org\.mpris\.MediaPlayer2\.[^"]+)"', text)
            return players
        except Exception as e:
            log.error(f"Failed to query active MPRIS players: {e}")
            return []

    async def _call_player_method(self, method: str, target_player: Optional[str] = None) -> bool:
        """Invoke an MPRIS Player method (e.g., PlayPause, Next, Previous, Stop)."""
        players = [target_player] if target_player else await self.get_active_players()
        if not players:
            return False

        success = False
        for player in players:
            try:
                proc = await asyncio.create_subprocess_exec(
                    self._dbus_send_bin,
                    "--session",
                    f"--dest={player}",
                    "--type=method_call",
                    "/org/mpris/MediaPlayer2",
                    f"org.mpris.MediaPlayer2.Player.{method}",
                    stdout=asyncio.subprocess.DEVNULL,
                    stderr=asyncio.subprocess.DEVNULL,
                )
                await proc.communicate()
                if proc.returncode == 0:
                    success = True
                    # If no specific player was requested, controlling the first active player is usually sufficient
                    if not target_player:
                        break
            except Exception as e:
                log.warning(f"Error calling {method} on {player}: {e}")

        return success

    async def media_play_pause(self, player: Optional[str] = None) -> str:
        """Toggle media play / pause on active media player."""
        log.info(f"[cyan]Toggling media play/pause (player={player})[/cyan]")
        success = await self._call_player_method("PlayPause", player)
        if success:
            return "Toggled media playback."
        return "No active media player detected."

    async def media_next(self, player: Optional[str] = None) -> str:
        """Skip to next media track."""
        log.info(f"[cyan]Skipping to next track (player={player})[/cyan]")
        success = await self._call_player_method("Next", player)
        if success:
            return "Skipped to next track."
        return "No active media player detected."

    async def media_previous(self, player: Optional[str] = None) -> str:
        """Skip to previous media track."""
        log.info(f"[cyan]Skipping to previous track (player={player})[/cyan]")
        success = await self._call_player_method("Previous", player)
        if success:
            return "Returned to previous track."
        return "No active media player detected."

    async def media_stop(self, player: Optional[str] = None) -> str:
        """Stop media playback."""
        log.info(f"[cyan]Stopping media playback (player={player})[/cyan]")
        success = await self._call_player_method("Stop", player)
        if success:
            return "Stopped media playback."
        return "No active media player detected."

    async def get_now_playing(self) -> str:
        """Get the title and artist of the currently playing media."""
        players = await self.get_active_players()
        if not players:
            return "No active media player found."

        tracks = []
        for player in players:
            try:
                proc = await asyncio.create_subprocess_exec(
                    self._dbus_send_bin,
                    "--session",
                    f"--dest={player}",
                    "--type=method_call",
                    "--print-reply",
                    "/org/mpris/MediaPlayer2",
                    "org.freedesktop.DBus.Properties.Get",
                    "string:org.mpris.MediaPlayer2.Player",
                    "string:Metadata",
                    stdout=asyncio.subprocess.PIPE,
                    stderr=asyncio.subprocess.DEVNULL,
                )
                stdout, _ = await proc.communicate()
                text = stdout.decode("utf-8", errors="replace")

                title_match = re.search(r'string\s+"xesam:title"\s+variant\s+string\s+"(.*?)"', text)
                artist_match = re.search(
                    r'string\s+"xesam:artist"\s+variant\s+(?:array\s+\[\s+)?string\s+"(.*?)"',
                    text,
                )

                title = title_match.group(1).strip() if title_match else ""
                artist = artist_match.group(1).strip() if artist_match else ""

                short_player = player.split(".")[-1]
                if "chromium" in player.lower():
                    short_player = "Chromium/Browser"
                elif "firefox" in player.lower():
                    short_player = "Firefox"
                elif "spotify" in player.lower():
                    short_player = "Spotify"

                if title:
                    track_info = f"'{title}'"
                    if artist:
                        track_info += f" by {artist}"
                    track_info += f" ({short_player})"
                    tracks.append(track_info)
            except Exception as e:
                log.warning(f"Failed to read metadata for {player}: {e}")

        if tracks:
            return f"Now playing: {'; '.join(tracks)}."
        return "Media player is open, but no active track info is available."
