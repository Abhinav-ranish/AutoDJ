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
- [x] `SchedulePoint` resolution in `TransportClock` (NextBeat, NextBar, AfterBeats, etc.)
- [x] Pre-allocated pending command queue in audio callback (sorted by target_sample)
- [x] Deck loops (set_loop, clear_loop) with beat-aligned wrapping
- [x] Deck seek + SetLoop/ClearLoop/Seek commands
- [x] Tauri commands: seek, set_loop, clear_loop, set_tempo
- [x] UI: loop buttons (4/8/16 bars) with active indicator
- [x] Tests: 31 total (25 core + 6 storage)

## Phase 5: Milestone D — Analysis
## Phase 6: Milestone E — Transitions
## Phase 7: Milestone F — AI Conductor
## Phase 8: Milestone G — Polish + Export
