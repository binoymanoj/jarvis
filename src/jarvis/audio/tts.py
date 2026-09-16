import asyncio
import io
from pathlib import Path
from typing import Optional
import numpy as np
import sounddevice as sd
import soundfile as sf
from jarvis.core.config import settings
from jarvis.core.logger import log

MODELS_DIR = Path(__file__).resolve().parent / "models"


class TextToSpeech:
    """Hybrid Text-to-Speech engine supporting Piper (offline CPU) and Edge-TTS (neural cloud)."""

    def __init__(self, engine: Optional[str] = None):
        self.engine = engine or settings.tts_engine
        self._piper_voice = None
        self._is_speaking = False

        # Preload Piper if selected and model exists
        if self.engine == "piper":
            self._init_piper()

    def _init_piper(self) -> None:
        """Initializes the offline Piper Alan voice model."""
        onnx_path = MODELS_DIR / "en_GB-alan-medium.onnx"
        json_path = MODELS_DIR / "en_GB-alan-medium.onnx.json"
        if onnx_path.exists() and json_path.exists():
            try:
                from piper import PiperVoice
                self._piper_voice = PiperVoice.load(str(onnx_path), config_path=str(json_path))
                log.info("[cyan]Piper British voice (Alan) loaded successfully.[/cyan]")
            except Exception as e:
                log.warning(f"Could not initialize Piper: {e}")
        else:
            log.warning("Piper model files not found, falling back to Edge-TTS.")
            self.engine = "edge"

    async def speak(self, text: str) -> None:
        """Synthesize and play audio response through PipeWire."""
        if not text or not text.strip():
            return

        clean_text = text.strip()
        log.info(f"[bold magenta]Jarvis Speaking:[/bold magenta] \"{clean_text}\"")

        if self.engine == "piper" and self._piper_voice:
            await self._speak_piper(clean_text)
        else:
            await self._speak_edge(clean_text)

    async def _speak_piper(self, text: str) -> None:
        """Synthesize using local Piper TTS model."""
        import wave
        
        loop = asyncio.get_running_loop()

        def _synthesize():
            buffer = io.BytesIO()
            with wave.open(buffer, "wb") as wf:
                self._piper_voice.synthesize_wav(text, wf)
            buffer.seek(0)
            data, sample_rate = sf.read(buffer)
            return data, sample_rate

        try:
            data, sample_rate = await loop.run_in_executor(None, _synthesize)
            self._is_speaking = True
            sd.play(data, samplerate=sample_rate)
            duration = len(data) / sample_rate
            elapsed = 0.0
            while self._is_speaking and elapsed < duration:
                await asyncio.sleep(min(0.05, duration - elapsed))
                elapsed += 0.05
        except Exception as e:
            log.error(f"Piper TTS playback failed: {e}")
        finally:
            self._is_speaking = False

    async def _speak_edge(self, text: str, voice: str = "en-GB-RyanNeural") -> None:
        """Synthesize using Microsoft Edge Neural TTS."""
        try:
            import edge_tts
            communicate = edge_tts.Communicate(text, voice)
            audio_buffer = bytearray()
            async for chunk in communicate.stream():
                if not self._is_speaking:
                    break
                if chunk["type"] == "audio":
                    audio_buffer.extend(chunk["data"])

            data, sample_rate = sf.read(io.BytesIO(audio_buffer))
            self._is_speaking = True
            sd.play(data, samplerate=sample_rate)
            duration = len(data) / sample_rate
            elapsed = 0.0
            while self._is_speaking and elapsed < duration:
                await asyncio.sleep(min(0.05, duration - elapsed))
                elapsed += 0.05
        except Exception as e:
            log.error(f"Edge TTS playback failed: {e}")
        finally:
            self._is_speaking = False

    def stop(self) -> None:
        """Immediately stop any current audio playback."""
        sd.stop()
        self._is_speaking = False
