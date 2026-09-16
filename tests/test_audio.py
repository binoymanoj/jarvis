import numpy as np
import pytest
from jarvis.audio.vad import SileroVAD
from jarvis.audio.tts import TextToSpeech
from jarvis.audio.stt import SpeechToText

def test_silero_vad_silence():
    vad = SileroVAD()
    silence = np.zeros(512, dtype=np.float32)
    prob = vad.get_speech_prob(silence)
    assert 0.0 <= prob <= 0.1
    assert not vad.is_speech(silence, threshold=0.5)

def test_tts_init():
    tts = TextToSpeech(engine="piper")
    assert tts._piper_voice is not None

def test_stt_init_without_key():
    stt = SpeechToText(api_key="")
    assert stt._client is None

def test_stt_init_with_key():
    stt = SpeechToText(api_key="mock_key")
    assert stt._client is not None

def test_audio_recorder_config():
    from jarvis.audio.recorder import AudioRecorder
    rec = AudioRecorder()
    assert rec.vad_threshold == 0.35
    assert rec.silence_chunks_limit >= 15
    assert rec.max_duration == 20.0
