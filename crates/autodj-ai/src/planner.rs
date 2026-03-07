//! AI Planner delegating decisions between the LLM and the Heuristic Scorer.

use crate::context::{AiContext, TrackClassificationData};
use crate::llm::LlmClient;
use crate::scorer::{HeuristicScorer, ScoringCandidate};
use crate::{AiConfig, AiError};
use autodj_analysis::profile::TrackProfile;
use serde::{Deserialize, Serialize};

const DJ_SYSTEM_PROMPT: &str = r#"You are Auto-DJ, an expert AI club DJ with deep knowledge of mixing techniques.

## Your Job
Analyze the current state, available tracks, and set flow to make the best mixing decision.

## Mixing Principles
1. HARMONIC MIXING: Prefer tracks within 1-2 Camelot positions. Same key (0) is ideal, adjacent (±1) is good, relative major/minor (A↔B) works well.
2. TEMPO MATCHING: Keep BPM within ±8 of current. Perceived tempo matters more than raw BPM (trap at 140 feels like 70).
3. ENERGY FLOW: Build energy gradually, maintain peaks, provide relief. Don't jump between extremes without a breakdown.
4. GENRE COHERENCE: Stay in related genres. House→techno is smooth. House→hip-hop needs careful transition (echo drop or breakdown).
5. MOOD CONTINUITY: Gradual mood shifts. Euphoric→uplifting is smooth. Euphoric→melancholic is jarring unless deliberate.
6. VOCAL AWARENESS: Never overlap two vocal-heavy tracks. If current track has vocals, next should be instrumental or have vocals in different section.

## Transition Templates
- smooth_beatmix: 16-32 bars, bass swap, best for same-genre/tempo tracks
- filter_sweep: 8-16 bars, high-pass sweep + incoming fade, good for energy builds or genre changes
- echo_drop: 4-8 bars, echo feedback → silence → drop, best for big energy moments or large tempo/genre jumps

## Timing Rules
- Use section boundaries (intro/outro) for transitions, not arbitrary positions
- Match outgoing outro length with incoming intro length when possible
- execute_at_ms should target the start of the outgoing track's outro section
- Never trigger during a vocal hook or drop buildup

## Output
Return valid JSON with fields: decision ("wait"|"transition_now"|"prepare_next"), reason, track_selection (track id), execute_at_ms, transition_plan ({template_name, parameter_overrides}).
Include detailed reasoning in the reason field."#;

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
        classifications: &std::collections::HashMap<String, TrackClassificationData>,
    ) -> Result<AiDecision, AiError> {
        if available_profiles.is_empty() {
            return Err(AiError::NoCandidates);
        }

        if self.config.enabled && self.config.api_key.is_some() {
            match self.query_llm(context).await {
                Ok(decision) => {
                    log::info!("LLM returned decision: {}", decision.decision);
                    if let Some(ref track_id) = decision.track_selection {
                        if !available_profiles.iter().any(|p| &p.track_id == track_id) {
                            log::warn!("LLM selected unknown track ID. Falling back.");
                            return self.fallback_heuristic(context, current_profile, available_profiles, classifications);
                        }
                    }
                    return Ok(decision);
                }
                Err(e) => {
                    log::warn!("LLM planning failed: {}. Falling back to heuristics.", e);
                }
            }
        }

        self.fallback_heuristic(context, current_profile, available_profiles, classifications)
    }

    async fn query_llm(&self, context: &AiContext) -> Result<AiDecision, AiError> {
        let system_prompt = self.config.prompt_template.as_deref()
            .unwrap_or(DJ_SYSTEM_PROMPT);

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
        classifications: &std::collections::HashMap<String, TrackClassificationData>,
    ) -> Result<AiDecision, AiError> {
        let current_class = classifications.get(&current_profile.track_id);
        let current_candidate = ScoringCandidate {
            profile: current_profile,
            genre: current_class.and_then(|c| c.genre.as_deref()),
            mood: current_class.and_then(|c| c.mood.as_deref()),
        };

        let candidates: Vec<ScoringCandidate> = available_profiles
            .iter()
            .map(|p| {
                let class = classifications.get(&p.track_id);
                ScoringCandidate {
                    profile: p,
                    genre: class.and_then(|c| c.genre.as_deref()),
                    mood: class.and_then(|c| c.mood.as_deref()),
                }
            })
            .collect();

        let next_track = self.scorer.select_next(
            &current_candidate,
            &candidates,
            &context.constraints.recent_tracks,
            &context.preferences.intent,
        ).ok_or(AiError::NoCandidates)?;

        let is_ending = context.current_state.time_remaining_ms < 60_000;
        let decision = if is_ending { "transition_now" } else { "prepare_next" };

        // Section-aware transition timing: use outro start if available
        let execute_at = context.current_state.duration_ms.saturating_sub(
            context.current_state.outro_length_ms.unwrap_or(10_000)
        );

        // Vocal-aware + genre-aware template selection
        let energy_jump = next_track.energy.average - current_profile.energy.average;
        let next_class = classifications.get(&next_track.track_id);
        let genre_change = match (
            current_class.and_then(|c| c.genre.as_deref()),
            next_class.and_then(|c| c.genre.as_deref()),
        ) {
            (Some(a), Some(b)) => !a.eq_ignore_ascii_case(b),
            _ => false,
        };
        let vocal_overlap = match (
            current_profile.vocal_presence,
            next_track.vocal_presence,
        ) {
            (Some(a), Some(b)) => a > 0.5 && b > 0.5,
            _ => false,
        };

        let template_name = if energy_jump > 0.3 {
            "echo_drop"
        } else if genre_change || vocal_overlap {
            "filter_sweep"
        } else {
            "smooth_beatmix"
        };

        Ok(AiDecision {
            decision: decision.to_string(),
            reason: format!(
                "Heuristic: selected {} (genre: {}, mood: {}, template: {}).",
                next_track.track_id,
                next_class.and_then(|c| c.genre.as_deref()).unwrap_or("unknown"),
                next_class.and_then(|c| c.mood.as_deref()).unwrap_or("unknown"),
                template_name,
            ),
            track_selection: Some(next_track.track_id.clone()),
            execute_at_ms: Some(execute_at),
            transition_plan: Some(TransitionPlan {
                template_name: template_name.to_string(),
                parameter_overrides: None,
            }),
        })
    }
}
