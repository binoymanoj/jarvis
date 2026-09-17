use std::path::PathBuf;

pub mod capture;
pub mod playback;
pub mod recorder;
pub mod tts;
pub mod vad;
pub mod wakeword;

pub use capture::AudioCapture;
pub use playback::{ensure_chime_file, play_audio_file, play_wake_chime};
pub use recorder::AudioRecorder;
pub use tts::TextToSpeech;
pub use vad::SileroVAD;
pub use wakeword::WakeWordDetector;

pub fn resolve_models_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let candidates = [
        PathBuf::from(&home).join(".config/jarvis/models"),
        PathBuf::from("/home/binoy/Codes/personal/jarvis/src/jarvis/audio/models"),
        PathBuf::from("src/jarvis/audio/models"),
        PathBuf::from("/usr/share/jarvis/models"),
    ];

    for candidate in &candidates {
        if candidate.join("silero_vad.onnx").is_file() {
            return candidate.clone();
        }
    }
    candidates[0].clone()
}
