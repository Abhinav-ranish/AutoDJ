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
- drop_swap: HP sweep out → hard cut to incoming on drop. Great for buildup-to-drop moments.
- loop_roll: 4→2→1 bar loop stutter on outgoing → cut to incoming. High impact, good for energy peaks.
- bass_kill_swap: Blend mids/highs first, then swap bass at a beat boundary. Clean power swap, great for same-genre tracks.
- break_blend: Gentle HP opening on incoming during outgoing's break/outro. Best for melodic or vocal-heavy transitions.- backspin_cut: Simulated backspin (rapid pitch sweep + filter close) → silence → hard cut. Stylistic flair for genre changes.
- drop_ride: Ride both tracks through their drops simultaneously with bass swap. High energy, same-genre dual-drop blend.
## Section-Aware Timing
- The current track has transition_windows: scored windows anchored to section boundaries.
  Use the highest-scored window whose start_ms is still in the future.
- Use phrase boundaries to quantize transition starts to the nearest 8-bar grid point.
- execute_at_ms should target a transition window start, not an arbitrary time.
- Never trigger during a vocal hook or drop buildup.
- Use auto_cues (first_beat, intro, drop, brk, outro) for precise section references.

## Action Timeline (v2)
Instead of (or in addition to) a transition_plan, you may provide an "actions" array.
Each action is: { "at_ms": <ms from transition start>, "kind": { "type": "...", ...fields } }.
Action types:
  - load_track: { deck, track_id }
  - set_tempo: { deck, bpm }
  - set_gain: { deck, gain_db } — clamped to [-60, 12] dB
  - set_filter: { deck, cutoff_hz, resonance } — cutoff clamped to [20, 20000]
  - set_eq_band: { deck, band ("low"|"mid"|"high"), gain_db } — clamped to [-24, 6] dB
  - set_eq_kill: { deck, band, enabled }
  - set_loop: { deck, length_bars }
  - clear_loop: { deck }
  - set_crossfader: { position } — 0.0=A, 0.5=center, 1.0=B
  - play/pause: { deck }
  - execute_template: { template_name, params }
If actions are present they override transition_plan.

## Safety Constraints
- EQ gain: [-24, 6] dB
- Filter cutoff: [20, 20000] Hz
- Gain: [-60, 12] dB
- Crossfader: [0.0, 1.0]
- Never issue two conflicting actions for the same deck at the same at_ms.

## Output
Return valid JSON with fields: decision ("wait"|"transition_now"|"prepare_next"), reason, track_selection (track id), execute_at_ms, transition_plan ({template_name, parameter_overrides}), actions (optional action timeline array), reasoning_tags (optional string array), confidence (0.0-1.0).
Include detailed reasoning in the reason field."#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiDecision {
    pub decision: String, // "wait", "transition_now", "prepare_next"
    pub reason: String,
    pub track_selection: Option<String>,
    pub execute_at_ms: Option<u64>,
    pub transition_plan: Option<TransitionPlan>,
    /// Phase 20: structured action timeline (optional, overrides template if present)
    pub actions: Option<Vec<MixAction>>,
    /// Phase 20: reasoning tags for debugging
    pub reasoning_tags: Option<Vec<String>>,
    /// Phase 20: overall confidence (0.0–1.0)
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionPlan {
    pub template_name: String,
    pub parameter_overrides: Option<serde_json::Value>,
}

/// Phase 20: A single timestamped mixing action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixAction {
    /// When to execute, in ms relative to the transition start.
    pub at_ms: u64,
    /// What to do.
    pub kind: MixActionKind,
}

