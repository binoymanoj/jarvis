"""Multi-workspace workflow orchestration and custom workflow manager for Jarvis."""

import asyncio
import json
from pathlib import Path
import re
import shutil
from typing import Any, Dict, List, Optional
from jarvis.core.logger import log
from jarvis.tools.hyprland import HyprlandController

CONFIG_DIR = Path.home() / ".config" / "jarvis"
WORKFLOWS_FILE = CONFIG_DIR / "workflows.json"

DEFAULT_WORKFLOWS: Dict[str, Dict[str, Any]] = {
    "coding": {
        "description": "Development environment: Editor and terminal on workspace 1, browser on workspace 2",
        "aliases": ["dev", "code", "development"],
        "primary_workspace": 1,
        "steps": [
            {"workspace": 1, "launch": "omarchy-launch-editor", "delay": 0.25},
            {"workspace": 1, "launch": "omarchy-launch-terminal", "delay": 0.25},
            {"workspace": 2, "launch": "omarchy-launch-browser", "delay": 0.25},
        ],
    },
    "research": {
        "description": "Research environment: Browser on workspace 1, Obsidian notes on workspace 2",
        "aliases": ["study", "reading"],
        "primary_workspace": 1,
        "steps": [
            {"workspace": 1, "launch": "omarchy-launch-browser", "delay": 0.25},
            {"workspace": 2, "launch": "obsidian", "delay": 0.25},
        ],
    },
    "writing": {
        "description": "Writing environment: Obsidian notes on workspace 1, browser on workspace 2",
        "aliases": ["notes", "write", "drafting"],
        "primary_workspace": 1,
        "steps": [
            {"workspace": 1, "launch": "obsidian", "delay": 0.25},
            {"workspace": 2, "launch": "omarchy-launch-browser", "delay": 0.25},
        ],
    },
    "communication": {
        "description": "Communication setup: Thunderbird email on workspace 1, web messaging on workspace 2",
        "aliases": ["social", "mail", "messaging"],
        "primary_workspace": 1,
        "steps": [
            {"workspace": 1, "launch": "thunderbird", "delay": 0.3},
            {"workspace": 2, "launch": "omarchy-launch-webapp https://web.whatsapp.com", "delay": 0.25},
        ],
    },
    "media": {
        "description": "Entertainment setup: YouTube browser on workspace 1, Spotify/cliamp on workspace 2",
        "aliases": ["chill", "music", "relax"],
        "primary_workspace": 1,
        "steps": [
            {"workspace": 1, "launch": "omarchy-launch-browser https://www.youtube.com", "delay": 0.25},
            {"workspace": 2, "launch": "cliamp", "delay": 0.25},
        ],
    },
}


def _clean_slug(name: str) -> str:
    """Normalize workflow name into a clean, safe identifier."""
    slug = re.sub(r"[^a-zA-Z0-9_\-]", "_", name.strip().lower())
    return re.sub(r"_+", "_", slug).strip("_")


def _map_client_to_launch_cmd(client: Dict[str, Any]) -> Optional[str]:
    """Maps a Hyprland window client object to an executable launch command."""
    cls = (client.get("initialClass") or client.get("class") or "").strip()
    title = (client.get("title") or "").strip()
    cls_lower = cls.lower()

    # 1. Web Browsers
    if any(b in cls_lower for b in ("helium", "chromium", "google-chrome", "firefox", "brave")):
        return "omarchy-launch-browser"

    # 2. Terminals
    if any(t in cls_lower for t in ("foot", "kitty", "alacritty", "wezterm", "gnome-terminal")):
        return "omarchy-launch-terminal"

    # 3. Editors & IDEs
    if "code" in cls_lower or "vscodium" in cls_lower:
        return "code"
    if "nvim" in cls_lower or "neovim" in cls_lower:
        return "omarchy-launch-editor"

    # 4. File Managers
    if "nautilus" in cls_lower or "org.gnome.nautilus" in cls_lower:
        return "nautilus"
    if "thunar" in cls_lower or "dolphin" in cls_lower:
        return cls_lower

    # 5. Media Players
    if "mpv" in cls_lower:
        return "mpv"
    if "spotify" in cls_lower:
        return "spotify"
    if "vlc" in cls_lower:
        return "vlc"

    # 6. Notes & Knowledge Base
    if "obsidian" in cls_lower:
        return "obsidian"

    # 7. Communications
    if "thunderbird" in cls_lower:
        return "thunderbird"
    if "slack" in cls_lower:
        return "slack"
    if "discord" in cls_lower:
        return "discord"
    if "telegram" in cls_lower:
        return "telegram-desktop"

    # 8. Webapps (e.g. chrome-www.icloud.com...)
    if cls_lower.startswith("chrome-") and "__" in cls_lower:
        try:
            domain = cls.split("-")[1].split("__")[0]
            return f"omarchy-launch-webapp https://{domain}"
        except Exception:
            return "omarchy-launch-browser"

    # 9. Generic binary resolution
    if cls:
        short_bin = cls.split(".")[-1].lower()
        if shutil.which(short_bin):
            return short_bin
        if shutil.which(cls_lower):
            return cls_lower
        return short_bin

    return None


