//! AI Planner delegating decisions between the LLM and the Heuristic Scorer.
//! Phases 31-35, 37-38: Enhanced with transition intelligence, smart entry points,
//! anti-repetition, timing guards, personality profiles, and set arc awareness.

use crate::context::{AiContext, TrackClassificationData};
use crate::llm::LlmClient;
use crate::scorer::{DjPersonality, HeuristicScorer, ScoringCandidate};
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
- fx_echo_slam: 4-bar echo out with feedback on outgoing → 1-bar silence → filter slam up on incoming. High impact for big energy jumps or genre changes.
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
  - echo_out: { deck, beats (1-8), feedback (0.0-0.8) } — echo/delay feedback effect on deck
  - reverb_tail: { deck, decay_seconds (0.5-4.0) } — reverb wash on deck
  - filter_slam: { deck, direction ("up"|"down") } — aggressive instant filter sweep
  - vinyl_brake: { deck, duration_ms (100-5000) } — turntable brake slowdown
  - spinback: { deck, duration_ms (100-5000) } — reverse scratch spin
  - beat_repeat: { deck, beats (0.25-8) } — rhythmic loop stutter
  - beat_repeat_off: { deck } — stop beat repeat
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
    /// Phase 34: entry marker for incoming track (intro, drop, break, first_beat)
    pub entry_marker: Option<String>,
    /// Phase 34: exit marker for outgoing track (outro, break, drop)
    pub exit_marker: Option<String>,
    /// Phase 34: creative intensity (0.0=safe, 1.0=aggressive)
    pub creative_intensity: Option<f64>,
    /// Phase 34: set arc phase (warmup, groove, peak, cooldown)
    pub set_arc: Option<String>,
    /// Phase 33: which planner produced this decision
    pub planner_mode: Option<String>,
    /// Phase 31: template score breakdown for debug
    pub template_scores: Option<Vec<(String, f64)>>,
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
    // Phase 30: DJ Performance actions
    BeatRepeat { deck: String, beats: f32 },
    BeatRepeatOff { deck: String },
    VinylBrake { deck: String, duration_ms: u32 },
    Spinback { deck: String, duration_ms: u32 },
    JumpToMarker { deck: String, marker: String, position_ms: u64 },
    SetSlipMode { deck: String, enabled: bool },
    // Phase 36: Performance FX & Turntablism
    EchoOut { deck: String, beats: f32, feedback: f32 },
    ReverbTail { deck: String, decay_seconds: f32 },
    FilterSlam { deck: String, direction: String },
    Scratch { deck: String, intensity: f32 },
}

pub struct AiPlanner {
    config: AiConfig,
    scorer: HeuristicScorer,
    llm: LlmClient,
    /// Phase 31: Track recently used transition templates for anti-repetition.
    recent_templates: Vec<String>,
    /// Phase 37: Track recently used artist names.
    recent_artists: Vec<String>,
    /// Phase 35: DJ personality profile.
    personality: DjPersonality,
    /// Phase 46: Track recently used performance FX for cooldown.
    recent_fx: Vec<String>,
    /// Phase 48: Track how many times each track has been played in the session.
    track_play_counts: std::collections::HashMap<String, usize>,
}

impl AiPlanner {
    pub fn new(config: AiConfig) -> Self {
        let scorer = HeuristicScorer::new(config.clone());
        let llm = LlmClient::new();
        Self {
            config,
            scorer,
            llm,
            recent_templates: Vec::new(),
            recent_artists: Vec::new(),
            personality: DjPersonality::Smooth,
            recent_fx: Vec::new(),
            track_play_counts: std::collections::HashMap::new(),
        }
    }

    pub fn config(&self) -> &AiConfig {
        &self.config
    }

    pub fn set_config(&mut self, config: AiConfig) {
        self.config = config.clone();
        self.scorer = HeuristicScorer::new(config);
    }

    pub fn set_personality(&mut self, name: &str) {
        self.personality = DjPersonality::from_str(name);
    }

    /// Record a used template for anti-repetition (Phase 31).
    pub fn record_template_use(&mut self, template: &str) {
        self.recent_templates.push(template.to_string());
        if self.recent_templates.len() > 6 {
            self.recent_templates.remove(0);
        }
    }

    /// Record a played artist for variety (Phase 37).
    pub fn record_artist(&mut self, artist: &str) {
        self.recent_artists.push(artist.to_string());
        if self.recent_artists.len() > 10 {
            self.recent_artists.remove(0);
        }
    }

    /// Record a track play for replay tracking (Phase 48).
    pub fn record_track_play(&mut self, track_id: &str) {
        *self.track_play_counts.entry(track_id.to_string()).or_insert(0) += 1;
    }

