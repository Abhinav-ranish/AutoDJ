//! LLM Client supporting Gemini, Groq (OpenAI-compatible), and Ollama (local) APIs.

use crate::{AiConfig, AiError};
use reqwest::{Client, header};
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Sliding-window token usage tracker for rate limiting.
struct TokenBucket {
    entries: VecDeque<(Instant, u32)>,
    limit_per_minute: u32,
}

impl TokenBucket {
    fn new(limit: u32) -> Self {
        Self {
            entries: VecDeque::new(),
            limit_per_minute: limit,
        }
    }

    /// Prune entries older than 60 seconds and return tokens used in the window.
    fn tokens_used(&mut self) -> u32 {
        let cutoff = Instant::now() - Duration::from_secs(60);
        while self.entries.front().map_or(false, |(t, _)| *t < cutoff) {
            self.entries.pop_front();
        }
        self.entries.iter().map(|(_, n)| n).sum()
    }

    /// Check if there's capacity for `estimated_tokens`. Returns Ok or the ms to wait.
    fn check_capacity(&mut self, estimated_tokens: u32) -> Result<(), u64> {
        let used = self.tokens_used();
        if used + estimated_tokens <= self.limit_per_minute {
            Ok(())
        } else {
            // Estimate wait time: when will enough tokens expire?
            let excess = (used + estimated_tokens).saturating_sub(self.limit_per_minute);
            let mut freed = 0u32;
            let now = Instant::now();
            for (t, n) in &self.entries {
                freed += n;
                if freed >= excess {
                    let age = now.duration_since(*t);
                    let wait = Duration::from_secs(60).saturating_sub(age);
                    return Err(wait.as_millis() as u64 + 100); // +100ms buffer
                }
            }
            Err(60_000) // worst case: wait a full minute
        }
    }

    fn record(&mut self, tokens: u32) {
        self.entries.push_back((Instant::now(), tokens));
    }
}

/// Global rate limiter, lazily initialized per provider config.
static RATE_LIMITER: std::sync::LazyLock<Mutex<TokenBucket>> =
    std::sync::LazyLock::new(|| Mutex::new(TokenBucket::new(0)));

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
        // Pre-flight rate limit check for token-constrained providers
        if let Some(tpm) = config.rate_limit_tpm {
            let estimated_tokens = ((prompt.len() + context_json.len()) as f64 / 4.0).ceil() as u32;
            let mut bucket = RATE_LIMITER.lock().unwrap_or_else(|e| e.into_inner());
            bucket.limit_per_minute = tpm;
            if let Err(wait_ms) = bucket.check_capacity(estimated_tokens) {
                log::warn!(
                    "[llm] Rate limiter: estimated {}tok would exceed {}tpm limit, wait {}ms",
                    estimated_tokens,
                    tpm,
                    wait_ms
                );
                return Err(AiError::RateLimited { retry_after_ms: wait_ms });
            }
        }

        let result = match config.provider.as_str() {
            "groq" => self.query_groq(prompt, context_json, config).await,
            "ollama" => self.query_ollama(prompt, context_json, config).await,
            _ => self.query_gemini(prompt, context_json, config).await,
        };

        // Record actual usage on success
        if result.is_ok() {
            if let Some(_tpm) = config.rate_limit_tpm {
                // Estimate total tokens (prompt + context + response)
                let response_len = result.as_ref().map(|r| r.len()).unwrap_or(0);
                let total_tokens = ((prompt.len() + context_json.len() + response_len) as f64 / 4.0).ceil() as u32;
                if let Ok(mut bucket) = RATE_LIMITER.lock() {
                    bucket.record(total_tokens);
                }
            }
        }

        result
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
            "llama-3.1-8b-instant"
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

        let backoff_ms: [u64; 2] = [2000, 5000];
        let max_attempts = 3u32;

        for attempt in 0..max_attempts {
            let response = self
                .client
                .post(url)
                .headers(headers.clone())
                .timeout(Duration::from_secs(config.timeout_seconds as u64))
                .json(payload)
                .send()
                .await
                .map_err(|e| AiError::LlmError(format!("Request failed: {e}")))?;

            if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                // Parse Retry-After header (seconds) if present
                let retry_after_ms = response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<f64>().ok())
                    .map(|s| (s * 1000.0) as u64)
                    .unwrap_or_else(|| {
                        backoff_ms.get(attempt as usize).copied().unwrap_or(5000)
                    });

                let text = response.text().await.unwrap_or_default();
                log::warn!(
                    "[llm] Rate limited (429), attempt {}/{}: retry_after={}ms body={}",
                    attempt + 1,
                    max_attempts,
                    retry_after_ms,
                    &text[..text.len().min(200)]
                );

                if attempt + 1 < max_attempts {
                    tokio::time::sleep(Duration::from_millis(retry_after_ms)).await;
                    continue;
                }

                return Err(AiError::RateLimited { retry_after_ms });
            }

            if !response.status().is_success() {
                let status = response.status();
                let text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                return Err(AiError::LlmError(format!("HTTP {}: {}", status, text)));
            }

            return response
                .json()
                .await
                .map_err(|e| AiError::ParseError(format!("Failed to parse response body: {e}")));
        }

        // Should not reach here, but just in case
        Err(AiError::LlmError("Max retry attempts exhausted".to_string()))
    }
}
