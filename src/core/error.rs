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

impl JarvisError {
    /// Returns true if the error indicates a model capacity, quota, unavailability, or transient condition that warrants automatic failover to the next fallback model.
    pub fn is_model_failover(&self) -> bool {
        match self {
            JarvisError::QuotaExceeded(_) => true,
            JarvisError::ApiError { status, body } => {
                let code = status.as_u16();
                code == 429 // Too Many Requests / Quota limit
                    || code == 503 // Service Unavailable / High demand spike
                    || code == 502 // Bad Gateway
                    || code == 504 // Gateway Timeout
                    || code == 500 // Internal Server Error
                    || code == 404 // Model retired / not found
                    || {
                        let lower = body.to_lowercase();
                        lower.contains("high demand")
                            || lower.contains("unavailable")
                            || lower.contains("resource_exhausted")
                            || lower.contains("quota")
                            || lower.contains("rate limit")
                            || lower.contains("overloaded")
                            || lower.contains("capacity")
                            || lower.contains("no longer available")
                            || lower.contains("temporarily")
                            || lower.contains("not found")
                    }
            }
            JarvisError::Http(e) => e.is_timeout() || e.is_connect(),
            _ => false,
        }
    }

    /// Clean, British assistant voice-friendly error message, completely avoiding raw JSON or HTTP traces in TTS speech.
    pub fn user_friendly_message(&self) -> &'static str {
        if self.is_model_failover() {
            "The AI service is temporarily experiencing high demand, sir. Please try again in a moment."
        } else {
            match self {
                JarvisError::Timeout(_) => "The operation timed out, sir.",
                JarvisError::Audio(_) | JarvisError::Vad(_) => "I had trouble capturing your audio, sir.",
                JarvisError::Socket { .. } => "I could not communicate with the desktop compositor, sir.",
                _ => "I encountered an issue communicating with the AI service, sir.",
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::StatusCode;

    #[test]
    fn test_is_model_failover_gemini_503_high_demand() {
        let err = JarvisError::ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            body: r#"{
  "error": {
    "code": 503,
    "message": "This model is currently experiencing high demand. Spikes in demand are usually temporary. Please try again later.",
    "status": "UNAVAILABLE"
  }
}"#.to_string(),
        };

        assert!(err.is_model_failover());
        assert_eq!(
            err.user_friendly_message(),
            "The AI service is temporarily experiencing high demand, sir. Please try again in a moment."
        );
    }

    #[test]
    fn test_is_model_failover_quota_and_codes() {
        let err_quota = JarvisError::QuotaExceeded("gemini-3.5-flash-lite".to_string());
        assert!(err_quota.is_model_failover());

        let err_429 = JarvisError::ApiError {
            status: StatusCode::TOO_MANY_REQUESTS,
            body: "Resource exhausted".to_string(),
        };
        assert!(err_429.is_model_failover());

        let err_404 = JarvisError::ApiError {
            status: StatusCode::NOT_FOUND,
            body: "This model is no longer available to new users".to_string(),
        };
        assert!(err_404.is_model_failover());

        let err_500 = JarvisError::ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            body: "Internal error".to_string(),
        };
        assert!(err_500.is_model_failover());

        // A standard 400 Bad Request should not trigger model failover
        let err_400 = JarvisError::ApiError {
            status: StatusCode::BAD_REQUEST,
            body: "Invalid argument".to_string(),
        };
        assert!(!err_400.is_model_failover());
        assert_eq!(
            err_400.user_friendly_message(),
            "I encountered an issue communicating with the AI service, sir."
        );
    }
}
