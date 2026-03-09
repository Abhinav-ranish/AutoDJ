# AutoDJ Project Handoff

## Project State
**AutoDJ** is a high-performance DJ application built with a memory-safe **Rust** audio engine and backend, coupled with a fast **TypeScript/React** frontend via **Tauri v2**. The core audio engine, UI, track analysis pipeline, AI DJ Conductor, all enrichment phases, autonomous mixing loop, DJ-style live mixing with AI performance controls, library management, settings persistence, visual polish, batch operations, and safety monitoring are fully implemented and functional through Phase 25.

## Current Position
- **Phases 1–25**: Complete
- **AI Enrichment Overhaul (Phases 12–17)**: All done
- **Autonomous Mixing & AI Controls (Phases 18–20)**: All done
- **Library, Settings, Polish, Safety (Phases 21–25)**: All done
- **65 tests passing**, zero warnings, TypeScript compiles clean

## Recent Accomplishments (Latest Session — Phases 21–25)

### Phase 21: Library Search, Sort & Filter
- SQL-backed text search on title/artist via `search_tracks` command
- BPM range, key, genre, mood filter support
- Column-header click-to-sort (toggles ASC/DESC) for all major columns
- Search bar + filter pills UI in library toolbar with clear filters button

### Phase 22: Preferences & Settings Persistence
- `get_preference()`/`set_preference()` DB methods for key-value storage
- AI config persisted to SQL preferences (with localStorage migration fallback)
- Audio device enumeration via `cpal` + `get_audio_devices` command
- `SettingsPanel` component with 3 tabs (AI, Audio, AutoDJ)
- Preferences auto-restored on app launch via `initStore`

### Phase 23: Visual Polish & UX
- Album art display in deck headers (36x36 thumbnail via `convertFileSrc`)
- 4 color-coded hot cue buttons (set/jump, red/green/blue/yellow)
- Right-click context menu on library rows (Load to A/B, Re-analyze, Classify)

### Phase 24: Batch Operations & AI Polish
- "Classify All" button for batch classification of unanalyzed tracks
- Sequential processing with progress indicator ("Classifying X/Y")
- Smart skip of already-classified and unanalyzed tracks

### Phase 25: Safety & Reliability
- RMS-based silence detection in audio callback (~30Hz, ~-80 dB threshold)
- Emergency recovery: 90 consecutive silence frames (~3s) → auto-load next track
- `master_rms`, `silence_frames`, `underrun_count` in engine snapshots
- AutoDJ controller monitors silence for autonomous recovery

## Previous Accomplishments (Phases 19–20)

### Phase 19: DJ-Style Live Mixing + Phrase/Section-Aware Transitions
- **Section-aware transition windows** (`sections.rs`): `generate_transition_windows()` computes safe mix windows from energy-based section markers (intro/outro/drop/break). `TransitionWindow` struct with anchor, start/end ms, and score.
- **Phrase alignment** (`sections.rs`): `build_phrase_grid()` derives 8-bar phrase boundaries from beat grid. `next_phrase_boundary()` snaps transition timing to musical phrases.
- **Auto-cue system** (`sections.rs` + `db.rs`): `generate_auto_cues()` produces canonical markers (CUE_INTRO, CUE_DROP, CUE_BREAK, CUE_OUTRO, CUE_FIRST_BEAT). New `auto_cues` DB table with save/get methods.
- **4 new transition templates** (`transition.rs`): `DropSwap` (HP sweep → hard cut on drop), `LoopRoll` (4→2→1 bar stutter), `BassKillSwap` (mids/highs first, bass swap), `BreakBlend` (gentle HP + quadratic fade avoiding vocals). Total: 7 templates.
- **AutoDJ controller overhaul** (`App.tsx`): State machine expanded to `idle → monitoring → planning → loading → aligning → transitioning`. Section-aware windowed triggers replace simple 75% position check. Phrase boundary alignment with polling (100ms interval, 15s timeout). Emergency fallback preserved.
- **Pipeline integration** (`pipeline.rs`): Transition windows, phrases, and auto-cues generated during analysis and stored in `TrackProfile`.

