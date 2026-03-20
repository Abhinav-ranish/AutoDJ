//! Backend conductor loop for AutoDJ.
//!
//! Owns the autonomous DJ state machine that was previously in AiBar.tsx.
//! Monitors playback, triggers planning, loads tracks, aligns to phrase
//! boundaries, and executes transitions — all on the backend.
//!
//! The frontend receives `ConductorSnapshot` events and renders state.
//! It never owns timing or decision logic.

use crate::state::{AppState, DjMode};
use autodj_ai::context::{build_context, ContextInput, TrackClassificationData};
use autodj_ai::planner::{AiDecision, MixActionKind};
use autodj_core::buffer::TrackBuffer;
use autodj_core::command::{Command, CommandSource, DeckId};
use autodj_core::engine::EngineStateSnapshot;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

/// Conductor phase — the backend state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConductorPhase {
    Idle,
    Monitoring,
    Planning,
    Loading,
    Aligning,
    Transitioning,
}

/// Why the conductor decided to trigger transition planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConductorTriggerKind {
    InitialLoad,
    TransitionWindow,
    PositionFallback,
    SilenceRecovery,
    EmergencyGuard,
}

/// Structured trigger metadata for runtime explainability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConductorTriggerMeta {
    pub kind: ConductorTriggerKind,
    pub reason: String,
    pub anchor: Option<String>,
    pub window_start_ms: Option<u64>,
    pub observed_position_ms: Option<u64>,
    pub observed_position_fraction: Option<f64>,
}

/// Structured decision metadata for runtime explainability.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConductorDecisionMeta {
    pub source: String,
    pub fallback_reason: Option<String>,
    pub selected_track_id: Option<String>,
    pub transition_template: Option<String>,
    pub template_is_simulated: Option<bool>,
    pub entry_marker: Option<String>,
    pub execute_at_ms: Option<u64>,
    pub confidence: Option<f64>,
    pub template_scores: Option<Vec<(String, f64)>>,
    pub reasoning_tags: Option<Vec<String>>,
    pub action_timeline_available: bool,
}

impl ConductorDecisionMeta {
    fn from_ai_decision(decision: &AiDecision) -> Self {
        let template = decision
            .transition_plan
            .as_ref()
            .map(|p| p.template_name.clone());
        Self {
            source: decision
                .planner_mode
                .clone()
                .unwrap_or_else(|| "heuristic_only".to_string()),
            fallback_reason: decision.fallback_reason.clone(),
            selected_track_id: decision.track_selection.clone(),
            transition_template: template.clone(),
            template_is_simulated: template.as_ref().map(|name| is_simulated_template(name)),
            entry_marker: decision.entry_marker.clone(),
            execute_at_ms: decision.execute_at_ms,
            confidence: decision.confidence,
            template_scores: decision.template_scores.clone(),
            reasoning_tags: decision.reasoning_tags.clone(),
            action_timeline_available: decision
                .actions
                .as_ref()
                .map(|a| !a.is_empty())
                .unwrap_or(false),
        }
    }
}

/// Snapshot of the conductor's internal state, emitted to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConductorSnapshot {
    pub phase: ConductorPhase,
    pub message: String,
    pub trigger_reason: Option<String>,
    /// Structured trigger metadata for this/last transition decision.
    pub trigger_meta: Option<ConductorTriggerMeta>,
    /// Which planner produced the last decision: "llm_live:<provider>" or "heuristic_only"
    pub decision_source: Option<String>,
    /// Structured decision metadata for the active/last transition decision.
    pub decision_meta: Option<ConductorDecisionMeta>,
    /// Why heuristic fallback was used (if applicable)
    pub fallback_reason: Option<String>,
    /// Last chosen transition template
    pub template_chosen: Option<String>,
    /// Entry marker for incoming track
    pub entry_marker: Option<String>,
    /// Track ID of the pending next track
    pub pending_track_id: Option<String>,
    /// Title of the pending next track
    pub pending_track_title: Option<String>,
    /// Confidence of the last decision
    pub confidence: Option<f64>,
    /// Template score breakdown [name, score] for the last decision
    pub template_scores: Option<Vec<(String, f64)>>,
    /// Reasoning tags from the last decision
    pub reasoning_tags: Option<Vec<String>>,
    /// Is the conductor active (mode == autodj)?
    pub active: bool,
    /// Error state
    pub error: Option<String>,
}

impl Default for ConductorSnapshot {
    fn default() -> Self {
        Self {
            phase: ConductorPhase::Idle,
            message: String::new(),
            trigger_reason: None,
            trigger_meta: None,
            decision_source: None,
            decision_meta: None,
            fallback_reason: None,
            template_chosen: None,
            entry_marker: None,
            pending_track_id: None,
            pending_track_title: None,
            confidence: None,
            template_scores: None,
            reasoning_tags: None,
            active: false,
            error: None,
        }
    }
}

/// Mutable conductor state held behind a RwLock in AppState.
pub struct ConductorState {
    pub phase: ConductorPhase,
    pub pending_track_id: Option<String>,
    pub transition_lock: bool,
    pub cached_profile: Option<serde_json::Value>,
    pub cached_profile_track_id: Option<String>,
    pub last_decision: Option<AiDecision>,
    pub last_trigger_reason: Option<String>,
    pub last_trigger_meta: Option<ConductorTriggerMeta>,
    pub last_decision_meta: Option<ConductorDecisionMeta>,
    pub last_message: String,
    pub last_error: Option<String>,
    /// Playlist queue (ordered track IDs)
    pub queue: Vec<String>,
    pub queue_index: usize,
    pub last_planning_instant: Option<std::time::Instant>,
}

