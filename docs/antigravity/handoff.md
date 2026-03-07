# AutoDJ Project Handoff

## Project State
**AutoDJ** is a high-performance DJ application built with a memory-safe **Rust** audio engine and backend, coupled with a fast **TypeScript/React** frontend via **Tauri**. The core audio engine, UI, track analysis pipeline, and AI DJ Conductor are largely implemented and functional.

## Recent Accomplishments (Latest Session)
- **Gemini AI Integration**: Successfully migrated the LLM client to use Google's Gemini API (`gemini-2.5-flash`) with strict JSON structured outputs parsing `execute_at_ms` for precise transition timing.
- **BPM & Metadata Rendering Fix**: Fixed a bug where deck BPM and track titles were incorrect because the frontend inferred tracks based on duration. The active `track_id` is now passed directly from the Rust engine's `DeckSnapshot` to the frontend via lock-free channels.
- **API Key Security & Persistence**: Implemented a secure settings modal for the Gemini API key. The key is now masked ("✓ AIz....123X") in the UI, persisted in browser `localStorage`, and dynamically loaded from a local `.env` file via the `dotenvy` crate. Accidental leakages in `.env` have been completely scrubbed from the git history and `.gitignore` updated.
- **UI Enhancements**: Implemented drag-and-drop file importing, keyboard shortcuts (Space, Z/X, Q/W), and a dynamic pitch range selector (±6%, ±8%, ±10%, ±16%) for precise tempo control.
- **Perceived Tempo Engine**: New analysis module that classifies groove types (four_on_floor, trap_half_time, dembow, breakbeat) and computes a perceived tempo metric. The AI scorer and LLM context now use perceived tempo instead of raw BPM for groove-aware mixing decisions.

## Core Features Implemented
- **Low-Latency Audio Engine**: `cpal`-based with `SPSC` lock-free command queues, 3-band EQ, SVF filters, and seamless loops.
- **Track Analysis (`autodj-analysis`)**: Parallel processing pipeline extracting BPM, Key (Camelot mapped), Loudness, Energy, and Waveform peaks.
- **Transitions**: Template-driven transitions (`SmoothBeatmix`, `FilterSweep`, `EchoDrop`) that schedule multiple concurrent parameter commands precisely on beat boundaries.
- **AI Conductor**: Heuristic scorer and Gemini LLM planner evaluating the next best track to play and the optimal transition point based on tempo, key, and energy dynamics.

## Next Steps / Pending Items (See `gaps.md` for full list)
1. **Mix History & Session Tracking**: Logic to record played tracks to a session log database table.
2. **Audio Export**: Ability to bounce/record the live mix directly to a `.wav` file.
3. **UI/UX Polish**: 
   - Album art extraction and display in the deck and library views.
4. **Analysis Enhancements**: 
   - Replace the synthetic beat grid generator with a proper Dynamic Programming (DP) beat tracker.
   - Improve structural section analysis (Intro, Drop, Breakdown) beyond the current 15% heuristic split.

## Environment & Build Setup
- **Requirements**: `rustup`, Node.js, `npm`, `tauri-cli` v2, and `ffmpeg` (for importing audio).
- **Start development**: `cargo tauri dev`
- **Run test suite**: `cargo test --workspace`
- **Config**: Create `.env` from `.env.example` in the root explicitly providing `GEMINI_API_KEY`.
