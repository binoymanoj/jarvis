use std::path::PathBuf;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, JarvisError>;

#[derive(Error, Debug)]
pub enum JarvisError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Environment variable missing: {0}")]
    EnvVarMissing(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("HTTP API error (status {status}): {body}")]
    ApiError {
        status: reqwest::StatusCode,
        body: String,
    },

    #[error("Quota exceeded (429) for model: {0}")]
    QuotaExceeded(String),

    #[error("Audio error: {0}")]
    Audio(String),

    #[error("Wake word error: {0}")]
    WakeWord(String),

    #[error("VAD error: {0}")]
    Vad(String),

    #[error("TTS error: {0}")]
    Tts(String),

    #[error("Tool execution failed [{tool}]: {message}")]
    ToolExecution { tool: String, message: String },

    #[error("Tool '{0}' not found in registry")]
    ToolNotFound(String),

    #[error("Tool '{0}' parameter error: {1}")]
    ToolParameter(String, String),

    #[error("Command timed out after {0} seconds")]
    Timeout(u64),

    #[error("System state error: {0}")]
    State(String),

    #[error("Process communication error: {0}")]
    Ipc(String),

    #[error("Socket error at {path}: {message}")]
    Socket { path: PathBuf, message: String },

    #[error("Dismissal / exit requested by user: {0}")]
    Exit(String),

    #[error("Operation interrupted: {0}")]
    Interrupted(String),

    #[error("{0}")]
    Other(String),
}
