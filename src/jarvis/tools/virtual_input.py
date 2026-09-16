"""Virtual input simulation tool using Wayland's wtype utility.

Enables hands-free typing, key pressing, keyboard shortcuts, and document scrolling.
"""

import asyncio
import shutil
from typing import Optional
from jarvis.core.logger import log


KEY_MAP = {
    "enter": "Return",
    "return": "Return",
    "tab": "Tab",
    "esc": "Escape",
    "escape": "Escape",
    "backspace": "BackSpace",
    "space": "space",
    "spacebar": "space",
    "up": "Up",
    "down": "Down",
    "left": "Left",
    "right": "Right",
    "page_up": "Page_Up",
    "pageup": "Page_Up",
    "page_down": "Page_Down",
    "pagedown": "Page_Down",
    "home": "Home",
    "end": "End",
    "delete": "Delete",
}


class VirtualInputManager:
    """Manages virtual keyboard events and text injection on Wayland via wtype."""

    def __init__(self):
        self._wtype_bin = shutil.which("wtype") or "/usr/bin/wtype"

    async def type_text(self, text: str, enter_after: bool = False) -> str:
        """Type arbitrary text into the currently focused application window."""
        if not text:
            return "No text provided to type."

        log.info(f"[cyan]Virtual input typing: '{text}' (enter={enter_after})[/cyan]")
        try:
            # Pass text via stdin to handle special characters and punctuation safely
            proc = await asyncio.create_subprocess_exec(
                self._wtype_bin,
                "-",
                stdin=asyncio.subprocess.PIPE,
                stdout=asyncio.subprocess.DEVNULL,
                stderr=asyncio.subprocess.DEVNULL,
            )
            await proc.communicate(input=text.encode("utf-8"))

            if enter_after:
                await asyncio.sleep(0.05)
                await self.press_key("Return")

            return f"Typed: \"{text}\""
        except Exception as e:
            log.error(f"Failed to type text via wtype: {e}")
            return f"Error typing text: {e}"

    async def press_key(self, key_name: str) -> str:
        """Press a single keyboard key (e.g., Return, Tab, Escape, BackSpace, space, Page_Down, Up)."""
        clean_key = KEY_MAP.get(key_name.lower().strip(), key_name.strip())
        log.info(f"[cyan]Virtual input pressing key: '{clean_key}'[/cyan]")

        try:
            proc = await asyncio.create_subprocess_exec(
                self._wtype_bin,
                "-k",
                clean_key,
                stdout=asyncio.subprocess.DEVNULL,
                stderr=asyncio.subprocess.DEVNULL,
            )
            await proc.communicate()
            return f"Pressed key '{clean_key}'."
        except Exception as e:
            log.error(f"Failed to press key '{clean_key}': {e}")
            return f"Error pressing key: {e}"

    async def send_shortcut(self, modifiers: str, key: str) -> str:
        """Send a keyboard shortcut combination (e.g. modifiers='ctrl', key='s' or modifiers='ctrl+shift', key='t')."""
        clean_key = KEY_MAP.get(key.lower().strip(), key.strip())
        mod_list = [m.lower().strip() for m in modifiers.replace("+", " ").split() if m.strip()]

        args = [self._wtype_bin]
        for mod in mod_list:
            args.extend(["-M", mod])

        args.extend(["-k", clean_key])

        for mod in reversed(mod_list):
            args.extend(["-m", mod])

        log.info(f"[cyan]Virtual input sending shortcut: {'+'.join(mod_list)}+{clean_key}[/cyan]")
        try:
            proc = await asyncio.create_subprocess_exec(
                *args,
                stdout=asyncio.subprocess.DEVNULL,
                stderr=asyncio.subprocess.DEVNULL,
            )
            await proc.communicate()
            return f"Sent shortcut {'+'.join(mod_list)}+{clean_key}."
        except Exception as e:
            log.error(f"Failed to send shortcut: {e}")
            return f"Error sending shortcut: {e}"

    async def scroll(self, direction: str = "down", amount: int = 2) -> str:
        """Scroll the active window up or down hands-free."""
        clean_dir = direction.lower().strip()
        key = "Page_Down" if clean_dir in ("down", "d", "next") else "Page_Up"
        repeats = max(1, min(amount, 10))

        log.info(f"[cyan]Virtual input scrolling {clean_dir} ({repeats}x)[/cyan]")
        for i in range(repeats):
            await self.press_key(key)
            if i < repeats - 1:
                await asyncio.sleep(0.08)

        return f"Scrolled {clean_dir}."
