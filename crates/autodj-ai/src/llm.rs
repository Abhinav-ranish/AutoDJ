//! LLM Client supporting Gemini, Groq (OpenAI-compatible), and Ollama (local) APIs.

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
    /// Routes to Gemini or Groq based on `config.provider`.
    pub async fn query(
        &self,
        prompt: &str,
        context_json: &str,
        config: &AiConfig,
    ) -> Result<String, AiError> {
        match config.provider.as_str() {
            "groq" => self.query_groq(prompt, context_json, config).await,
            "ollama" => self.query_ollama(prompt, context_json, config).await,
            _ => self.query_gemini(prompt, context_json, config).await,
        }
    }

    /// Gemini generateContent API.
    async fn query_gemini(
        &self,
        prompt: &str,
        context_json: &str,
        config: &AiConfig,
    ) -> Result<String, AiError> {
        let api_key = config
            .api_key
            .as_ref()
            .ok_or_else(|| AiError::LlmError("API key not configured".to_string()))?;

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

        let body = self.send_request(&url, &payload, config, None).await?;

        // Parse Gemini: body.candidates[0].content.parts[0].text
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

    /// Groq chat completions API (OpenAI-compatible).
    async fn query_groq(
        &self,
        prompt: &str,
        context_json: &str,
        config: &AiConfig,
    ) -> Result<String, AiError> {
        let api_key = config
            .api_key
            .as_ref()
            .ok_or_else(|| AiError::LlmError("API key not configured".to_string()))?;

        let model = if config.model.is_empty() {
            "llama-3.3-70b-versatile"
        } else {
            &config.model
        };

        let base_url = config
            .endpoint_url
            .as_deref()
            .unwrap_or("https://api.groq.com/openai/v1");

        let url = format!("{}/chat/completions", base_url);

        let payload = json!({
            "model": model,
            "messages": [
                { "role": "system", "content": prompt },
                { "role": "user", "content": context_json }
            ],
            "temperature": 0.2,
            "response_format": { "type": "json_object" }
        });

        let body = self.send_request(&url, &payload, config, Some(api_key)).await?;

        // Parse OpenAI-compatible: body.choices[0].message.content
        let choices = body.get("choices")
            .and_then(|c| c.as_array())
            .ok_or_else(|| AiError::ParseError("Missing 'choices' array in Groq response".to_string()))?;

        if choices.is_empty() {
            return Err(AiError::ParseError("Empty 'choices' array in Groq response".to_string()));
        }

        let content = choices[0]
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .ok_or_else(|| AiError::ParseError("Failed to extract message content from Groq payload".to_string()))?;

        Ok(content.to_string())
    }

    /// Ollama local inference API (OpenAI-compatible).
    ///
    /// No API key required — Ollama runs locally on the user's machine.
    /// Default endpoint: http://localhost:11434/v1
    async fn query_ollama(
        &self,
        prompt: &str,
        context_json: &str,
        config: &AiConfig,
    ) -> Result<String, AiError> {
        let model = if config.model.is_empty() {
            "qwen3:4b"
        } else {
            &config.model
        };

        let base_url = config
            .endpoint_url
            .as_deref()
            .unwrap_or("http://localhost:11434/v1");

        let url = format!("{}/chat/completions", base_url);

        let payload = json!({
            "model": model,
            "messages": [
                { "role": "system", "content": prompt },
                { "role": "user", "content": context_json }
            ],
            "temperature": 0.2,
            "response_format": { "type": "json_object" },
            "stream": false
        });

        // Ollama is local — no bearer token needed, but use longer timeout
        // since local inference can be much slower than cloud APIs,
        // especially on cold start when the model is loading into memory.
        let timeout_secs = config.timeout_seconds.max(120) as u64;
        let mut ollama_config = config.clone();
        ollama_config.timeout_seconds = timeout_secs as u32;

        let body = self
            .send_request(&url, &payload, &ollama_config, None)
            .await
            .map_err(|e| {
                let msg = format!("{e}");
                if msg.contains("Connection refused") || msg.contains("connect error") {
                    AiError::LlmError(
                        "Ollama is not running. Start it with `ollama serve` or launch the Ollama app."
                            .to_string(),
                    )
                } else {
                    e
                }
            })?;

        // Parse OpenAI-compatible response: body.choices[0].message.content
        let choices = body
            .get("choices")
            .and_then(|c| c.as_array())
            .ok_or_else(|| {
                AiError::ParseError(
                    "Missing 'choices' array in Ollama response".to_string(),
                )
            })?;

        if choices.is_empty() {
            return Err(AiError::ParseError(
                "Empty 'choices' array in Ollama response".to_string(),
            ));
        }

        let content = choices[0]
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .ok_or_else(|| {
                AiError::ParseError(
                    "Failed to extract message content from Ollama payload".to_string(),
                )
            })?;

        Ok(content.to_string())
    }

    /// Shared HTTP request logic.
    async fn send_request(
        &self,
        url: &str,
        payload: &Value,
        config: &AiConfig,
        bearer_token: Option<&str>,
    ) -> Result<Value, AiError> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );

        if let Some(token) = bearer_token {
            let auth_value = header::HeaderValue::from_str(&format!("Bearer {}", token))
                .map_err(|e| AiError::LlmError(format!("Invalid API key header: {e}")))?;
            headers.insert(header::AUTHORIZATION, auth_value);
        }

        let response = self
            .client
            .post(url)
            .headers(headers)
            .timeout(Duration::from_secs(config.timeout_seconds as u64))
            .json(payload)
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

        response
            .json()
            .await
            .map_err(|e| AiError::ParseError(format!("Failed to parse response body: {e}")))
    }
}
