use crate::ai::gemini::GeminiClient;
use crate::core::config::Settings;
use crate::core::error::{JarvisError, Result};
use crate::tools::ToolRegistry;
use reqwest::Client;
use serde_json::{json, Value};
use tracing::{debug, error, warn};

#[derive(Debug, Clone)]
pub struct AIToolCall {
    pub id: String,
    pub name: String,
    pub args: Value,
}

#[derive(Debug, Clone)]
pub struct AIResponse {
    pub text: Option<String>,
    pub tool_calls: Vec<AIToolCall>,
}

// -----------------------------------------------------------------------------
// OpenAI-Compatible REST Client (OpenAI, Groq, OpenRouter)
// -----------------------------------------------------------------------------

#[derive(Clone)]
pub struct OpenAICompatibleClient {
    client: Client,
    api_key: String,
    base_url: String,
    provider_name: String,
}

impl OpenAICompatibleClient {
    pub fn new(provider_name: &str, api_key: &str, base_url: &str) -> Self {
        let client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();
        Self {
            client,
            api_key: api_key.trim().to_string(),
            base_url: base_url.trim_end_matches('/').to_string(),
            provider_name: provider_name.to_string(),
        }
    }

    pub async fn chat_completion(
        &self,
        model: &str,
        messages: &[Value],
        tools: Option<Value>,
    ) -> Result<AIResponse> {
        let url = format!("{}/chat/completions", self.base_url);

        let mut body = json!({
            "model": model,
            "messages": messages,
            "temperature": 0.2,
        });

        if let Some(t) = tools {
            if let Some(arr) = t.as_array() {
                if !arr.is_empty() {
                    body["tools"] = t;
                }
            }
        }

        debug!(
            "Sending {} chat completion request to model: {}",
            self.provider_name, model
        );

        let mut req = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json");

        if self.provider_name == "openrouter" {
            req = req
                .header("HTTP-Referer", "https://github.com/binoymanoj/jarvis")
                .header("X-Title", "Jarvis Desktop Assistant");
        }

        let resp = req.json(&body).send().await?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            warn!(
                "{} 429 Too Many Requests on model {model}",
                self.provider_name
            );
            return Err(JarvisError::QuotaExceeded(model.to_string()));
        }

        if !status.is_success() {
            let lower = text.to_lowercase();
            if lower.contains("rate limit") || lower.contains("quota") {
                return Err(JarvisError::QuotaExceeded(model.to_string()));
            }
            error!("{} API error (HTTP {status}): {text}", self.provider_name);
            return Err(JarvisError::ApiError { status, body: text });
        }

        let val: Value = serde_json::from_str(&text).map_err(JarvisError::Serialization)?;

        let choice = val
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .ok_or_else(|| {
                JarvisError::Other(format!(
                    "Invalid response format from {}",
                    self.provider_name
                ))
            })?;

        let text_content = choice
            .get("content")
            .and_then(|c| c.as_str())
            .map(|s| s.trim().to_string());

        let mut tool_calls = Vec::new();
        if let Some(calls) = choice.get("tool_calls").and_then(|tc| tc.as_array()) {
            for (idx, call) in calls.iter().enumerate() {
                let id = call
                    .get("id")
                    .and_then(|i| i.as_str())
                    .unwrap_or(&format!("call_{idx}"))
                    .to_string();
                let fn_obj = call.get("function");
                let name = fn_obj
                    .and_then(|f| f.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or_default()
                    .to_string();
                let args_val = match fn_obj.and_then(|f| f.get("arguments")) {
                    Some(Value::String(s)) => serde_json::from_str(s).unwrap_or_else(|_| json!({})),
                    Some(v) => v.clone(),
                    None => json!({}),
                };

                tool_calls.push(AIToolCall {
                    id,
                    name,
                    args: args_val,
                });
            }
        }

        Ok(AIResponse {
            text: text_content,
            tool_calls,
        })
    }
}

// -----------------------------------------------------------------------------
// Anthropic Claude REST Client
// -----------------------------------------------------------------------------

#[derive(Clone)]
pub struct AnthropicClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl AnthropicClient {
    pub fn new(api_key: &str) -> Self {
        let client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();
        Self {
            client,
            api_key: api_key.trim().to_string(),
            base_url: "https://api.anthropic.com/v1".to_string(),
        }
    }

    pub async fn create_message(
        &self,
        model: &str,
        system_prompt: &str,
        messages: &[Value],
        tools: Option<Value>,
    ) -> Result<AIResponse> {
        let url = format!("{}/messages", self.base_url);

        let mut body = json!({
            "model": model,
            "max_tokens": 1024,
            "system": system_prompt,
            "messages": messages,
            "temperature": 0.2,
        });

        if let Some(t) = tools {
            if let Some(arr) = t.as_array() {
                if !arr.is_empty() {
                    body["tools"] = t;
                }
            }
        }

        debug!("Sending Anthropic messages request to model: {}", model);

        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            warn!("Anthropic 429 Too Many Requests on model {model}");
            return Err(JarvisError::QuotaExceeded(model.to_string()));
        }

