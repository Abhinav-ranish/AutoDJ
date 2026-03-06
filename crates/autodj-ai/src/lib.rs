//! autodj-ai: AI conductor (stub for Milestone F).
//!
//! This crate will contain the AI planner, heuristic scorer,
//! transition template system, and LLM API integration.

pub mod camelot;
pub mod context;
pub mod llm;
pub mod planner;
pub mod scorer;

use serde::{Deserialize, Serialize};

/// Configuration for the AI Conductor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub enabled: bool,
    pub model: String,
    pub api_key: Option<String>,
    pub endpoint_url: Option<String>,
    pub timeout_seconds: u32,
    pub prompt_template: Option<String>,
    
    // Heuristic Weights
    pub weight_tempo: f64,
    pub weight_key: f64,
    pub weight_energy: f64,
    pub weight_variety: f64,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            model: "gpt-4o-mini".to_string(),
            api_key: None,
            endpoint_url: None,
            timeout_seconds: 5,
            prompt_template: None,
            weight_tempo: 0.3,
            weight_key: 0.3,
            weight_energy: 0.2,
            weight_variety: 0.2,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("LLM Error: {0}")]
    LlmError(String),
    #[error("Parsing Error: {0}")]
    ParseError(String),
    #[error("No candidates available")]
    NoCandidates,
    #[error("Heuristic fallback triggered due to: {0}")]
    Fallback(String),
}
