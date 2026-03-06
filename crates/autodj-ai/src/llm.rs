//! LLM Client for calling OpenAI-compatible APIs.

use crate::{AiConfig, AiError};
use reqwest::{Client, header};
use serde_json::{json, Value};
use std::time::Duration;

pub struct LlmClient {
    client: Client,
}

impl Default for LlmClient {
    fn default() -> Self {
        Self::new()
    }
}

impl LlmClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Send a prompt to the LLM and return the JSON response string.
    pub async fn query(
        &self,
        prompt: &str,
        context_json: &str,
        config: &AiConfig,
    ) -> Result<String, AiError> {
        let api_key = config
            .api_key
            .as_ref()
            .ok_or_else(|| AiError::LlmError("API key not configured".to_string()))?;

        let url = config
            .endpoint_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1/chat/completions");

        let messages = vec![
            json!({
                "role": "system",
                "content": prompt
            }),
            json!({
                "role": "user",
                "content": context_json
            }),
        ];

        let payload = json!({
            "model": config.model,
            "messages": messages,
            "temperature": 0.2,
            "response_format": { "type": "json_object" }
        });

        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {}", api_key))
                .map_err(|e| AiError::LlmError(format!("Invalid API Key header: {e}")))?,
        );
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );

        let response = self
            .client
            .post(url)
            .headers(headers)
            .timeout(Duration::from_secs(config.timeout_seconds as u64))
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiError::LlmError(format!("Request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AiError::LlmError(format!("HTTP {}: {}", status, text)));
        }

        let body: Value = response
            .json()
            .await
            .map_err(|e| AiError::ParseError(format!("Failed to parse response body: {e}")))?;

        let content = body["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| AiError::ParseError("Missing content in LLM response".to_string()))?;

        Ok(content.to_string())
    }
}
