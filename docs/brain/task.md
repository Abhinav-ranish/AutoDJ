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

## Phase 27: Real-time Metering & Beat Tracking ✅
- [x] Replace synthetic beat grid with DP beat tracker (Ellis 2007) — already implemented in beats.rs
- [x] Real-time RMS/peak level meters per deck + master (UI VU meters)
- [x] Spectrum analyzer visualization (FFT-based, per-deck or master)
- [x] BPM confidence indicator in UI (show when detection is uncertain)

## Phase 28: Playlist & Set Management ✅
- [x] Create/save/delete playlists (tagged track collections)
- [x] Drag-and-drop reorder tracks within playlist
- [x] Load playlist into AutoDJ queue (play in order or shuffle)
- [x] Export set as cue sheet (tracklist with timestamps)
- [x] "Prepare set" mode — manually order tracks, preview transitions

## Phase 29: Adding Groq Integration ✅
- [x] Add Groq Integeration
- [x] Clean up the settings tab
- [x] Update the .env.example file to work with Groq

## Phase 30: Real DJ Performance Engine ✅

* [x] Add **cue-juggling / beat repeat** primitives (1/4, 1/2, 1, 2 bar repeat)
* [x] Add **slip mode** so stutters/repeats don't permanently derail track position
* [x] Add **jog/scratch simulation** primitives for vinyl brake, baby scratch, release
* [x] Add **spinback / brake / tape-stop** effect actions with safe timing windows
* [x] Add **micro-loop performance mode** (8/4/2/1 beat rolls)
* [x] Add **jump-to-phrase / jump-to-drop / jump-to-break** actions instead of always starting from track beginning
* [x] Add **incoming track start-point selection** using intro, first phrase, break, drop, hot cues
* [x] Add **phrase-preserving groove hold** so outgoing track can loop while incoming track is aligned
* [x] Add tests for repeat/roll/scratch actions not causing dead air or drift

## Phase 31: Transition Intelligence Overhaul ✅

* [x] Stop defaulting to `smooth_beatmix`; add stronger weighted competition across all templates
* [x] Enforce **template rotation / anti-repetition memory** so same transition is not reused too often
* [x] Score transitions using: section match, energy delta, vocal overlap, groove similarity, genre fit, tension/release
* [x] Add **late-groove protection** so transition is delayed if current section just became high-engagement
* [x] Add **too-early transition guard** to avoid cutting before groove settles
* [x] Add **too-late transition guard** to avoid dead-end outros unless intentional
* [x] Add pair-specific preferred transitions learned from successful prior mixes
* [x] Add fallback hierarchy per scenario: vocal clash, tempo mismatch, high energy, breakdown, drop-entry
* [x] Expose chosen transition score breakdown in debug UI

## Phase 32: Smart Entry Point & Track Timing ✅

* [x] Replace "start incoming track from beginning" with **marker-aware entry selection**
* [x] Compute best entry cue from intro/break/drop/first clean downbeat
* [x] Add **skip weak intros** for tracks whose intro is too sparse/long
* [x] Add **drop-entry mixing** when target track works better entering at drop
* [x] Add **break-entry mixing** for smoother harmonic/vocal blends
* [x] Add **dynamic offset alignment** so incoming phrasing lands on the correct bar of outgoing section
* [x] Add **entry confidence scoring** and fallback to safest cue
* [x] Persist AI-selected entry points to session history for future reuse

## Phase 33: LLM Decision Layer Audit + Hardening ✅

* [x] Audit whether Gemini/Groq calls are actually executed end-to-end during live planning
* [x] Trace all planner paths and log when system falls back to heuristics instead of LLM
* [x] Add explicit **planner mode state**: `llm_live`, `llm_failed_fallback`, `heuristic_only`
* [x] Verify `llm.rs` request path, auth headers, payload shape, timeout, and response parsing
* [x] Verify frontend settings truly persist provider/model/API key and are loaded into runtime planner
* [x] Add provider health-check command: `test_ai_provider`
* [x] Add structured logging of raw provider response, parsed decision, validation failures, fallback reason
* [x] Add UI badge showing whether current decision came from Gemini, Groq, or heuristic fallback
* [x] Add integration tests with mocked Gemini/Groq responses and forced failure cases

## Phase 34: LLM-Guided DJ Decision Schema v3 ✅

