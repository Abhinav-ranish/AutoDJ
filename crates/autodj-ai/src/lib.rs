//! autodj-ai: AI conductor (stub for Milestone F).
//!
//! This crate will contain the AI planner, heuristic scorer,
//! transition template system, and LLM API integration.

pub mod camelot;
pub mod classifier;
pub mod context;
pub mod llm;
pub mod planner;
pub mod quality;
pub mod scorer;

use serde::{Deserialize, Serialize};

/// Configuration for the AI Conductor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub enabled: bool,
    /// LLM provider: "gemini" or "groq"
    #[serde(default = "default_provider")]
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
    pub endpoint_url: Option<String>,
    pub timeout_seconds: u32,
    pub prompt_template: Option<String>,

    // Heuristic Weights (8 dimensions, should sum to ~1.0)
    pub weight_tempo: f64,
    pub weight_key: f64,
    pub weight_energy: f64,
    pub weight_variety: f64,
    pub weight_genre: f64,
    pub weight_mood: f64,
    pub weight_spectral: f64,
    /// Phase 47: Weight for vibe compatibility scoring (opener/anthem/filler/etc.)
    #[serde(default = "default_weight_vibe")]
    pub weight_vibe: f64,
    /// Phase 50: Weight for vocal compatibility in track selection.
    /// Penalizes selecting a vocal-heavy track when current is also vocal-heavy.
    #[serde(default = "default_weight_vocal")]
    pub weight_vocal: f64,

    /// Phase 48: Allow replaying tracks in a set (anthem callbacks).
    #[serde(default)]
    pub allow_replay: bool,
    /// Phase 48: Minimum number of tracks before an anthem can be replayed.
    #[serde(default = "default_anthem_callback_distance")]
    pub anthem_callback_distance: usize,
}

fn default_provider() -> String {
    "gemini".to_string()
}

fn default_weight_vibe() -> f64 {
    0.10
}

fn default_weight_vocal() -> f64 {
    0.05
}

fn default_anthem_callback_distance() -> usize {
    5
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: "gemini".to_string(),
            model: "gemini-2.5-flash".to_string(),
            api_key: None,
            endpoint_url: None,
            timeout_seconds: 5,
            prompt_template: None,
            weight_tempo: 0.20,
            weight_key: 0.20,
            weight_energy: 0.15,
            weight_variety: 0.05,
            weight_genre: 0.15,
            weight_mood: 0.10,
            weight_spectral: 0.05,
            weight_vibe: 0.10,
            weight_vocal: 0.05,
            allow_replay: false,
            anthem_callback_distance: 5,
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
