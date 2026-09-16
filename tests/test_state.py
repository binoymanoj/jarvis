import pytest
from jarvis.core.state import (
    read_status,
    update_status,
    set_recording,
    set_processing,
    set_speaking,
    set_idle,
)


def test_state_lifecycle():
    # Set idle
    st = update_status(state="idle", mic_active=False)
    assert st["mic_active"] is False

    # Set recording (mic is ON)
    set_recording("listening prompt")
    st = read_status()
    assert st["state"] == "recording"
    assert st["mic_active"] is True
    assert st["last_transcript"] == "listening prompt"

    # Set processing (mic is OFF)
    set_processing("user speech")
    st = read_status()
    assert st["state"] == "processing"
    assert st["mic_active"] is False

    # Set speaking
    set_speaking("hello sir")
    st = read_status()
    assert st["state"] == "speaking"
    assert st["mic_active"] is False
    assert st["last_reply"] == "hello sir"

    # Set idle
    set_idle()
    st = read_status()
    assert st["state"] == "idle"
    assert st["mic_active"] is False
