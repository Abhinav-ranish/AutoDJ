# Auto-DJ — Phases 5, 6, 7 Implementation Plan

## Phase 5: Milestone D — Track Analysis

Fully implement the `autodj-analysis` crate to produce `TrackProfile` data (BPM, key, beats, energy, sections, waveform peaks) per the spec in `docs/04-track-analysis.md`.

### New Dependencies

#### [MODIFY] [Cargo.toml](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/Cargo.toml)
Add workspace deps: `rustfft = "6"`, `rayon = "1"`.

#### [MODIFY] [Cargo.toml](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-analysis/Cargo.toml)
Add: `rustfft`, `rayon`, `autodj-storage` (for `TrackBuffer` access).

---

### autodj-analysis Crate

#### [MODIFY] [lib.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-analysis/src/lib.rs)
- Expand `TrackProfile` struct to match the full JSON schema from spec (tempo, beats, key, loudness, energy, regions, sections, waveform).
- Add sub-structs: `TempoResult`, `BeatGrid`, `KeyResult`, `LoudnessInfo`, `EnergyCurve`, `RegionInfo`, `Section`, `WaveformMeta`.
- Add `pub mod` declarations for all new modules.
- Add `AnalysisError` enum with `thiserror`.

#### [NEW] [bpm.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-analysis/src/bpm.rs)
- `detect_bpm(samples: &[f32], sample_rate: u32) -> TempoResult`
- Spectral flux onset detection (STFT with 2048-sample window, 512 hop).
- Autocorrelation of onset function over 4-8 second windows.
- Peak picking → candidate BPM list, scored by plausibility (prefer 80-160 range).
- Fallback: inter-onset interval histogram if confidence < 0.5.

#### [NEW] [beats.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-analysis/src/beats.rs)
- `track_beats(samples: &[f32], sample_rate: u32, bpm: f64) -> BeatGrid`
- Dynamic programming beat tracker (Ellis 2007 style).
- Downbeat detection via bar-level periodicity.
- Fallback: synthetic grid from BPM starting at first onset.

#### [NEW] [key.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-analysis/src/key.rs)
- `detect_key(samples: &[f32], sample_rate: u32) -> KeyResult`
- Chromagram via STFT (12-bin pitch class profile).
- Correlate with Krumhansl-Kessler profiles (24 total: 12 major + 12 minor).
- Map to Camelot wheel using lookup table from spec.

#### [NEW] [loudness.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-analysis/src/loudness.rs)
- `measure_loudness(samples: &[f32], sample_rate: u32) -> LoudnessInfo`
- Integrated LUFS (simplified BS.1770), true peak (4x oversample), RMS, dynamic range.

#### [NEW] [energy.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-analysis/src/energy.rs)
- `compute_energy_curve(samples: &[f32], sample_rate: u32) -> EnergyCurve`
- Windowed RMS (0.5s default) with A-weighting.
- Normalize to 0.0-1.0. Identify drop candidates (>2x increase within 2s after a local min).

#### [NEW] [sections.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-analysis/src/sections.rs)
- `segment_track(energy: &EnergyCurve, beats: &BeatGrid) -> Vec<Section>`
- Self-similarity matrix from per-beat feature vectors (energy + chromagram).
- Novelty function via checkerboard kernel convolution.
- Label by energy profile (intro, buildup, drop, breakdown, outro).
- Fallback: equal thirds.

#### [NEW] [waveform.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-analysis/src/waveform.rs)
- `generate_peaks(samples: &[f32], samples_per_pixel: usize) -> Vec<(f32, f32)>`
- Min/max pairs per pixel bucket. Write to binary `.peaks` file.

#### [NEW] [pipeline.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-analysis/src/pipeline.rs)
- `analyze_track(pcm_path: &Path) -> Result<TrackProfile, AnalysisError>`
- Orchestrates the full pipeline: BPM → Beats → Key, Loudness, Energy (parallel via rayon) → Sections, Safe Loops → Waveform Peaks.
- Steps 1, 3, 4, 5, 8 run concurrently; steps 2, 6, 7 depend on step 1.

---

### Integration

