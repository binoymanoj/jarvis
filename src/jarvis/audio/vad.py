from pathlib import Path
from typing import Optional
import numpy as np
from jarvis.core.logger import log


class SileroVAD:
    """Voice Activity Detection using the verified Silero VAD model via openWakeWord."""

    def __init__(self, model_path: Optional[Path] = None):
        from openwakeword.vad import VAD

        try:
            if model_path and Path(model_path).exists():
                self._vad = VAD(model_path=str(model_path))
            else:
                self._vad = VAD()
            log.debug("Silero VAD initialized successfully.")
        except Exception as e:
            log.error(f"Failed to initialize Silero VAD: {e}")
            raise

    def reset(self) -> None:
        """Reset internal recurrent state."""
        self._vad.reset_states()

    def get_speech_prob(self, chunk: np.ndarray) -> float:
        """Calculate speech probability for an audio chunk (nominally 512 samples at 16kHz).

        Args:
            chunk: 1D or 2D numpy array of audio samples (float32 [-1.0, 1.0] or int16 [-32768, 32767]).

        Returns:
            Probability of speech between 0.0 and 1.0.
        """
        if chunk.ndim > 1:
            chunk = chunk.squeeze()

        # Convert float32 to int16 format expected by VAD
        if chunk.dtype != np.int16:
            chunk = np.clip(chunk * 32767.0, -32768, 32767).astype(np.int16)

        frame_size = len(chunk)
        try:
            prob = float(self._vad.predict(chunk, frame_size=frame_size))
            return prob
        except Exception as e:
            log.debug(f"VAD prediction error: {e}")
            return 0.0

    def is_speech(self, chunk: np.ndarray, threshold: float = 0.35) -> bool:
        """Check if audio chunk contains speech."""
        return self.get_speech_prob(chunk) >= threshold
