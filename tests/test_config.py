from jarvis.core.config import Settings

def test_default_settings():
    config = Settings()
    assert config.model_name in ("gemini-3.5-flash-lite", "gemini-3.5-flash")
    assert config.stt_engine == "groq"
    assert config.tts_engine == "piper"
    assert config.sample_rate == 16000

def test_load_omarchy_theme():
    from jarvis.core.theme import load_omarchy_theme
    theme = load_omarchy_theme()
    assert "accent" in theme
    assert "foreground" in theme
    assert "background" in theme
