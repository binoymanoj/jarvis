import numpy as np
import pytest
from jarvis.audio.wakeword import WakeWordDetector


def test_wakeword_initialization():
    detector = WakeWordDetector()
    assert detector.model is not None
    assert detector._model_key is not None
    assert "jarvis" in detector._model_key


def test_wakeword_process_zeros():
    detector = WakeWordDetector()
    chunk = np.zeros(1280, dtype=np.int16)
    detected, score = detector.process_chunk(chunk)
    assert detected is False
    assert 0.0 <= score <= 1.0


def test_wakeword_threshold_and_reset():
    detector = WakeWordDetector(threshold=0.35)
    assert detector.threshold == 0.35
    detector.reset()
    assert detector.model is not None


def test_wakeword_soft_agc_float_input():
    detector = WakeWordDetector()
    # Float array representing low-level signal
    chunk_float = np.sin(np.linspace(0, 10, 1280)).astype(np.float32) * 0.05
    detected, score = detector.process_chunk(chunk_float)
    assert isinstance(detected, bool)
    assert isinstance(score, float)


def test_wake_chime_plays():
    from jarvis.audio.wakeword import play_wake_chime
    # Should execute cleanly without throwing exceptions
    play_wake_chime()
