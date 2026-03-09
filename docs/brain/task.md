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

## Phase 11: Code Quality Cleanup ✅
> Detailed design: `docs/plans/2026-03-07-ai-enrichment-design.md`
> Detailed implementation: `docs/plans/2026-03-07-ai-enrichment-implementation.md`
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

## Phase 18: Autonomous AutoDJ Loop ✅
- [x] Auto-classify tracks after analysis (if Gemini API key configured)
- [x] Position-aware AutoDJ controller (replaces blind 10s polling)
- [x] Auto-pick and load first track when entering AutoDJ with empty decks
- [x] Intelligent transition triggering at 75% track position / outro start
- [x] End-of-track safety fallback (emergency transition at 92%)
- [x] Fix decision lifecycle: clear AI decisions after transition completes
- [x] Chain transitions continuously (Deck A → B → A → B...)
- [x] AutoDJ status feedback UI (monitoring, selecting, loading, transitioning)
- [x] Return `AiDecision` from `requestNextTrack` for controller consumption
- [x] All Rust tests pass (61), TypeScript compiles clean, Vite builds

---

## Phase 19: DJ-Style Live Mixing + Phrase/Section-Aware Transitions ✅

Goal: stop “wait until track ends + SmoothBeatmix only”. Make the engine + controller mix *inside* songs using musical landmarks (intro/outro/drop/break) and phrase alignment.

* [x] **Section-aware transition windows**

  * [x] Define “safe mix windows” per track from existing markers: `intro`, `buildup`, `drop`, `breakdown`, `outro`, `first_drop_ms`, `intro_length_ms`, `outro_length_ms`
  * [x] Generate candidate transition anchors: `start_of_outro`, `pre_drop_8/16`, `post_drop_8/16`, `break_start`, etc.
  * [x] Add `TransitionWindow` struct and persist into `TrackProfile` (or compute on-demand in AI context)

* [x] **Phrase alignment (4/8/16 bar alignment)**

  * [x] Add phrase grid derived from beat grid (even if synthetic for now)
  * [x] Teach controller to schedule transitions at next phrase boundary (not “at end”)
  * [x] Add engine helper: `next_phrase_boundary(now, bars=N) -> execute_at_ms`

* [x] **Loop-driven “DJ mix” behavior**

  * [x] Allow transition logic to **loop** outgoing track outro or incoming track intro to “hold the groove” until the perfect entry
  * [x] Add transition step primitives: `SetLoop(bars)`, `ClearLoop`, `SeekToMarker(marker_id)`
  * [x] Add “emergency loop” when mismatch/confidence low (keep playing while AI replans)

* [x] **Hot-cue / marker system usable by AI**

  * [x] Define canonical cue markers per track: `CUE_INTRO`, `CUE_DROP`, `CUE_BREAK`, `CUE_OUTRO`, `CUE_FIRST_BEAT`
  * [x] Auto-generate these cues during analysis (store in DB via `auto_cues` table)
  * [x] Add engine command: `SeekMs` + `SetEqKill` for marker-based navigation

* [x] **Expand transition template library to real DJ moves**

  * [x] `DropSwap` (cut on drop with pre-drop tension)
  * [x] `LoopRoll` (4→2→1 bar roll into next track)
  * [x] `BassKillSwap` (bass swap earlier than full swap)
  * [ ] `BackspinExit` (optional / gated by style flag) — deferred
  * [x] `BreakBlend` (blend during breakdown, avoid vocal clash)
  * [x] Each template must output a **command timeline** (EQ/filter/gain/cue/loop) with beat-accurate scheduling

* [x] **Transition selection rules (non-LLM)**

  * [x] If vocal overlap detected → prefer breakdown/outro blends + filter sweeps
  * [x] If energy trajectory “building” → avoid hard cuts unless target drop is stronger
  * [x] If perceived tempo mismatch high → prefer longer blend + loop roll

* [x] **Autonomous AutoDJ timing overhaul**

  * [x] Replace “75% position trigger only” with: “best next window within next X seconds”
  * [x] Add replan loop: if window missed → pick next best anchor, don’t wait to finish track
  * [x] Add controller state machine: `idle → monitoring → planning → loading → aligning → transitioning`

* [x] **Test coverage**

  * [x] Unit tests for phrase boundary math, window generation, template expansion correctness
  * [x] Integration test: simulated track A/B with markers → transition schedules before end-of-track
  * [ ] Regression test: never schedules transition in “unsafe” zones (like mid-vocal phrase if flagged) — deferred