### Phase 20: AI Performance Controls — EQ/Filter Automation + Decision Schema v2
- **EQ isolator mode** (`eq.rs`): LR4 cascade filters for steep isolator slopes. Per-band kill switches (`kill_low`, `kill_mid`, `kill_high`). `set_kill()` and `set_isolator_mode()` methods.
- **New engine commands** (`command.rs` + `engine.rs`): `SetEqKill(deck, band, enabled)`, `SetEqMode(deck, Classic|Isolator)`, `SeekMs(deck, position)`. DeckSnapshot extended with `eq_kill_low/mid/high` + `eq_isolator` fields.
- **AI Decision schema v2** (`planner.rs`): `MixAction` struct with `at_ms` + `MixActionKind` enum (12 types: LoadTrack, SetTempo, SetGain, SetFilter, SetEqBand, SetEqKill, SetLoop, ClearLoop, SetCrossfader, Play, Pause, ExecuteTemplate). AiDecision now includes `actions`, `reasoning_tags`, `confidence`.
- **Heuristic timeline generation** (`planner.rs`): `build_heuristic_timeline()` generates full action sequences for each template type. Section-aware timing via transition windows + phrase snap. Improved template selection: vocal overlap → break_blend, energy jump → drop_swap/loop_roll, genre change → bass_kill_swap.
- **Expanded system prompt** (`planner.rs`): Describes all 7 templates, section-aware timing rules, action timeline v2 format, and safety constraints for LLM.
- **Context enrichment** (`context.rs`): `CurrentStateContext` now includes `transition_windows` and `auto_cues`. `TrackProfileShort` includes `auto_cues`.
- **User feedback** (`db.rs`): `transition_feedback` table (session_id, from/to track, template, rating, notes, decision_json). `save_transition_feedback()` method.
- **5 new Tauri commands** (`commands.rs` + `lib.rs`): `set_eq_kill`, `set_eq_mode`, `seek_ms`, `get_auto_cues`, `submit_transition_feedback`.
- **UI updates** (`App.tsx` + `store.ts`): EQ kill buttons wired to `setEqKill`. AI panel shows reasoning tags, confidence badge, action timeline preview (first 6 actions). All 7 templates in transition dropdown. Store updated with `MixAction` interface and 5 new actions.

### Phase 18: Autonomous AutoDJ Loop (previous session)
- Position-aware controller, auto-classify on analysis, auto-pick first track, intelligent transition at 75%/outro, emergency fallback at 92%, continuous deck chaining.

## Core Features Implemented (Phases 1–10)
- **Low-Latency Audio Engine**: `cpal`-based with `SPSC` lock-free command queues, 3-band EQ, SVF filters, and seamless loops.
- **Track Analysis (`autodj-analysis`)**: Parallel processing pipeline extracting BPM, Key (Camelot mapped), Loudness, Energy, Waveform peaks, Perceived Tempo/Groove classification, Spectral profile, Vocal presence, Rhythm complexity/danceability, and structural markers (intro/outro/drop).
- **Transitions**: Template-driven transitions (`SmoothBeatmix`, `FilterSweep`, `EchoDrop`) that schedule commands on beat boundaries.
- **AI Conductor**: Heuristic scorer and Gemini LLM planner (`gemini-2.5-flash`) evaluating the next best track and optimal transition point. Uses perceived tempo over raw BPM for groove-aware mixing.
- **UI/UX**: Waveform canvas, drag-and-drop import, keyboard shortcuts, hot cues, dynamic pitch range, ID3 metadata display, Gemini API key management.

## Architecture

| Crate | Purpose |
|-------|---------|
| `autodj-core` | Audio engine, decks, mixer, EQ, filters, transport, recorder |
| `autodj-analysis` | BPM, key, energy, loudness, spectral, vocal, rhythm, sections, waveform |
| `autodj-ai` | Heuristic scorer, AI planner, LLM client, classifier, Camelot utils |
| `autodj-storage` | SQLite database, PCM/art cache, file hashing, decoder |
| `autodj-app` | Tauri commands, app state, main entrypoint |

## Known Gaps (Future Work)
- `beats.rs` — synthetic grid only, no DP tracker (Phase 26 will address)
- Recording limited to one session per app launch
- `BackspinExit` transition template (style-gated, deferred)
- 4-band EQ mode + per-deck EQ presets
- AI action rate limiting (>N actions per 8 bars)
- Preference bias from feedback in scorer
- Live AI overlay on deck UI + "Lock controls" toggles
- Waveform hot cue markers, mix history view, responsive layout
- Inline edit for AI classifications, energy arc goal parameter
- End-of-track detection by audio signal, crash-safe session recovery
- MIDI controller support
- Key shift / harmonic pitch lock

## Environment & Build Setup
- **Requirements**: `rustup`, Node.js, `npm`, `tauri-cli` v2, and `ffmpeg` (for importing audio).
- **Start development**: `cargo tauri dev`
- **Run test suite**: `cargo test --workspace`
- **Config**: Create `.env` from `.env.example` in the root providing `GEMINI_API_KEY`.

## Key Documentation
- `docs/brain/task.md` — Master task checklist (Phases 1–25 complete, 26–28 planned)
- `docs/brain/gaps.md` — Known bugs, gaps, and recommendations
- `docs/brain/implementation_plan.md` — Historical plan for Phases 5–9
- `docs/brain/walkthrough.md` — Git history + phase implementation walkthrough
- `docs/plans/2026-03-07-ai-enrichment-*.md` — Plans for Phases 12–17