* [x] Expand AI output schema beyond "pick next track + template" into **full DJ intent**
* [x] Add fields for: `entry_marker`, `exit_marker`, `transition_style`, `loop_strategy`, `eq_strategy`, `fx_strategy`, `timing_reason`
* [x] Add **creative intensity** parameter so AI can choose safe vs aggressive mixing
* [x] Add **set arc awareness**: warmup / groove / peak / cooldown
* [x] Add **crowd-energy target** and **groove continuity target**
* [x] Add **anti-boredom rules** so AI avoids same texture repeatedly
* [x] Allow AI to suggest beat repeats, loop rolls, drop swaps, scratches, spinbacks only within validated safety limits
* [x] Add schema validator that converts risky AI ideas into safe engine actions
* [x] Add comparison mode: heuristic decision vs LLM decision vs hybrid winner

## Phase 35: Human-Like DJ Behavior Modeling ✅

* [x] Add **DJ personality profiles**: smooth, hype, aggressive, festival, lounge, open-format
* [x] Add profile-specific preferences for transition types, loop usage, cuts, FX density, timing aggressiveness
* [x] Add **tension-and-release engine** so AI sometimes holds a groove longer before switching
* [x] Add **phrase teasing**: preview incoming motif briefly before full transition
* [x] Add **double-drop logic** for compatible high-energy track pairs
* [x] Add **break-reset behavior** after multiple peak tracks
* [x] Add **energy pacing memory** so set feels intentional rather than random
* [x] Add user-selectable "play it safer" vs "DJ me harder" intensity slider

## Phase 36: Performance FX & Turntablism Toolkit ✅

* [x] Add engine actions for **echo out**, **reverb tail**, **filter slam**, **vinyl brake**, **spinback**, **transform cut**
* [x] Add optional **scratch pack** with a few safe canned scratch gestures
* [x] Add **beatmasher / stutter / gate** effect primitives
* [x] Add **one-shot hype actions** near drops only when confidence is high
* [x] Add style gating so FX-heavy behavior only happens in matching genres/moods
* [x] Add cooldown rules so flashy effects are not spammed
* [x] Add safety checks to prevent FX stacking that causes mud/clipping
* [x] Add tests for effect timelines and transition recovery behavior

## Phase 37: Repetition, Variety, and Set Memory ✅

* [x] Prevent repeated songs/artists too close together unless explicitly allowed
* [x] Prevent repeated **BPM rescue behavior** that keeps picking same tracks just to match tempo
* [x] Add **variety memory** across genre, vibe, key movement, transition type, and artist frequency
* [x] Add **recently played penalty** and **recently rejected penalty**
* [x] Add **transition diversity quota** over rolling session window
* [x] Add **motif fatigue detection** if too many tracks feel texturally similar
* [x] Add session-level reports: repeated transitions, repeated energy shapes, repeated artists
* [x] Add smarter tempo range expansion before reusing familiar tracks

## Phase 38: Live Timing, Groove, and Crowd Window Control ✅

* [x] Add **groove engagement detector** to identify when section is getting good and should ride longer
* [x] Add **momentum decay detector** to identify when it's time to transition sooner
* [x] Add **phrase-tension estimator** so transitions land at musically satisfying boundaries
* [x] Add "hold for 8/16/32 more bars" decision support
* [x] Add **window confidence scoring** for early / ideal / late mix points
* [x] Add AI/controller coordination so missed windows trigger replans, not awkward forced transitions
* [x] Add debugging timeline showing why a track was mixed early, on time, or late

## Phase 39: DJ Debug + Explainability Console ✅

* [x] Build advanced debug panel showing current section, phrase position, chosen entry cue, chosen transition, provider used
* [x] Show why a transition was selected over alternatives
* [x] Show why a track was started from intro, break, or drop
* [x] Show whether loop/repeat/scratch actions were AI-driven or rule-driven
* [x] Add planner trace export for one full session
* [x] Add "why not this track?" inspector for rejected candidates
* [x] Add "AI vs heuristic" side-by-side decision snapshots
* [x] Add replayable mix simulation for debugging decisions without real playback

## Phase 40: DJ Feel Tuning & Regression Suite ✅

