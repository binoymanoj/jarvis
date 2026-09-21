use crate::core::config::Settings;
use crate::core::error::{JarvisError, Result};
use reqwest::multipart::{Form, Part};
use reqwest::Client;
use tracing::{debug, error, info, warn};

pub struct SpeechToText {
    client: Client,
    api_key: String,
    model: String,
}

impl SpeechToText {
    pub fn new(api_key: Option<String>, model: Option<String>) -> Self {
        let key = api_key.unwrap_or_default().trim().to_string();
        let model_name = model.unwrap_or_else(|| "whisper-large-v3-turbo".to_string());
        let client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap_or_default();
        Self {
            client,
            api_key: key,
            model: model_name,
        }
    }

    pub fn from_settings(settings: &Settings) -> Self {
        Self::new(
            settings.groq_api_key.clone(),
            Some(settings.whisper_model.clone()),
        )
    }

    /// Transcribe 16-bit mono WAV audio bytes to text via Groq Whisper API
    pub async fn transcribe(&self, wav_bytes: &[u8]) -> Result<String> {
        if wav_bytes.is_empty() {
            return Ok(String::new());
        }

        if self.api_key.is_empty() {
            warn!("GROQ_API_KEY not configured. Cloud STT transcription skipped.");
            return Ok(String::new());
        }

        info!(
            "Transcribing {} bytes of audio via Groq ({})",
            wav_bytes.len(),
            self.model
        );

        let file_part = Part::bytes(wav_bytes.to_vec())
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| JarvisError::Audio(format!("Failed to build WAV multipart part: {e}")))?;

        let form = Form::new()
            .part("file", file_part)
            .text("model", self.model.clone())
            .text("response_format", "text")
            .text("language", "en")
            .text("temperature", "0.0");

        let response = self
            .client
            .post("https://api.groq.com/openai/v1/audio/transcriptions")
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            error!(
                "Groq STT transcription failed with status {}: {}",
                status, body
            );
            return Err(JarvisError::ApiError { status, body });
        }

        let transcription = response.text().await?.trim().to_string();
        debug!("Transcription completed: \"{}\"", transcription);
        Ok(transcription)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stt_instantiation() {
        let stt = SpeechToText::new(Some("test_key".to_string()), None);
        assert_eq!(stt.model, "whisper-large-v3-turbo");
        assert_eq!(stt.api_key, "test_key");
    }

    #[tokio::test]
    async fn test_stt_empty_bytes() {
        let stt = SpeechToText::new(Some("test_key".to_string()), None);
        let result = stt.transcribe(&[]).await.unwrap();
        assert_eq!(result, "");
    }

    #[tokio::test]
    async fn test_stt_empty_key() {
        let stt = SpeechToText::new(None, None);
        let result = stt.transcribe(&[1, 2, 3]).await.unwrap();
        assert_eq!(result, "");
    }
}
