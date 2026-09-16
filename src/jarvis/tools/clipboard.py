"""Wayland clipboard management tool for Jarvis.

Enables hands-free clipboard reading and writing using wl-clipboard (wl-paste, wl-copy).
"""

import asyncio
import shutil
from jarvis.core.logger import log


class ClipboardManager:
    """Manages Wayland system clipboard interactions."""

    def __init__(self):
        self._wl_paste_bin = shutil.which("wl-paste") or "/usr/bin/wl-paste"
        self._wl_copy_bin = shutil.which("wl-copy") or "/usr/bin/wl-copy"

    async def get_clipboard(self) -> str:
        """Get the current textual content of the Wayland clipboard."""
        try:
            proc = await asyncio.create_subprocess_exec(
                self._wl_paste_bin,
                "-n",
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            stdout, stderr = await proc.communicate()
            if proc.returncode != 0:
                err = stderr.decode("utf-8", errors="replace").strip()
                log.warning(f"wl-paste returned non-zero code {proc.returncode}: {err}")
                return "Clipboard is empty or contains non-text data."

            text = stdout.decode("utf-8", errors="replace").strip()
            if not text:
                return "Clipboard is currently empty."

            # Truncate if excessively long
            if len(text) > 1500:
                text = text[:1500] + "... [truncated]"

            log.info(f"[cyan]Read clipboard content ({len(text)} chars)[/cyan]")
            return f"Clipboard content: \"{text}\""
        except Exception as e:
            log.error(f"Failed to read clipboard: {e}")
            return f"Error reading clipboard: {e}"

    async def set_clipboard(self, text: str) -> str:
        """Copy specified text into the Wayland clipboard."""
        if not text:
            return "No text provided to copy to clipboard."

        try:
            proc = await asyncio.create_subprocess_exec(
                self._wl_copy_bin,
                stdin=asyncio.subprocess.PIPE,
                stdout=asyncio.subprocess.DEVNULL,
                stderr=asyncio.subprocess.DEVNULL,
            )
            await proc.communicate(input=text.encode("utf-8"))
            log.info(f"[cyan]Copied text to clipboard ({len(text)} chars)[/cyan]")
            return f"Copied to clipboard: \"{text[:60]}{'...' if len(text) > 60 else ''}\""
        except Exception as e:
            log.error(f"Failed to write to clipboard: {e}")
            return f"Error writing to clipboard: {e}"
