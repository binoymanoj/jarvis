import asyncio
import io
import time
import wave
from typing import Callable, Optional
import numpy as np
import sounddevice as sd
from jarvis.audio.vad import SileroVAD
from jarvis.core.config import settings
from jarvis.core.logger import log


class AudioRecorder:
    """High-speed microphone recorder with dual VAD+RMS fast silence detection and push-to-talk support."""

    def __init__(
        self,
        sample_rate: int = 16000,
        chunk_size: int = 512,
        silence_timeout: Optional[float] = None,
        max_duration: Optional[float] = None,
        vad_threshold: float = 0.35,
        energy_threshold: float = 0.003,
    ):
        self.sample_rate = sample_rate
        self.chunk_size = chunk_size
        self.silence_timeout = silence_timeout if silence_timeout is not None else settings.silence_threshold_seconds
        self.max_duration = max_duration if max_duration is not None else settings.max_recording_seconds
        self.vad_threshold = vad_threshold
        self.energy_threshold = energy_threshold
        self.silence_chunks_limit = max(15, int(self.silence_timeout / (self.chunk_size / self.sample_rate)))
        self.vad = SileroVAD()
        self._is_recording = False
        self._stop_requested = False

    def request_stop(self) -> None:
        """Signal the recording loop to stop immediately and submit audio."""
        self._stop_requested = True

    async def record_phrase(
        self,
        on_volume: Optional[Callable[[float], None]] = None,
        initial_timeout: Optional[float] = None,
    ) -> bytes:
        """Record audio from microphone with robust VAD pause detection and push-to-talk support."""
        self._is_recording = True
        self._stop_requested = False
        self.vad.reset()

        recorded_chunks: list[np.ndarray] = []
        speech_started = False
        consecutive_speech_chunks = 0
        consecutive_silence_chunks = 0
        start_time = time.monotonic()
        wait_timeout = initial_timeout if initial_timeout is not None else settings.initial_listen_timeout

        loop = asyncio.get_running_loop()
        audio_queue: asyncio.Queue[np.ndarray] = asyncio.Queue()

        def audio_callback(indata, frames, time_info, status):
            if status:
                log.debug(f"Audio status: {status}")
            chunk = indata[:, 0].copy()
            loop.call_soon_threadsafe(audio_queue.put_nowait, chunk)

        # 512 samples at 16kHz = 32ms per chunk
        stream = sd.InputStream(
            samplerate=self.sample_rate,
            channels=1,
            dtype="float32",
            blocksize=self.chunk_size,
            callback=audio_callback,
        )

        log.info("[bold green]Listening for speech (fast VAD active)...[/bold green]")
        with stream:
            while self._is_recording and not self._stop_requested:
                try:
                    chunk = await asyncio.wait_for(audio_queue.get(), timeout=0.08)
                except asyncio.TimeoutError:
                    continue

                recorded_chunks.append(chunk)

                # Compute RMS energy
                rms = float(np.sqrt(np.mean(chunk**2)))
                volume_level = min(1.0, rms * 14.0)
                if on_volume:
                    try:
                        on_volume(volume_level)
                    except Exception:
                        pass

                # Accurate VAD speech detection: Silero probability + voice energy floor
                vad_prob = self.vad.get_speech_prob(chunk)
                is_real_speech = (vad_prob >= self.vad_threshold) and (rms >= self.energy_threshold)

                current_time = time.monotonic()
                elapsed = current_time - start_time

                if is_real_speech:
                    consecutive_speech_chunks += 1
                    consecutive_silence_chunks = 0
                    if not speech_started and consecutive_speech_chunks >= 2:
                        speech_started = True
                        log.info(f"Speech actively detected (vad={vad_prob:.3f}, rms={rms:.5f}).")
                else:
                    consecutive_speech_chunks = 0
                    if speech_started:
                        consecutive_silence_chunks += 1
                        if consecutive_silence_chunks >= self.silence_chunks_limit:
                            log.info(f"Natural pause detected (~{self.silence_timeout:.2f}s). Submitting complete phrase.")
                            break
                    else:
                        # Initial wait limit: if user didn't say anything for wait_timeout seconds after trigger
                        if elapsed >= wait_timeout:
                            log.info(f"No speech detected within {wait_timeout:.1f}s. Ending.")
                            break

                # Safety max duration
                if elapsed >= self.max_duration:
                    log.info(f"Reached max duration ({self.max_duration}s).")
                    break

        self._is_recording = False

        if not recorded_chunks:
            return b""

        # If speech was never detected and user did not push-to-talk release, discard ambient silence
        if not speech_started and not self._stop_requested:
            log.info("No speech detected in audio stream. Discarding.")
            return b""

        full_audio = np.concatenate(recorded_chunks, axis=0)
        int16_audio = np.clip(full_audio * 32767, -32768, 32767).astype(np.int16)

        wav_buffer = io.BytesIO()
        with wave.open(wav_buffer, "wb") as wf:
            wf.setnchannels(1)
            wf.setsampwidth(2)
            wf.setframerate(self.sample_rate)
            wf.writeframes(int16_audio.tobytes())

        return wav_buffer.getvalue()
