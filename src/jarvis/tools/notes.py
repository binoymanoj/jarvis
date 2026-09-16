"""Note-taking and scratchpad integration for Jarvis."""

import asyncio
import datetime
from pathlib import Path
import re
import shutil
from typing import Optional
from jarvis.core.logger import log

NOTES_DIR = Path.home() / "Notes"


class NoteManager:
    """Manages markdown notes and integration with Obsidian or the default editor."""

    def __init__(self, notes_dir: Optional[Path] = None):
        self.notes_dir = notes_dir or NOTES_DIR
        self.notes_dir.mkdir(parents=True, exist_ok=True)
        self._obsidian_bin = shutil.which("obsidian")
        self._editor_bin = shutil.which("omarchy-launch-editor") or shutil.which("nvim")

    async def create_note(
        self,
        title: str,
        content: str,
        open_in_editor: bool = False,
    ) -> str:
        """Create a markdown note with title, timestamp, and content."""
        clean_title = re.sub(r"[^\w\s-]", "", title).strip() or "Untitled_Note"
        filename = f"{clean_title.replace(' ', '_')}.md"
        file_path = self.notes_dir / filename

        timestamp = datetime.datetime.now().strftime("%A, %B %d, %Y at %I:%M %p")
        note_body = f"# {title}\n\n*Created on {timestamp}*\n\n{content}\n"

        if file_path.exists():
            # Append if note with this title exists
            existing = file_path.read_text()
            file_path.write_text(f"{existing}\n---\n*Updated on {timestamp}*\n\n{content}\n")
            action = "Appended to"
        else:
            file_path.write_text(note_body)
            action = "Created"

        log.info(f"[cyan]{action} note: {file_path}[/cyan]")

        if open_in_editor:
            target_bin = self._obsidian_bin or self._editor_bin
            if target_bin:
                try:
                    await asyncio.create_subprocess_exec(
                        target_bin,
                        str(file_path),
                        stdout=asyncio.subprocess.DEVNULL,
                        stderr=asyncio.subprocess.DEVNULL,
                    )
                except Exception as e:
                    log.warning(f"Could not open note in editor: {e}")

        return f"{action} note '{title}' in {file_path.name}, sir."

    def list_recent_notes(self, limit: int = 5) -> str:
        """List recently modified notes in the Notes folder."""
        if not self.notes_dir.exists():
            return "No notes directory found."

        notes = sorted(self.notes_dir.glob("*.md"), key=lambda f: f.stat().st_mtime, reverse=True)
        if not notes:
            return "You have no saved notes yet, sir."

        recent = notes[:limit]
        lines = ["Recent notes:"]
        for n in recent:
            mod_time = datetime.datetime.fromtimestamp(n.stat().st_mtime).strftime("%b %d, %H:%M")
            lines.append(f"- {n.stem} (modified {mod_time})")
        return "\n".join(lines)
