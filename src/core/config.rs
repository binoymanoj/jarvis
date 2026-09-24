use crate::core::error::{JarvisError, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info};

// -----------------------------------------------------------------------------
// TOML Configuration Structures
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TomlConfig {
    pub ai: Option<TomlAiConfig>,
    pub wakeword: Option<TomlWakewordConfig>,
    pub editor: Option<TomlEditorConfig>,
    pub audio: Option<TomlAudioConfig>,
    pub media: Option<TomlMediaConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TomlMediaConfig {
    pub media_dirs: Option<Vec<String>>,
    pub player: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TomlAiConfig {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub gemini_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub groq_api_key: Option<String>,
    pub openrouter_api_key: Option<String>,
    pub cli_tool: Option<String>,
    pub typesafe_api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TomlWakewordConfig {
    pub enabled: Option<bool>,
    pub name: Option<String>,
    pub threshold: Option<f32>,
    pub model_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TomlEditorConfig {
    pub default: Option<String>,
    pub terminal: Option<String>,
    pub project_dirs: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TomlAudioConfig {
    pub stt_engine: Option<String>,
    pub whisper_model: Option<String>,
    pub local_whisper_size: Option<String>,
    pub tts_engine: Option<String>,
    pub edge_voice: Option<String>,
    pub piper_voice: Option<String>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    pub silence_threshold_seconds: Option<f32>,
    pub max_recording_seconds: Option<f32>,
    pub initial_listen_timeout: Option<f32>,
    pub followup_listen_timeout: Option<f32>,
    pub duck_media: Option<bool>,
    pub duck_mode: Option<String>,
}

// -----------------------------------------------------------------------------
// Merged Runtime Settings
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    // AI Provider & Reasoning
    pub ai_provider: String, // "gemini", "openai", "anthropic", "groq", "openrouter"
    pub model_name: String,

    // API Keys
    pub gemini_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub groq_api_key: Option<String>,
    pub openrouter_api_key: Option<String>,
    pub typesafe_api_key: Option<String>,

    // CLI Coding AI Tool ("claude", "codex", "agy")
    pub cli_ai_tool: String,

    // Editor & Code Paths
    pub editor: String,
    pub terminal: String,
    pub project_dirs: Vec<String>,

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
    pub duck_media: bool,
    pub duck_mode: String,

    // Wake Word Detection
    pub wakeword_enabled: bool,
    pub wakeword_name: String,
    pub wakeword_model: String,
    pub wakeword_threshold: f32,

    // Media Playback & Video Paths
    pub media_dirs: Vec<String>,
    pub media_player: String,

    // Runtime IPC paths
    pub daemon_socket_path: PathBuf,
}

/// Normalizes a wake word name and determines the appropriate ONNX model filename.
/// Enforces standard wake words follow the "hey <name>" pattern (e.g. "hey jarvis"),
/// preventing standalone names like "jarvis" from being used as the wake word and triggering false activations.
pub fn normalize_wakeword_name(raw: &str) -> (String, String) {
    let clean = raw.trim().to_lowercase().replace(['_', '-'], " ");
    let parts: Vec<&str> = clean.split_whitespace().collect();
    let normalized = parts.join(" ");

    match normalized.as_str() {
        "" | "jarvis" | "hey jarvis" => {
            ("hey jarvis".to_string(), "hey_jarvis_v0.1.onnx".to_string())
        }
        "alexa" => ("alexa".to_string(), "alexa_v0.1.onnx".to_string()),
        "mycroft" | "hey mycroft" => (
            "hey mycroft".to_string(),
            "hey_mycroft_v0.1.onnx".to_string(),
        ),
        "rhasspy" | "hey rhasspy" => (
            "hey rhasspy".to_string(),
            "hey_rhasspy_v0.1.onnx".to_string(),
        ),
        custom => {
            let name = if custom.starts_with("hey ") {
                custom.to_string()
            } else {
                format!("hey {}", custom)
            };
            let model_stem = name.replace(' ', "_");
            (name, format!("{}.onnx", model_stem))
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        let username = env::var("USER").unwrap_or_else(|_| "user".to_string());
        Self {
            ai_provider: "gemini".to_string(),
            model_name: "gemini-2.5-flash".to_string(),
            gemini_api_key: None,
            openai_api_key: None,
            anthropic_api_key: None,
            groq_api_key: None,
            openrouter_api_key: None,
            typesafe_api_key: None,
            cli_ai_tool: "agy".to_string(),
            editor: "nvim".to_string(),
            terminal: "kitty".to_string(),
            project_dirs: vec![
                "~/Codes/personal".to_string(),
                "~/Codes".to_string(),
                "~/Projects".to_string(),
                "~".to_string(),
            ],
            stt_engine: "groq".to_string(),
            whisper_model: "whisper-large-v3-turbo".to_string(),
            local_whisper_size: "base.en".to_string(),
            tts_engine: "edge".to_string(),
            edge_voice: "en-GB-RyanNeural".to_string(),
            piper_voice: "en_GB-alan-medium".to_string(),
            sample_rate: 16000,
            channels: 1,
            silence_threshold_seconds: 0.85,
            max_recording_seconds: 25.0,
            initial_listen_timeout: 10.0,
            followup_listen_timeout: 8.0,
            duck_media: true,
            duck_mode: "mute".to_string(),
            wakeword_enabled: true,
            wakeword_name: "hey jarvis".to_string(),
            wakeword_model: "hey_jarvis_v0.1.onnx".to_string(),
            wakeword_threshold: 0.50,
            daemon_socket_path: PathBuf::from(format!("/tmp/jarvis-{}.sock", username)),
            media_dirs: vec![
                "~/Videos".to_string(),
                "~/Movies".to_string(),
                "~/Downloads".to_string(),
                "~".to_string(),
            ],
            media_player: "mpv".to_string(),
        }
    }
}

pub const DEFAULT_CONFIG_TEMPLATE: &str = r#"# ==============================================================================
# Jarvis Configuration File (~/.config/jarvis/config.toml)
# ==============================================================================

[ai]
# AI Provider: "gemini" (free default), "openai", "anthropic" (claude), "groq", "openrouter"
provider = "gemini"

# Model name for chosen provider:
# - Gemini: "gemini-2.5-flash", "gemini-3.5-flash-lite", "gemini-3.1-flash-lite"
# - OpenAI: "gpt-4o", "gpt-4o-mini"
# - Anthropic: "claude-3-7-sonnet-latest", "claude-3-5-haiku-latest"
# - Groq: "llama-3.3-70b-versatile"
# - OpenRouter: "anthropic/claude-3.7-sonnet", "deepseek/deepseek-r1"
model = "gemini-2.5-flash"

# Provider API Keys (Leave blank if already defined in environment or .env)
gemini_api_key = ""
openai_api_key = ""
anthropic_api_key = ""
groq_api_key = ""
openrouter_api_key = ""

# TypeSafe AI Jev (Fast-Path 70ms intent routing and type-safe decisions)
typesafe_api_key = ""

# Autonomous coding CLI assistant ("claude", "codex", "agy")
cli_tool = "agy"

[wakeword]
# Background wake word detection
enabled = true

# Wake word name: "hey jarvis" (default), "hey <name>", "alexa", "hey_mycroft", "hey_rhasspy"
name = "hey jarvis"

# Sensitivity threshold (0.30 - 0.70). Default: 0.50 (requires clear 'Hey Jarvis', preventing accidental triggers from 'Jarvis' or ambient speech)
threshold = 0.50

# Optional custom ONNX model path (relative to models dir or absolute)
# model_path = "models/hey_jarvis_v0.1.onnx"

[editor]
# Code editor to open project files with ("nvim", "neovim", "code", "helix")
default = "nvim"

# Terminal emulator used to host the editor ("kitty", "foot")
terminal = "kitty"

# Project search directories (searched when asking e.g. "open models.go from tracky-researcher-tui")
project_dirs = ["~/Codes/personal", "~/Codes", "~/Projects", "~"]

[media]
# Directories searched when asking to play movies, TV series, or media files
# Supports tilde expansion (e.g. ~/Videos, ~/Movies, ~/Downloads)
media_dirs = ["~/Videos", "~/Movies", "~/Downloads"]

# Preferred media player executable ("mpv", "vlc")
player = "mpv"

[audio]
# Speech-to-Text engine ("groq" or "local")
stt_engine = "groq"
whisper_model = "whisper-large-v3-turbo"

# Text-to-Speech engine: "edge" (pure Rust WebSocket, zero latency) or "piper" (offline C++)
tts_engine = "edge"
edge_voice = "en-GB-RyanNeural"
piper_voice = "en_GB-alan-medium"

# VAD and Silence detection
silence_threshold_seconds = 0.85
initial_listen_timeout = 10.0
followup_listen_timeout = 8.0
max_recording_seconds = 25.0

# Automatic audio ducking during voice capture (prevents speaker acoustic bleed into microphone)
# duck_media: true to automatically duck/mute speakers while you are speaking
# duck_mode: "mute" (zero bleed, instant silence detection) or "lower" (reduces volume to 15%)
duck_media = true
duck_mode = "mute"
"#;

impl Settings {
    /// Resolves configured media search directories, expanding ~ to user's home path
    pub fn resolved_media_dirs(&self) -> Vec<PathBuf> {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let home_path = PathBuf::from(&home);

        if self.media_dirs.is_empty() {
            vec![
                home_path.join("Videos"),
                home_path.join("Movies"),
                home_path.join("Downloads"),
                home_path,
            ]
        } else {
            self.media_dirs
                .iter()
                .map(|p| {
                    if let Some(stripped) = p.strip_prefix("~/") {
                        home_path.join(stripped)
                    } else if p == "~" {
                        home_path.clone()
                    } else {
                        PathBuf::from(p)
                    }
                })
                .collect()
        }
    }

    /// Load settings by parsing config.toml (with fallback template creation) and merging with .env
    pub fn load() -> Result<Self> {
        let config_dir = dirs_hint();
        let primary_toml_path = config_dir.join("config.toml");

        // Try locating config.toml in config dir, current working dir, or repo root
        let toml_candidates = [
            primary_toml_path.clone(),
            PathBuf::from("config.toml"),
        ];

        let mut toml_config = TomlConfig::default();
        let mut loaded_toml_path = None;

        for candidate in &toml_candidates {
            if candidate.is_file() {
                if let Ok(content) = fs::read_to_string(candidate) {
                    if let Ok(parsed) = toml::from_str::<TomlConfig>(&content) {
                        debug!("Loaded config.toml from {:?}", candidate);
                        toml_config = parsed;
                        loaded_toml_path = Some(candidate.clone());
                        break;
                    }
                }
            }
        }

        // If no config.toml exists at all, initialize default in ~/.config/jarvis/config.toml
        if loaded_toml_path.is_none() {
            if let Some(parent) = primary_toml_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if fs::write(&primary_toml_path, DEFAULT_CONFIG_TEMPLATE).is_ok() {
                info!("Initialized default config file at {:?}", primary_toml_path);
            }
        }

        // Try locating .env file
        let env_candidates = [
            PathBuf::from(".env"),
            PathBuf::from("../.env"),
            config_dir.join(".env"),
        ];

        for candidate in &env_candidates {
            if candidate.is_file() {
                let _ = dotenvy::from_path(candidate);
                break;
            }
        }
        let _ = dotenvy::dotenv();

        let mut s = Settings::default();

        // 1. First load from environment variables and .env
        if let Ok(val) = env::var("GEMINI_API_KEY") {
            if !val.trim().is_empty() {
                s.gemini_api_key = Some(val.trim().to_string());
            }
        }
        if let Ok(val) = env::var("OPENAI_API_KEY") {
            if !val.trim().is_empty() {
                s.openai_api_key = Some(val.trim().to_string());
            }
        }
        if let Ok(val) = env::var("ANTHROPIC_API_KEY") {
            if !val.trim().is_empty() {
                s.anthropic_api_key = Some(val.trim().to_string());
            }
        }
        if let Ok(val) = env::var("GROQ_API_KEY") {
            if !val.trim().is_empty() {
                s.groq_api_key = Some(val.trim().to_string());
            }
        }
        if let Ok(val) = env::var("OPENROUTER_API_KEY") {
            if !val.trim().is_empty() {
                s.openrouter_api_key = Some(val.trim().to_string());
            }
        }
        if let Ok(val) = env::var("TYPESAFE_API_KEY") {
            if !val.trim().is_empty() {
                s.typesafe_api_key = Some(val.trim().to_string());
            }
        }

        if let Ok(val) = env::var("JARVIS_AI_PROVIDER") {
            s.ai_provider = val.to_lowercase().trim().to_string();
        }
        if let Ok(val) = env::var("JARVIS_MODEL") {
            s.model_name = val;
        }
        if let Ok(val) = env::var("JARVIS_CLI_AI_TOOL") {
            s.cli_ai_tool = val;
        }
        if let Ok(val) = env::var("JARVIS_EDITOR") {
            s.editor = val;
        }
        if let Ok(val) = env::var("JARVIS_TERMINAL") {
            s.terminal = val;
        }
        if let Ok(val) = env::var("JARVIS_STT_ENGINE") {
            s.stt_engine = val;
        }
        if let Ok(val) = env::var("JARVIS_TTS_ENGINE") {
            s.tts_engine = val;
        }
        if let Ok(val) = env::var("JARVIS_DUCK_MEDIA") {
            s.duck_media = val.to_lowercase() == "true" || val == "1";
        }
        if let Ok(val) = env::var("JARVIS_DUCK_MODE") {
            s.duck_mode = val;
        }
        if let Ok(val) = env::var("JARVIS_WAKEWORD_ENABLED") {
            s.wakeword_enabled = val.to_lowercase() == "true" || val == "1";
        }
        if let Ok(val) = env::var("JARVIS_WAKEWORD_NAME").or_else(|_| env::var("JARVIS_WAKEWORD")) {
            if !val.trim().is_empty() {
                let (name, model) = normalize_wakeword_name(&val);
                s.wakeword_name = name;
                s.wakeword_model = model;
            }
        }
        if let Ok(val) = env::var("JARVIS_WAKEWORD_THRESHOLD") {
            if let Ok(parsed) = val.parse::<f32>() {
                if (parsed - 0.22).abs() < 0.001 {
                    s.wakeword_threshold = 0.50;
                } else {
                    s.wakeword_threshold = parsed;
                }
            }
        }
        if let Ok(val) = env::var("JARVIS_MEDIA_DIRS") {
            let dirs: Vec<String> = val
                .split([',', ':'])
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect();
            if !dirs.is_empty() {
                s.media_dirs = dirs;
            }
        }
        if let Ok(val) = env::var("JARVIS_MEDIA_PLAYER") {
            if !val.trim().is_empty() {
                s.media_player = val.trim().to_string();
            }
        }

        // 2. Now apply config.toml values (config.toml takes precedence!)
        if let Some(ai) = toml_config.ai {
            if let Some(p) = ai.provider {
                if !p.trim().is_empty() {
                    s.ai_provider = p.to_lowercase().trim().to_string();
                }
            }
            if let Some(m) = ai.model {
                if !m.trim().is_empty() {
                    s.model_name = m.trim().to_string();
                }
            }
            if let Some(k) = ai.gemini_api_key {
                if !k.trim().is_empty() {
                    s.gemini_api_key = Some(k.trim().to_string());
                }
            }
            if let Some(k) = ai.openai_api_key {
                if !k.trim().is_empty() {
                    s.openai_api_key = Some(k.trim().to_string());
                }
            }
            if let Some(k) = ai.anthropic_api_key {
                if !k.trim().is_empty() {
                    s.anthropic_api_key = Some(k.trim().to_string());
                }
            }
            if let Some(k) = ai.groq_api_key {
                if !k.trim().is_empty() {
                    s.groq_api_key = Some(k.trim().to_string());
                }
            }
            if let Some(k) = ai.openrouter_api_key {
                if !k.trim().is_empty() {
                    s.openrouter_api_key = Some(k.trim().to_string());
                }
            }
            if let Some(k) = ai.typesafe_api_key {
                if !k.trim().is_empty() {
                    s.typesafe_api_key = Some(k.trim().to_string());
                }
            }
            if let Some(c) = ai.cli_tool {
                if !c.trim().is_empty() {
                    s.cli_ai_tool = c.trim().to_string();
                }
            }
        }

        // Merge Wakeword settings from TOML
        if let Some(ww) = toml_config.wakeword {
            if let Some(en) = ww.enabled {
                s.wakeword_enabled = en;
            }
            if let Some(n) = ww.name {
                if !n.trim().is_empty() {
                    let (name, model) = normalize_wakeword_name(&n);
                    s.wakeword_name = name;
                    s.wakeword_model = model;
                }
            }
            if let Some(th) = ww.threshold {
                if (th - 0.22).abs() < 0.001 {
                    s.wakeword_threshold = 0.50;
                } else {
                    s.wakeword_threshold = th;
                }
            }
            if let Some(mp) = ww.model_path {
                if !mp.trim().is_empty() {
                    s.wakeword_model = mp.trim().to_string();
                }
            }
        }

        // Merge Editor settings from TOML
        if let Some(ed) = toml_config.editor {
            if let Some(d) = ed.default {
                if !d.trim().is_empty() {
                    s.editor = d.trim().to_string();
                }
            }
            if let Some(t) = ed.terminal {
                if !t.trim().is_empty() {
                    s.terminal = t.trim().to_string();
                }
            }
            if let Some(dirs) = ed.project_dirs {
                if !dirs.is_empty() {
                    s.project_dirs = dirs;
                }
            }
        }

        // Merge Audio settings from TOML
        if let Some(aud) = toml_config.audio {
            if let Some(stt) = aud.stt_engine {
                s.stt_engine = stt;
            }
            if let Some(wm) = aud.whisper_model {
                s.whisper_model = wm;
            }
            if let Some(lws) = aud.local_whisper_size {
                s.local_whisper_size = lws;
            }
            if let Some(tts) = aud.tts_engine {
                s.tts_engine = tts;
            }
            if let Some(ev) = aud.edge_voice {
                s.edge_voice = ev;
            }
            if let Some(pv) = aud.piper_voice {
                s.piper_voice = pv;
            }
            if let Some(sr) = aud.sample_rate {
                s.sample_rate = sr;
            }
            if let Some(ch) = aud.channels {
                s.channels = ch;
            }
            if let Some(st) = aud.silence_threshold_seconds {
                s.silence_threshold_seconds = st;
            }
            if let Some(mr) = aud.max_recording_seconds {
                s.max_recording_seconds = mr;
            }
            if let Some(ilt) = aud.initial_listen_timeout {
                s.initial_listen_timeout = ilt;
            }
            if let Some(flt) = aud.followup_listen_timeout {
                s.followup_listen_timeout = flt;
            }
            if let Some(dm) = aud.duck_media {
                s.duck_media = dm;
            }
            if let Some(dmode) = aud.duck_mode {
                s.duck_mode = dmode;
            }
        }

        // Merge Media settings from TOML
        if let Some(med) = toml_config.media {
            if let Some(dirs) = med.media_dirs {
                if !dirs.is_empty() {
                    s.media_dirs = dirs;
                }
            }
            if let Some(p) = med.player {
                if !p.trim().is_empty() {
                    s.media_player = p.trim().to_string();
                }
            }
        }

        // Map wakeword name to model file if not explicitly set
        if s.wakeword_model == "hey_jarvis_v0.1.onnx" {
            match s.wakeword_name.as_str() {
                "alexa" => s.wakeword_model = "alexa_v0.1.onnx".to_string(),
                "hey_mycroft" | "mycroft" => s.wakeword_model = "hey_mycroft_v0.1.onnx".to_string(),
                "hey_rhasspy" | "rhasspy" => s.wakeword_model = "hey_rhasspy_v0.1.onnx".to_string(),
                _ => s.wakeword_model = "hey_jarvis_v0.1.onnx".to_string(),
            }
        }

        Ok(s)
    }

    /// Check if API key for currently configured AI provider is present
    pub fn require_active_provider_key(&self) -> Result<&str> {
        match self.ai_provider.as_str() {
            "openai" => self
                .openai_api_key
                .as_deref()
                .ok_or_else(|| JarvisError::EnvVarMissing("OPENAI_API_KEY".to_string())),
            "anthropic" | "claude" => self
                .anthropic_api_key
                .as_deref()
                .ok_or_else(|| JarvisError::EnvVarMissing("ANTHROPIC_API_KEY".to_string())),
            "groq" => self
                .groq_api_key
                .as_deref()
                .ok_or_else(|| JarvisError::EnvVarMissing("GROQ_API_KEY".to_string())),
            "openrouter" => self
                .openrouter_api_key
                .as_deref()
                .ok_or_else(|| JarvisError::EnvVarMissing("OPENROUTER_API_KEY".to_string())),
            _ => self
                .gemini_api_key
                .as_deref()
                .ok_or_else(|| JarvisError::EnvVarMissing("GEMINI_API_KEY".to_string())),
        }
    }

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

pub fn dirs_hint() -> PathBuf {
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
        assert_eq!(settings.ai_provider, "gemini");
        assert_eq!(settings.editor, "nvim");
        assert_eq!(settings.terminal, "kitty");
        assert_eq!(settings.sample_rate, 16000);
        assert_eq!(settings.channels, 1);
        assert_eq!(settings.model_name, "gemini-2.5-flash");
        assert!(settings.wakeword_enabled);
        assert_eq!(settings.wakeword_name, "hey jarvis");
        assert_eq!(settings.wakeword_threshold, 0.50);
        assert!(settings.duck_media);
        assert_eq!(settings.duck_mode, "mute");
    }

    #[test]
    fn test_normalize_wakeword_name() {
        assert_eq!(
            normalize_wakeword_name("jarvis"),
            ("hey jarvis".to_string(), "hey_jarvis_v0.1.onnx".to_string())
        );
        assert_eq!(
            normalize_wakeword_name("hey jarvis"),
            ("hey jarvis".to_string(), "hey_jarvis_v0.1.onnx".to_string())
        );
        assert_eq!(
            normalize_wakeword_name("hey_jarvis"),
            ("hey jarvis".to_string(), "hey_jarvis_v0.1.onnx".to_string())
        );
        assert_eq!(
            normalize_wakeword_name("alexa"),
            ("alexa".to_string(), "alexa_v0.1.onnx".to_string())
        );
        assert_eq!(
            normalize_wakeword_name("mycroft"),
            (
                "hey mycroft".to_string(),
                "hey_mycroft_v0.1.onnx".to_string()
            )
        );
        assert_eq!(
            normalize_wakeword_name("computer"),
            ("hey computer".to_string(), "hey_computer.onnx".to_string())
        );
        assert_eq!(
            normalize_wakeword_name("hey computer"),
            ("hey computer".to_string(), "hey_computer.onnx".to_string())
        );
    }

    #[test]
    fn test_load_settings() {
        let settings = Settings::load().expect("Failed to load settings");
        assert!(settings.sample_rate > 0);
        assert!(!settings.project_dirs.is_empty());
    }

    #[test]
    fn test_parse_toml() {
        let toml_sample = r#"
            [ai]
            provider = "anthropic"
            model = "claude-3-7-sonnet-latest"
            anthropic_api_key = "test-key"

            [wakeword]
            name = "alexa"
            threshold = 0.35

            [editor]
            default = "helix"
            terminal = "foot"
            project_dirs = ["/tmp/test"]

            [media]
            media_dirs = ["/tmp/videos", "/tmp/movies"]
            player = "mpv"
        "#;

        let parsed: TomlConfig = toml::from_str(toml_sample).unwrap();
        assert_eq!(parsed.ai.unwrap().provider.unwrap(), "anthropic");
        assert_eq!(parsed.wakeword.unwrap().name.unwrap(), "alexa");
        assert_eq!(parsed.editor.unwrap().default.unwrap(), "helix");
        assert_eq!(
            parsed.media.as_ref().unwrap().player.as_deref(),
            Some("mpv")
        );
        assert_eq!(parsed.media.unwrap().media_dirs.unwrap().len(), 2);
    }

    #[test]
    fn test_resolved_media_dirs() {
        let settings = Settings {
            media_dirs: vec!["~/Videos".to_string(), "/var/media".to_string()],
            ..Default::default()
        };
        let resolved = settings.resolved_media_dirs();
        assert_eq!(resolved.len(), 2);
        assert!(!resolved[0].to_str().unwrap().contains('~'));
        assert_eq!(resolved[1], PathBuf::from("/var/media"));
    }
}
