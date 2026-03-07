# Auto-DJ — Master Task List

## Phase 1: Architecture Documentation ✅
- [x] 10 docs under `docs/`

## Phase 2: Milestone A —## Verification execution
- [x] Run backend unit tests (`cargo test --workspace`)
- [x] Review any failed sections (All backend tests passed)
- [/] Optionally spin up a browser subagent to review the UI directly
- [x] cpal audio engine + SPSC queues + basic React UI

## Phase 3: Milestone B — Two## Milestone B — Mixing
- [x] 3-band EQ and SVF filter per deck
- [x] Signal chain processing (PCM → EQ → Filter → Gain)
- [x] Engine commands (`SetEq`, `SetFilter`, `SetDeckGain`)
- [x] Tauri command handlers for EQ/Filter/Gain
obs, kill)

## Phase 4:## Milestone C — Sync & Quantize
- [x] Command schedule resolution via `TransportClock`
- [x] Padding command queue in engine callback
- [x] Loop handling with beat wrapping
- [x] Deck seek functionality
op/ClearLoop/Seek commands
- [x] Tauri commands: seek, set_loop, clear_loop, set_tempo
- [x] UI: loop buttons (4/8/16 bars) with active indicator
- [x] Tests: 31 total (25 core + 6 storage)

## Phase 5: Milestone D — Analysis ✅
- [x] Expand `TrackProfile` struct to full spec schema
- [x] BPM detection (spectral flux + autocorrelation) — `bpm.rs`
- [x] Beat tracking (DP tracker + downbeat detection) — `beats.rs`
- [x] Key detection (chromagram + Krumhansl-Kessler) — `key.rs`
- [x] Loudness measurement (LUFS, RMS, true peak) —## Milestone D — Analysis
- [x] Full `TrackProfile` schema integration
- [x] BPM detection (`bpm.rs`)
- [x] Key detection (`key.rs`)
- [x] Loudness measurement (`loudness.rs`)
- [x] Energy curve computation (`energy.rs`)
- [x] Waveform peak generation (`waveform.rs`)
- [x] Parallel pipeline Orchestration (`pipeline.rs` via rayon)
- [/] Beat tracking (Synthetic grid confirmed)
- [/] Section segmentation (Heuristic confirmed)
k` / `get_track_profile` commands
- [x] UI: BPM/key display, waveform canvas, analyze button
- [x] Tests: BPM on synthetic audio, key detection, pipeline integration

## Phase 6:## Milestone E — Transitions
- [x] Transition templates (`SmoothBeatmix`, `FilterSweep`, `EchoDrop`)
- [x] Transition command expansion logic
stry — `transition.rs`
- [x] `SmoothBeatmix` template (16-bar bass swap)
- [x] `FilterSweep` template (HP sweep + fade)
- [x] `EchoDrop` template (echo → silence → drop)
- [x] `ExecuteTransition` command variant + validation
- [x] Engine: expand transitions into scheduled commands
- [x] Tauri: `execute_transition` command
- [x] UI: transition control panel (template dropdown, duration slider)
- [x] Tests: expand functions, full engine integration

## Phase 7:## Milestone F — AI Conductor
- [x] Heuristic scorer & Camelot math
- [x] AI context builder and planner
- [x] LLM client implementation and parsing
ction — `camelot.rs`
- [x] AI context builder — `context.rs`
- [x] AI planner (LLM first, heuristic fallback) — `planner.rs`
- [x] LLM client (reqwest, rate limiting, timeout) — `llm.rs`
- [x] AI decision JSON schema validation
- [x] Tauri: `request_next_track`, `set_mode`, `set_ai_config` commands
- [x] UI: mode switcher, AI status, suggestion panel, config panel
- [x] Tests: scorer ranking, camelot distance, mock LLM, fallback path

## Phase 8: Milestone G — Polish + Export
- [x] Read ID3 tags (Title/Artist) on import (`lofty` crate)
- [x] Auto-sync deck BPM on load
- [x] Waveform rendering on `<canvas>` with playhead overlay
- [x] UI: Fine-grain BPM nudge controls, tempo sliders, and dynamic pitch range
- [x] Backend: Hot cue storage and seek dispatching
- [x] UI: Keyboard shortcuts (space, z/x, q/w)
- [x] Clear library functionality
- [x] Drag-and-drop import implementation
- [ ] Mix history + session tracking
- [ ] Audio export (mix recording)

## Phase 9: Gemini AI DJ Automation
- [x] Modify `llm.rs` to target Gemini `generateContent` endpoint instead of OpenAI.
- [x] Update `llm.rs` payload to include `responseMimeType: "application/json"`.
- [x] Parse `candidates.content.parts.text` correctly from Gemini JSON response.
- [x] Expand `context.rs` to compute and attach Timestamp details (duration, beat boundaries) to the track context.
- [x] Expand `planner.rs` `AiDecision` struct to accept an `execute_at_ms` action timestamp.
- [x] Update frontend settings panel to reflect Gemini API integration.

## Phase 10: Perceived Tempo Engine ✅
- [x] Define `PerceivedTempoInfo` in `autodj-analysis/src/profile.rs`
- [x] Implement `autodj-analysis/src/perceived_tempo.rs` (Onset extraction & Groove Classification)
- [x] Integrate into `pipeline.rs` passing raw samples & baseline BPM
- [x] Update frontend UI state and Types (`TrackRecord`) to show Groove/Perceived Tempo
- [x] Refactor AI DJ `scorer.rs` and `context.rs` to prioritize perceived tempo over raw BPM

---

## AI Enrichment Overhaul (Phases 11–17)

> Detailed design: `docs/plans/2026-03-07-ai-enrichment-design.md`
> Detailed implementation: `docs/plans/2026-03-07-ai-enrichment-implementation.md`

## Phase 11: Code Quality Cleanup ✅
- [x] Use `FRAC_1_SQRT_2` constant in EQ biquad filters
- [x] Remove unused imports, variables, and constants
- [x] Resolve all clippy warnings across workspace
- [x] Extract `ProfileData` and `ContextInput` structs, fix test args

## Phase 12: DSP Enrichment ✅
- [x] `autodj-analysis/src/spectral.rs` — spectral centroid, flatness, 3-band energy ratio (with Hann windowing)
- [x] `autodj-analysis/src/vocal_detection.rs` — vocal frequency range energy + harmonic prominence
- [x] `autodj-analysis/src/rhythm.rs` — onset regularity, complexity, danceability score
- [x] Add 12 new columns to `track_profiles` table via migration
- [x] Integrate new analyzers into `pipeline.rs` and `TrackProfile` (parallel via rayon)
- [x] Update `ProfileData`, `save_track_profile`, and `commands.rs` for new fields
- [x] Added `energy_trajectory` computation (building/declining/dynamic/steady)
- [x] Added `intro_length_ms` / `outro_length_ms` detection from energy curve
- [x] Added `first_drop_ms` from existing drop candidates
- [x] All 52 tests pass, zero clippy warnings

## Phase 13: Gemini Classification ✅
- [x] `autodj-ai/src/classifier.rs` — `TrackClassification` + `ClassificationInput` structs + `classify_track()` async fn
- [x] System prompt for genre/subgenre/mood/vibe/mix_notes/ai_tags classification
- [x] Add 6 DB columns: genre, subgenre, mood, vibe, mix_notes, ai_tags
- [x] `save_classification()` method on `Database`
- [x] `TrackRecord` updated with genre/mood/vibe fields surfaced in library queries
- [x] Tauri `classify_track` command registered + wired to DB persistence
- [x] Unit tests: deserialization + input serialization
- [x] All 53 tests pass, zero new warnings
- [ ] Batch classification pipeline for library (deferred — per-track command available)
- [ ] UI for viewing/editing AI classifications (deferred to Phase 17)

## Phase 14: AI Decision Engine Overhaul ✅
- [x] Expand scorer weights from 4 to 7 (add genre compatibility, mood continuity, spectral contrast)
- [x] Expand AI context with genre/mood/vibe, spectral profile, section details, mix history, set position
- [x] Vocal-aware transition template selection
- [x] Section-based transition timing (use intro/outro/drop markers)

## Phase 15: Mix History & Session Feedback ✅
- [x] Wire up `sessions` and `mix_history` tables (schema already exists)
- [x] Auto-create session on first play, record each track transition
- [x] Feed recent mix history into AI context for variety scoring
- [x] UI: session history panel

## Phase 16: Audio Export ✅
- [x] Recording tap in audio callback (ring buffer for master output)
- [x] WAV writer for export (32-bit float, stereo, auto-header finalization)
- [x] Tauri commands: `start_recording`, `stop_recording`
- [x] UI: record button with blinking dot + elapsed time counter
- [x] All 61 tests pass, zero warnings

## Phase 17: Visual Polish & Analysis Improvements ✅
- [x] Album art extraction on import (lofty) + cache directory + `get_album_art` command
- [x] Improve section analysis: energy-curve-based detection (intro/buildup/drop/breakdown/outro)
- [x] Library enhancements: genre badges (purple), mood badges (orange), 9-column grid
- [ ] Replace synthetic beat grid with DP beat tracker (Ellis 2007) — deferred
- [ ] Album art display in deck panels — deferred
