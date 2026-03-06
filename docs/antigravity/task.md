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
- [ ] Tests: expand functions, full engine integration

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
