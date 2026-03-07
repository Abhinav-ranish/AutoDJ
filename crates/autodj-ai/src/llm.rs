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

        // Extract model name or default to gemini-1.5-pro
        let model = if config.model.is_empty() {
            "gemini-2.5-flash"
        } else {
            &config.model
        };

        let base_url = config
            .endpoint_url
            .as_deref()
            .unwrap_or("https://generativelanguage.googleapis.com/v1beta/models");
        
        let url = format!("{}/{}:generateContent?key={}", base_url, model, api_key);

        let payload = json!({
            "system_instruction": {
                "parts": { "text": prompt }
            },
            "contents": [{
                "parts": [{ "text": context_json }]
            }],
            "generationConfig": {
                "temperature": 0.2,
                "responseMimeType": "application/json"
            }
        });

        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );

        let response = self
            .client
            .post(&url)
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

        // Parse Gemini JSON output: body.candidates[0].content.parts[0].text
        let candidates = body.get("candidates")
            .and_then(|c| c.as_array())
            .ok_or_else(|| AiError::ParseError("Missing 'candidates' array in Gemini response".to_string()))?;

        if candidates.is_empty() {
            return Err(AiError::ParseError("Empty 'candidates' array in Gemini response".to_string()));
        }

        let content = candidates[0]
            .get("content")
            .and_then(|c| c.get("parts"))
            .and_then(|p| p.as_array())
            .and_then(|p| p.first())
            .and_then(|p| p.get("text"))
            .and_then(|t| t.as_str())
            .ok_or_else(|| AiError::ParseError("Failed to extract content text from Gemini payload".to_string()))?;

        Ok(content.to_string())
    }
}