class WorkflowManager:
    """Manages, captures, and executes desktop workflow presets across Hyprland workspaces."""

    def __init__(self, hyprland: Optional[HyprlandController] = None):
        self.hyprland = hyprland or HyprlandController()
        self._ensure_config()

    def _ensure_config(self) -> None:
        """Create workflows.json with default workflows if not present."""
        if not WORKFLOWS_FILE.exists():
            CONFIG_DIR.mkdir(parents=True, exist_ok=True)
            try:
                WORKFLOWS_FILE.write_text(json.dumps(DEFAULT_WORKFLOWS, indent=2))
            except Exception as e:
                log.warning(f"Failed to write default workflows: {e}")

    def get_workflows(self) -> Dict[str, Dict[str, Any]]:
        """Load all configured workflows."""
        self._ensure_config()
        try:
            return json.loads(WORKFLOWS_FILE.read_text())
        except Exception:
            return DEFAULT_WORKFLOWS.copy()

    def _save_to_disk(self, workflows: Dict[str, Dict[str, Any]]) -> bool:
        """Write workflows dictionary to disk."""
        try:
            CONFIG_DIR.mkdir(parents=True, exist_ok=True)
            WORKFLOWS_FILE.write_text(json.dumps(workflows, indent=2))
            return True
        except Exception as e:
            log.error(f"Failed to persist workflows to {WORKFLOWS_FILE}: {e}")
            return False

    def find_workflow(self, name: str) -> Optional[Dict[str, Any]]:
        """Find a workflow by name or alias."""
        clean_name = name.strip().lower()
        workflows = self.get_workflows()
        for key, wf in workflows.items():
            if key.lower() == clean_name:
                return wf
            for alias in wf.get("aliases", []):
                if alias.lower() == clean_name:
                    return wf
        return None

    def list_workflows(self) -> str:
        """List all available workflows with their descriptions and workspace count."""
        workflows = self.get_workflows()
        lines = ["Available workflows:"]
        for key, wf in sorted(workflows.items()):
            aliases = f" (aliases: {', '.join(wf.get('aliases', []))})" if wf.get("aliases") else ""
            desc = wf.get("description", "No description")
            steps = wf.get("steps", [])
            ws_list = sorted(list(set(s.get("workspace", 1) for s in steps)))
            ws_str = f" [Workspaces: {', '.join(str(w) for w in ws_list)}]" if ws_list else ""
            lines.append(f"• **{key}**{aliases}{ws_str}: {desc}")
        return "\n".join(lines)

    def get_workflow_details(self, name: str) -> str:
        """Get detailed breakdown of a specific workflow."""
        wf = self.find_workflow(name)
        if not wf:
            return f"Workflow '{name}' not found."

        lines = [
            f"### Workflow: {name}",
            f"**Description**: {wf.get('description', 'None')}",
            f"**Aliases**: {', '.join(wf.get('aliases', [])) or 'None'}",
            f"**Primary Workspace**: {wf.get('primary_workspace', 1)}",
            "**Launch Steps**:",
        ]
        for idx, step in enumerate(wf.get("steps", []), 1):
            ws = step.get("workspace", 1)
            cmd = step.get("launch", "")
            delay = step.get("delay", 0.25)
            lines.append(f"  {idx}. Workspace {ws}: `{cmd}` (delay: {delay}s)")

        return "\n".join(lines)

    def save_workflow(
        self,
        name: str,
        description: str,
        steps: List[Dict[str, Any]],
        aliases: Optional[List[str]] = None,
        primary_workspace: int = 1,
    ) -> str:
        """Create or update a custom workflow configuration."""
        slug = _clean_slug(name)
        if not slug:
            return "Invalid workflow name provided."

        if not steps or not isinstance(steps, list):
            return "A workflow requires at least one launch step."

        valid_steps = []
        for step in steps:
            if not isinstance(step, dict):
                continue
            ws = step.get("workspace", 1)
            cmd = step.get("launch", "").strip()
            delay = float(step.get("delay", 0.25))
            if cmd:
                valid_steps.append({
                    "workspace": int(ws),
                    "launch": cmd,
                    "delay": delay,
                })

        if not valid_steps:
            return "No valid launch commands found in steps."

        clean_aliases = []
        if aliases:
            for a in aliases:
                a_clean = str(a).strip().lower()
                if a_clean and a_clean != slug and a_clean not in clean_aliases:
                    clean_aliases.append(a_clean)

        workflows = self.get_workflows()
        workflows[slug] = {
            "description": description.strip() or f"Custom workflow '{slug}'",
            "aliases": clean_aliases,
            "primary_workspace": int(primary_workspace),
            "steps": valid_steps,
        }

        success = self._save_to_disk(workflows)
        if success:
            log.info(f"[green]Saved custom workflow '{slug}' with {len(valid_steps)} steps[/green]")
            return f"Successfully saved custom workflow '{slug}' with {len(valid_steps)} launch steps."
        return f"Failed to save workflow '{slug}' to disk."

    def delete_workflow(self, name: str) -> str:
        """Delete a configured workflow by name or alias."""
        clean_name = name.strip().lower()
        workflows = self.get_workflows()

        target_key = None
        for key, wf in workflows.items():
            if key.lower() == clean_name:
                target_key = key
                break
            for alias in wf.get("aliases", []):
                if alias.lower() == clean_name:
                    target_key = key
                    break

        if not target_key:
            return f"Workflow '{name}' not found."

        del workflows[target_key]
        success = self._save_to_disk(workflows)
        if success:
            log.info(f"[green]Deleted workflow '{target_key}'[/green]")
            return f"Workflow '{target_key}' has been deleted."
        return f"Failed to delete workflow '{target_key}' from disk."

    async def capture_current_setup(
        self,
        name: str,
        description: str = "",
        aliases: Optional[List[str]] = None,
    ) -> str:
        """Capture all currently open application windows on Hyprland into a new workflow."""
        slug = _clean_slug(name)
        if not slug:
            return "Please provide a valid name for the workflow to capture."

        try:
            clients = await self.hyprland.get_clients()
        except Exception as e:
            log.error(f"Failed to query Hyprland clients: {e}")
            return f"Error querying active Hyprland windows: {e}"

        if not clients:
            return "No active application windows found to capture."

        # Filter out special/scratchpad workspaces (id <= 0)
        valid_clients = [c for c in clients if c.get("workspace", {}).get("id", -1) > 0]
        if not valid_clients:
            return "No standard workspace windows found to capture."

        # Group and deduplicate by (workspace, launch_cmd)
        seen_launches = set()
        steps = []
        captured_apps = []

        for client in valid_clients:
            ws_id = client.get("workspace", {}).get("id", 1)
            cmd = _map_client_to_launch_cmd(client)
            if not cmd:
                continue

            pair = (ws_id, cmd)
            if pair in seen_launches:
                continue
            seen_launches.add(pair)

            steps.append({
                "workspace": ws_id,
                "launch": cmd,
                "delay": 0.25,
            })
            captured_apps.append(f"WS {ws_id}: {cmd}")

        if not steps:
            return "Could not resolve executable commands for the currently open windows."

        # Sort steps by workspace ascending
        steps.sort(key=lambda s: s["workspace"])

        # Determine active workspace for primary focus
        primary_ws = 1
        try:
            active_win = await self.hyprland.get_active_window()
            primary_ws = active_win.get("workspace", {}).get("id", 1)
        except Exception:
            pass

        final_desc = description.strip() or f"Captured setup: {', '.join(captured_apps[:5])}"

        save_msg = self.save_workflow(
            name=slug,
            description=final_desc,
            steps=steps,
            aliases=aliases,
            primary_workspace=primary_ws,
        )

        return (
            f"Captured {len(steps)} applications across workspaces and saved as workflow '{slug}'.\n"
            f"Details: {', '.join(captured_apps)}"
        )

    async def launch_workflow(self, name: str) -> str:
        """Launch a workflow preset across Hyprland workspaces."""
        wf = self.find_workflow(name)
        if not wf:
            available = ", ".join(self.get_workflows().keys())
            return f"Workflow '{name}' not found. Available workflows: {available}."

        steps = wf.get("steps", [])
        primary = wf.get("primary_workspace", 1)

        log.info(f"[cyan]Initializing workflow '{name}' across workspaces...[/cyan]")
        for step in steps:
            ws = step.get("workspace", 1)
            launch_cmd = step.get("launch", "")
            delay = step.get("delay", 0.25)

            if ws:
                await self.hyprland.change_workspace(ws)
                await asyncio.sleep(0.1)

            if launch_cmd:
                cmd_parts = launch_cmd.split()
                bin_path = shutil.which(cmd_parts[0]) or cmd_parts[0]
                try:
                    await asyncio.create_subprocess_exec(
                        bin_path,
                        *cmd_parts[1:],
                        stdout=asyncio.subprocess.DEVNULL,
                        stderr=asyncio.subprocess.DEVNULL,
                    )
                except Exception as e:
                    log.warning(f"Could not launch '{launch_cmd}': {e}")

            if delay > 0:
                await asyncio.sleep(delay)

        # Switch back to primary workspace
        await self.hyprland.change_workspace(primary)
        return f"Workflow '{name}' initialized across workspaces. Switched to workspace {primary}, sir."