impl Default for ConductorState {
    fn default() -> Self {
        Self {
            phase: ConductorPhase::Idle,
            pending_track_id: None,
            transition_lock: false,
            cached_profile: None,
            cached_profile_track_id: None,
            last_decision: None,
            last_trigger_reason: None,
            last_trigger_meta: None,
            last_decision_meta: None,
            last_message: String::new(),
            last_error: None,
            queue: Vec::new(),
            queue_index: 0,
            last_planning_instant: None,
        }
    }
}

impl ConductorState {
    pub fn snapshot(&self, active: bool) -> ConductorSnapshot {
        let decision = self.last_decision.as_ref();
        let decision_meta = self
            .last_decision_meta
            .clone()
            .or_else(|| decision.map(ConductorDecisionMeta::from_ai_decision));
        let decision_source = decision_meta
            .as_ref()
            .map(|m| m.source.clone())
            .or_else(|| decision.and_then(|d| d.planner_mode.clone()));
        let fallback_reason = decision_meta
            .as_ref()
            .and_then(|m| m.fallback_reason.clone())
            .or_else(|| decision.and_then(|d| d.fallback_reason.clone()));
        let template_chosen = decision_meta
            .as_ref()
            .and_then(|m| m.transition_template.clone())
            .or_else(|| {
                decision.and_then(|d| d.transition_plan.as_ref().map(|t| t.template_name.clone()))
            });
        let entry_marker = decision_meta
            .as_ref()
            .and_then(|m| m.entry_marker.clone())
            .or_else(|| decision.and_then(|d| d.entry_marker.clone()));
        let confidence = decision_meta
            .as_ref()
            .and_then(|m| m.confidence)
            .or_else(|| decision.and_then(|d| d.confidence));
        let template_scores = decision_meta
            .as_ref()
            .and_then(|m| m.template_scores.clone())
            .or_else(|| decision.and_then(|d| d.template_scores.clone()));
        let reasoning_tags = decision_meta
            .as_ref()
            .and_then(|m| m.reasoning_tags.clone())
            .or_else(|| decision.and_then(|d| d.reasoning_tags.clone()));
        ConductorSnapshot {
            phase: self.phase,
            message: self.last_message.clone(),
            trigger_reason: self.last_trigger_reason.clone(),
            trigger_meta: self.last_trigger_meta.clone(),
            decision_source,
            decision_meta,
            fallback_reason,
            template_chosen,
            entry_marker,
            pending_track_id: self.pending_track_id.clone(),
            pending_track_title: None, // filled in by the tick
            confidence,
            template_scores,
            reasoning_tags,
            active,
            error: self.last_error.clone(),
        }
    }

    /// Advance the playlist queue, returning the next track ID (or None).
    pub fn advance_queue(&mut self) -> Option<String> {
        if self.queue_index >= self.queue.len() {
            return None;
        }
        let track_id = self.queue[self.queue_index].clone();
        self.queue_index += 1;
        Some(track_id)
    }

    fn reset(&mut self) {
        self.phase = ConductorPhase::Idle;
        self.pending_track_id = None;
        self.transition_lock = false;
        self.cached_profile = None;
        self.cached_profile_track_id = None;
        self.last_decision = None;
        self.last_trigger_reason = None;
        self.last_trigger_meta = None;
        self.last_decision_meta = None;
        self.last_message.clear();
        self.last_error = None;
    }
}

/// Spawn the conductor background task. Call once at app startup.
pub fn spawn_conductor(handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        conductor_loop(handle).await;
    });
}

/// The main conductor loop — runs forever, ticking at ~500ms when in autodj mode.
async fn conductor_loop(handle: AppHandle) {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let state = handle.state::<AppState>();

        // Check mode
        let mode = {
            let m = state.dj_mode.read().unwrap_or_else(|e| e.into_inner());
            m.clone()
        };

        let active = mode == DjMode::Autodj;

        // Handle mode transitions
        {
            let mut cond = state.conductor.write().await;
            if !active {
                if cond.phase != ConductorPhase::Idle {
                    cond.reset();
                    let snap = cond.snapshot(false);
                    let _ = handle.emit("conductor-state", &snap);
                }
                continue;
            }
            // If we just entered autodj mode
            if cond.phase == ConductorPhase::Idle && active {
                cond.phase = ConductorPhase::Monitoring;
                cond.last_message = "Initializing AutoDJ...".to_string();
                let snap = cond.snapshot(true);
                let _ = handle.emit("conductor-state", &snap);
            }
        }

        // Get engine state
        let es = match state.latest_state() {
            Some(s) => s,
            None => continue,
        };

        // Run the tick
        let result = conductor_tick(&handle, &state, &es).await;
        if let Err(e) = result {
            log::error!("[conductor] tick error: {}", e);
            let mut cond = state.conductor.write().await;
            cond.last_error = Some(e.to_string());
        }

        // Emit snapshot
        {
            let cond = state.conductor.read().await;
            let mut snap = cond.snapshot(true);
            // Resolve pending track title
            if let Some(ref tid) = snap.pending_track_id {
                if let Ok(db) = state.db.lock() {
                    if let Ok(Some(track)) = db.get_track(tid) {
                        snap.pending_track_title = track.title;
                    }
                }
            }
            let _ = handle.emit("conductor-state", &snap);
        }
    }
}

