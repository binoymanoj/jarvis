use crate::core::error::{JarvisError, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    // API Keys
    pub gemini_api_key: Option<String>,
    pub groq_api_key: Option<String>,

    // Reasoning Model
    pub model_name: String,

    // CLI Coding AI Tool ("claude", "codex", "agy")
    pub cli_ai_tool: String,

    // Speech-to-Text
    pub stt_engine: String,
    pub whisper_model: String,
    pub local_whisper_size: String,

    // Text-to-Speech
    pub tts_engine: String,
    pub edge_voice: String,
    pub piper_voice: String,

    // Audio recording & VAD settings
    pub sample_rate: u32,
    pub channels: u16,
    pub silence_threshold_seconds: f32,
    pub max_recording_seconds: f32,
    pub initial_listen_timeout: f32,
    pub followup_listen_timeout: f32,

    // Wake Word Detection
    pub wakeword_enabled: bool,
    pub wakeword_model: String,
    pub wakeword_threshold: f32,

    // Runtime IPC paths
    pub daemon_socket_path: PathBuf,
}

impl Default for Settings {
    fn default() -> Self {
        let username = env::var("USER").unwrap_or_else(|_| "user".to_string());
        Self {
            gemini_api_key: None,
            groq_api_key: None,
            model_name: "gemini-3.5-flash-lite".to_string(),
            cli_ai_tool: "claude".to_string(),
            stt_engine: "groq".to_string(),
            whisper_model: "whisper-large-v3-turbo".to_string(),
            local_whisper_size: "base.en".to_string(),
            tts_engine: "edge".to_string(),
            edge_voice: "en-GB-RyanNeural".to_string(),
            piper_voice: "en_GB-alan-medium".to_string(),
            sample_rate: 16000,
            channels: 1,
            silence_threshold_seconds: 0.85,
            max_recording_seconds: 20.0,
            initial_listen_timeout: 8.0,
            followup_listen_timeout: 7.0,
            wakeword_enabled: true,
            wakeword_model: "hey_jarvis".to_string(),
            wakeword_threshold: 0.28,
            daemon_socket_path: PathBuf::from(format!("/tmp/jarvis-{}.sock", username)),
        }
    }
}

impl Settings {
    /// Load settings by locating and parsing `.env` file, falling back to environment variables.
    pub fn load() -> Result<Self> {
        // Try locating .env in current working dir, git root, or home config
        let env_candidates = [
            PathBuf::from(".env"),
            PathBuf::from("../.env"),
            PathBuf::from("/home/binoy/Codes/personal/jarvis/.env"),
            dirs_hint().join(".env"),
        ];

        for candidate in &env_candidates {
            if candidate.is_file() {
                let _ = dotenvy::from_path(candidate);
                break;
            }
        }
        let _ = dotenvy::dotenv();

        let mut s = Settings::default();

        if let Ok(val) = env::var("GEMINI_API_KEY") {
            if !val.trim().is_empty() {
                s.gemini_api_key = Some(val.trim().to_string());
            }
        }
        if let Ok(val) = env::var("GROQ_API_KEY") {
            if !val.trim().is_empty() {
                s.groq_api_key = Some(val.trim().to_string());
            }
        }

        if let Ok(val) = env::var("JARVIS_MODEL") {
            s.model_name = val;
        }
        if let Ok(val) = env::var("JARVIS_CLI_AI_TOOL") {
            s.cli_ai_tool = val;
        }
        if let Ok(val) = env::var("JARVIS_STT_ENGINE") {
            s.stt_engine = val;
        }
        if let Ok(val) = env::var("JARVIS_WHISPER_MODEL") {
            s.whisper_model = val;
        }
        if let Ok(val) = env::var("JARVIS_LOCAL_WHISPER_SIZE") {
            s.local_whisper_size = val;
        }
        if let Ok(val) = env::var("JARVIS_TTS_ENGINE") {
            s.tts_engine = val;
        }
        if let Ok(val) = env::var("JARVIS_EDGE_VOICE") {
            s.edge_voice = val;
        }
        if let Ok(val) = env::var("JARVIS_VOICE") {
            s.piper_voice = val;
        }

        if let Ok(val) = env::var("JARVIS_SAMPLE_RATE") {
            if let Ok(parsed) = val.parse::<u32>() {
                s.sample_rate = parsed;
            }
        }
        if let Ok(val) = env::var("JARVIS_CHANNELS") {
            if let Ok(parsed) = val.parse::<u16>() {
                s.channels = parsed;
            }
        }
        if let Ok(val) = env::var("JARVIS_SILENCE_THRESHOLD") {
            if let Ok(parsed) = val.parse::<f32>() {
                s.silence_threshold_seconds = parsed;
            }
        }
        if let Ok(val) = env::var("JARVIS_MAX_RECORDING_SECONDS") {
            if let Ok(parsed) = val.parse::<f32>() {
                s.max_recording_seconds = parsed;
            }
        }
        if let Ok(val) = env::var("JARVIS_INITIAL_LISTEN_TIMEOUT") {
            if let Ok(parsed) = val.parse::<f32>() {
                s.initial_listen_timeout = parsed;
            }
        }
        if let Ok(val) = env::var("JARVIS_FOLLOWUP_LISTEN_TIMEOUT") {
            if let Ok(parsed) = val.parse::<f32>() {
                s.followup_listen_timeout = parsed;
            }
        }

        if let Ok(val) = env::var("JARVIS_WAKEWORD_ENABLED") {
            s.wakeword_enabled = val.to_lowercase() == "true" || val == "1";
        }
        if let Ok(val) = env::var("JARVIS_WAKEWORD_MODEL") {
            s.wakeword_model = val;
        }
        if let Ok(val) = env::var("JARVIS_WAKEWORD_THRESHOLD") {
            if let Ok(parsed) = val.parse::<f32>() {
                s.wakeword_threshold = parsed;
            }
        }

        Ok(s)
    }

    /// Check if required cloud API keys are present
    pub fn require_gemini_key(&self) -> Result<&str> {
        self.gemini_api_key
            .as_deref()
            .ok_or_else(|| JarvisError::EnvVarMissing("GEMINI_API_KEY".to_string()))
    }

    pub fn require_groq_key(&self) -> Result<&str> {
        self.groq_api_key
            .as_deref()
            .ok_or_else(|| JarvisError::EnvVarMissing("GROQ_API_KEY".to_string()))
    }
}

fn dirs_hint() -> PathBuf {
    if let Ok(home) = env::var("HOME") {
        PathBuf::from(home).join(".config").join("jarvis")
    } else {
        PathBuf::from(".")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert_eq!(settings.sample_rate, 16000);
        assert_eq!(settings.channels, 1);
        assert_eq!(settings.model_name, "gemini-3.5-flash-lite");
        assert_eq!(settings.cli_ai_tool, "claude");
        assert_eq!(settings.tts_engine, "edge");
        assert_eq!(settings.edge_voice, "en-GB-RyanNeural");
        assert!(settings.wakeword_enabled);
    }

    #[test]
    fn test_load_settings() {
        let settings = Settings::load().expect("Failed to load settings");
        assert!(settings.sample_rate > 0);
    }
}