---

## Phase 20: AI Performance Controls — EQ/Filter Automation + “Real Mixing” Decision Schema ✅

Goal: AI doesn’t touch EQ because it has too few “safe” knobs + no structured way to apply them. Give it DJ-grade controls with guardrails + expose them in the action schema.

* [x] **Upgrade EQ model (DJ-friendly)**

  * [x] Add **3-band isolator** mode (kills allowed): `LOW/MID/HIGH` with steep LR4 slopes
  * [ ] Add optional **4-band** EQ (LOW/LOWMID/HIMID/HIGH) for finer shaping — deferred
  * [x] Add per-band **kill switches** and “soft kill” curves
  * [ ] Add per-deck **EQ presets**: “Clean Blend”, “Club Punch”, “Vocal Focus” — deferred
  * [x] Engine commands:

    * [x] `SetEqBand(deck, band, gain_db)` (existing)
    * [x] `SetEqKill(deck, band, enabled)`
    * [x] `SetEqMode(deck, mode)` (classic vs isolator)

* [x] **Safe automation primitives (what AI can control)**

  * [x] **Bass swap automation** (classic DJ move) — via BassKillSwap template
  * [x] **High-pass sweep** on outgoing track during blend — via DropSwap/FilterSweep templates
  * [x] **Vocal ducking**: reduce mid band when vocals clash (vocal overlap → break_blend template)
  * [x] **Energy shaping**: subtle gain ramps via action timelines

* [x] **AI Decision schema v2 (structured, timestamped, multi-step)**

  * [x] Replace “one transition template” with a timeline of actions:

    * [x] `actions: [{ at_ms, kind, params }]`
    * [x] Action kinds: `LoadTrack`, `SetTempo`, `SetGain`, `SetFilter`, `SetEqBand`, `SetEqKill`, `SetLoop`, `ClearLoop`, `SetCrossfader`, `Play`, `Pause`, `ExecuteTemplate` (12 total)
  * [x] Add `reasoning_tags` (short enums) for debugging: `VOCAL_CLASH_AVOID`, `BASS_SWAP`, `DROP_SWAP`, etc.
  * [x] Add `confidence` per action group + fallback plan id

* [x] **Planner changes**

  * [x] Heuristic planner can fully generate action timelines (`build_heuristic_timeline()`) — no LLM required
  * [x] LLM output becomes optional “creative layer”, constrained by schema + validation
  * [x] Add clamp rules:

    * [x] EQ gain limits (e.g., ±10 dB), kill only on LOW during bass swap unless explicit
    * [x] Filter cutoff ranges + max sweep speed
    * [ ] Never do >N actions per 8 bars (avoid twitchy automation) — deferred

* [x] **Learning hooks (user feedback → future mixes)**

  * [x] Store user feedback per transition/action set (`transition_feedback` table + Tauri command)
  * [ ] Add preference bias in scorer (e.g., prefers FilterSweep over DropSwap) — deferred

* [x] **UI/Debug tooling (so you can see what AI is doing)**

  * [x] “AI Mix Timeline” panel: upcoming actions with timestamps/bars (6-action preview)
  * [ ] Live overlay on deck UI: which bands AI is touching + current automation state — deferred
  * [ ] “Lock controls” toggles: let user forbid AI from touching EQ/filter/tempo independently — deferred

* [ ] **Reliability + safety** — deferred to Phase 25

  * [ ] Hard “panic” rule: if silence/underrun detected → disable automation + emergency load next
  * [ ] Action de-duplication: prevent oscillation (AI repeatedly toggling EQ every tick)

---


## Phase 21: Library Search, Sort & Filter ✅
- [x] SQL-backed text search (title, artist) via `search_tracks` command
- [x] BPM range filter, key filter, genre/mood filter
- [x] Column-header click-to-sort (title, artist, BPM, key, genre, duration)
- [x] UI: search bar above library + filter pills/dropdowns
- [x] Clear filters / show all button

## Phase 22: Preferences & Settings Persistence ✅
- [x] Implement `get_preference()` / `set_preference()` DB methods
- [x] Persist AI config (API key, model) to SQL preferences table
- [x] Audio device enumeration (`cpal` host devices) + `get_audio_devices` command
- [x] Audio device selection + `set_audio_device` command (hot-swap output)
- [x] UI: settings panel with audio device picker, transition defaults, AutoDJ thresholds
- [x] Save/restore user preferences on app launch

