import json
import os
from pathlib import Path
import time
from typing import Any, Dict, Optional
from jarvis.core.config import settings
from jarvis.core.logger import log

def get_runtime_dir() -> Path:
    """Returns the user runtime directory (e.g. /run/user/1000 or /tmp)."""
    xdg = os.environ.get("XDG_RUNTIME_DIR")
    if xdg and Path(xdg).is_dir():
        return Path(xdg)
    return Path("/tmp")


STATE_JSON_PATH = get_runtime_dir() / "jarvis-status.json"
PID_FILE = get_runtime_dir() / f"jarvis-{os.getuid()}.pid"
DAEMON_PID_FILE = get_runtime_dir() / f"jarvis-daemon-{os.getuid()}.pid"
WAKEWORD_LOCK_FILE = get_runtime_dir() / f"jarvis-wakeword-{os.getuid()}.state"


def get_default_status() -> Dict[str, Any]:
    """Returns the base schema for Jarvis status."""
    return {
        "active": False,
        "daemon_running": False,
        "state": "idle",  # "idle", "wakeword", "recording", "processing", "speaking"
        "mic_active": False,  # True ONLY when actively listening to user voice
        "wakeword_enabled": settings.wakeword_enabled,
        "cli_tool": settings.cli_ai_tool,
        "last_transcript": "",
        "last_reply": "",
        "updated_at": time.time(),
    }


def read_status() -> Dict[str, Any]:
    """Read the current status from the runtime JSON file."""
    status = get_default_status()
    if STATE_JSON_PATH.exists():
        try:
            data = json.loads(STATE_JSON_PATH.read_text(encoding="utf-8"))
            if isinstance(data, dict):
                status.update(data)
        except Exception as e:
            log.debug(f"Failed to read {STATE_JSON_PATH}: {e}")
    return status


def update_status(**kwargs: Any) -> Dict[str, Any]:
    """Atomically update status fields and write to the runtime JSON file."""
    current = read_status()
    current.update(kwargs)
    current["updated_at"] = time.time()
    
    # mic_active is true strictly if state is recording
    if "state" in kwargs:
        current["mic_active"] = (kwargs["state"] == "recording")

    try:
        tmp_file = STATE_JSON_PATH.with_suffix(".tmp")
        tmp_file.write_text(json.dumps(current, indent=2), encoding="utf-8")
        tmp_file.replace(STATE_JSON_PATH)
    except Exception as e:
        log.debug(f"Failed to write {STATE_JSON_PATH}: {e}")
        
    return current


def set_recording(prompt_hint: str = "") -> None:
    """Set state to recording (mic is ON)."""
    update_status(
        active=True,
        state="recording",
        mic_active=True,
        last_transcript=prompt_hint,
    )


def set_processing(transcript: str = "") -> None:
    """Set state to processing (mic is OFF, thinking/executing)."""
    update_status(
        active=True,
        state="processing",
        mic_active=False,
        last_transcript=transcript or read_status().get("last_transcript", ""),
    )


def set_speaking(reply: str = "") -> None:
    """Set state to speaking (mic is OFF, TTS playing)."""
    update_status(
        active=True,
        state="speaking",
        mic_active=False,
        last_reply=reply,
    )


def set_idle(wakeword_active: bool = False) -> None:
    """Set state to idle or wakeword standby."""
    state_str = "wakeword" if wakeword_active else "idle"
    update_status(
        active=False,
        state=state_str,
        mic_active=False,
    )
