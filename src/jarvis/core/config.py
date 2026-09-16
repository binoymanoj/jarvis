from pathlib import Path
from typing import Literal, Optional
from pydantic import Field
from pydantic_settings import BaseSettings, SettingsConfigDict

# Root project directory
PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent.parent


class Settings(BaseSettings):
    """Application configuration loaded from environment variables and .env file."""

    model_config = SettingsConfigDict(
        env_file=str(PROJECT_ROOT / ".env"),
        env_file_encoding="utf-8",
        extra="ignore",
    )

    # API Keys (Free tier)
    gemini_api_key: Optional[str] = Field(default=None, alias="GEMINI_API_KEY")
    groq_api_key: Optional[str] = Field(default=None, alias="GROQ_API_KEY")

    # Model & Agent
    model_name: str = Field(default="gemini-3.5-flash-lite", alias="JARVIS_MODEL")

    # Coding & Project Scaffolding CLI Tool ("claude", "codex", "agy")
    cli_ai_tool: Literal["claude", "codex", "agy"] = Field(default="claude", alias="JARVIS_CLI_AI_TOOL")

    # Speech-to-Text
    stt_engine: Literal["groq", "local"] = Field(default="groq", alias="JARVIS_STT_ENGINE")
    whisper_model: str = Field(default="whisper-large-v3-turbo", alias="JARVIS_WHISPER_MODEL")
    local_whisper_size: str = Field(default="base.en", alias="JARVIS_LOCAL_WHISPER_SIZE")

    # Text-to-Speech
    tts_engine: Literal["piper", "edge"] = Field(default="piper", alias="JARVIS_TTS_ENGINE")
    piper_voice: str = Field(default="en_GB-alan-medium", alias="JARVIS_VOICE")

    # Audio recording settings (tuned for natural conversational response)
    sample_rate: int = Field(default=16000, alias="JARVIS_SAMPLE_RATE")
    channels: int = Field(default=1, alias="JARVIS_CHANNELS")
    silence_threshold_seconds: float = Field(default=0.85, alias="JARVIS_SILENCE_THRESHOLD")
    max_recording_seconds: float = Field(default=20.0, alias="JARVIS_MAX_RECORDING_SECONDS")
    initial_listen_timeout: float = Field(default=8.0, alias="JARVIS_INITIAL_LISTEN_TIMEOUT")
    followup_listen_timeout: float = Field(default=7.0, alias="JARVIS_FOLLOWUP_LISTEN_TIMEOUT")

    # Wake Word Detection ("Hey Jarvis" / "Jarvis")
    wakeword_enabled: bool = Field(default=True, alias="JARVIS_WAKEWORD_ENABLED")
    wakeword_model: str = Field(default="hey_jarvis", alias="JARVIS_WAKEWORD_MODEL")
    wakeword_threshold: float = Field(default=0.28, alias="JARVIS_WAKEWORD_THRESHOLD")

    # IPC & Sockets
    daemon_socket_path: Path = Path(f"/tmp/jarvis-{Path.home().name}.sock")


settings = Settings()