    /// Check if a track has hit the hard replay cap (3 plays per session).
    pub fn track_at_replay_cap(&self, track_id: &str) -> bool {
        self.track_play_counts.get(track_id).copied().unwrap_or(0) >= 3
    }

    /// Record a used performance FX for cooldown (Phase 46).
    pub fn record_fx_use(&mut self, fx_name: &str) {
        self.recent_fx.push(fx_name.to_string());
        if self.recent_fx.len() > 6 {
            self.recent_fx.remove(0);
        }
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
                Ok(mut decision) => {
                    log::info!("LLM returned decision: {}", decision.decision);
                    decision.planner_mode = Some(format!("llm_live:{}", self.config.provider));
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
            artist: None,
            vibe: current_class.and_then(|c| c.vibe.as_deref()),
        };

        let candidates: Vec<ScoringCandidate> = available_profiles
            .iter()
            .map(|p| {
                let class = classifications.get(&p.track_id);
                ScoringCandidate {
                    profile: p,
                    genre: class.and_then(|c| c.genre.as_deref()),
                    mood: class.and_then(|c| c.mood.as_deref()),
                    artist: None,
                    vibe: class.and_then(|c| c.vibe.as_deref()),
                }
            })
            .collect();

        // Phase 37: compute set position from session flow
        let set_position = context.set_flow.as_ref().map(|f| {
            // Estimate position in set: assume ~20 tracks for a typical set
            let est_total = 20.0_f64;
            (f.tracks_played as f64 / est_total).min(1.0)
        });

        let next_track = self.scorer.select_next_extended(
            &current_candidate,
            &candidates,
            &context.constraints.recent_tracks,
            &context.preferences.intent,
            &self.recent_artists,
            set_position,
            Some(self.personality),
        ).ok_or(AiError::NoCandidates)?;

        // ── Phase 38: Timing guards ──────────────────────────────────────────
        let now_ms = context.current_state.elapsed_ms;
        let duration_ms = context.current_state.duration_ms;
        let position_frac = if duration_ms > 0 { now_ms as f64 / duration_ms as f64 } else { 0.0 };

        // Too-early guard: don't transition before 20% unless it's an emergency
        let too_early = position_frac < 0.20;
        // Too-late guard: avoid dead-end outros — transition before 95%
        let too_late = position_frac > 0.95;

        let is_ending = context.current_state.time_remaining_ms < 60_000;
        let decision = if too_late || is_ending { "transition_now" }
        else if too_early { "wait" }
        else { "prepare_next" };

        // If we should wait, return early
        if decision == "wait" {
            return Ok(AiDecision {
                decision: "wait".to_string(),
                reason: format!("Too early to transition ({}% through track)", (position_frac * 100.0) as u32),
                track_selection: Some(next_track.track_id.clone()),
                execute_at_ms: None,
                transition_plan: None,
                actions: None,
                reasoning_tags: Some(vec!["too_early_guard".to_string()]),
                confidence: Some(0.5),
                entry_marker: None,
                exit_marker: None,
                creative_intensity: None,
                set_arc: set_position.map(|p| {
                    crate::scorer::SetArcPhase::from_set_position(p)
                }).map(|a| format!("{:?}", a).to_lowercase()),
                planner_mode: Some("heuristic_only".to_string()),
                template_scores: None,
            });
        }

        // ── Section-aware transition timing ──────────────────────────────────
        let best_window = current_profile.transition_windows.iter()
            .filter(|w| w.start_ms > now_ms)
            .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal));

        let exit_marker = best_window.map(|w| w.anchor.clone());

        let execute_at = if let Some(w) = best_window {
            if let Some(ref phrases) = current_profile.phrases {
                autodj_analysis::sections::next_phrase_boundary(phrases, w.start_ms)
                    .unwrap_or(w.start_ms)
            } else {
                w.start_ms
            }
        } else {
            duration_ms.saturating_sub(
                context.current_state.outro_length_ms.unwrap_or(10_000)
            )
        };

        // ── Phase 38: Late-groove protection ─────────────────────────────────
        // If current section just became high-engagement, delay transition
        let engagement_hold = best_window.map(|w| {
            w.anchor.contains("drop") && (execute_at.saturating_sub(now_ms) < 8000)
        }).unwrap_or(false);

        let execute_at = if engagement_hold {
            // Hold for 16 more bars then transition
            let bar_ms = (60_000.0 / current_profile.tempo.bpm * 4.0) as u64;
            execute_at + 16 * bar_ms
        } else {
            execute_at
        };

        // ── Template selection ───────────────────────────────────────────────
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

        let near_drop = best_window.map(|w| w.anchor.contains("drop")).unwrap_or(false);
        let near_breakdown = best_window.map(|w| {
            w.anchor.contains("break") || w.anchor.contains("breakdown")
        }).unwrap_or(false);
        let near_outro = best_window.map(|w| w.anchor.contains("outro")).unwrap_or(false);

        let trajectory_building = current_profile.energy_trajectory.as_deref() == Some("building");
        let trajectory_declining = current_profile.energy_trajectory.as_deref() == Some("declining");

        let high_complexity = current_profile.rhythm.as_ref()
            .map(|r| r.complexity > 0.7).unwrap_or(false);
        let high_danceability = current_profile.rhythm.as_ref()
            .map(|r| r.danceability > 0.7).unwrap_or(false);

        let incoming_has_drop = next_track.first_drop_ms.is_some();

        // ── Score each template ──────────────────────────────────────────────
        let mut template_candidates: Vec<(&str, f64, Vec<&str>)> = Vec::new();

        // SmoothBeatmix
        {
            let mut score = 0.5;
            if !genre_change && energy_jump.abs() < 0.15 { score += 0.2; }
            if near_outro { score += 0.1; }
            if bpm_diff < 4.0 { score += 0.1; }
            if vocal_overlap { score -= 0.3; }
            template_candidates.push(("smooth_beatmix", score, vec!["compatible", "smooth_beatmix"]));
        }
        // FilterSweep
        {
            let mut score = 0.3;
            if trajectory_building { score += 0.3; }
            if genre_change { score += 0.25; }
            if energy_jump > 0.1 { score += 0.2; }
            if near_breakdown { score += 0.15; }
            template_candidates.push(("filter_sweep", score, vec!["filter_sweep"]));
        }
        // DropSwap
        {
            let mut score = 0.2;
            if near_drop { score += 0.4; }
            if energy_jump > 0.2 { score += 0.25; }
            if trajectory_building && incoming_has_drop { score += 0.2; }
            if bpm_diff > 10.0 { score -= 0.2; }
            template_candidates.push(("drop_swap", score, vec!["drop_swap"]));
        }
        // EchoDrop
        {
            let mut score = 0.15;
            if energy_jump > 0.3 { score += 0.3; }
            if bpm_diff > 10.0 { score += 0.25; }
            if genre_change && energy_jump.abs() > 0.2 { score += 0.2; }
            template_candidates.push(("echo_drop", score, vec!["echo_drop"]));
        }
        // LoopRoll
        {
            let mut score = 0.15;
            if high_complexity { score += 0.3; }
            if energy_jump > 0.25 { score += 0.2; }
            if bpm_diff > 8.0 { score += 0.15; }
            if near_drop { score += 0.1; }
            template_candidates.push(("loop_roll", score, vec!["loop_roll"]));
        }
        // BassKillSwap
        {
            let mut score = 0.3;
            if !genre_change { score += 0.15; }
            if high_danceability { score += 0.2; }
            if energy_jump.abs() < 0.15 { score += 0.15; }
            if near_outro { score += 0.1; }
            if vocal_overlap { score -= 0.2; }
            template_candidates.push(("bass_kill_swap", score, vec!["bass_kill_swap"]));
        }
        // BreakBlend
        {
            let mut score = 0.25;
            if vocal_overlap { score += 0.4; }
            if trajectory_declining { score += 0.25; }
            if energy_jump < -0.15 { score += 0.2; }
            if near_breakdown { score += 0.15; }
            template_candidates.push(("break_blend", score, vec!["break_blend"]));
        }
        // BackspinCut
        {
            let mut score = 0.1;
            if genre_change { score += 0.25; }
            if energy_jump.abs() > 0.15 && energy_jump.abs() < 0.35 { score += 0.2; }
            if !near_drop && !near_breakdown { score += 0.1; }
            template_candidates.push(("backspin_cut", score, vec!["backspin_cut"]));
        }
        // DropRide
        {
            let mut score = 0.1;
            if incoming_has_drop && near_drop { score += 0.35; }
            if energy_jump.abs() < 0.2 { score += 0.15; }
            if !genre_change { score += 0.1; }
            if high_danceability { score += 0.1; }
            template_candidates.push(("drop_ride", score, vec!["drop_ride"]));
        }
        // Phase 46: FxEchoSlam — echo out → silence → filter slam incoming
        {
            let mut score = 0.1;
            if energy_jump > 0.25 { score += 0.35; }
            if genre_change { score += 0.2; }
            if near_drop && incoming_has_drop { score += 0.15; }
            if bpm_diff > 8.0 { score += 0.1; }
            // Personality boost for aggressive styles
            match self.personality {
                DjPersonality::Hype | DjPersonality::Festival | DjPersonality::Aggressive => { score += 0.15; }
                DjPersonality::Lounge => { score -= 0.3; }
                _ => {}
            }
            template_candidates.push(("fx_echo_slam", score, vec!["fx_echo_slam", "performance_fx"]));
        }

        // Phase 31: Anti-repetition penalty — penalize recently used templates
        for tc in &mut template_candidates {
            let recent_count = self.recent_templates.iter()
                .filter(|t| t.as_str() == tc.0)
                .count();
            if recent_count > 0 {
                tc.1 *= 0.6_f64.powi(recent_count as i32);
            }
        }

        // Phase 35: Personality-based template preferences
        match self.personality {
            DjPersonality::Aggressive | DjPersonality::Festival => {
                for tc in &mut template_candidates {
                    match tc.0 {
                        "drop_swap" | "loop_roll" | "backspin_cut" => tc.1 += 0.1,
                        "smooth_beatmix" | "break_blend" => tc.1 -= 0.05,
                        _ => {}
                    }
                }
            }
            DjPersonality::Lounge => {
                for tc in &mut template_candidates {
                    match tc.0 {
                        "smooth_beatmix" | "break_blend" | "filter_sweep" => tc.1 += 0.1,
                        "drop_swap" | "loop_roll" | "backspin_cut" => tc.1 -= 0.1,
                        _ => {}
                    }
                }
            }
            DjPersonality::Hype => {
                for tc in &mut template_candidates {
                    match tc.0 {
                        "drop_swap" | "drop_ride" | "echo_drop" => tc.1 += 0.08,
                        _ => {}
                    }
                }
            }
            _ => {}
        }

        // Sort descending by score
        template_candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Capture template scores for debug (Phase 39)
        let template_scores: Vec<(String, f64)> = template_candidates.iter()
            .map(|(name, score, _)| (name.to_string(), *score))
            .collect();

        // Add variety: if top 2 scores are within 0.1, randomly pick between them
        let (template_name, _, reasoning_tags) = if template_candidates.len() >= 2
            && (template_candidates[0].1 - template_candidates[1].1) < 0.1
        {
            if context.current_state.elapsed_ms % 2 == 0 {
                template_candidates.remove(1)
            } else {
                template_candidates.remove(0)
            }
        } else {
            template_candidates.remove(0)
        };

        // ── Phase 32+47: Smart entry point selection with section similarity ─
        let entry_marker = select_entry_point(
            next_track,
            template_name,
            energy_jump,
            Some(current_profile),
            Some(now_ms),
        );

        // Creative intensity based on personality + energy context
        let creative_intensity = match self.personality {
            DjPersonality::Aggressive | DjPersonality::Festival => 0.8,
            DjPersonality::Hype => 0.7,
            DjPersonality::OpenFormat => 0.6,
            DjPersonality::Smooth => 0.4,
            DjPersonality::Lounge => 0.2,
        };

        // ── Build action timeline ────────────────────────────────────────────
        let beat_ms = (60_000.0 / current_profile.tempo.bpm) as u64;
        let bar_ms = beat_ms * 4;
        let incoming_deck = if context.current_state.playing_deck == "A" { "B" } else { "A" };
        let outgoing_deck = context.current_state.playing_deck.clone();

        // Phase 32+49: Determine incoming start position from entry point
        let find_cue_ms = |cue_name: &str| -> u64 {
            next_track
                .auto_cues
                .iter()
                .find(|c| c.cue_type == cue_name)
                .map(|c| c.position_ms)
                .unwrap_or(0)
        };
        let incoming_start_ms = match entry_marker.as_deref() {
            Some("drop") => next_track.first_drop_ms.unwrap_or(0),
            Some("break") => {
                // Use section markers if available, else estimate from intro
                next_track.intro_length_ms.map(|i| i + 8000).unwrap_or(0)
            }
            Some("first_beat") => {
                // Skip silence, start at first beat
                next_track.intro_length_ms.unwrap_or(0).min(2000)
            }
            // Phase 49 cue types
            Some("best_mix_in") => find_cue_ms("best_mix_in"),
            Some("vocal_free_start") => find_cue_ms("vocal_free_start"),
            Some("impact") => find_cue_ms("impact"),
            _ => 0, // "intro" or default — start from beginning
        };

        let mut actions = build_heuristic_timeline(
            template_name,
            &outgoing_deck,
            incoming_deck,
            &next_track.track_id,
            bar_ms,
            next_track.tempo.bpm,
            incoming_start_ms,
        );

        // Phase 46: Inject performance FX into the timeline
        inject_transition_fx(
            &mut actions,
            template_name,
            &outgoing_deck,
            incoming_deck,
            bar_ms,
            self.personality,
            &self.recent_fx,
        );

        let arc_label = set_position.map(|p| {
            crate::scorer::SetArcPhase::from_set_position(p)
        }).map(|a| format!("{:?}", a).to_lowercase());

        Ok(AiDecision {
            decision: decision.to_string(),
            reason: format!(
                "Heuristic: selected {} (genre: {}, mood: {}, template: {}, window: {}, entry: {}).",
                next_track.track_id,
                next_class.and_then(|c| c.genre.as_deref()).unwrap_or("unknown"),
                next_class.and_then(|c| c.mood.as_deref()).unwrap_or("unknown"),
                template_name,
                best_window.map(|w| format!("{}ms score={:.2}", w.start_ms, w.score))
                    .unwrap_or_else(|| "fallback".to_string()),
                entry_marker.as_deref().unwrap_or("intro"),
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
            entry_marker: entry_marker.clone(),
            exit_marker,
            creative_intensity: Some(creative_intensity),
            set_arc: arc_label,
            planner_mode: Some("heuristic_only".to_string()),
            template_scores: Some(template_scores),
        })
    }
}

