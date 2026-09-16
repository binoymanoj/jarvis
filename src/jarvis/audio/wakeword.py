import asyncio
import os
from pathlib import Path
import time
from typing import Callable, Coroutine, List, Optional
import numpy as np
import sounddevice as sd
from jarvis.core.config import settings
from jarvis.core.logger import log

try:
    import openwakeword
    from openwakeword.model import Model as OWWModel
    _OWW_AVAILABLE = True
except ImportError:
    _OWW_AVAILABLE = False


def play_wake_chime() -> None:
    """Play a short, subtle, pleasant two-tone acknowledgement chime."""
    try:
        sr = 24000
        t1 = np.linspace(0, 0.06, int(sr * 0.06), endpoint=False)
        t2 = np.linspace(0, 0.08, int(sr * 0.08), endpoint=False)
        fade1 = np.sin(np.pi * np.linspace(0, 1, len(t1)))
        fade2 = np.sin(np.pi * np.linspace(0, 1, len(t2)))
        tone1 = (np.sin(2 * np.pi * 587.33 * t1) * 0.12 * fade1).astype(np.float32)  # D5
        tone2 = (np.sin(2 * np.pi * 880.00 * t2) * 0.16 * fade2).astype(np.float32)  # A5
        chime = np.concatenate([tone1, tone2])
        sd.play(chime, samplerate=sr)
    except Exception as e:
        log.debug(f"Wake chime playback skipped: {e}")


