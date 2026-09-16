"""CLI AI tool integration for autonomous project creation and code scaffolding.

Supports Anthropic Claude Code (claude), OpenAI Codex (codex), and Google Antigravity (agy).
Configurable via JARVIS_CLI_AI_TOOL in .env (defaults to claude).
"""

import asyncio
from pathlib import Path
import re
import shlex
import shutil
import subprocess
from typing import Optional
from jarvis.core.config import settings
from jarvis.core.logger import log
from jarvis.tools.omarchy import OmarchyBridge


class CodingCLIManager:
    """Manages autonomous project creation and code generation using preferred CLI AI tools (claude, codex, agy)."""

    def __init__(self, cli_tool: Optional[str] = None):
        self.cli_tool = (cli_tool or settings.cli_ai_tool).lower().strip()
        self.kitty_bin = shutil.which("kitty")
        self.foot_bin = shutil.which("foot")
        self.terminal_launcher = shutil.which("omarchy-launch-terminal")
        self.omarchy = OmarchyBridge()

    @property
    def display_name(self) -> str:
        """Human-readable display name of the active CLI AI tool."""
        if self.cli_tool == "claude":
            return "Claude Code"
        elif self.cli_tool == "codex":
            return "Codex"
        return "Antigravity"

    def get_binary_path(self) -> str:
        """Find the executable binary for the active CLI AI tool."""
        if self.cli_tool == "claude":
            return shutil.which("claude") or str(Path.home() / ".local/bin/claude")
        elif self.cli_tool == "codex":
            return (
                shutil.which("codex")
                or str(Path.home() / ".local/share/mise/shims/codex")
                or "/usr/local/bin/codex"
            )
        else:  # agy
            return shutil.which("agy") or str(Path.home() / ".local/bin/agy")

    def resolve_target_dir(self, name: str, requested_dir: Optional[str] = None) -> Path:
        """Resolve the target directory path for a new project or file."""
        clean_name = re.sub(r"[^\w.-]", "_", name.strip()) or "new_project"

        if requested_dir:
            raw = requested_dir.strip()
            # Check for home directory indicators
            if raw.lower() in ("home", "home directory", "~", "$home", str(Path.home()).lower()):
                return Path.home() / clean_name
            expanded = Path(raw).expanduser().resolve()
            if expanded.name == clean_name:
                return expanded
            return expanded / clean_name

        # Default search locations
        codes_personal = Path.home() / "Codes" / "personal"
        if codes_personal.exists():
            return codes_personal / clean_name

        projects_dir = Path.home() / "Projects"
        if projects_dir.exists():
            return projects_dir / clean_name

        return Path.home() / clean_name

    def build_scaffold_prompt(
        self,
        name: str,
        description: str = "",
        target_path: Optional[Path] = None,
        project_type: str = "project",
    ) -> str:
        """Construct a high-quality autonomous instruction prompt for the CLI tool."""
        type_str = project_type.strip() if project_type else "project"
        desc_str = description.strip() if description else f"Initialize a complete {type_str} named {name}."
        path_str = str(target_path) if target_path else str(Path.home() / name)

        return (
            f"Active Workspace Directory: {path_str}\n"
            f"You have full autonomous permissions. Please create and completely initialize the {type_str} named '{name}' directly inside the workspace directory `{path_str}`.\n\n"
            f"Specifications:\n{desc_str}\n\n"
            f"Requirements:\n"
            f"1. Generate all required directory structure, source files, and configuration files (e.g. pyproject.toml, package.json, Cargo.toml, or Makefile as appropriate) directly inside `{path_str}`.\n"
            f"2. Write complete, functional, production-ready code with proper error handling and documentation. Avoid placeholder code, stubs, or TODO comments.\n"
            f"3. Create a comprehensive README.md in `{path_str}` explaining what the project does, prerequisites, and how to run or test it.\n"
            f"4. Ensure all files are written directly to disk in `{path_str}` and formatted cleanly."
        )

    def _build_autonomous_command(
        self,
        target_path: Path,
        prompt: str,
        log_file: Path,
        success_cmd: str,
        fail_cmd: str,
    ) -> str:
        """Build the shell command string for headless background execution with bypassed permissions."""
        bin_path = self.get_binary_path()
        if self.cli_tool == "claude":
            return (
                f"setsid {shlex.quote(bin_path)} -p {shlex.quote(prompt)} --dangerously-skip-permissions "
                f"> {shlex.quote(str(log_file))} 2>&1 && {success_cmd} || {fail_cmd}"
            )
        elif self.cli_tool == "codex":
            return (
                f"setsid {shlex.quote(bin_path)} exec -C {shlex.quote(str(target_path))} "
                f"--dangerously-bypass-approvals-and-sandbox -a never {shlex.quote(prompt)} "
                f"> {shlex.quote(str(log_file))} 2>&1 && {success_cmd} || {fail_cmd}"
            )
        else:  # agy
            return (
                f"setsid {shlex.quote(bin_path)} --add-dir {shlex.quote(str(target_path))} "
                f"-p {shlex.quote(prompt)} --dangerously-skip-permissions "
                f"> {shlex.quote(str(log_file))} 2>&1 && {success_cmd} || {fail_cmd}"
            )

    def _build_interactive_command(self, cwd: Path, prompt: str) -> str:
        """Build the shell command string for floating popup terminal execution."""
        bin_path = self.get_binary_path()
        if self.cli_tool == "claude":
            core_cmd = f"{shlex.quote(bin_path)} {shlex.quote(prompt)} --dangerously-skip-permissions"
        elif self.cli_tool == "codex":
            core_cmd = f"{shlex.quote(bin_path)} -C {shlex.quote(str(cwd))} --dangerously-bypass-approvals-and-sandbox {shlex.quote(prompt)}"
        else:  # agy
            core_cmd = f"{shlex.quote(bin_path)} --add-dir {shlex.quote(str(cwd))} -i {shlex.quote(prompt)} --dangerously-skip-permissions"

        return (
            f"{core_cmd}; "
            f"echo; echo '══════════════════════════════════════════════════════════════════'; "
            f"echo '{self.display_name} session complete. Press Enter to close this window...'; read"
        )

    async def launch_popup_terminal(self, cwd: Path, prompt: str, title: str = "Jarvis - Coding Assistant") -> bool:
        """Launch an interactive floating popup terminal window running the CLI tool with full permissions."""
        cwd.mkdir(parents=True, exist_ok=True)
        interactive_sh = self._build_interactive_command(cwd, prompt)

        try:
            if self.kitty_bin:
                # Class TUI.float triggers Omarchy Hyprland's centered floating window rule
                cmd = [
                    self.kitty_bin,
                    "--class",
                    "TUI.float",
                    "-T",
                    title,
                    "-d",
                    str(cwd),
                    "bash",
                    "-c",
                    interactive_sh,
                ]
                await asyncio.create_subprocess_exec(
                    *cmd,
                    stdout=asyncio.subprocess.DEVNULL,
                    stderr=asyncio.subprocess.DEVNULL,
                )
                return True
            elif self.foot_bin:
                cmd = [
                    self.foot_bin,
                    "--app-id",
                    "TUI.float",
                    "-T",
                    title,
                    "-D",
                    str(cwd),
                    "bash",
                    "-c",
                    interactive_sh,
                ]
                await asyncio.create_subprocess_exec(
                    *cmd,
                    stdout=asyncio.subprocess.DEVNULL,
                    stderr=asyncio.subprocess.DEVNULL,
                )
                return True
            elif self.terminal_launcher:
                cmd = [self.terminal_launcher, "bash", "-c", interactive_sh]
                await asyncio.create_subprocess_exec(
                    *cmd,
                    cwd=str(cwd),
                    stdout=asyncio.subprocess.DEVNULL,
                    stderr=asyncio.subprocess.DEVNULL,
                )
                return True
            else:
                log.warning("No GUI terminal emulator found to launch popup window.")
                return False
        except Exception as e:
            log.warning(f"Failed to launch popup terminal: {e}")
            return False

    def _run_autonomous_job(self, target_path: Path, name: str, prompt: str) -> None:
        """Run the CLI tool autonomously in a detached session with auto-approved permissions."""
        log.info(f"[cyan]{self.display_name} dispatching autonomous job for '{name}' in {target_path}[/cyan]")
        log_file = target_path / f".{self.cli_tool}_scaffold.log"

        notify_bin = self.omarchy.notify_bin or "notify-send"
        success_cmd = (
            f"{shlex.quote(notify_bin)} -g '✔' '✔ Project {name} Ready' "
            f"'Successfully created by {self.display_name} in {str(target_path)}' 2>/dev/null || true"
        )
        fail_cmd = (
            f"{shlex.quote(notify_bin)} -g '⚠' '⚠ Project {name} Failed' "
            f"'{self.display_name} encountered an issue. Check .{self.cli_tool}_scaffold.log' 2>/dev/null || true"
        )

        wrapper_cmd = self._build_autonomous_command(target_path, prompt, log_file, success_cmd, fail_cmd)

        try:
            subprocess.Popen(
                ["bash", "-c", wrapper_cmd],
                cwd=str(target_path),
                start_new_session=True,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                stdin=subprocess.DEVNULL,
            )
            log.info(f"[bold green]{self.display_name} background job active for '{name}'[/bold green]")
        except Exception as e:
            log.error(f"Failed to spawn detached {self.display_name} job: {e}")

    async def create_project(
        self,
        name: str,
        description: str = "",
        target_dir: Optional[str] = None,
        project_type: str = "project",
        open_terminal: bool = False,
    ) -> str:
        """Create a complete project using the preferred CLI AI tool with full autonomous permissions or floating popup."""
        target_path = self.resolve_target_dir(name, target_dir)
        target_path.mkdir(parents=True, exist_ok=True)
        prompt = self.build_scaffold_prompt(name, description, target_path, project_type)

        log.info(f"[cyan]Creating {project_type} '{name}' in {target_path} using {self.display_name} (popup={open_terminal})[/cyan]")

        if open_terminal:
            success = await self.launch_popup_terminal(
                target_path,
                prompt,
                title=f"Jarvis - {self.display_name} Scaffolding {name}",
            )
            if success:
                return f"I have opened {self.display_name} in a floating popup window to create '{name}' in {target_path}, sir. It is executing with full permissions."
            # Fallback to background execution if popup launch failed

        # Background autonomous execution with full permissions
        await self.omarchy.notify(
            f"󰚩 Creating {name}",
            f"{self.display_name} is building project in {target_path}...",
            glyph="󰚩",
        )
        self._run_autonomous_job(target_path, name, prompt)
        return f"I have dispatched {self.display_name} to create your project '{name}' in {target_path}. It has full autonomous permissions to generate all code and files, sir."

    async def create_anything(
        self,
        task_description: str,
        target_dir: Optional[str] = None,
        open_terminal: bool = False,
    ) -> str:
        """Delegate any arbitrary code generation, note, or document creation task to the CLI AI tool."""
        target_path = Path(target_dir).expanduser().resolve() if target_dir else Path.home()
        target_path.mkdir(parents=True, exist_ok=True)

        full_prompt = (
            f"Active Workspace Directory: {target_path}\n"
            f"You have full autonomous permissions.\n"
            f"Task: {task_description}\n"
            f"Ensure all files are completely written to disk in `{target_path}` with no placeholders."
        )

        log.info(f"[cyan]Delegating task to {self.display_name} in {target_path}: '{task_description}'[/cyan]")

        if open_terminal:
            success = await self.launch_popup_terminal(
                target_path,
                full_prompt,
                title=f"Jarvis - {self.display_name} Task",
            )
            if success:
                return f"I have opened {self.display_name} in a floating popup window to execute your task with full permissions, sir."

        await self.omarchy.notify(
            f"󰚩 {self.display_name} Task Started",
            f"{task_description[:60]}...",
            glyph="󰚩",
        )
        self._run_autonomous_job(target_path, "Custom Task", full_prompt)
        return f"I have dispatched {self.display_name} with full permissions to fulfill your request in the background, sir."


# Backward compatibility alias
AntigravityManager = CodingCLIManager