## Phase 23: Visual Polish & UX ✅
- [x] Album art display in deck panels (use cached art from `get_album_art`)
- [x] Hot cue buttons in deck UI (set/jump/clear, color-coded)
- [ ] Waveform hot cue markers (vertical lines at cue positions) — deferred
- [ ] Mix history view — browse past sessions with tracklist + transitions — deferred
- [x] Right-click context menu on library rows (load to A/B, classify, delete)
- [ ] Responsive layout improvements (smaller screens, collapsible panels) — deferred

## Phase 24: Batch Operations & AI Polish ✅
- [x] Batch classification — "Classify All" button for unclassified tracks
- [x] Progress indicator for batch classification (X/Y tracks done)
- [ ] UI to view/edit AI classifications per track (inline edit or modal) — deferred
- [ ] AI feedback loop — thumb up/down on transitions to weight future scoring — deferred
- [ ] Set energy arc goal (build → peak → cool) as AI input parameter — deferred

## Phase 25: Safety & Reliability ✅
- [x] Silence detection in engine (RMS monitoring on master output)
- [x] Fallback ladder: silence detected → auto-load + play next track
- [ ] End-of-track detection (audio signal, not just position %) — deferred
- [x] Buffer underrun counter + logging
- [ ] Crash-safe session recovery (persist playback state, resume on relaunch) — deferred

## Phase 26: Transition Diversity & Fade Quality

Goal: Fix the AI's heavy bias toward `smooth_beatmix` and improve fade-in/fade-out quality across all transitions. Make the heuristic planner section-aware, drop-aware, and capable of selecting the full template palette.

* [x] **Fix template selection bias in heuristic planner**
  * [x] Use section proximity (near drop → `drop_swap`; near breakdown → `break_blend`/`filter_sweep`)
  * [x] Use energy trajectory: "building" → `filter_sweep`/`drop_swap`; "declining" → `break_blend`
  * [x] Use rhythm complexity/danceability: high complexity → `loop_roll`; groovy → `bass_kill_swap`
  * [x] Add randomized variety: when multiple templates score similarly, pick randomly instead of defaulting
  * [x] Lower the energy thresholds so more tracks trigger non-default templates

* [x] **Improve fade curves in action timelines**
  * [x] Replace linear dB ramps with equal-power / S-curve crossfades (perceptually smooth)
  * [x] Start incoming gain at -40 dB (audible sooner) instead of -60 dB (inaudible for most of ramp)
  * [x] Add crossfader movement to `smooth_beatmix` and `bass_kill_swap` timelines
  * [x] Outgoing fade starts earlier (bar 4 of 16 instead of bar 8) for longer, gentler exit
  * [x] Add gain compensation: slight boost at crossover point to prevent volume dip

* [x] **New transition template: `BackspinCut`**
  * [x] Simulated backspin: rapid pitch sweep down on outgoing → silence → hard cut to incoming
  * [x] Timeline: ramp outgoing gain down with filter sweep over 2 bars, then cut

* [x] **New transition template: `DropRide`**
  * [x] Blend incoming track starting from its drop cue, riding both drops simultaneously
  * [x] Timeline: bass-swap during dual-drop, outgoing fades over 8 bars

* [x] **Section-aware template scoring function**
  * [x] Score each template against current section context and pick the best fit
  * [x] Factor in: section proximity, energy delta, genre change, vocal overlap, rhythm profile

---

## Phase 27: Real-time Metering & Beat Tracking
- [ ] Replace synthetic beat grid with DP beat tracker (Ellis 2007)
- [ ] Real-time RMS/peak level meters per deck + master (UI VU meters)
- [ ] Spectrum analyzer visualization (FFT-based, per-deck or master)
- [ ] BPM confidence indicator in UI (show when detection is uncertain)

## Phase 28: Playlist & Set Management
- [ ] Create/save/delete playlists (tagged track collections)
- [ ] Drag-and-drop reorder tracks within playlist
- [ ] Load playlist into AutoDJ queue (play in order or shuffle)
- [ ] Export set as cue sheet (tracklist with timestamps)
- [ ] "Prepare set" mode — manually order tracks, preview transitions

## Phase 29: Final Polish & Export
- [ ] FLAC/MP3 export option (in addition to WAV)
- [ ] Normalize exported mix (LUFS target)
- [ ] CPU profiling + optimization pass on audio callback
- [ ] App icon, splash screen, about dialog
- [ ] Package for macOS (.dmg), Windows (.msi), Linux (.AppImage)