/// Phase 20: Types of mixing actions the AI can issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MixActionKind {
    LoadTrack { deck: String, track_id: String },
    SetTempo { deck: String, bpm: f64 },
    SetGain { deck: String, gain_db: f32 },
    SetFilter { deck: String, cutoff_hz: f32, resonance: f32 },
    SetEqBand { deck: String, band: String, gain_db: f32 },
    SetEqKill { deck: String, band: String, enabled: bool },
    SetLoop { deck: String, length_bars: u32 },
    ClearLoop { deck: String },
    SetCrossfader { position: f32 },
    Play { deck: String },
    Pause { deck: String },
    ExecuteTemplate { template_name: String, params: serde_json::Value },
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

        // ── Section-aware transition timing ──────────────────────────────────
        // Pick the best transition window whose start is still in the future.
        let now_ms = context.current_state.elapsed_ms;
        let best_window = current_profile.transition_windows.iter()
            .filter(|w| w.start_ms > now_ms)
            .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal));

        let execute_at = if let Some(w) = best_window {
            // Snap to nearest phrase boundary if we have phrase data
            if let Some(ref phrases) = current_profile.phrases {
                autodj_analysis::sections::next_phrase_boundary(phrases, w.start_ms)
                    .unwrap_or(w.start_ms)
            } else {
                w.start_ms
            }
        } else {
            // Fallback: use outro start
            context.current_state.duration_ms.saturating_sub(
                context.current_state.outro_length_ms.unwrap_or(10_000)
            )
        };

        // ── Template selection rules ─────────────────────────────────────────
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
        let bpm_diff = (current_profile.tempo.bpm - next_track.tempo.bpm).abs();

        // Section proximity: check which section we're transitioning from
        let near_drop = best_window.map(|w| w.anchor.contains("drop")).unwrap_or(false);
        let near_breakdown = best_window.map(|w| {
            w.anchor.contains("break") || w.anchor.contains("breakdown")
        }).unwrap_or(false);
        let near_outro = best_window.map(|w| w.anchor.contains("outro")).unwrap_or(false);

        // Energy trajectory
        let trajectory_building = current_profile.energy_trajectory.as_deref() == Some("building");
        let trajectory_declining = current_profile.energy_trajectory.as_deref() == Some("declining");

        // Rhythm profile
        let high_complexity = current_profile.rhythm.as_ref()
            .map(|r| r.complexity > 0.7).unwrap_or(false);
        let high_danceability = current_profile.rhythm.as_ref()
            .map(|r| r.danceability > 0.7).unwrap_or(false);

        // Incoming track has a strong drop we can ride into
        let incoming_has_drop = next_track.first_drop_ms.is_some();

        // ── Score each template and pick best ────────────────────────────────
        let mut candidates: Vec<(&str, f64, Vec<&str>)> = Vec::new();

        // SmoothBeatmix: safe default, best for same-genre similar-energy
        {
            let mut score = 0.5; // base score (no longer the overwhelming default)
            if !genre_change && energy_jump.abs() < 0.15 { score += 0.2; }
            if near_outro { score += 0.1; }
            if bpm_diff < 4.0 { score += 0.1; }
            if vocal_overlap { score -= 0.3; } // bad for vocal overlap
            candidates.push(("smooth_beatmix", score, vec!["compatible", "smooth_beatmix"]));
        }

        // FilterSweep: good for energy builds, genre changes, building trajectory
        {
            let mut score = 0.3;
            if trajectory_building { score += 0.3; }
            if genre_change { score += 0.25; }
            if energy_jump > 0.1 { score += 0.2; }
            if near_breakdown { score += 0.15; }
            candidates.push(("filter_sweep", score, vec!["filter_sweep"]));
        }

        // DropSwap: dramatic, for drop proximity or big energy jumps
        {
            let mut score = 0.2;
            if near_drop { score += 0.4; }
            if energy_jump > 0.2 { score += 0.25; }
            if trajectory_building && incoming_has_drop { score += 0.2; }
            if bpm_diff > 10.0 { score -= 0.2; } // hard cuts need tempo alignment
            candidates.push(("drop_swap", score, vec!["drop_swap"]));
        }

        // EchoDrop: big drama moments, large tempo/genre jumps
        {
            let mut score = 0.15;
            if energy_jump > 0.3 { score += 0.3; }
            if bpm_diff > 10.0 { score += 0.25; }
            if genre_change && energy_jump.abs() > 0.2 { score += 0.2; }
            candidates.push(("echo_drop", score, vec!["echo_drop"]));
        }

        // LoopRoll: high impact, works with complex rhythms at peak energy
        {
            let mut score = 0.15;
            if high_complexity { score += 0.3; }
            if energy_jump > 0.25 { score += 0.2; }
            if bpm_diff > 8.0 { score += 0.15; }
            if near_drop { score += 0.1; }
            candidates.push(("loop_roll", score, vec!["loop_roll"]));
        }

        // BassKillSwap: clean power swap, same genre, groovy tracks
        {
            let mut score = 0.3;
            if !genre_change { score += 0.15; }
            if high_danceability { score += 0.2; }
            if energy_jump.abs() < 0.15 { score += 0.15; }
            if near_outro { score += 0.1; }
            if vocal_overlap { score -= 0.2; }
            candidates.push(("bass_kill_swap", score, vec!["bass_kill_swap"]));
        }

        // BreakBlend: gentle, vocal-friendly, low energy or declining
        {
            let mut score = 0.25;
            if vocal_overlap { score += 0.4; }
            if trajectory_declining { score += 0.25; }
            if energy_jump < -0.15 { score += 0.2; }
            if near_breakdown { score += 0.15; }
            candidates.push(("break_blend", score, vec!["break_blend"]));
        }

        // BackspinCut: stylistic flair, genre changes, moderate energy
        {
            let mut score = 0.1;
            if genre_change { score += 0.25; }
            if energy_jump.abs() > 0.15 && energy_jump.abs() < 0.35 { score += 0.2; }
            if !near_drop && !near_breakdown { score += 0.1; } // works anywhere
            candidates.push(("backspin_cut", score, vec!["backspin_cut"]));
        }

        // DropRide: ride both drops simultaneously, high energy
        {
            let mut score = 0.1;
            if incoming_has_drop && near_drop { score += 0.35; }
            if energy_jump.abs() < 0.2 { score += 0.15; }
            if !genre_change { score += 0.1; }
            if high_danceability { score += 0.1; }
            candidates.push(("drop_ride", score, vec!["drop_ride"]));
        }

        // Sort descending by score
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Add variety: if top 2 scores are within 0.1, randomly pick between them
        // using a simple deterministic "random" based on elapsed time
        let (template_name, _, reasoning_tags) = if candidates.len() >= 2
            && (candidates[0].1 - candidates[1].1) < 0.1
        {
            // Use elapsed_ms as a simple entropy source to alternate picks
            if context.current_state.elapsed_ms % 2 == 0 {
                candidates.remove(1)
            } else {
                candidates.remove(0)
            }
        } else {
            candidates.remove(0)
        };

        // ── Build action timeline ────────────────────────────────────────────
        let beat_ms = (60_000.0 / current_profile.tempo.bpm) as u64;
        let bar_ms = beat_ms * 4;
        let incoming_deck = if context.current_state.playing_deck == "A" { "B" } else { "A" };
        let outgoing_deck = context.current_state.playing_deck.clone();
        let actions = build_heuristic_timeline(
            template_name,
            &outgoing_deck,
            incoming_deck,
            &next_track.track_id,
            bar_ms,
            next_track.tempo.bpm,
        );

        Ok(AiDecision {
            decision: decision.to_string(),
            reason: format!(
                "Heuristic: selected {} (genre: {}, mood: {}, template: {}, window: {}).",
                next_track.track_id,
                next_class.and_then(|c| c.genre.as_deref()).unwrap_or("unknown"),
                next_class.and_then(|c| c.mood.as_deref()).unwrap_or("unknown"),
                template_name,
                best_window.map(|w| format!("{}ms score={:.2}", w.start_ms, w.score))
                    .unwrap_or_else(|| "fallback".to_string()),
            ),
            track_selection: Some(next_track.track_id.clone()),
            execute_at_ms: Some(execute_at),
            transition_plan: Some(TransitionPlan {
                template_name: template_name.to_string(),
                parameter_overrides: None,
            }),
            actions: Some(actions),
            reasoning_tags: Some(reasoning_tags.into_iter().map(String::from).collect()),
            confidence: Some(0.7),
        })
    }
}

