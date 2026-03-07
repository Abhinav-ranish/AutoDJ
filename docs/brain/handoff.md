# AutoDJ Project Handoff

## Project State
**AutoDJ** is a high-performance DJ application built with a memory-safe **Rust** audio engine and backend, coupled with a fast **TypeScript/React** frontend via **Tauri v2**. The core audio engine, UI, track analysis pipeline, and AI DJ Conductor are fully implemented and functional through Phase 12.

## Current Position
- **Phases 1–13**: Complete
- **Phase 14–17 (AI Enrichment Overhaul)**: Active roadmap
- **Next concrete work**: Phase 14 — AI Decision Engine Overhaul

## Recent Accomplishments (Latest Session — Phase 13)
- **AI Track Classifier** (`classifier.rs`): Gemini-powered classification producing genre, subgenre, mood, vibe, mix_notes, and ai_tags per track from DSP features + metadata.
- **Classification DB Schema**: 6 new columns on `track_profiles` (genre, subgenre, mood, vibe, mix_notes, ai_tags) + `save_classification()` method.
- **TrackRecord Expansion**: Library queries now surface genre/mood/vibe fields to the frontend.
- **Tauri Command**: `classify_track` command wired end-to-end (get profile → build ClassificationInput → call Gemini → save to DB → return result).
- **Tests**: 53 total passing (3 AI, 15 analysis, 29 core, 6 storage).

## Core Features Implemented (Phases 1–10)
- **Low-Latency Audio Engine**: `cpal`-based with `SPSC` lock-free command queues, 3-band EQ, SVF filters, and seamless loops.
- **Track Analysis (`autodj-analysis`)**: Parallel processing pipeline extracting BPM, Key (Camelot mapped), Loudness, Energy, Waveform peaks, Perceived Tempo/Groove classification, Spectral profile, Vocal presence, Rhythm complexity/danceability, and structural markers (intro/outro/drop).
- **Transitions**: Template-driven transitions (`SmoothBeatmix`, `FilterSweep`, `EchoDrop`) that schedule commands on beat boundaries.
- **AI Conductor**: Heuristic scorer and Gemini LLM planner (`gemini-2.5-flash`) evaluating the next best track and optimal transition point. Uses perceived tempo over raw BPM for groove-aware mixing.
- **UI/UX**: Waveform canvas, drag-and-drop import, keyboard shortcuts, hot cues, dynamic pitch range, ID3 metadata display, Gemini API key management.

## What's Next — AI Enrichment Overhaul (Phases 13–17)

| Phase | What | Key Files |
|-------|------|-----------|
| ~~12~~ | ~~DSP Enrichment~~ | ~~`spectral.rs`, `vocal_detection.rs`, `rhythm.rs`~~ **DONE** |
| ~~13~~ | ~~Gemini Classification — genre/mood/vibe per track via LLM~~ | ~~`classifier.rs`~~ **DONE** |
| 14 | AI Decision Engine — 7-weight scorer, section-aware transitions | `scorer.rs`, `context.rs` (modify) |
| 15 | Mix History — wire up existing session/history DB tables | `db.rs`, `planner.rs` (modify) |
| 16 | Audio Export — record live mix to WAV | New recording module |
| 17 | Visual Polish — album art, DP beat tracker, better sections | Multiple files |

**Detailed plans**: `docs/plans/2026-03-07-ai-enrichment-design.md` and `docs/plans/2026-03-07-ai-enrichment-implementation.md`

## Known Gaps (Carried Forward)
- `sections.rs` — 15% heuristic split, not self-similarity matrix (Phase 17)
- `beats.rs` — synthetic grid only, no DP tracker (Phase 17)
- `safe_loops` — now populated from energy+beat analysis, could be improved (Phase 17)
- `request_next_track` — placeholder artist context (Phase 14)
- `AiConfig::default()` model field still says `"gpt-4o-mini"` (Phase 14)
- `mix_history` — DB schema exists, no app logic wired (Phase 15)

## Environment & Build Setup
- **Requirements**: `rustup`, Node.js, `npm`, `tauri-cli` v2, and `ffmpeg` (for importing audio).
- **Start development**: `cargo tauri dev`
- **Run test suite**: `cargo test --workspace`
- **Config**: Create `.env` from `.env.example` in the root providing `GEMINI_API_KEY`.

## Key Documentation
- `docs/brain/task.md` — Master task checklist (Phases 1–17)
- `docs/brain/gaps.md` — Known bugs, gaps, and recommendations
- `docs/brain/implementation_plan.md` — Historical plan for Phases 5–9
- `docs/plans/2026-03-07-ai-enrichment-*.md` — Active plan for Phases 12–17
