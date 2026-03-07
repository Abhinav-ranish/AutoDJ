# AutoDJ Project Handoff

## Project State
**AutoDJ** is a high-performance DJ application built with a memory-safe **Rust** audio engine and backend, coupled with a fast **TypeScript/React** frontend via **Tauri v2**. The core audio engine, UI, track analysis pipeline, AI DJ Conductor, and all enrichment phases are fully implemented and functional through Phase 17.

## Current Position
- **Phases 1–17**: Complete
- **AI Enrichment Overhaul (Phases 12–17)**: All done
- **61 tests passing**, zero warnings

## Recent Accomplishments (Latest Session — Phases 14–17)

### Phase 14: AI Decision Engine Overhaul
- **7-weight heuristic scorer** (`scorer.rs`): tempo, key, energy, variety, genre, mood, spectral. Added `ScoringCandidate` struct, genre family groups (10 families), mood adjacency wheel.
- **Enriched AI context** (`context.rs`): genre/mood/vibe, vocal_presence, energy_trajectory, intro/outro/drop timestamps, set flow tracking.
- **Section-aware transitions**: Uses outro_length_ms for optimal transition timing.
- **Vocal-aware template selection**: Avoids vocal overlap, uses filter_sweep for genre changes.
- **Fixed AiConfig model** from "gpt-4o-mini" to "gemini-2.5-flash".

### Phase 15: Mix History & Session Tracking
- **DB methods**: `get_all_classifications`, `create_session`, `end_session`, `record_track_play`, `get_recent_track_ids`.
- **Auto-session**: Session created on first `load_track`, track plays recorded automatically.
- **Mix history**: AI decisions recorded in mix_history for feedback loop.

### Phase 16: Audio Export
- **WAV Recorder** (`recorder.rs`): Lock-free ring buffer capture from audio callback, writer thread drains to 32-bit float WAV file.
- **Tauri commands**: `start_recording(filename)`, `stop_recording()`.
- **UI**: Recording button with blinking red dot + elapsed time counter.

### Phase 17: Visual Polish & Analysis
- **Album art extraction**: Lofty extracts cover art during import, saved to `~/.autodj/cache/art/`.
- **Energy-based sections**: Replaced 15% heuristic with energy curve analysis detecting intro/buildup/drop/breakdown/outro.
- **Library UI**: Genre badges (purple) + mood badges (orange), 9-column grid layout.

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
- `beats.rs` — synthetic grid only, no DP tracker
- Album art display in deck panels — extracted but not shown in deck UI
- Recording limited to one session per app launch
- Batch classification pipeline (per-track available but no bulk UI)
- MIDI controller support
- Key shift / harmonic pitch lock

## Environment & Build Setup
- **Requirements**: `rustup`, Node.js, `npm`, `tauri-cli` v2, and `ffmpeg` (for importing audio).
- **Start development**: `cargo tauri dev`
- **Run test suite**: `cargo test --workspace`
- **Config**: Create `.env` from `.env.example` in the root providing `GEMINI_API_KEY`.

## Key Documentation
- `docs/brain/task.md` — Master task checklist (Phases 1–17)
- `docs/brain/gaps.md` — Known bugs, gaps, and recommendations
- `docs/brain/implementation_plan.md` — Historical plan for Phases 5–9
- `docs/plans/2026-03-07-ai-enrichment-*.md` — Plans for Phases 12–17
