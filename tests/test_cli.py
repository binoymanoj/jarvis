import os
from pathlib import Path
import pytest
from jarvis.cli import is_process_alive, get_current_state, set_current_state, STATE_FILE, PID_FILE, stop_running_session

def test_process_alive():
    assert is_process_alive(os.getpid()) is True
    assert is_process_alive(9999999) is False

def test_state_management():
    set_current_state("recording")
    assert get_current_state() == "recording"
    set_current_state("speaking")
    assert get_current_state() == "speaking"
    if STATE_FILE.exists():
        STATE_FILE.unlink()

def test_stop_running_session_when_idle():
    # If no session is running, should return False cleanly
    if PID_FILE.exists():
        PID_FILE.unlink()
    assert stop_running_session() is False
