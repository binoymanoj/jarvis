use crate::core::error::{JarvisError, Result};
use base64::prelude::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{debug, error, warn};

// -----------------------------------------------------------------------------
// Gemini API Schema
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentMessage {
    pub role: String,
    pub parts: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInstruction {
    pub parts: Vec<SystemPart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemPart {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationConfig {
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateContentRequest {
    pub contents: Vec<ContentMessage>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "systemInstruction")]
    pub system_instruction: Option<SystemInstruction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "generationConfig")]
    pub generation_config: Option<GenerationConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateContentResponse {
    pub candidates: Option<Vec<Candidate>>,
    #[serde(rename = "promptFeedback")]
    pub prompt_feedback: Option<Value>,
    pub error: Option<GeminiErrorPayload>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Candidate {
    pub content: Option<ContentMessage>,
    #[serde(rename = "finishReason")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GeminiErrorPayload {
    pub code: Option<i64>,
    pub message: Option<String>,
    pub status: Option<String>,
}

// -----------------------------------------------------------------------------
// Gemini Client
// -----------------------------------------------------------------------------

#[derive(Clone)]
pub struct GeminiClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl GeminiClient {
    pub fn new(api_key: &str) -> Self {
        Self {
            client: Client::builder().build().unwrap_or_default(),
            api_key: api_key.trim().to_string(),
            base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
        }
    }

    pub fn with_base_url(api_key: &str, base_url: &str) -> Self {
        Self {
            client: Client::builder().build().unwrap_or_default(),
            api_key: api_key.trim().to_string(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Primary call to generateContent endpoint with automatic quota/rate-limit detection
    pub async fn generate_content(
        &self,
        model: &str,
        request: &GenerateContentRequest,
    ) -> Result<GenerateContentResponse> {
        let url = format!(
            "{}/models/{}:generateContent?key={}",
            self.base_url, model, self.api_key
        );

        debug!("Sending Gemini generateContent request to model: {}", model);

        let response = self
            .client
            .post(&url)
            .json(request)
            .send()
            .await?;

        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            warn!("Gemini 429 Too Many Requests on model {model}");
            return Err(JarvisError::QuotaExceeded(model.to_string()));
        }

        if !status.is_success() {
            let lower_body = body_text.to_lowercase();
            if lower_body.contains("resource_exhausted") || lower_body.contains("quota") {
                warn!("Gemini quota exhausted on model {model}: {body_text}");
                return Err(JarvisError::QuotaExceeded(model.to_string()));
            }

            error!("Gemini API error (HTTP {status}): {body_text}");
            return Err(JarvisError::ApiError {
                status,
                body: body_text,
            });
        }

        let parsed: GenerateContentResponse = serde_json::from_str(&body_text).map_err(|e| {
            JarvisError::Serialization(e)
        })?;

        if let Some(err) = &parsed.error {
            let status_code = err.status.as_deref().unwrap_or("");
            if status_code == "RESOURCE_EXHAUSTED" {
                return Err(JarvisError::QuotaExceeded(model.to_string()));
            }
            return Err(JarvisError::Other(format!(
                "Gemini error: {}",
                err.message.as_deref().unwrap_or("Unknown API error")
            )));
        }

        Ok(parsed)
    }

    /// Vision analysis: sends PNG image bytes and prompt to Gemini
    pub async fn analyze_image(
        &self,
        model: &str,
        image_bytes: &[u8],
        prompt: &str,
    ) -> Result<String> {
        let base64_image = BASE64_STANDARD.encode(image_bytes);
        let request = GenerateContentRequest {
            contents: vec![ContentMessage {
                role: "user".to_string(),
                parts: vec![
                    json!({
                        "inlineData": {
                            "mimeType": "image/png",
                            "data": base64_image
                        }
                    }),
                    json!({
                        "text": format!(
                            "The user asked: '{prompt}'. Based on this screenshot, give a direct, concise diagnosis or answer in 1 to 2 sentences."
                        )
                    }),
                ],
            }],
            system_instruction: None,
            tools: None,
            generation_config: Some(GenerationConfig { temperature: 0.2 }),
        };

        let response = self.generate_content(model, &request).await?;
        if let Some(candidates) = response.candidates {
            if let Some(candidate) = candidates.first() {
                if let Some(content) = &candidate.content {
                    for part in &content.parts {
                        if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                            return Ok(text.trim().to_string());
                        }
                    }
                }
            }
        }

        Ok("Screen analyzed successfully, sir.".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_serialization() {
        let req = GenerateContentRequest {
            contents: vec![ContentMessage {
                role: "user".to_string(),
                parts: vec![json!({"text": "Hello"})],
            }],
            system_instruction: Some(SystemInstruction {
                parts: vec![SystemPart {
                    text: "System prompt".to_string(),
                }],
            }),
            tools: Some(json!([{"functionDeclarations": []}])),
            generation_config: Some(GenerationConfig { temperature: 0.2 }),
        };

        let serialized = serde_json::to_string(&req).unwrap();
        assert!(serialized.contains("systemInstruction"));
        assert!(serialized.contains("functionDeclarations"));
    }
}
