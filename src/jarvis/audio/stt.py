import io
from typing import Optional
from groq import AsyncGroq
from jarvis.core.config import settings
from jarvis.core.logger import log


class SpeechToText:
    """High-speed Speech-to-Text transcriber using Groq Whisper API."""

    def __init__(self, api_key: Optional[str] = None):
        key = api_key if api_key is not None else settings.groq_api_key
        self.api_key = key.strip() if key else ""
        self._client: Optional[AsyncGroq] = None
        if self.api_key:
            self._client = AsyncGroq(api_key=self.api_key)

    async def transcribe(self, wav_bytes: bytes) -> str:
        """Transcribe WAV audio bytes to text.

        Args:
            wav_bytes: 16-bit PCM WAV audio content.

        Returns:
            Transcribed string text.
        """
        if not wav_bytes:
            return ""

        if not self._client:
            log.warning("[yellow]GROQ_API_KEY not set in .env. Skipping cloud STT transcription.[/yellow]")
            return ""

        log.info("[cyan]Transcribing audio with Groq Whisper Large v3...[/cyan]")
        try:
            audio_file = ("input.wav", io.BytesIO(wav_bytes), "audio/wav")
            response = await self._client.audio.transcriptions.create(
                file=audio_file,
                model=settings.whisper_model,
                response_format="text",
                language="en",
                temperature=0.0,
            )
            result = str(response).strip()
            log.info(f"[bold green]Transcription:[/bold green] \"{result}\"")
            return result
        except Exception as e:
            log.error(f"Groq STT transcription error: {e}")
            return ""