/// S-curve for perceptually smooth fades: 0→1 with gentle start and end.
fn s_curve(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Build a concrete action timeline for a given template.
fn build_heuristic_timeline(
    template: &str,
    outgoing: &str,
    incoming: &str,
    incoming_track_id: &str,
    bar_ms: u64,
    incoming_bpm: f64,
) -> Vec<MixAction> {
    let mut actions = Vec::new();
    let out = outgoing.to_string();
    let inc = incoming.to_string();

    // Always: load + cue incoming, sync tempo
    actions.push(MixAction { at_ms: 0, kind: MixActionKind::LoadTrack { deck: inc.clone(), track_id: incoming_track_id.to_string() } });
    actions.push(MixAction { at_ms: 0, kind: MixActionKind::SetTempo { deck: inc.clone(), bpm: incoming_bpm } });
    actions.push(MixAction { at_ms: 0, kind: MixActionKind::SetGain { deck: inc.clone(), gain_db: -40.0 } });
    actions.push(MixAction { at_ms: 0, kind: MixActionKind::Play { deck: inc.clone() } });

    match template {
        "smooth_beatmix" => {
            // 16-bar blend with bass swap at bar 8, S-curve fades, crossfader movement
            let blend_bars: u64 = 16;
            let swap_bar: u64 = 8;
            for i in 0..=blend_bars {
                let t = i * bar_ms;
                let frac = i as f32 / blend_bars as f32;
                let s = s_curve(frac);
                // Incoming: S-curve from -40 to 0 dB
                let in_gain = -40.0 + 40.0 * s;
                actions.push(MixAction { at_ms: t, kind: MixActionKind::SetGain { deck: inc.clone(), gain_db: in_gain } });
                // Crossfader: move from outgoing side to center then to incoming
                let cf = 0.5 * s; // 0.0 → 0.5 (we set final to 1.0 at end)
                actions.push(MixAction { at_ms: t, kind: MixActionKind::SetCrossfader { position: cf } });
            }
            // Bass swap: kill incoming bass until swap, then kill outgoing bass
            actions.push(MixAction { at_ms: 0, kind: MixActionKind::SetEqKill { deck: inc.clone(), band: "low".into(), enabled: true } });
            actions.push(MixAction { at_ms: swap_bar * bar_ms, kind: MixActionKind::SetEqKill { deck: inc.clone(), band: "low".into(), enabled: false } });
            actions.push(MixAction { at_ms: swap_bar * bar_ms, kind: MixActionKind::SetEqKill { deck: out.clone(), band: "low".into(), enabled: true } });
            // Fade out outgoing starting at bar 4 (earlier, gentler exit) with S-curve
            let fade_start: u64 = 4;
            let fade_len = blend_bars - fade_start;
            for i in fade_start..=blend_bars {
                let t = i * bar_ms;
                let frac = (i - fade_start) as f32 / fade_len as f32;
                let s = s_curve(frac);
                actions.push(MixAction { at_ms: t, kind: MixActionKind::SetGain { deck: out.clone(), gain_db: -40.0 * s } });
            }
            // Cleanup
            let end = blend_bars * bar_ms;
            actions.push(MixAction { at_ms: end, kind: MixActionKind::SetEqKill { deck: out.clone(), band: "low".into(), enabled: false } });
            actions.push(MixAction { at_ms: end, kind: MixActionKind::SetCrossfader { position: 0.5 } });
            actions.push(MixAction { at_ms: end, kind: MixActionKind::Pause { deck: out.clone() } });
        }
        "filter_sweep" => {
            // 8-bar HP sweep on outgoing + S-curve incoming fade
            let bars: u64 = 8;
            for i in 0..=bars {
                let t = i * bar_ms;
                let frac = i as f32 / bars as f32;
                let s = s_curve(frac);
                // Exponential filter sweep (200 → 8000 Hz)
                let log_start = 200.0_f32.ln();
                let log_end = 8000.0_f32.ln();
                let cutoff = (log_start + frac * (log_end - log_start)).exp();
                actions.push(MixAction { at_ms: t, kind: MixActionKind::SetFilter { deck: out.clone(), cutoff_hz: cutoff, resonance: 0.5 } });
                // Incoming S-curve gain
                actions.push(MixAction { at_ms: t, kind: MixActionKind::SetGain { deck: inc.clone(), gain_db: -40.0 + 40.0 * s } });
                // Outgoing gentle fade in second half
                if frac > 0.4 {
                    let fade_frac = (frac - 0.4) / 0.6;
                    actions.push(MixAction { at_ms: t, kind: MixActionKind::SetGain { deck: out.clone(), gain_db: -30.0 * s_curve(fade_frac) } });
                }
            }
            let end = bars * bar_ms;
            actions.push(MixAction { at_ms: end, kind: MixActionKind::SetFilter { deck: out.clone(), cutoff_hz: 20000.0, resonance: 0.0 } });
            actions.push(MixAction { at_ms: end, kind: MixActionKind::Pause { deck: out.clone() } });
        }
        "echo_drop" => {
            // Echo tail simulation: fast exponential fade + silence gap + hard drop
            let echo_bars: u64 = 2;
            for i in 0..=(echo_bars * 4) {
                let t = i * (bar_ms / 4); // per-beat resolution
                let frac = i as f32 / (echo_bars * 4) as f32;
                let out_gain = -48.0 * frac * frac; // exponential fade
                actions.push(MixAction { at_ms: t, kind: MixActionKind::SetGain { deck: out.clone(), gain_db: out_gain } });
            }
            let silence_at = echo_bars * bar_ms;
            actions.push(MixAction { at_ms: silence_at, kind: MixActionKind::Pause { deck: out.clone() } });
            // Hard cut after 1-bar silence
            let cut_at = (echo_bars + 1) * bar_ms;
            actions.push(MixAction { at_ms: cut_at, kind: MixActionKind::SetGain { deck: inc.clone(), gain_db: 0.0 } });
        }
        "drop_swap" => {
            // 4-bar buildup HP sweep, then hard cut to incoming at drop
            let buildup_bars: u64 = 4;
            for i in 0..=(buildup_bars * 4) {
                let t = i * (bar_ms / 4);
                let frac = i as f32 / (buildup_bars * 4) as f32;
                let log_start = 200.0_f32.ln();
                let log_end = 4000.0_f32.ln();
                let cutoff = (log_start + frac * (log_end - log_start)).exp();
                actions.push(MixAction { at_ms: t, kind: MixActionKind::SetFilter { deck: out.clone(), cutoff_hz: cutoff, resonance: 0.7 } });
            }
            let cut_at = buildup_bars * bar_ms;
            actions.push(MixAction { at_ms: cut_at, kind: MixActionKind::Pause { deck: out.clone() } });
            actions.push(MixAction { at_ms: cut_at, kind: MixActionKind::SetFilter { deck: out.clone(), cutoff_hz: 20000.0, resonance: 0.0 } });
            actions.push(MixAction { at_ms: cut_at, kind: MixActionKind::SetGain { deck: inc.clone(), gain_db: 0.0 } });
        }
        "loop_roll" => {
            // 4-bar→2-bar→1-bar loop rolls on outgoing, then cut
            actions.push(MixAction { at_ms: 0, kind: MixActionKind::SetLoop { deck: out.clone(), length_bars: 4 } });
            actions.push(MixAction { at_ms: 4 * bar_ms, kind: MixActionKind::SetLoop { deck: out.clone(), length_bars: 2 } });
            actions.push(MixAction { at_ms: 6 * bar_ms, kind: MixActionKind::SetLoop { deck: out.clone(), length_bars: 1 } });
            let cut_at = 7 * bar_ms;
            actions.push(MixAction { at_ms: cut_at, kind: MixActionKind::ClearLoop { deck: out.clone() } });
            actions.push(MixAction { at_ms: cut_at, kind: MixActionKind::Pause { deck: out.clone() } });
            actions.push(MixAction { at_ms: cut_at, kind: MixActionKind::SetGain { deck: inc.clone(), gain_db: 0.0 } });
        }
        "bass_kill_swap" => {
            // 16-bar blend: bring in mids/highs first, swap bass at bar 8, crossfader movement
            let blend_bars: u64 = 16;
            let swap_bar: u64 = 8;
            actions.push(MixAction { at_ms: 0, kind: MixActionKind::SetEqKill { deck: inc.clone(), band: "low".into(), enabled: true } });
            for i in 0..=blend_bars {
                let t = i * bar_ms;
                let frac = i as f32 / blend_bars as f32;
                let s = s_curve(frac);
                actions.push(MixAction { at_ms: t, kind: MixActionKind::SetGain { deck: inc.clone(), gain_db: -40.0 + 40.0 * s } });
                actions.push(MixAction { at_ms: t, kind: MixActionKind::SetCrossfader { position: 0.5 * s } });
            }
            actions.push(MixAction { at_ms: swap_bar * bar_ms, kind: MixActionKind::SetEqKill { deck: inc.clone(), band: "low".into(), enabled: false } });
            actions.push(MixAction { at_ms: swap_bar * bar_ms, kind: MixActionKind::SetEqKill { deck: out.clone(), band: "low".into(), enabled: true } });
            // Outgoing S-curve fade starting at bar 4
            let fade_start: u64 = 4;
            let fade_len = blend_bars - fade_start;
            for i in fade_start..=blend_bars {
                let t = i * bar_ms;
                let frac = (i - fade_start) as f32 / fade_len as f32;
                actions.push(MixAction { at_ms: t, kind: MixActionKind::SetGain { deck: out.clone(), gain_db: -40.0 * s_curve(frac) } });
            }
            let end = blend_bars * bar_ms;
            actions.push(MixAction { at_ms: end, kind: MixActionKind::SetEqKill { deck: out.clone(), band: "low".into(), enabled: false } });
            actions.push(MixAction { at_ms: end, kind: MixActionKind::SetCrossfader { position: 0.5 } });
            actions.push(MixAction { at_ms: end, kind: MixActionKind::Pause { deck: out.clone() } });
        }
        "break_blend" => {
            // 12-bar gentle blend: HP opening on incoming, S-curve fade on outgoing
            let bars: u64 = 12;
            for i in 0..=bars {
                let t = i * bar_ms;
                let frac = i as f32 / bars as f32;
                let s = s_curve(frac);
                let cutoff = 4000.0 - 3800.0 * s; // HP opening from 4kHz → 200Hz
                actions.push(MixAction { at_ms: t, kind: MixActionKind::SetFilter { deck: inc.clone(), cutoff_hz: cutoff, resonance: 0.3 } });
                actions.push(MixAction { at_ms: t, kind: MixActionKind::SetGain { deck: inc.clone(), gain_db: -40.0 + 40.0 * s } });
                // Gentle outgoing fade starting at 1/3
                if frac > 0.33 {
                    let fade_frac = (frac - 0.33) / 0.67;
                    actions.push(MixAction { at_ms: t, kind: MixActionKind::SetGain { deck: out.clone(), gain_db: -30.0 * s_curve(fade_frac) } });
                }
            }
            let end = bars * bar_ms;
            actions.push(MixAction { at_ms: end, kind: MixActionKind::SetFilter { deck: inc.clone(), cutoff_hz: 20000.0, resonance: 0.0 } });
            actions.push(MixAction { at_ms: end, kind: MixActionKind::Pause { deck: out.clone() } });
        }
        "backspin_cut" => {
            // 2-bar backspin simulation: rapid gain drop + filter close, then silence + cut
            let spin_bars: u64 = 2;
            let spin_beats = spin_bars * 4;
            for i in 0..=spin_beats {
                let t = i * (bar_ms / 4);
                let frac = i as f32 / spin_beats as f32;
                let out_gain = -48.0 * frac * frac; // exponential fade
                let cutoff = 20000.0 * (1.0 - frac).max(0.01);
                actions.push(MixAction { at_ms: t, kind: MixActionKind::SetGain { deck: out.clone(), gain_db: out_gain } });
                actions.push(MixAction { at_ms: t, kind: MixActionKind::SetFilter { deck: out.clone(), cutoff_hz: cutoff, resonance: 1.2 } });
            }
            let kill_at = spin_bars * bar_ms;
            actions.push(MixAction { at_ms: kill_at, kind: MixActionKind::Pause { deck: out.clone() } });
            actions.push(MixAction { at_ms: kill_at, kind: MixActionKind::SetFilter { deck: out.clone(), cutoff_hz: 20000.0, resonance: 0.0 } });
            // Cut to incoming after 1-bar silence
            let cut_at = (spin_bars + 1) * bar_ms;
            actions.push(MixAction { at_ms: cut_at, kind: MixActionKind::SetGain { deck: inc.clone(), gain_db: 0.0 } });
        }
        "drop_ride" => {
            // 8-bar dual-drop ride: bring incoming in hot with bass killed, swap bass at bar 2
            let ride_bars: u64 = 8;
            let swap_bar: u64 = 2;
            actions.push(MixAction { at_ms: 0, kind: MixActionKind::SetEqKill { deck: inc.clone(), band: "low".into(), enabled: true } });
            actions.push(MixAction { at_ms: 0, kind: MixActionKind::SetGain { deck: inc.clone(), gain_db: -12.0 } });
            // Quick ramp incoming to full over 2 bars
            for i in 0..=(swap_bar * 4) {
                let t = i * (bar_ms / 4);
                let frac = i as f32 / (swap_bar * 4) as f32;
                let in_gain = -12.0 + 12.0 * s_curve(frac);
                actions.push(MixAction { at_ms: t, kind: MixActionKind::SetGain { deck: inc.clone(), gain_db: in_gain } });
            }
            // Bass swap
            actions.push(MixAction { at_ms: swap_bar * bar_ms, kind: MixActionKind::SetEqKill { deck: inc.clone(), band: "low".into(), enabled: false } });
            actions.push(MixAction { at_ms: swap_bar * bar_ms, kind: MixActionKind::SetEqKill { deck: out.clone(), band: "low".into(), enabled: true } });
            // Fade outgoing over remaining bars
            let fade_start = swap_bar;
            let fade_len = ride_bars - fade_start;
            for i in fade_start..=ride_bars {
                let t = i * bar_ms;
                let frac = (i - fade_start) as f32 / fade_len as f32;
                actions.push(MixAction { at_ms: t, kind: MixActionKind::SetGain { deck: out.clone(), gain_db: -40.0 * s_curve(frac) } });
            }
            let end = ride_bars * bar_ms;
            actions.push(MixAction { at_ms: end, kind: MixActionKind::SetEqKill { deck: out.clone(), band: "low".into(), enabled: false } });
            actions.push(MixAction { at_ms: end, kind: MixActionKind::Pause { deck: out.clone() } });
        }
        _ => {
            // Unknown template — just execute it directly
            actions.push(MixAction {
                at_ms: 0,
                kind: MixActionKind::ExecuteTemplate {
                    template_name: template.to_string(),
                    params: serde_json::json!({}),
                },
            });
        }
    }

    actions
}