        if !status.is_success() {
            let lower = text.to_lowercase();
            if lower.contains("rate_limit") || lower.contains("quota") {
                return Err(JarvisError::QuotaExceeded(model.to_string()));
            }
            error!("Anthropic API error (HTTP {status}): {text}");
            return Err(JarvisError::ApiError { status, body: text });
        }

        let val: Value = serde_json::from_str(&text).map_err(JarvisError::Serialization)?;

        let mut text_parts = Vec::new();
        let mut tool_calls = Vec::new();

        if let Some(content_arr) = val.get("content").and_then(|c| c.as_array()) {
            for (idx, item) in content_arr.iter().enumerate() {
                let item_type = item
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or_default();
                if item_type == "text" {
                    if let Some(t) = item.get("text").and_then(|txt| txt.as_str()) {
                        text_parts.push(t.trim().to_string());
                    }
                } else if item_type == "tool_use" {
                    let id = item
                        .get("id")
                        .and_then(|i| i.as_str())
                        .unwrap_or(&format!("call_{idx}"))
                        .to_string();
                    let name = item
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let args = item.get("input").cloned().unwrap_or_else(|| json!({}));
                    tool_calls.push(AIToolCall { id, name, args });
                }
            }
        }

        let combined_text = if text_parts.is_empty() {
            None
        } else {
            Some(text_parts.join("\n"))
        };

        Ok(AIResponse {
            text: combined_text,
            tool_calls,
        })
    }
}

// -----------------------------------------------------------------------------
// Unified AI Client Enum
// -----------------------------------------------------------------------------

#[derive(Clone)]
pub enum AIClient {
    Gemini(GeminiClient),
    OpenAI(OpenAICompatibleClient),
    Anthropic(AnthropicClient),
}

impl AIClient {
    pub fn from_settings(settings: &Settings) -> Result<Self> {
        let provider = settings.ai_provider.to_lowercase();
        match provider.as_str() {
            "openai" => {
                let key = settings
                    .openai_api_key
                    .as_deref()
                    .ok_or_else(|| JarvisError::EnvVarMissing("OPENAI_API_KEY".to_string()))?;
                Ok(AIClient::OpenAI(OpenAICompatibleClient::new(
                    "openai",
                    key,
                    "https://api.openai.com/v1",
                )))
            }
            "anthropic" | "claude" => {
                let key = settings
                    .anthropic_api_key
                    .as_deref()
                    .ok_or_else(|| JarvisError::EnvVarMissing("ANTHROPIC_API_KEY".to_string()))?;
                Ok(AIClient::Anthropic(AnthropicClient::new(key)))
            }
            "groq" => {
                let key = settings
                    .groq_api_key
                    .as_deref()
                    .ok_or_else(|| JarvisError::EnvVarMissing("GROQ_API_KEY".to_string()))?;
                Ok(AIClient::OpenAI(OpenAICompatibleClient::new(
                    "groq",
                    key,
                    "https://api.groq.com/openai/v1",
                )))
            }
            "openrouter" => {
                let key = settings
                    .openrouter_api_key
                    .as_deref()
                    .ok_or_else(|| JarvisError::EnvVarMissing("OPENROUTER_API_KEY".to_string()))?;
                Ok(AIClient::OpenAI(OpenAICompatibleClient::new(
                    "openrouter",
                    key,
                    "https://openrouter.ai/api/v1",
                )))
            }
            _ => {
                // Default to Gemini
                let key = settings
                    .gemini_api_key
                    .as_deref()
                    .ok_or_else(|| JarvisError::EnvVarMissing("GEMINI_API_KEY".to_string()))?;
                Ok(AIClient::Gemini(GeminiClient::new(key)))
            }
        }
    }

    /// Converts tool registry schemas into the provider-specific format
    pub fn format_tools_for_provider(&self, registry: &ToolRegistry) -> Value {
        match self {
            AIClient::Gemini(_) => registry.gemini_function_declarations(),
            AIClient::OpenAI(_) => {
                let mut tools = Vec::new();
                for tool in registry.all() {
                    tools.push(json!({
                        "type": "function",
                        "function": {
                            "name": tool.name(),
                            "description": tool.description(),
                            "parameters": tool.parameters_schema()
                        }
                    }));
                }
                json!(tools)
            }
            AIClient::Anthropic(_) => {
                let mut tools = Vec::new();
                for tool in registry.all() {
                    tools.push(json!({
                        "name": tool.name(),
                        "description": tool.description(),
                        "input_schema": tool.parameters_schema()
                    }));
                }
                json!(tools)
            }
        }
    }
}