* [x] Create subjective quality checklist: DJ vibe, groove hold, transition timing, variation, hype moments, smoothness
* [x] Build regression scenarios from real bad behaviors you observed
* [x] Add test fixtures for: too-early mix, too-late mix, repeated song, wrong entry point, overused smooth beatmix
* [x] Add scoring harness that ranks sessions on diversity, flow, timing, and musicality
* [x] Add manual review mode to compare old planner vs new planner on same playlist
* [x] Freeze "golden mixes" as benchmarks for future changes
* [x] Add final tuning pass for default settings so app feels like a DJ first, automation second

## Phase 41: Final Polish & Export
- [ ] FLAC/MP3 export option (in addition to WAV)
- [ ] Normalize exported mix (LUFS target)
- [ ] CPU profiling + optimization pass on audio callback
- [ ] App icon, splash screen, about dialog
- [ ] Package for macOS (.dmg), Windows (.msi), Linux (.AppImage)

## Phase 42: UI Redesign — Component Extraction ✅
- [x] Audit UI and create design doc (`docs/plans/2026-03-08-ui-redesign.md`)
- [x] Extract `utils.ts` (formatTime, formatGroove, filterToSlider, sliderToFilter)
- [x] Create `Icons.tsx` — 13 SVG icon components replacing all emoji icons
- [x] Extract `DeckPanel.tsx` (header + track info + waveform + progress + tempo + mixer + controls)
- [x] Extract `WaveformDisplay.tsx` (canvas renderer + VuMeter, HiDPI support)
- [x] Extract `TempoStrip.tsx` (tempo slider, pitch range, nudge, SYNC, reset)
- [x] Extract `MixerChannel.tsx` (EqKnob, MixerControls with gain/EQ/filter)
- [x] Extract `ControlStrip.tsx` (grouped rows: Transport, Loop+FX, Performance)
- [x] Extract `AiBar.tsx` (horizontal AI bar + SpectrumAnalyzer + TransitionsPanel + AutoDJ state machine)
- [x] Rewrite `App.tsx` to import extracted components (2554 → 993 lines, 61% reduction)
- [x] Add all new CSS classes to `index.css` (~730 lines)
- [x] Remove AiDebugPanel from DJ view (absorbed into AiBar)
- [x] Clean build verified (TypeScript + Vite)

## Phase 43: UI Redesign — Visual Polish & Deck Redesign ✅
- [x] Increase waveform height to 120px with better visual presence
- [x] Large monospace BPM readout per deck (42px, hardware-inspired)
- [x] Track title/artist get more space, playing state as glow badge
- [x] Center mixer channel strip between decks (3-column grid: Deck A | Mixer | Deck B)
- [x] Crossfader moved into center mixer with deck-colored labels
- [x] Panel depth: gradient surfaces, shadows, layered feel for all panels
- [x] Playing deck glow effect via `:has(.deck-state-badge.playing)`
- [x] Album art thumbnail styling

## Phase 44: UI Redesign — Control Grouping & Interaction Polish ✅
- [x] DJButton primitive with consistent sizing, press feedback, radial hover effect
- [x] Loop/Cue/FX/PERF rows get subtle panel backgrounds and group labels
- [x] Active state polish: loop-active glow, armed cue highlight, playing deck border glow
- [x] Hover/press microinteractions (scale transforms) for all interactive elements
- [x] Transitions panel redesigned with gradient surface, spinner, progress bar
- [x] Deck state badge pulse animation

## Phase 45: UI Redesign — AI Bar & Final Integration ✅
- [x] AI bar gradient surface with inset highlight
- [x] Mode pill active state glow (cyan for manual/assisted, green for auto-DJ)
- [x] Spectrum analyzer gradient bars with peak hold indicator
- [x] AI thinking pulse animation
- [x] Label hover glow (NOW cyan, NEXT amber)
- [x] Responsive layout: 3 breakpoints (1200px, 960px, 720px)
- [x] Visual harmonization: consistent tempo strip, slider transitions, scrollbar styling
- [x] Press feedback on sync, nudge, EQ kill buttons

## Phase 46: Performance FX-Driven Transitions ✅

Goal: Wire the existing FX action types (`EchoOut`, `ReverbTail`, `FilterSlam`, `VinylBrake`, `Spinback`) into heuristic transition templates. Currently defined as `MixActionKind` variants but never emitted by `build_heuristic_timeline`.

