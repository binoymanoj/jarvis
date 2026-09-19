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
    let home_path = PathBuf::from(&home);

    let candidates = [
        home_path.join(".local/share/jarvis/models"),
        home_path.join(".config/jarvis/models"),
        PathBuf::from("models"),
        PathBuf::from("/usr/share/jarvis/models"),
        PathBuf::from("/usr/local/share/jarvis/models"),
    ];

    for candidate in &candidates {
        if candidate.join("silero_vad.onnx").is_file() {
            return candidate.clone();
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let exe_models = parent.join("models");
            if exe_models.join("silero_vad.onnx").is_file() {
                return exe_models;
            }
            if let Some(grandparent) = parent.parent() {
                let share_models = grandparent.join("share/jarvis/models");
                if share_models.join("silero_vad.onnx").is_file() {
                    return share_models;
                }
            }
        }
    }

    candidates[0].clone()
}
