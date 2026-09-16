"""Email drafting and composition integration for Jarvis."""

import asyncio
import shutil
import urllib.parse
from typing import Optional
from jarvis.core.logger import log


class EmailManager:
    """Manages email drafting and opening compose windows in Thunderbird or webmail."""

    def __init__(self):
        self._thunderbird_bin = shutil.which("thunderbird")
        self._browser_bin = shutil.which("omarchy-launch-browser") or shutil.which("xdg-open") or "xdg-open"
        self._xdg_open_bin = shutil.which("xdg-open") or "xdg-open"

    async def draft_email(
        self,
        recipient: str,
        subject: str,
        body: str,
        client: str = "auto",
    ) -> str:
        """Draft an email and open the compose window with fields prefilled for user review.

        Args:
            recipient: Target recipient email address or contact name.
            subject: Email subject line.
            body: Full drafted email body text.
            client: 'auto', 'thunderbird', or 'gmail'.
        """
        clean_recipient = recipient.strip()
        clean_subject = subject.strip()
        clean_body = body.strip()

        log.info(f"[cyan]Drafting email to '{clean_recipient}' with subject '{clean_subject}'[/cyan]")

        # Option 1: Gmail web compose if requested
        if client.lower() == "gmail":
            encoded_to = urllib.parse.quote(clean_recipient)
            encoded_su = urllib.parse.quote(clean_subject)
            encoded_body = urllib.parse.quote(clean_body)
            gmail_url = f"https://mail.google.com/mail/?view=cm&fs=1&to={encoded_to}&su={encoded_su}&body={encoded_body}"
            await asyncio.create_subprocess_exec(
                self._browser_bin,
                gmail_url,
                stdout=asyncio.subprocess.DEVNULL,
                stderr=asyncio.subprocess.DEVNULL,
            )
            return f"Drafted email to {clean_recipient} and opened Gmail compose window for review, sir."

        # Option 2: Native Thunderbird CLI
        if self._thunderbird_bin and client.lower() in ("auto", "thunderbird"):
            compose_args = f"to='{clean_recipient}',subject='{clean_subject}',body='{clean_body}'"
            try:
                await asyncio.create_subprocess_exec(
                    self._thunderbird_bin,
                    "-compose",
                    compose_args,
                    stdout=asyncio.subprocess.DEVNULL,
                    stderr=asyncio.subprocess.DEVNULL,
                )
                return f"Drafted email to {clean_recipient} with subject '{clean_subject}'. Thunderbird compose window is open for your review, sir."
            except Exception as e:
                log.warning(f"Thunderbird compose failed: {e}. Falling back to standard mailto link.")

        # Option 3: Standard mailto URL via xdg-open
        encoded_to = urllib.parse.quote(clean_recipient)
        encoded_su = urllib.parse.quote(clean_subject)
        encoded_body = urllib.parse.quote(clean_body)
        mailto_url = f"mailto:{encoded_to}?subject={encoded_su}&body={encoded_body}"

        try:
            await asyncio.create_subprocess_exec(
                self._xdg_open_bin,
                mailto_url,
                stdout=asyncio.subprocess.DEVNULL,
                stderr=asyncio.subprocess.DEVNULL,
            )
            return f"Drafted email to {clean_recipient} and opened your default email client for review, sir."
        except Exception as e:
            log.error(f"Failed to open mail client: {e}")
            return f"Drafted email content, but failed to open email client: {e}"
