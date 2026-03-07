//! AI Planner delegating decisions between the LLM and the Heuristic Scorer.

use crate::context::AiContext;
use crate::llm::LlmClient;
use crate::scorer::HeuristicScorer;
use crate::{AiConfig, AiError};
use autodj_analysis::profile::TrackProfile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiDecision {
    pub decision: String, // "wait", "transition_now", "prepare_next"
    pub reason: String,
    pub track_selection: Option<String>,
    pub execute_at_ms: Option<u64>,
    pub transition_plan: Option<TransitionPlan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionPlan {
    pub template_name: String,
    pub parameter_overrides: Option<serde_json::Value>,
}

pub struct AiPlanner {
    config: AiConfig,
    scorer: HeuristicScorer,
    llm: LlmClient,
}

impl AiPlanner {
    pub fn new(config: AiConfig) -> Self {
        let scorer = HeuristicScorer::new(config.clone());
        let llm = LlmClient::new();
        Self { config, scorer, llm }
    }

    pub fn config(&self) -> &AiConfig {
        &self.config
    }

    pub fn set_config(&mut self, config: AiConfig) {
        self.config = config.clone();
        self.scorer = HeuristicScorer::new(config);
    }

    /// Try to get a plan from the LLM. If it fails or is disabled, fallback to heuristics.
    pub async fn plan_next(
        &self,
        context: &AiContext,
        current_profile: &TrackProfile,
        available_profiles: &[TrackProfile],
    ) -> Result<AiDecision, AiError> {
        if available_profiles.is_empty() {
            return Err(AiError::NoCandidates);
        }

        if self.config.enabled && self.config.api_key.is_some() {
            match self.query_llm(context).await {
                Ok(decision) => {
                    log::info!("LLM returned decision: {}", decision.decision);
                    // Minimal validation to ensure track exists
                    if let Some(ref track_id) = decision.track_selection {
                        if !available_profiles.iter().any(|p| &p.track_id == track_id) {
                            log::warn!("LLM selected unknown track ID. Falling back.");
                            return self.fallback_heuristic(context, current_profile, available_profiles);
                        }
                    }
                    return Ok(decision);
                }
                Err(e) => {
                    log::warn!("LLM planning failed: {}. Falling back to heuristics.", e);
                }
            }
        }

        self.fallback_heuristic(context, current_profile, available_profiles)
    }

    async fn query_llm(&self, context: &AiContext) -> Result<AiDecision, AiError> {
        let system_prompt = self.config.prompt_template.as_deref().unwrap_or(
            "You are Auto-DJ, an expert AI club DJ. Analyze the current state, timing metrics, and available tracks. \
            Output valid JSON conforming to the AiDecision schema. Make sure to keep the energy flowing \
            and select tracks that mix harmonically. Use the `duration_ms`, `elapsed_ms`, and `phrase_length_ms` \
            to calculate the absolute optimal `execute_at_ms` timestamp to fire the next transition!"
        );

        let context_json = serde_json::to_string(context)
            .map_err(|e| AiError::ParseError(format!("Failed to serialize context: {e}")))?;

        let response_text = self.llm.query(system_prompt, &context_json, &self.config).await?;

        let decision: AiDecision = serde_json::from_str(&response_text)
            .map_err(|e| AiError::ParseError(format!("Failed to parse LLM JSON: {e}")))?;

        Ok(decision)
    }

    fn fallback_heuristic(
        &self,
        context: &AiContext,
        current_profile: &TrackProfile,
        available_profiles: &[TrackProfile],
    ) -> Result<AiDecision, AiError> {
        let next_track = self.scorer.select_next(
            current_profile,
            available_profiles,
            &context.constraints.recent_tracks,
            &context.preferences.intent,
        ).ok_or(AiError::NoCandidates)?;

        // Automatically prepare the transition when time is running low, else wait.
        let is_ending = context.current_state.time_remaining_ms < 60_000;
        
        let decision = if is_ending {
            "transition_now"
        } else {
            "prepare_next"
        };

        // Determine template (heuristic)
        let energy_jump = next_track.energy.average - current_profile.energy.average;
        let template_name = if energy_jump > 0.3 {
            "echo_drop"
        } else {
            "smooth_beatmix"
        };

        Ok(AiDecision {
            decision: decision.to_string(),
            reason: format!("Heuristic fallback matched {} due to energy/key rating.", next_track.track_id),
            track_selection: Some(next_track.track_id.clone()),
            execute_at_ms: Some(context.current_state.duration_ms.saturating_sub(10_000)),
            transition_plan: Some(TransitionPlan {
                template_name: template_name.to_string(),
                parameter_overrides: None,
            }),
        })
    }
}