class WakeWordDetector:
    """Local offline wake word detector powered by openWakeWord."""

    def __init__(
        self,
        model_name: str = "hey_jarvis",
        threshold: Optional[float] = None,
    ):
        self.model_name = model_name or settings.wakeword_model
        self.threshold = threshold if threshold is not None else settings.wakeword_threshold
        self.is_enabled = settings.wakeword_enabled
        self.model: Optional[OWWModel] = None
        self._model_key: Optional[str] = None
        self._last_trigger_time = 0.0
        self._cooldown_seconds = 1.5

        if _OWW_AVAILABLE and self.is_enabled:
            self._init_model()

    def _init_model(self) -> None:
        """Initialize the ONNX wake word model."""
        try:
            available_paths = openwakeword.get_pretrained_model_paths()
            # Match specifically for hey_jarvis
            matched = [p for p in available_paths if self.model_name in Path(p).name]
            if not matched:
                matched = [p for p in available_paths if "jarvis" in Path(p).name]
            if not matched:
                matched = available_paths[:1]

            if matched:
                self.model = OWWModel(wakeword_model_paths=matched)
                keys = list(self.model.models.keys())
                self._model_key = keys[0] if keys else None
                log.info(f"Wake word model initialized: {self._model_key} (threshold: {self.threshold})")
            else:
                log.warning("No pre-trained wake word models found in openwakeword.")
        except Exception as e:
            log.error(f"Failed to initialize wake word model: {e}")
            self.model = None

    def process_chunk(self, chunk: np.ndarray) -> tuple[bool, float]:
        """Process a 1280-sample 16kHz chunk and return (is_detected, score)."""
        if not self.model or not self._model_key:
            return False, 0.0

        # Ensure correct shape and int16 dtype
        if chunk.ndim == 2:
            chunk = chunk[:, 0]
        if chunk.dtype != np.int16:
            chunk = np.clip(chunk * 32767.0, -32768, 32767).astype(np.int16)

        # Soft AGC: If audio has speech-like energy but low gain (e.g. Bluetooth headset mic),
        # apply soft scaling to bring mel features into nominal range
        rms = float(np.sqrt(np.mean(chunk.astype(np.float32)**2)))
        if 40.0 < rms < 3500.0:
            gain = min(3.5, 5500.0 / max(rms, 80.0))
            chunk = np.clip(chunk.astype(np.float32) * gain, -32768, 32767).astype(np.int16)

        try:
            predictions = self.model.predict(chunk)
            score = float(predictions.get(self._model_key, 0.0))
            
            now = time.monotonic()
            if score >= self.threshold and (now - self._last_trigger_time) > self._cooldown_seconds:
                self._last_trigger_time = now
                self.reset()
                return True, score

            return False, score
        except Exception as e:
            log.debug(f"Wake word inference error: {e}")
            return False, 0.0

    def reset(self) -> None:
        """Reset internal prediction buffers and preprocessor state thoroughly."""
        if self.model:
            try:
                self.model.reset()
                # Deep reset of preprocessor state to avoid audio buffer drift
                if hasattr(self.model, "preprocessor") and self.model.preprocessor:
                    prep = self.model.preprocessor
                    if hasattr(prep, "raw_data_buffer"):
                        prep.raw_data_buffer.clear()
                    if hasattr(prep, "accumulated_samples"):
                        prep.accumulated_samples = 0
                    if hasattr(prep, "melspectrogram_buffer"):
                        prep.melspectrogram_buffer = np.ones((76, 32))
                    if hasattr(prep, "feature_buffer") and hasattr(prep, "_get_embeddings"):
                        prep.feature_buffer = prep._get_embeddings(np.zeros(160000).astype(np.int16))
            except Exception:
                pass

    async def listen_loop(
        self,
        on_detected: Callable[[], Coroutine],
        stop_event: asyncio.Event,
        pause_event: Optional[asyncio.Event] = None,
        trigger_event: Optional[asyncio.Event] = None,
    ) -> None:
        """Asynchronously listen to the microphone stream for the wake word."""
        if not self.model or not self._model_key:
            log.warning("Wake word model not loaded. Wake word listening is inactive.")
            return

        loop = asyncio.get_running_loop()
        audio_queue: asyncio.Queue[np.ndarray] = asyncio.Queue(maxsize=50)

        def audio_callback(indata, frames, time_info, status):
            if pause_event and pause_event.is_set():
                return
            try:
                loop.call_soon_threadsafe(audio_queue.put_nowait, indata.copy())
            except asyncio.QueueFull:
                pass

        blocksize = 1280  # 80ms at 16000Hz
        log.info("Starting background wake word listening ('Hey Jarvis')...")

        stream = sd.InputStream(
            samplerate=16000,
            channels=1,
            dtype="int16",
            blocksize=blocksize,
            callback=audio_callback,
        )

        try:
            stream.start()
            while not stop_event.is_set():
                # Check manual trigger event (from hotkey or menubar)
                manual_triggered = False
                if trigger_event and trigger_event.is_set():
                    trigger_event.clear()
                    manual_triggered = True

                if manual_triggered:
                    log.info("[bold cyan]󰚩 External trigger received! Activating voice assistant...[/bold cyan]")
                    play_wake_chime()
                    if pause_event:
                        pause_event.set()
                    try:
                        stream.stop()
                    except Exception:
                        pass
                    try:
                        await on_detected()
                    finally:
                        while not audio_queue.empty():
                            try:
                                audio_queue.get_nowait()
                            except asyncio.QueueEmpty:
                                break
                        if pause_event:
                            pause_event.clear()
                        self.reset()
                        try:
                            stream.start()
                        except Exception:
                            pass
                    continue

                try:
                    chunk = await asyncio.wait_for(audio_queue.get(), timeout=0.15)
                except asyncio.TimeoutError:
                    continue

                if pause_event and pause_event.is_set():
                    continue

                detected, score = self.process_chunk(chunk)
                if detected:
                    log.info(f"[bold cyan]󰚩 Wake word 'Hey Jarvis' detected![/bold cyan] (score: {score:.2f})")
                    play_wake_chime()
                    if pause_event:
                        pause_event.set()
                    try:
                        # Stop stream while recording voice turn to avoid device contention
                        stream.stop()
                    except Exception:
                        pass
                    try:
                        await on_detected()
                    finally:
                        while not audio_queue.empty():
                            try:
                                audio_queue.get_nowait()
                            except asyncio.QueueEmpty:
                                break
                        if pause_event:
                            pause_event.clear()
                        self.reset()
                        try:
                            stream.start()
                        except Exception:
                            pass

        except Exception as e:
            log.error(f"Error in wake word listener stream: {e}")
        finally:
            try:
                stream.stop()
                stream.close()
            except Exception:
                pass