/// Single tick of the conductor state machine.
async fn conductor_tick(
    handle: &AppHandle,
    app_state: &AppState,
    es: &EngineStateSnapshot,
) -> Result<(), String> {
    let deck_a = &es.deck_a;
    let deck_b = &es.deck_b;
    let a_playing = deck_a.state == "playing";
    let b_playing = deck_b.state == "playing";

    // Get analyzed tracks
    let analyzed_tracks = {
        let db = app_state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
        db.get_all_tracks()
            .map_err(|e| format!("DB: {e}"))?
            .into_iter()
            .filter(|t| t.import_status == autodj_storage::db::ImportStatus::Analyzed)
            .collect::<Vec<_>>()
    };

    let mut cond = app_state.conductor.write().await;

    // ── Nothing playing ─────────────────────────────────────────────
    if !a_playing && !b_playing {
        if cond.phase == ConductorPhase::Transitioning {
            return Ok(());
        }
        if analyzed_tracks.is_empty() {
            cond.last_error = Some("No analyzed tracks in library".to_string());
            return Ok(());
        }
        if cond.phase == ConductorPhase::Idle || cond.phase == ConductorPhase::Monitoring {
            cond.phase = ConductorPhase::Planning;
            cond.last_message = "Selecting first track...".to_string();

            let queue_pick = cond.advance_queue().and_then(|qid| {
                analyzed_tracks
                    .iter()
                    .find(|t| t.id == qid)
                    .map(|t| t.id.clone())
            });
            let pick_id = queue_pick.clone().unwrap_or_else(|| {
                let idx = pseudo_random_index(analyzed_tracks.len(), es.sample_pos);
                analyzed_tracks[idx].id.clone()
            });
            cond.last_trigger_meta = Some(ConductorTriggerMeta {
                kind: ConductorTriggerKind::InitialLoad,
                reason: "initial_load".to_string(),
                anchor: None,
                window_start_ms: None,
                observed_position_ms: Some(0),
                observed_position_fraction: Some(0.0),
            });
            cond.last_trigger_reason = Some("initial_load".to_string());
            cond.last_decision_meta = Some(ConductorDecisionMeta {
                source: if queue_pick.is_some() {
                    "queue_only".to_string()
                } else {
                    "runtime_fallback".to_string()
                },
                fallback_reason: if queue_pick.is_some() {
                    Some("queue_seed".to_string())
                } else {
                    Some("initial_random_seed".to_string())
                },
                selected_track_id: Some(pick_id.clone()),
                transition_template: None,
                template_is_simulated: None,
                entry_marker: None,
                execute_at_ms: None,
                confidence: None,
                template_scores: None,
                reasoning_tags: None,
                action_timeline_available: false,
            });

            // Load and play
            drop(cond);
            load_track_on_deck(app_state, DeckId::A, &pick_id).await?;
            record_runtime_memory(app_state, Some(&pick_id), None, None).await;
            let mut cond = app_state.conductor.write().await;
            cond.phase = ConductorPhase::Monitoring;
            cond.last_message = "Playing first track".to_string();
        }
        return Ok(());
    }

    // ── Determine active/idle decks ─────────────────────────────────
    let playing_deck: DeckId = if a_playing { DeckId::A } else { DeckId::B };
    let idle_deck: DeckId = if playing_deck == DeckId::A {
        DeckId::B
    } else {
        DeckId::A
    };
    let playing_snap = if a_playing { deck_a } else { deck_b };
    let idle_snap = if a_playing { deck_b } else { deck_a };
    let position_frac = playing_snap.position_fraction;
    let position_ms = playing_snap.position_ms;

    // ── Cache playing track profile ─────────────────────────────────
    let playing_track_id = playing_snap.track_id.clone();
    if let Some(ref tid) = playing_track_id {
        if cond.cached_profile_track_id.as_deref() != Some(tid) {
            if let Ok(db) = app_state.db.lock() {
                if let Ok(Some(json)) = db.get_track_profile_json(tid) {
                    if let Ok(profile) = serde_json::from_str::<serde_json::Value>(&json) {
                        cond.cached_profile = Some(profile);
                        cond.cached_profile_track_id = Some(tid.clone());
                    }
                }
            }
        }
    }

    let profile = cond.cached_profile.clone();

    // ── Silence detection ───────────────────────────────────────────
    if es.silence_frames > 90 && !cond.transition_lock {
        cond.phase = ConductorPhase::Planning;
        cond.last_message = "Silence detected — loading next track...".to_string();
        cond.last_trigger_reason = Some("silence_recovery".to_string());
        cond.last_trigger_meta = Some(ConductorTriggerMeta {
            kind: ConductorTriggerKind::SilenceRecovery,
            reason: "silence_recovery".to_string(),
            anchor: None,
            window_start_ms: None,
            observed_position_ms: Some(position_ms),
            observed_position_fraction: Some(position_frac),
        });

        let others: Vec<_> = analyzed_tracks
            .iter()
            .filter(|t| playing_track_id.as_deref() != Some(&t.id))
            .collect();

        if !others.is_empty() {
            let pick_id = {
                let idx = pseudo_random_index(others.len(), es.sample_pos);
                others[idx].id.clone()
            };

            drop(cond);
            load_track_on_deck(app_state, idle_deck, &pick_id).await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            record_runtime_memory(app_state, Some(&pick_id), Some("filter_sweep"), None).await;

            let mut cond = app_state.conductor.write().await;
            cond.transition_lock = true;
            cond.phase = ConductorPhase::Transitioning;
            cond.last_decision = None;
            cond.last_decision_meta = Some(ConductorDecisionMeta {
                source: "runtime_fallback".to_string(),
                fallback_reason: Some("silence_recovery".to_string()),
                selected_track_id: Some(pick_id.clone()),
                transition_template: Some("filter_sweep".to_string()),
                template_is_simulated: Some(false),
                entry_marker: None,
                execute_at_ms: None,
                confidence: None,
                template_scores: None,
                reasoning_tags: Some(vec!["silence_recovery".to_string()]),
                action_timeline_available: false,
            });

            let deck_str = if idle_deck == DeckId::A { "A" } else { "B" };
            app_state.send_command(
                Command::ExecuteTransition {
                    template: "filter_sweep".to_string(),
                    params: serde_json::json!({ "incoming_deck": deck_str, "duration_bars": 4 }),
                },
                CommandSource::System,
            );

            // Schedule unlock — use cloned values, not references to AppState
            schedule_transition_unlock(
                handle.clone(),
                tokio::time::Duration::from_secs(3),
                "Recovered from silence".to_string(),
            );
        }
        return Ok(());
    }

    // ── Emergency transition (position > 92% and not yet transitioning) ──
    if cond.phase != ConductorPhase::Monitoring {
        if position_frac > 0.92 && !cond.transition_lock {
            if idle_snap.state != "empty" && idle_snap.state != "playing" {
                cond.transition_lock = true;
                cond.phase = ConductorPhase::Transitioning;
                cond.last_message = "Emergency transition!".to_string();
                cond.last_trigger_reason = Some("emergency:>92%".to_string());
                cond.last_trigger_meta = Some(ConductorTriggerMeta {
                    kind: ConductorTriggerKind::EmergencyGuard,
                    reason: "emergency:>92%".to_string(),
                    anchor: None,
                    window_start_ms: None,
                    observed_position_ms: Some(position_ms),
                    observed_position_fraction: Some(position_frac),
                });
                cond.last_decision = None;
                cond.last_decision_meta = Some(ConductorDecisionMeta {
                    source: "runtime_fallback".to_string(),
                    fallback_reason: Some("emergency_guard".to_string()),
                    selected_track_id: idle_snap.track_id.clone(),
                    transition_template: Some("filter_sweep".to_string()),
                    template_is_simulated: Some(false),
                    entry_marker: None,
                    execute_at_ms: None,
                    confidence: None,
                    template_scores: None,
                    reasoning_tags: Some(vec!["emergency_guard".to_string()]),
                    action_timeline_available: false,
                });

                let deck_str = if idle_deck == DeckId::A { "A" } else { "B" };
                app_state.send_command(
                    Command::ExecuteTransition {
                        template: "filter_sweep".to_string(),
                        params: serde_json::json!({ "incoming_deck": deck_str, "duration_bars": 8 }),
                    },
                    CommandSource::System,
                );
                drop(cond);
                record_runtime_memory(
                    app_state,
                    idle_snap.track_id.as_deref(),
                    Some("filter_sweep"),
                    None,
                )
                .await;

                schedule_transition_unlock(
                    handle.clone(),
                    tokio::time::Duration::from_secs(5),
                    "Monitoring playback...".to_string(),
                );
            }
        }
        return Ok(());
    }

    // ── Section-aware transition trigger ─────────────────────────────
    let mut should_trigger = false;
    let mut trigger_reason = String::new();
    let mut trigger_meta: Option<ConductorTriggerMeta> = None;

    if let Some(ref profile_val) = profile {
        if let Some(windows) = profile_val
            .get("transition_windows")
            .and_then(|v| v.as_array())
        {
            let future_windows: Vec<&serde_json::Value> = windows
                .iter()
                .filter(|w| {
                    let start = w.get("start_ms").and_then(|v| v.as_u64()).unwrap_or(0);
                    start > position_ms && start - position_ms < 30_000
                })
                .collect();

            // Sort by score descending
            let mut sorted = future_windows;
            sorted.sort_by(|a, b| {
                let sa = a.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let sb = b.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
            });

            if let Some(best) = sorted.first() {
                let start = best.get("start_ms").and_then(|v| v.as_u64()).unwrap_or(0);
                let anchor = best
                    .get("anchor")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                if start - position_ms < 2000 {
                    should_trigger = true;
                    trigger_reason = format!("window:{}@{}ms", anchor, start);
                    trigger_meta = Some(ConductorTriggerMeta {
                        kind: ConductorTriggerKind::TransitionWindow,
                        reason: trigger_reason.clone(),
                        anchor: Some(anchor.to_string()),
                        window_start_ms: Some(start),
                        observed_position_ms: Some(position_ms),
                        observed_position_fraction: Some(position_frac),
                    });
                }
            } else if position_frac >= 0.75 {
                should_trigger = true;
                trigger_reason = "fallback:75%".to_string();
                trigger_meta = Some(ConductorTriggerMeta {
                    kind: ConductorTriggerKind::PositionFallback,
                    reason: trigger_reason.clone(),
                    anchor: None,
                    window_start_ms: None,
                    observed_position_ms: Some(position_ms),
                    observed_position_fraction: Some(position_frac),
                });
            }
        } else if position_frac >= 0.75 {
            should_trigger = true;
            trigger_reason = "classic:75%".to_string();
            trigger_meta = Some(ConductorTriggerMeta {
                kind: ConductorTriggerKind::PositionFallback,
                reason: trigger_reason.clone(),
                anchor: None,
                window_start_ms: None,
                observed_position_ms: Some(position_ms),
                observed_position_fraction: Some(position_frac),
            });
        }
    } else if position_frac >= 0.75 {
        should_trigger = true;
        trigger_reason = "no_profile:75%".to_string();
        trigger_meta = Some(ConductorTriggerMeta {
            kind: ConductorTriggerKind::PositionFallback,
            reason: trigger_reason.clone(),
            anchor: None,
            window_start_ms: None,
            observed_position_ms: Some(position_ms),
            observed_position_fraction: Some(position_frac),
        });
    }

    let cooldown_ok = cond.last_planning_instant
        .map(|t| t.elapsed() > std::time::Duration::from_secs(15))
        .unwrap_or(true);

    if !should_trigger || cond.transition_lock || !cooldown_ok {
        return Ok(());
    }

    // ── Planning phase ──────────────────────────────────────────────
    cond.phase = ConductorPhase::Planning;
    cond.last_trigger_reason = Some(trigger_reason.clone());
    cond.last_trigger_meta = trigger_meta;

    let next_track_id: String;
    let mut decision_for_memory: Option<AiDecision> = None;

    // Try playlist queue first
    if let Some(queue_id) = cond.advance_queue() {
        cond.last_message = "Queue: next track".to_string();
        next_track_id = queue_id;
        cond.last_decision = None;
        cond.last_decision_meta = Some(ConductorDecisionMeta {
            source: "queue_only".to_string(),
            fallback_reason: Some("queue_override".to_string()),
            selected_track_id: Some(next_track_id.clone()),
            transition_template: None,
            template_is_simulated: None,
            entry_marker: None,
            execute_at_ms: None,
            confidence: None,
            template_scores: None,
            reasoning_tags: Some(vec!["queue_override".to_string()]),
            action_timeline_available: false,
        });
    } else {
        cond.last_message = "AI selecting...".to_string();

        // Release conductor lock for the async planner call
        drop(cond);

        let decision = request_next_track_internal(app_state, es).await;

        let mut cond = app_state.conductor.write().await;
        cond.last_planning_instant = Some(std::time::Instant::now());

        match decision {
            Ok(d) => {
                let mut meta = ConductorDecisionMeta::from_ai_decision(&d);
                let tid = d.track_selection.clone();
                cond.last_decision = Some(d.clone());
                decision_for_memory = Some(d.clone());
                if let Some(tid) = tid {
                    next_track_id = tid;
                    meta.selected_track_id = Some(next_track_id.clone());
                } else {
                    // AI returned no track — pick random
                    let others: Vec<_> = analyzed_tracks
                        .iter()
                        .filter(|t| playing_track_id.as_deref() != Some(&t.id))
                        .collect();
                    if others.is_empty() {
                        cond.phase = ConductorPhase::Monitoring;
                        cond.last_error = Some("No tracks to transition to".to_string());
                        return Ok(());
                    }
                    let idx = pseudo_random_index(others.len(), es.sample_pos);
                    next_track_id = others[idx].id.clone();
                    meta.selected_track_id = Some(next_track_id.clone());
                    if meta.fallback_reason.is_none() {
                        meta.fallback_reason = Some("insufficient_confidence".to_string());
                    }
                }
                cond.last_decision_meta = Some(meta);
            }
            Err(e) => {
                log::warn!("[conductor] AI planning failed: {} — picking random", e);
                let others: Vec<_> = analyzed_tracks
                    .iter()
                    .filter(|t| playing_track_id.as_deref() != Some(&t.id))
                    .collect();
                if others.is_empty() {
                    cond.phase = ConductorPhase::Monitoring;
                    cond.last_error = Some("No tracks to transition to".to_string());
                    return Ok(());
                }
                let idx = pseudo_random_index(others.len(), es.sample_pos);
                next_track_id = others[idx].id.clone();
                cond.last_decision = None;
                cond.last_decision_meta = Some(ConductorDecisionMeta {
                    source: "runtime_fallback".to_string(),
                    fallback_reason: Some("planner_error".to_string()),
                    selected_track_id: Some(next_track_id.clone()),
                    transition_template: None,
                    template_is_simulated: None,
                    entry_marker: None,
                    execute_at_ms: None,
                    confidence: None,
                    template_scores: None,
                    reasoning_tags: Some(vec!["planner_error".to_string()]),
                    action_timeline_available: false,
                });
            }
        }
    }

    // ── Loading phase ───────────────────────────────────────────────
    {
        let mut cond = app_state.conductor.write().await;
        cond.pending_track_id = Some(next_track_id.clone());
        cond.phase = ConductorPhase::Loading;
        cond.last_message = "Loading next track...".to_string();
    }

    load_track_on_deck(app_state, idle_deck, &next_track_id).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    // ── Phrase alignment ────────────────────────────────────────────
    if let Some(ref profile_val) = profile {
        if let Some(phrases) = profile_val.get("phrases") {
            if let Some(boundaries) = phrases.get("boundaries_ms").and_then(|v| v.as_array()) {
                let boundaries_ms: Vec<u64> =
                    boundaries.iter().filter_map(|v| v.as_u64()).collect();

                if !boundaries_ms.is_empty() {
                    let mut cond = app_state.conductor.write().await;
                    cond.phase = ConductorPhase::Aligning;
                    cond.last_message = "Aligning to phrase boundary...".to_string();
                    drop(cond);

                    // Get current position
                    let cur_pos = app_state
                        .latest_state()
                        .map(|s| {
                            let snap = if playing_deck == DeckId::A {
                                &s.deck_a
                            } else {
                                &s.deck_b
                            };
                            snap.position_ms
                        })
                        .unwrap_or(position_ms);

                    if let Some(&next_boundary) = boundaries_ms.iter().find(|&&b| b > cur_pos) {
                        if next_boundary - cur_pos < 15_000 {
                            // Wait for phrase boundary
                            let deadline =
                                tokio::time::Instant::now() + tokio::time::Duration::from_secs(15);

                            loop {
                                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                                if tokio::time::Instant::now() > deadline {
                                    break;
                                }

                                if let Some(snap) = app_state.latest_state() {
                                    let now_pos = if playing_deck == DeckId::A {
                                        snap.deck_a.position_ms
                                    } else {
                                        snap.deck_b.position_ms
                                    };
                                    if now_pos >= next_boundary.saturating_sub(100) {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // ── Transitioning phase ─────────────────────────────────────────
    let (template_name, duration_bars, transition_ms) = {
        let cond = app_state.conductor.read().await;
        let plan = cond
            .last_decision
            .as_ref()
            .and_then(|d| d.transition_plan.as_ref());
        let template = plan
            .map(|p| p.template_name.clone())
            .unwrap_or_else(|| "smooth_beatmix".to_string());
        let overrides = plan.and_then(|p| p.parameter_overrides.as_ref());
        let dur_bars = overrides
            .and_then(|o| o.get("duration_bars").and_then(|v| v.as_u64()))
            .unwrap_or(16) as u32;

        let bpm = es.master_bpm.max(60.0);
        let t_ms = (dur_bars as f64 / bpm) * 60.0 * 4.0 * 1000.0;

        (template, dur_bars, t_ms.min(60_000.0) as u64)
    };

    {
        let mut cond = app_state.conductor.write().await;
        cond.transition_lock = true;
        cond.phase = ConductorPhase::Transitioning;
        cond.last_message = format!("{} → transitioning", template_name);
        if cond.last_decision_meta.is_none() {
            cond.last_decision_meta = Some(ConductorDecisionMeta {
                source: "runtime_fallback".to_string(),
                fallback_reason: Some("default_template".to_string()),
                selected_track_id: Some(next_track_id.clone()),
                transition_template: Some(template_name.clone()),
                template_is_simulated: Some(is_simulated_template(&template_name)),
                entry_marker: None,
                execute_at_ms: None,
                confidence: None,
                template_scores: None,
                reasoning_tags: None,
                action_timeline_available: false,
            });
        } else if let Some(meta) = cond.last_decision_meta.as_mut() {
            if meta.selected_track_id.is_none() {
                meta.selected_track_id = Some(next_track_id.clone());
            }
            if meta.transition_template.is_none() {
                meta.transition_template = Some(template_name.clone());
            }
            meta.template_is_simulated = Some(is_simulated_template(&template_name));
            if let Some(decision) = decision_for_memory.as_ref() {
                meta.action_timeline_available = decision
                    .actions
                    .as_ref()
                    .map(|a| !a.is_empty())
                    .unwrap_or(false);
            }
        }
    }

    let deck_str = if idle_deck == DeckId::A { "A" } else { "B" };
    app_state.send_command(
        Command::ExecuteTransition {
            template: template_name.clone(),
            params: serde_json::json!({
                "incoming_deck": deck_str,
                "duration_bars": duration_bars,
            }),
        },
        CommandSource::System,
    );
    record_runtime_memory(
        app_state,
        Some(&next_track_id),
        Some(&template_name),
        decision_for_memory.as_ref(),
    )
    .await;

    // Schedule transition completion
    schedule_transition_unlock(
        handle.clone(),
        tokio::time::Duration::from_millis(transition_ms + 1000),
        "Monitoring playback...".to_string(),
    );

    Ok(())
}

/// Returns true when a transition template is an approximation rather than true DSP FX.
fn is_simulated_template(template: &str) -> bool {
    matches!(template, "echo_drop" | "backspin_cut" | "fx_echo_slam")
}

/// Record runtime outcomes back into planner memory so anti-repetition logic has real data.
async fn record_runtime_memory(
    app_state: &AppState,
    selected_track_id: Option<&str>,
    template_name: Option<&str>,
    decision: Option<&AiDecision>,
) {
    let artist = selected_track_id.and_then(|track_id| {
        app_state
            .db
            .lock()
            .ok()
            .and_then(|db| db.get_track(track_id).ok().flatten())
            .and_then(|t| t.artist)
    });

    let mut fx_markers: Vec<&str> = Vec::new();
    if let Some(actions) = decision.and_then(|d| d.actions.as_ref()) {
        for action in actions {
            match &action.kind {
                MixActionKind::EchoOut { .. } => push_unique(&mut fx_markers, "echo_out"),
                MixActionKind::ReverbTail { .. } => push_unique(&mut fx_markers, "reverb_tail"),
                MixActionKind::FilterSlam { .. } => push_unique(&mut fx_markers, "filter_slam"),
                MixActionKind::Scratch { .. } => push_unique(&mut fx_markers, "scratch"),
                MixActionKind::BeatRepeat { .. } => push_unique(&mut fx_markers, "beat_repeat"),
                MixActionKind::Spinback { .. } => push_unique(&mut fx_markers, "spinback"),
                MixActionKind::VinylBrake { .. } => push_unique(&mut fx_markers, "vinyl_brake"),
                _ => {}
            }
        }
    }

    let mut planner = app_state.ai_planner.write().await;
    if let Some(name) = template_name {
        planner.record_template_use(name);
    }
    if let Some(track_id) = selected_track_id {
        planner.record_track_play(track_id);
    }
    if let Some(artist) = artist.as_deref() {
        planner.record_artist(artist);
    }
    for fx in fx_markers {
        planner.record_fx_use(fx);
    }
}

fn push_unique<'a>(items: &mut Vec<&'a str>, value: &'a str) {
    if !items.contains(&value) {
        items.push(value);
    }
}

/// Schedule an unlock of the conductor transition lock after a delay.
/// Resets conductor state back to Monitoring.
fn schedule_transition_unlock(handle: AppHandle, delay: tokio::time::Duration, message: String) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(delay).await;
        let state = handle.state::<AppState>();
        let mut cond = state.conductor.write().await;
        cond.transition_lock = false;
        cond.phase = ConductorPhase::Monitoring;
        cond.pending_track_id = None;
        cond.cached_profile = None;
        cond.cached_profile_track_id = None;
        cond.last_message = message;
        let snap = cond.snapshot(true);
        let _ = handle.emit("conductor-state", &snap);
    });
}

/// Call the AI planner (same logic as the `request_next_track` command, but internal).
async fn request_next_track_internal(
    app_state: &AppState,
    snapshot: &EngineStateSnapshot,
) -> Result<AiDecision, String> {
    let (
        playing_profile,
        playing_artist,
        playing_title,
        avail_profiles,
        avail_metadata,
        classifications,
        recent_tracks,
    ) = {
        let db = app_state
            .db
            .lock()
            .map_err(|e| format!("DB Lock error: {e}"))?;
        let tracks = db.get_all_tracks().map_err(|e| format!("DB error: {e}"))?;

        let mut profiles = Vec::new();
        let mut metadata = std::collections::HashMap::new();

        let deck_a_playing = snapshot.deck_a.state == "playing";
        let playing_id = if deck_a_playing {
            snapshot.deck_a.track_id.clone()
        } else {
            snapshot.deck_b.track_id.clone()
        };

        let mut current_profile = None;
        let mut cur_artist = "Unknown".to_string();
        let mut cur_title = "Unknown".to_string();

        for track in &tracks {
            if let Ok(Some(json)) = db.get_track_profile_json(&track.id) {
                if let Ok(profile) =
                    serde_json::from_str::<autodj_analysis::profile::TrackProfile>(&json)
                {
                    let artist = track
                        .artist
                        .clone()
                        .unwrap_or_else(|| "Unknown".to_string());
                    let title = track.title.clone().unwrap_or_else(|| "Unknown".to_string());
                    metadata.insert(track.id.clone(), (artist.clone(), title.clone()));

                    if playing_id.as_deref() == Some(&track.id) {
                        current_profile = Some(profile.clone());
                        cur_artist = artist;
                        cur_title = title;
                    }

                    profiles.push(profile);
                }
            }
        }

        if current_profile.is_none() {
            if let Some(first) = profiles.first() {
                current_profile = Some(first.clone());
                if let Some((a, t)) = metadata.get(&first.track_id) {
                    cur_artist = a.clone();
                    cur_title = t.clone();
                }
            }
        }

        let p = current_profile.ok_or("No current track profile found!")?;

        let mut class_map = std::collections::HashMap::new();
        if let Ok(class_rows) = db.get_all_classifications() {
            for (tid, genre, subgenre, mood, vibe, mix_notes) in class_rows {
                class_map.insert(
                    tid,
                    TrackClassificationData {
                        genre: Some(genre),
                        subgenre: Some(subgenre),
                        mood: Some(mood),
                        vibe: Some(vibe),
                        mix_notes: Some(mix_notes),
                    },
                );
            }
        }

        let recent = db.get_recent_track_ids(10).unwrap_or_default();

        (
            p, cur_artist, cur_title, profiles, metadata, class_map, recent,
        )
    };

    let recent_templates: Vec<String> = {
        let planner = app_state.ai_planner.read().await;
        planner.recent_templates().to_vec()
    };

    let context = build_context(&ContextInput {
        engine_state: snapshot,
        playing_track_profile: &playing_profile,
        playing_artist: &playing_artist,
        playing_title: &playing_title,
        available_profiles: &avail_profiles,
        intent: "maintain",
        target_energy: 0.5,
        recent_tracks: &recent_tracks,
        recent_templates: &recent_templates,
        available_metadata: &avail_metadata,
        classifications: &classifications,
        set_flow: None,
    });

    let planner = app_state.ai_planner.read().await;
    let decision = planner
        .plan_next(
            &context,
            &playing_profile,
            &avail_profiles,
            &classifications,
        )
        .await
        .map_err(|e| format!("AI Planning Error: {e}"))?;

    // Record in session history
    if let Some(ref track_id) = decision.track_selection {
        if let Ok(db) = app_state.db.lock() {
            if let Ok(session_id) = app_state.current_session_id.read() {
                if let Some(ref sid) = *session_id {
                    let decision_json = serde_json::to_string(&decision).ok();
                    let _ = db.record_track_play(
                        sid,
                        track_id,
                        decision
                            .transition_plan
                            .as_ref()
                            .map(|t| t.template_name.as_str()),
                        decision_json.as_deref(),
                    );
                }
            }
        }
    }

    Ok(decision)
}

/// Load a track onto a deck (replicates the load_track command logic).
async fn load_track_on_deck(
    app_state: &AppState,
    deck: DeckId,
    track_id: &str,
) -> Result<(), String> {
    let pcm_path = app_state.cache.pcm_path(track_id);
    if !pcm_path.exists() {
        return Err(format!("PCM cache not found for track {track_id}"));
    }

    let track_buffer =
        TrackBuffer::open(&pcm_path).map_err(|e| format!("Failed to open track buffer: {e}"))?;

    app_state.load_track_to_deck(deck, track_id.to_string(), Arc::new(track_buffer));

    // Auto-sync BPM
    if let Some(snap) = app_state.latest_state() {
        let master_bpm = snap.master_bpm;
        if let Ok(db) = app_state.db.lock() {
            if let Ok(Some(track)) = db.get_track(track_id) {
                if let Some(track_bpm) = track.bpm {
                    if track_bpm > 0.0 {
                        let speed = master_bpm / track_bpm;
                        app_state
                            .send_command(Command::SetSpeed { deck, speed }, CommandSource::System);
                    }
                }
            }
        }
    }

    // Record track play
    {
        if let Ok(db) = app_state.db.lock() {
            if let Ok(mut session) = app_state.current_session_id.write() {
                if session.is_none() {
                    if let Ok(sid) = db.create_session() {
                        *session = Some(sid);
                    }
                }
                if let Some(ref sid) = *session {
                    let _ = db.record_track_play(sid, track_id, None, None);
                }
            }
        }
    }

    app_state.send_command(Command::Play { deck }, CommandSource::System);

    Ok(())
}

/// Deterministic-ish pseudo-random index using sample position as entropy.
fn pseudo_random_index(len: usize, sample_pos: u64) -> usize {
    if len == 0 {
        return 0;
    }
    let hash = sample_pos.wrapping_mul(2654435761) >> 16;
    (hash as usize) % len
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conductor_state_default_phase_is_idle() {
        let cs = ConductorState::default();
        assert_eq!(cs.phase, ConductorPhase::Idle);
        assert!(cs.pending_track_id.is_none());
        assert!(!cs.transition_lock);
        assert!(cs.last_decision.is_none());
        assert!(cs.last_error.is_none());
        assert!(cs.queue.is_empty());
        assert_eq!(cs.queue_index, 0);
    }

    #[test]
    fn conductor_state_reset_clears_all() {
        let mut cs = ConductorState::default();
        cs.phase = ConductorPhase::Transitioning;
        cs.pending_track_id = Some("track1".to_string());
        cs.transition_lock = true;
        cs.last_message = "doing stuff".to_string();
        cs.last_error = Some("bad thing".to_string());
        cs.last_trigger_reason = Some("window:drop@12000ms".to_string());

        cs.reset();
        assert_eq!(cs.phase, ConductorPhase::Idle);
        assert!(cs.pending_track_id.is_none());
        assert!(!cs.transition_lock);
        assert!(cs.last_message.is_empty());
        assert!(cs.last_error.is_none());
        assert!(cs.last_trigger_reason.is_none());
    }

    #[test]
    fn advance_queue_returns_none_when_empty() {
        let mut cs = ConductorState::default();
        assert!(cs.advance_queue().is_none());
    }

    #[test]
    fn advance_queue_returns_tracks_in_order() {
        let mut cs = ConductorState::default();
        cs.queue = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        cs.queue_index = 0;

        assert_eq!(cs.advance_queue(), Some("a".to_string()));
        assert_eq!(cs.advance_queue(), Some("b".to_string()));
        assert_eq!(cs.advance_queue(), Some("c".to_string()));
        assert!(cs.advance_queue().is_none());
    }

    #[test]
    fn snapshot_reflects_phase_and_active() {
        let mut cs = ConductorState::default();
        cs.phase = ConductorPhase::Monitoring;
        cs.last_message = "test".to_string();

        let snap = cs.snapshot(true);
        assert_eq!(snap.phase, ConductorPhase::Monitoring);
        assert!(snap.active);
        assert_eq!(snap.message, "test");

        let snap_inactive = cs.snapshot(false);
        assert!(!snap_inactive.active);
    }

    #[test]
    fn snapshot_includes_decision_metadata() {
        let mut cs = ConductorState::default();
        cs.last_decision = Some(AiDecision {
            decision: "transition_now".to_string(),
            reason: "test".to_string(),
            track_selection: Some("track42".to_string()),
            execute_at_ms: Some(30000),
            transition_plan: Some(autodj_ai::planner::TransitionPlan {
                template_name: "filter_sweep".to_string(),
                parameter_overrides: None,
            }),
            actions: None,
            reasoning_tags: Some(vec!["tag1".to_string()]),
            confidence: Some(0.85),
            entry_marker: Some("drop".to_string()),
            exit_marker: Some("outro".to_string()),
            creative_intensity: Some(0.6),
            set_arc: Some("peak".to_string()),
            planner_mode: Some("heuristic_only".to_string()),
            template_scores: Some(vec![("filter_sweep".to_string(), 0.7)]),
            fallback_reason: Some("missing_api_key".to_string()),
        });

        let snap = cs.snapshot(true);
        assert_eq!(snap.decision_source.as_deref(), Some("heuristic_only"));
        assert_eq!(snap.fallback_reason.as_deref(), Some("missing_api_key"));
        assert_eq!(snap.template_chosen.as_deref(), Some("filter_sweep"));
        assert_eq!(snap.entry_marker.as_deref(), Some("drop"));
        assert_eq!(snap.confidence, Some(0.85));
        assert_eq!(snap.reasoning_tags.as_ref().unwrap().len(), 1);
        assert_eq!(
            snap.decision_meta
                .as_ref()
                .and_then(|m| m.selected_track_id.as_deref()),
            Some("track42")
        );
    }

    #[test]
    fn snapshot_preserves_structured_metadata_without_last_decision() {
        let mut cs = ConductorState::default();
        cs.last_decision = None;
        cs.last_trigger_reason = Some("fallback:75%".to_string());
        cs.last_trigger_meta = Some(ConductorTriggerMeta {
            kind: ConductorTriggerKind::PositionFallback,
            reason: "fallback:75%".to_string(),
            anchor: None,
            window_start_ms: None,
            observed_position_ms: Some(90_000),
            observed_position_fraction: Some(0.75),
        });
        cs.last_decision_meta = Some(ConductorDecisionMeta {
            source: "queue_only".to_string(),
            fallback_reason: Some("queue_override".to_string()),
            selected_track_id: Some("track_queue".to_string()),
            transition_template: Some("smooth_beatmix".to_string()),
            template_is_simulated: Some(false),
            entry_marker: None,
            execute_at_ms: None,
            confidence: None,
            template_scores: None,
            reasoning_tags: Some(vec!["queue_override".to_string()]),
            action_timeline_available: false,
        });

        let snap = cs.snapshot(true);
        assert_eq!(snap.decision_source.as_deref(), Some("queue_only"));
        assert_eq!(snap.template_chosen.as_deref(), Some("smooth_beatmix"));
        assert_eq!(snap.fallback_reason.as_deref(), Some("queue_override"));
        assert_eq!(
            snap.trigger_meta.as_ref().map(|m| m.kind),
            Some(ConductorTriggerKind::PositionFallback)
        );
    }

    #[test]
    fn simulated_template_detection_is_honest() {
        assert!(is_simulated_template("echo_drop"));
        assert!(is_simulated_template("backspin_cut"));
        assert!(is_simulated_template("fx_echo_slam"));
        assert!(!is_simulated_template("smooth_beatmix"));
        assert!(!is_simulated_template("filter_sweep"));
    }

    #[test]
    fn pseudo_random_index_within_bounds() {
        for len in 1..=50 {
            for sample_pos in [0, 1, 100, 44100, u64::MAX] {
                let idx = pseudo_random_index(len, sample_pos);
                assert!(
                    idx < len,
                    "idx={} >= len={} for sample_pos={}",
                    idx,
                    len,
                    sample_pos
                );
            }
        }
    }

    #[test]
    fn pseudo_random_index_zero_len() {
        assert_eq!(pseudo_random_index(0, 12345), 0);
    }

    #[test]
    fn pseudo_random_index_produces_variation() {
        // Different sample positions should produce different indices (for a meaningful list)
        let indices: std::collections::HashSet<usize> = (0..100)
            .map(|i| pseudo_random_index(10, i * 44100))
            .collect();
        // With 100 different sample positions and 10 buckets, we should hit at least 5
        assert!(
            indices.len() >= 5,
            "Only {} distinct indices — not enough variation",
            indices.len()
        );
    }
}