/// Phase 32+47: Select the best entry point for the incoming track based on
/// template, energy delta, section similarity, and available cue/section data.
fn select_entry_point(
    track: &TrackProfile,
    template: &str,
    energy_jump: f64,
    current_profile: Option<&TrackProfile>,
    current_elapsed_ms: Option<u64>,
) -> Option<String> {
    let has_drop = track.first_drop_ms.is_some();
    let has_intro = track.intro_length_ms.is_some();

    // Helper: check if incoming track has a given cue type
    let has_cue = |cue_name: &str| track.auto_cues.iter().any(|c| c.cue_type == cue_name);

    // Template-driven overrides (these are always respected)
    match template {
        // Phase 49: prefer impact cues for drop-style templates if available
        "drop_swap" | "drop_ride" if has_cue("impact") => return Some("impact".to_string()),
        "drop_swap" | "drop_ride" if has_drop => return Some("drop".to_string()),
        "echo_drop" if has_cue("impact") && energy_jump > 0.2 => return Some("impact".to_string()),
        "echo_drop" if has_drop && energy_jump > 0.2 => return Some("drop".to_string()),
        "fx_echo_slam" if has_drop => return Some("drop".to_string()),
        "break_blend" => return Some("intro".to_string()),
        _ => {}
    }

    // Phase 49: If outgoing track has high vocal presence, prefer a vocal-free entry
    if let Some(cur_prof) = current_profile {
        if cur_prof.vocal_presence.unwrap_or(0.0) > 0.5 && has_cue("vocal_free_start") {
            return Some("vocal_free_start".to_string());
        }
    }

    // Phase 49: Prefer best_mix_in if the incoming track has one
    if has_cue("best_mix_in") {
        return Some("best_mix_in".to_string());
    }

    // Phase 47: Section-to-section similarity matching
    if let (Some(cur_prof), Some(elapsed)) = (current_profile, current_elapsed_ms) {
        if !track.sections.is_empty() && !cur_prof.sections.is_empty() {
            // Find what section the outgoing track is currently in
            let current_section = cur_prof.sections.iter()
                .find(|s| elapsed >= s.start_ms && elapsed < s.end_ms);

            if let Some(cur_section) = current_section {
                let cur_energy = autodj_analysis::sections::section_average_energy(
                    &cur_prof.energy.curve, cur_prof.duration_ms,
                    cur_section.start_ms, cur_section.end_ms,
                );

                // Score each section in the incoming track
                let mut best_entry: Option<(&str, f64)> = None;
                for section in &track.sections {
                    let sec_energy = autodj_analysis::sections::section_average_energy(
                        &track.energy.curve, track.duration_ms,
                        section.start_ms, section.end_ms,
                    );

                    // Energy similarity (0-1)
                    let energy_sim = 1.0 - (cur_energy - sec_energy).abs();
                    // Label compatibility (some combos work better)
                    let label_bonus = match (cur_section.label.as_str(), section.label.as_str()) {
                        ("outro", "intro") => 0.3,
                        ("drop", "drop") => 0.25,
                        ("breakdown", "breakdown") | ("breakdown", "intro") => 0.2,
                        ("body", "body") => 0.1,
                        ("drop", "intro") | ("body", "intro") => 0.05,
                        _ => 0.0,
                    };

                    let score = energy_sim * 0.6 + label_bonus + section.confidence * 0.1;
                    if best_entry.map(|(_, s)| score > s).unwrap_or(true) {
                        let label = match section.label.as_str() {
                            "intro" => "intro",
                            "drop" => "drop",
                            "breakdown" => "break",
                            _ => "first_beat",
                        };
                        best_entry = Some((label, score));
                    }
                }

                if let Some((label, _)) = best_entry {
                    return Some(label.to_string());
                }
            }
        }
    }

    // Fallback: energy-based heuristic
    if energy_jump > 0.25 && has_drop { Some("drop".to_string()) }
    else if has_intro { Some("intro".to_string()) }
    else { Some("first_beat".to_string()) }
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
    incoming_start_ms: u64,
) -> Vec<MixAction> {
    let mut actions = Vec::new();
    let out = outgoing.to_string();
    let inc = incoming.to_string();

    // Always: load + cue incoming, sync tempo
    actions.push(MixAction { at_ms: 0, kind: MixActionKind::LoadTrack { deck: inc.clone(), track_id: incoming_track_id.to_string() } });
    actions.push(MixAction { at_ms: 0, kind: MixActionKind::SetTempo { deck: inc.clone(), bpm: incoming_bpm } });
    actions.push(MixAction { at_ms: 0, kind: MixActionKind::SetGain { deck: inc.clone(), gain_db: -40.0 } });
    // Phase 32: Jump to entry point if not starting from zero
    if incoming_start_ms > 0 {
        actions.push(MixAction { at_ms: 0, kind: MixActionKind::JumpToMarker { deck: inc.clone(), marker: "entry".to_string(), position_ms: incoming_start_ms } });
    }
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
        "fx_echo_slam" => {
            // Phase 46: 4-bar echo out → 1-bar silence → filter slam incoming with hard cut
            let echo_bars: u64 = 4;
            // Echo out on outgoing with increasing feedback feel (exponential gain fade)
            actions.push(MixAction { at_ms: 0, kind: MixActionKind::EchoOut { deck: out.clone(), beats: 4.0, feedback: 0.65 } });
            for i in 0..=(echo_bars * 4) {
                let t = i * (bar_ms / 4);
                let frac = i as f32 / (echo_bars * 4) as f32;
                let out_gain = -48.0 * frac * frac;
                actions.push(MixAction { at_ms: t, kind: MixActionKind::SetGain { deck: out.clone(), gain_db: out_gain } });
            }
            let silence_at = echo_bars * bar_ms;
            actions.push(MixAction { at_ms: silence_at, kind: MixActionKind::Pause { deck: out.clone() } });
            // After 1-bar silence, filter slam on incoming + hard cut to full volume
            let slam_at = (echo_bars + 1) * bar_ms;
            actions.push(MixAction { at_ms: slam_at, kind: MixActionKind::FilterSlam { deck: inc.clone(), direction: "up".to_string() } });
            actions.push(MixAction { at_ms: slam_at, kind: MixActionKind::SetGain { deck: inc.clone(), gain_db: 0.0 } });
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

/// Phase 46: Get FX density for a DJ personality (0.0 = no FX, 1.0 = maximum FX).
fn fx_density_for_personality(personality: DjPersonality) -> f64 {
    match personality {
        DjPersonality::Lounge => 0.1,
        DjPersonality::Smooth => 0.3,
        DjPersonality::OpenFormat => 0.6,
        DjPersonality::Hype => 0.8,
        DjPersonality::Festival | DjPersonality::Aggressive => 1.0,
    }
}

/// Phase 46: Inject performance FX into an existing action timeline based on
/// template type, DJ personality, and FX cooldown state.
fn inject_transition_fx(
    actions: &mut Vec<MixAction>,
    template: &str,
    outgoing: &str,
    incoming: &str,
    bar_ms: u64,
    personality: DjPersonality,
    recent_fx: &[String],
) {
    let density = fx_density_for_personality(personality);
    let out = outgoing.to_string();
    let inc = incoming.to_string();

    // Deterministic "randomness" based on bar_ms to decide if FX fires
    let fx_roll = (bar_ms % 100) as f64 / 100.0;
    if fx_roll > density {
        return; // Skip FX for this transition based on personality density
    }

    // Helper: check if an FX was used in last 2 transitions
    let on_cooldown = |fx_name: &str| -> bool {
        recent_fx.iter().rev().take(2).any(|f| f == fx_name)
    };

    match template {
        "echo_drop" => {
            // Add actual EchoOut FX to the exponential fade
            if !on_cooldown("echo_out") {
                let echo_start = bar_ms; // Start echo 1 bar in
                actions.push(MixAction {
                    at_ms: echo_start,
                    kind: MixActionKind::EchoOut { deck: out, beats: 4.0, feedback: 0.6 },
                });
            }
        }
        "backspin_cut" => {
            // Add Spinback (aggressive) or VinylBrake (smooth) before the pause
            let fx_at = bar_ms; // 1 bar into the spin
            match personality {
                DjPersonality::Aggressive | DjPersonality::Hype | DjPersonality::Festival => {
                    if !on_cooldown("spinback") {
                        actions.push(MixAction {
                            at_ms: fx_at,
                            kind: MixActionKind::Spinback { deck: out, duration_ms: (bar_ms * 2) as u32 },
                        });
                    }
                }
                _ => {
                    if !on_cooldown("vinyl_brake") {
                        actions.push(MixAction {
                            at_ms: fx_at,
                            kind: MixActionKind::VinylBrake { deck: out, duration_ms: (bar_ms * 2) as u32 },
                        });
                    }
                }
            }
        }
        "drop_swap" => {
            // FilterSlam on incoming at the cut point for impact
            if !on_cooldown("filter_slam") {
                // Find the cut point (where outgoing pauses)
                let cut_at = 4 * bar_ms;
                actions.push(MixAction {
                    at_ms: cut_at,
                    kind: MixActionKind::FilterSlam { deck: inc, direction: "up".to_string() },
                });
            }
        }
        "loop_roll" => {
            // Replace SetLoop with BeatRepeat + SlipMode for tighter stutter
            if !on_cooldown("beat_repeat") {
                actions.push(MixAction {
                    at_ms: 0,
                    kind: MixActionKind::SetSlipMode { deck: out.clone(), enabled: true },
                });
                actions.push(MixAction {
                    at_ms: 0,
                    kind: MixActionKind::BeatRepeat { deck: out.clone(), beats: 4.0 },
                });
                actions.push(MixAction {
                    at_ms: 4 * bar_ms,
                    kind: MixActionKind::BeatRepeat { deck: out.clone(), beats: 2.0 },
                });
                actions.push(MixAction {
                    at_ms: 6 * bar_ms,
                    kind: MixActionKind::BeatRepeat { deck: out.clone(), beats: 1.0 },
                });
                // Cleanup at cut point
                let cut_at = 7 * bar_ms;
                actions.push(MixAction {
                    at_ms: cut_at,
                    kind: MixActionKind::BeatRepeatOff { deck: out.clone() },
                });
                actions.push(MixAction {
                    at_ms: cut_at,
                    kind: MixActionKind::SetSlipMode { deck: out, enabled: false },
                });
            }
        }
        "break_blend" => {
            // Add ReverbTail on outgoing for a spacious exit
            if !on_cooldown("reverb_tail") {
                let reverb_start = 4 * bar_ms; // Start reverb at 1/3 point
                actions.push(MixAction {
                    at_ms: reverb_start,
                    kind: MixActionKind::ReverbTail { deck: out, decay_seconds: 2.0 },
                });
            }
        }
        "drop_ride" => {
            // FilterSlam on outgoing at the bass swap point for emphasis
            if !on_cooldown("filter_slam") {
                let swap_bar = 2;
                actions.push(MixAction {
                    at_ms: swap_bar * bar_ms,
                    kind: MixActionKind::FilterSlam { deck: out, direction: "down".to_string() },
                });
            }
        }
        // fx_echo_slam already has FX baked into its template — no injection needed
        // smooth_beatmix and bass_kill_swap are intentionally FX-free
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bar_ms(bpm: f64) -> u64 {
        let beat_ms = (60_000.0 / bpm) as u64;
        beat_ms * 4
    }

    #[test]
    fn test_echo_drop_emits_echo_out_fx() {
        let bar_ms = make_bar_ms(128.0);
        let mut actions = build_heuristic_timeline(
            "echo_drop", "A", "B", "track_1", bar_ms, 128.0, 0,
        );
        inject_transition_fx(
            &mut actions, "echo_drop", "A", "B", bar_ms,
            DjPersonality::Hype, &[],
        );
        let has_echo = actions.iter().any(|a| matches!(&a.kind, MixActionKind::EchoOut { .. }));
        assert!(has_echo, "echo_drop template should emit EchoOut FX");
    }

    #[test]
    fn test_backspin_cut_emits_spinback_or_brake() {
        let bar_ms = make_bar_ms(128.0);
        let mut actions = build_heuristic_timeline(
            "backspin_cut", "A", "B", "track_1", bar_ms, 128.0, 0,
        );
        inject_transition_fx(
            &mut actions, "backspin_cut", "A", "B", bar_ms,
            DjPersonality::Aggressive, &[],
        );
        let has_spinback = actions.iter().any(|a| matches!(&a.kind, MixActionKind::Spinback { .. }));
        let has_brake = actions.iter().any(|a| matches!(&a.kind, MixActionKind::VinylBrake { .. }));
        assert!(has_spinback || has_brake, "backspin_cut should emit Spinback or VinylBrake");
    }

    #[test]
    fn test_backspin_cut_lounge_uses_vinyl_brake() {
        let bar_ms = make_bar_ms(120.0);
        let mut actions = build_heuristic_timeline(
            "backspin_cut", "A", "B", "track_1", bar_ms, 120.0, 0,
        );
        inject_transition_fx(
            &mut actions, "backspin_cut", "A", "B", bar_ms,
            DjPersonality::Lounge, &[],
        );
        // Lounge density is 0.1 — bar_ms for 120 BPM = 2000ms, 2000 % 100 = 0, 0/100 = 0.0 ≤ 0.1
        let has_brake = actions.iter().any(|a| matches!(&a.kind, MixActionKind::VinylBrake { .. }));
        assert!(has_brake, "Lounge personality should prefer VinylBrake over Spinback");
    }

    #[test]
    fn test_fx_cooldown_prevents_spam() {
        let bar_ms = make_bar_ms(128.0);
        let recent_fx = vec!["echo_out".to_string(), "echo_out".to_string()];
        let mut actions = build_heuristic_timeline(
            "echo_drop", "A", "B", "track_1", bar_ms, 128.0, 0,
        );
        inject_transition_fx(
            &mut actions, "echo_drop", "A", "B", bar_ms,
            DjPersonality::Hype, &recent_fx,
        );
        let echo_count = actions.iter().filter(|a| matches!(&a.kind, MixActionKind::EchoOut { .. })).count();
        assert_eq!(echo_count, 0, "EchoOut should be skipped when on cooldown");
    }

    #[test]
    fn test_drop_swap_emits_filter_slam() {
        let bar_ms = make_bar_ms(128.0);
        let mut actions = build_heuristic_timeline(
            "drop_swap", "A", "B", "track_1", bar_ms, 128.0, 0,
        );
        inject_transition_fx(
            &mut actions, "drop_swap", "A", "B", bar_ms,
            DjPersonality::Festival, &[],
        );
        let has_slam = actions.iter().any(|a| matches!(&a.kind, MixActionKind::FilterSlam { .. }));
        assert!(has_slam, "drop_swap should emit FilterSlam on incoming");
    }

    #[test]
    fn test_fx_echo_slam_template_has_echo_and_slam() {
        let bar_ms = make_bar_ms(128.0);
        let actions = build_heuristic_timeline(
            "fx_echo_slam", "A", "B", "track_1", bar_ms, 128.0, 0,
        );
        let has_echo = actions.iter().any(|a| matches!(&a.kind, MixActionKind::EchoOut { .. }));
        let has_slam = actions.iter().any(|a| matches!(&a.kind, MixActionKind::FilterSlam { .. }));
        assert!(has_echo, "fx_echo_slam should contain EchoOut");
        assert!(has_slam, "fx_echo_slam should contain FilterSlam");
    }

    #[test]
    fn test_loop_roll_emits_beat_repeat() {
        let bar_ms = make_bar_ms(128.0);
        let mut actions = build_heuristic_timeline(
            "loop_roll", "A", "B", "track_1", bar_ms, 128.0, 0,
        );
        inject_transition_fx(
            &mut actions, "loop_roll", "A", "B", bar_ms,
            DjPersonality::Hype, &[],
        );
        let has_repeat = actions.iter().any(|a| matches!(&a.kind, MixActionKind::BeatRepeat { .. }));
        let has_slip = actions.iter().any(|a| matches!(&a.kind, MixActionKind::SetSlipMode { .. }));
        assert!(has_repeat, "loop_roll should emit BeatRepeat FX");
        assert!(has_slip, "loop_roll should enable SlipMode for BeatRepeat");
    }

    #[test]
    fn test_break_blend_emits_reverb_tail() {
        let bar_ms = make_bar_ms(120.0);
        let mut actions = build_heuristic_timeline(
            "break_blend", "A", "B", "track_1", bar_ms, 120.0, 0,
        );
        inject_transition_fx(
            &mut actions, "break_blend", "A", "B", bar_ms,
            DjPersonality::Smooth, &[],
        );
        // Smooth density = 0.3, bar_ms for 120 BPM = 2000, 2000 % 100 = 0, 0.0 ≤ 0.3
        let has_reverb = actions.iter().any(|a| matches!(&a.kind, MixActionKind::ReverbTail { .. }));
        assert!(has_reverb, "break_blend should emit ReverbTail for Smooth personality");
    }

    #[test]
    fn test_smooth_beatmix_no_fx() {
        let bar_ms = make_bar_ms(128.0);
        let mut actions = build_heuristic_timeline(
            "smooth_beatmix", "A", "B", "track_1", bar_ms, 128.0, 0,
        );
        inject_transition_fx(
            &mut actions, "smooth_beatmix", "A", "B", bar_ms,
            DjPersonality::Smooth, &[],
        );
        let has_fx = actions.iter().any(|a| matches!(&a.kind,
            MixActionKind::EchoOut { .. } |
            MixActionKind::ReverbTail { .. } |
            MixActionKind::FilterSlam { .. } |
            MixActionKind::Spinback { .. } |
            MixActionKind::VinylBrake { .. }
        ));
        assert!(!has_fx, "smooth_beatmix should not have performance FX");
    }

    #[test]
    fn test_fx_density_values() {
        assert!(fx_density_for_personality(DjPersonality::Lounge) < 0.2);
        assert!(fx_density_for_personality(DjPersonality::Smooth) < 0.5);
        assert!(fx_density_for_personality(DjPersonality::Hype) > 0.7);
        assert!((fx_density_for_personality(DjPersonality::Festival) - 1.0).abs() < f64::EPSILON);
    }
}