#### [MODIFY] [db.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-storage/src/db.rs)
- Add `save_track_profile(&self, track_id: &str, profile: &TrackProfile)` — writes to `track_profiles` table (schema already exists).
- Add `get_track_profile(&self, track_id: &str) -> Option<TrackProfile>` — reads back.

#### [MODIFY] [commands.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-app/src/commands.rs)
- Add `analyze_track` Tauri command that triggers analysis on a background thread and returns the profile.
- Add `get_track_profile` Tauri command.

#### [MODIFY] [store.ts](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/ui/src/store.ts)
- Add `analyzeTrack(trackId: string)` and `getTrackProfile(trackId: string)` store actions.

#### [MODIFY] [App.tsx](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/ui/src/App.tsx)
- Display BPM, key (Camelot), and energy in the library list.
- Add waveform display component (render `.peaks` data on canvas).
- Add "Analyze" button per track.

### Verification

```bash
cargo test --workspace
```
- Unit tests for BPM detection on a synthetic 128 BPM sine wave.
- Unit tests for key detection on a known-key chromatic signal.
- Unit test for waveform peak generation.
- Integration test: `pipeline::analyze_track` on a test PCM file → verify profile fields populate.

---

## Phase 6: Milestone E — Transitions

Implement the transition template system that expands high-level transition commands into sequences of primitive engine commands.

### autodj-core Changes

#### [NEW] [transition.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-core/src/transition.rs)
- `TransitionTemplate` trait with `fn expand(&self, params) -> Vec<ScheduledCommand>`.
- `SmoothBeatmix` struct: 16-bar mix with bass swap.
  - Expand into: LoadTrack → Play → interpolated crossfader → bass swap EQ at bar N → final crossfader + Pause.
- `FilterSweep` struct: HP filter sweep on outgoing + incoming fade.
  - Expand into: HP filter interpolation from `start_hz` to `end_hz` over N bars + gain swap.
- `EchoDrop` struct: echo/delay on outgoing → silence → drop incoming.
  - Expand into: gain ramp down with echo feedback → silence gap → incoming play from specified section.
- `TransitionExpander` — registry of templates. `expand(template_name, params) -> Result<Vec<ScheduledCommand>>`.

#### [MODIFY] [command.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-core/src/command.rs)
- Add `ExecuteTransition { template: String, params: serde_json::Value }` variant to `Command` enum.
- Add validation for `ExecuteTransition` in `CommandValidator` (check template exists, params valid).

#### [MODIFY] [engine.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-core/src/engine.rs)
- When an `ExecuteTransition` command is received, call `TransitionExpander::expand()` and enqueue the resulting scheduled commands into the pending queue.

#### [MODIFY] [lib.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-core/src/lib.rs)
- Add `pub mod transition;`.

---

### Integration

#### [MODIFY] [commands.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-app/src/commands.rs)
- Add `execute_transition(template: String, params: Value)` Tauri command.

#### [MODIFY] [store.ts](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/ui/src/store.ts)
- Add `executeTransition(template: string, params: object)` store action.

#### [MODIFY] [App.tsx](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/ui/src/App.tsx)
- Add transition control panel: dropdown (smooth_beatmix, filter_sweep, echo_drop) + "Go" button.
- Duration slider (4-64 bars).

### Verification

```bash
cargo test --workspace
```
- Unit test: `SmoothBeatmix::expand()` → verify correct command sequence and timing.
- Unit test: `FilterSweep::expand()` → verify HP filter ramp values.
- Unit test: `EchoDrop::expand()` → verify silence gap and incoming start.
- Integration test: Full engine with two loaded tracks, execute a transition, verify deck states change correctly.

---

## Phase 7: Milestone F — AI Conductor

Implement the AI planner, heuristic scorer, and LLM integration per `docs/05-ai-conductor.md`.

### New Dependencies

#### [MODIFY] [Cargo.toml](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/Cargo.toml)
Add workspace deps: `reqwest = { version = "0.12", features = ["json"] }`, `tokio = { version = "1", features = ["full"] }`.

#### [MODIFY] [Cargo.toml](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-ai/Cargo.toml)
Add: `reqwest`, `tokio`, `autodj-analysis` (for `TrackProfile`), `autodj-core` (for `Command`, `DeckId`).

---

### autodj-ai Crate