- [x] Add `inject_transition_fx()` helper that appends FX actions into existing template timelines
- [x] Wire FX per template: `echo_drop` → actual `EchoOut`, `backspin_cut` → `Spinback`/`VinylBrake`, `drop_swap` → `FilterSlam`, `break_blend` → `ReverbTail`, `loop_roll` → `BeatRepeat` with slip mode
- [x] Add personality-gated FX density (Lounge=rare, Smooth=occasional, Hype/Festival/Aggressive=heavy)
- [x] Add FX cooldown tracking (`recent_fx` on `AiPlanner`) — skip FX if same one used in last 2 transitions
- [x] Add FX stacking safety (no EchoOut + ReverbTail simultaneously, no VinylBrake + Spinback simultaneously)
- [x] Add new template `fx_echo_slam`: echo out → silence → filter slam incoming (scored high for energy jumps + Hype/Festival personality)
- [x] Update LLM system prompt with FX action documentation and parameter ranges
- [x] Tests: 10 new tests — verify templates emit FX, verify cooldown prevents spam, verify personality gating

## Phase 47: Vibe-Matched Transitions & Section Seeking ✅

Goal: Make the AI select tracks that match the current vibe and seek to the section in the incoming track that best matches what's currently playing, instead of always entering at intro/drop/first_beat.

- [x] Add `score_vibe()` to scorer with vibe flow rules (opener→warm_up=1.0, anthem→anthem=0.7, mismatched=0.2)
- [x] Add `weight_vibe: 0.10` to `AiConfig`, redistribute weights (key 0.25→0.20, variety 0.10→0.05)
- [x] Enhance `score_spectral()` to compare all 5 spectral dimensions (bass_energy, flatness, high_energy, centroid) not just centroid
- [x] Upgrade `select_entry_point` to section-to-section similarity: compare energy + label compatibility between current outgoing section and each candidate section in incoming track
- [x] Add `section_average_energy()` helper in sections.rs for section-level energy comparison
- [x] Expose `spectral_bass_energy`, `spectral_flatness`, `spectral_high_energy` in AI context
- [x] Tests: 4 new tests — vibe scoring flow, awkward flow detection, missing data handling, multi-dimensional spectral

## Phase 48: Track Recycling & Anthem Callbacks ✅

Goal: Allow the DJ to replay songs and bring back bangers. Currently `score_variety` hard-returns 0.0 for any recently played track.

- [x] Change `score_variety` from binary 0.0/1.0 to distance-based decay with vibe awareness (anthems can return after shorter cooldown)
- [x] Add `allow_replay: bool` and `anthem_callback_distance: usize` to `AiConfig`
- [x] Add anthem callback bonus in `select_next_extended`: at peak set position (>0.6), anthems past cooldown get +0.15 bonus
- [x] Add `track_play_counts: HashMap<String, usize>` to `AiPlanner` with hard cap of 3 plays per session
- [x] Callback transitions use different section/FX via existing `select_entry_point` section-similarity logic
- [x] Update quality evaluation: `RepeatedTrack` severity conditional on replay setting, add `ExcessiveReplay` for >3 plays
- [x] Expose replay config in AI context for LLM awareness
- [x] Tests: 3 new quality tests — replay severity conditional, excessive replay flagging, hard cap enforcement

## Phase 49: Advanced Auto-Cueing & Mix Point Intelligence ✅

Goal: Upgrade auto-cues from basic section markers (intro/drop/break/outro/first_beat) to intelligent mix points.

- [x] Add `detect_vocal_curve()` returning per-window vocal presence values (reuses existing FFT logic)
- [x] Add `detect_vocal_free_zones()` — consecutive windows with vocal energy < threshold for 4+ windows
- [x] Add `detect_impact_moments()` — energy curve derivative spikes (sudden energy jumps), deduplicated within 4 seconds
- [x] Add `compute_best_mix_points()` — best mix-in (vocal-free, low energy, phrase-aligned) and best mix-out (near outro, pre-decline, phrase-aligned)
- [x] Integrate into analysis pipeline after existing `generate_auto_cues`, append new cue types
- [x] New cue types: `best_mix_in`, `best_mix_out`, `vocal_free_start`, `vocal_free_end`, `impact`
- [x] Use in entry point selection: prefer `best_mix_in` cue, use `vocal_free_start` when outgoing has vocals, use `impact` for drop_swap/echo_drop
- [x] Add `best_mix_out` as transition window (score 0.9), vocal-free zones as windows (0.75), impacts as windows (0.55)
- [x] Tests: vocal-free detection, impact detection, mix-in prefers vocal-free, mix-out near outro

