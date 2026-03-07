# AutoDJ Project Handoff

## Project State
**AutoDJ** is a high-performance DJ application built with a memory-safe **Rust** audio engine and backend, coupled with a fast **TypeScript/React** frontend via **Tauri v2**. The core audio engine, UI, track analysis pipeline, and AI DJ Conductor are fully implemented and functional through Phase 12.

## Current Position
- **Phases 1–12**: Complete
- **Phase 13–17 (AI Enrichment Overhaul)**: Not started — this is the active roadmap
- **Next concrete work**: Phase 13 — Gemini Classification

## Recent Accomplishments (Latest Session — Phase 12)
- **Spectral Analysis Module** (`spectral.rs`): FFT-based spectral centroid (brightness), spectral flatness (tonality vs noise), and 3-band energy distribution (bass/mid/high) with Hann windowing.
- **Vocal Detection Module** (`vocal_detection.rs`): Vocal frequency range energy ratio + harmonic prominence scoring, returns 0.0–1.0 vocal presence.
- **Rhythm Analysis Module** (`rhythm.rs`): Onset-based rhythm complexity and danceability scoring using inter-onset interval statistics and tempo suitability.
- **Pipeline Integration**: All three new analyzers run in parallel via rayon within the existing analysis pipeline.
- **DB Schema Expansion**: 12 new columns added to `track_profiles` via migrations.
- **Energy Trajectory + Structural Markers**: Auto-computed energy trajectory classification, intro/outro length detection from energy curve, first drop timestamp.

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
| 13 | Gemini Classification — genre/mood/vibe per track via LLM | `classifier.rs` (new) |
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
