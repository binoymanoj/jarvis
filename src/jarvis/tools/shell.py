"""Universal shell execution tool for Jarvis.

Enables hands-free execution of arbitrary Linux commands and shell scripts.
"""

import asyncio
import os
from typing import Optional
from jarvis.core.logger import log


class ShellExecutor:
    """Safely executes shell commands on Omarchy Linux with output truncation and timeouts."""

    def __init__(self, default_timeout: float = 15.0, max_output_chars: int = 2000):
        self.default_timeout = default_timeout
        self.max_output_chars = max_output_chars

    async def execute_command(self, command: str, timeout: Optional[float] = None) -> str:
        """Execute an arbitrary bash command and return its output.

        Args:
            command: The bash command string to execute.
            timeout: Maximum execution time in seconds before terminating (default: 15.0s).
        """
        if not command or not command.strip():
            return "No command provided."

        cmd_clean = command.strip()
        actual_timeout = timeout if timeout is not None and timeout > 0 else self.default_timeout
        log.info(f"[cyan]Executing shell command (timeout={actual_timeout}s): {cmd_clean}[/cyan]")

        try:
            proc = await asyncio.create_subprocess_shell(
                cmd_clean,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
                executable="/bin/bash",
                env=os.environ.copy(),
            )

            try:
                stdout_bytes, stderr_bytes = await asyncio.wait_for(
                    proc.communicate(),
                    timeout=actual_timeout,
                )
            except asyncio.TimeoutError:
                try:
                    proc.kill()
                    await proc.wait()
                except Exception:
                    pass
                log.warning(f"Shell command timed out after {actual_timeout}s: {cmd_clean}")
                return f"Command timed out after {actual_timeout} seconds."

            stdout = stdout_bytes.decode("utf-8", errors="replace").strip()
            stderr = stderr_bytes.decode("utf-8", errors="replace").strip()
            return_code = proc.returncode

            # Format result
            output_parts = []
            if stdout:
                output_parts.append(stdout)
            if stderr:
                output_parts.append(f"[stderr]:\n{stderr}")

            combined = "\n".join(output_parts).strip()

            if not combined:
                if return_code == 0:
                    return "Command executed successfully (no output)."
                else:
                    return f"Command exited with return code {return_code} (no output)."

            # Truncate if exceeds max_output_chars
            if len(combined) > self.max_output_chars:
                half = self.max_output_chars // 2
                truncated = (
                    combined[:half]
                    + f"\n\n... [output truncated: {len(combined)} characters total] ...\n\n"
                    + combined[-half:]
                )
                return truncated

            return combined

        except Exception as e:
            log.error(f"Failed to execute shell command '{cmd_clean}': {e}")
            return f"Execution error: {e}"