#### [MODIFY] [lib.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-ai/src/lib.rs)
- Expand `AiConfig` with API key, endpoint URL, weights, and prompt template path.
- Add `pub mod` declarations for all new modules.
- Add `AiError` enum.

#### [NEW] [scorer.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-ai/src/scorer.rs)
- `HeuristicScorer` struct with configurable weights (tempo, key, energy, vocal, variety).
- `score(current: &TrackProfile, candidate: &TrackProfile, recent: &[String], intent: &str) -> f64`
- Compatibility functions per spec:
  - Tempo: `1.0 - abs(bpm_a - bpm_b) / 20.0`, clamped [0,1].
  - Key: Camelot distance → 1.0 / 0.6 / 0.2.
  - Energy: depends on intent (maintain/build/drop).
  - Variety: recency penalty.
- `select_next(current: &TrackProfile, candidates: &[TrackProfile], recent: &[String], intent: &str) -> TrackProfile` — returns top scorer.

#### [NEW] [context.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-ai/src/context.rs)
- `AiContext` struct matching the AI input schema from spec.
- `build_context(engine_state, library, preferences) -> AiContext` — assembles the full JSON context.

#### [NEW] [planner.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-ai/src/planner.rs)
- `AiPlanner` struct (owns `HeuristicScorer` + optional LLM client).
- `plan_next(context: &AiContext) -> Result<AiDecision, AiError>` — tries LLM first, falls back to heuristic.
- `AiDecision` struct matching the AI output schema (decision, track_selection, transition_plan).
- JSON schema validation of LLM responses.
- Rate limiting (max 1 call per 10 seconds), timeout (5 seconds).

#### [NEW] [llm.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-ai/src/llm.rs)
- `LlmClient` struct wrapping `reqwest::Client`.
- `query(prompt: &str, config: &AiConfig) -> Result<String, AiError>` — sends request to OpenAI-compatible API.
- Prompt assembly using the template from spec with substituted context values.
- Response parsing + JSON validation against output schema.

#### [NEW] [camelot.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-ai/src/camelot.rs)
- `camelot_distance(a: &str, b: &str) -> u32` — distance on the Camelot wheel.
- Full Camelot lookup table from spec.
- Used by both `scorer.rs` and `key.rs` in analysis.

---

### Integration

#### [MODIFY] [commands.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-app/src/commands.rs)
- Add `request_next_track` Tauri command (triggers AI planner).
- Add `set_mode(mode: DjMode)` Tauri command.
- Add `set_ai_config(config: AiConfig)` Tauri command.

#### [MODIFY] [state.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-app/src/state.rs)
- Add `AiPlanner` to `AppState`.
- Add mode tracking (`DjMode`).

#### [MODIFY] [store.ts](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/ui/src/store.ts)
- Add `requestNextTrack()`, `setMode(mode)`, `setAiConfig(config)` actions.

#### [MODIFY] [App.tsx](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/ui/src/App.tsx)
- Add mode switcher (Manual / Assisted / AutoDJ).
- Add AI status indicator (idle, thinking, fallback).
- Add "Next Track" suggestion panel showing AI reasoning.
- Add AI config panel (API key input, model selector, weight sliders).

### Verification

```bash
cargo test --workspace
```
- Unit test: `HeuristicScorer::score()` with known inputs → verify ranking.
- Unit test: `camelot_distance` for all edge cases (same key, adjacent, opposite).
- Unit test: `AiDecision` JSON parsing and validation.
- Integration test: `AiPlanner::plan_next()` with mock LLM (return canned JSON) → verify decision parses and validates.
- Integration test: Fallback path (LLM timeout) → verify heuristic scorer is used.

---

## Summary Table

| Phase | Milestone | New Files | Modified Files | New Dependencies |
|-------|-----------|-----------|----------------|------------------|
| 5 | D — Analysis | 7 new modules in `autodj-analysis` | db.rs, commands.rs, store.ts, App.tsx | `rustfft`, `rayon` |
| 6 | E — Transitions | 1 new module in `autodj-core` | command.rs, engine.rs, lib.rs, commands.rs, store.ts, App.tsx | None |
| 7 | F — AI Conductor | 5 new modules in `autodj-ai` | lib.rs, commands.rs, state.rs, store.ts, App.tsx | `reqwest`, `tokio` |
