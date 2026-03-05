//! autodj-ai: AI conductor (stub for Milestone F).
//!
//! This crate will contain the AI planner, heuristic scorer,
//! transition template system, and LLM API integration.

/// Placeholder for AI conductor configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AiConfig {
    pub enabled: bool,
    pub model: String,
    pub timeout_seconds: u32,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            model: "gpt-4o-mini".to_string(),
            timeout_seconds: 5,
        }
    }
}
