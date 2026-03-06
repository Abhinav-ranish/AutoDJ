# Auto-DJ — Master Task List

## Phase 1: Architecture Documentation ✅
- [x] 10 docs under `docs/`

## Phase 2: Milestone A — Foundations ✅
- [x] Tauri skeleton + Rust workspace (5 crates)
- [x] SQLite storage + file import pipeline + FFmpeg decode
- [x] cpal audio engine + SPSC queues + basic React UI

## Phase 3: Milestone B — Two Decks + Mixing ✅
- [x] 3-band EQ (biquad) + SVF filter
- [x] Signal chain: PCM → EQ → Filter → Gain
- [x] SetEq/SetFilter commands + Tauri handlers + UI (sliders, knobs, kill)

## Phase 4: Milestone C — Sync + Quantization ✅
- [x] `SchedulePoint` resolution in `TransportClock`
- [x] Pre-allocated pending command queue in audio callback
- [x] Deck loops (set_loop, clear_loop) with beat-aligned wrapping
- [x] Deck seek + SetLoop/ClearLoop/Seek commands
- [x] Tauri commands: seek, set_loop, clear_loop, set_tempo
- [x] UI: loop buttons (4/8/16 bars) with active indicator
- [x] Tests: 31 total (25 core + 6 storage)

## Phase 5: Milestone D — Analysis ✅
- [x] Expand `TrackProfile` struct to full spec schema
- [x] BPM detection (spectral flux + autocorrelation) — `bpm.rs`
- [x] Beat tracking (DP tracker + downbeat detection) — `beats.rs`
- [x] Key detection (chromagram + Krumhansl-Kessler) — `key.rs`
- [x] Loudness measurement (LUFS, RMS, true peak) — `loudness.rs`
- [x] Energy curve (windowed RMS + drop candidates) — `energy.rs`
- [x] Section segmentation (self-similarity matrix) — `sections.rs`
- [x] Waveform peak generation — `waveform.rs`
- [x] Analysis pipeline orchestrator — `pipeline.rs`
- [x] DB: `save_track_profile` / `get_track_profile`
- [x] Tauri: `analyze_track` / `get_track_profile` commands
- [x] UI: BPM/key display, waveform canvas, analyze button
- [x] Tests: BPM on synthetic audio, key detection, pipeline integration

## Phase 6: Milestone E — Transitions
- [ ] `TransitionTemplate` trait + `TransitionExpander` registry — `transition.rs`
- [ ] `SmoothBeatmix` template (16-bar bass swap)
- [ ] `FilterSweep` template (HP sweep + fade)
- [ ] `EchoDrop` template (echo → silence → drop)
- [ ] `ExecuteTransition` command variant + validation
- [ ] Engine: expand transitions into scheduled commands
- [ ] Tauri: `execute_transition` command
- [ ] UI: transition control panel (template dropdown, duration slider)
- [ ] Tests: expand functions, full engine integration

## Phase 7: Milestone F — AI Conductor
- [ ] Heuristic scorer with configurable weights — `scorer.rs`
- [ ] Camelot wheel distance function — `camelot.rs`
- [ ] AI context builder — `context.rs`
- [ ] AI planner (LLM first, heuristic fallback) — `planner.rs`
- [ ] LLM client (reqwest, rate limiting, timeout) — `llm.rs`
- [ ] AI decision JSON schema validation
- [ ] Tauri: `request_next_track`, `set_mode`, `set_ai_config` commands
- [ ] AppState: add AiPlanner + DjMode tracking
- [ ] UI: mode switcher, AI status, suggestion panel, config panel
- [ ] Tests: scorer ranking, camelot distance, mock LLM, fallback path

## Phase 8: Milestone G — Polish + Export
