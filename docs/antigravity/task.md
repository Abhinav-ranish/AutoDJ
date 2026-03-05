# Auto-DJ — Master Task List

## Phase 1: Architecture Documentation
- [x] Write `docs/00-overview.md` through `docs/09-mvp-roadmap.md`
- [x] Get user approval on architecture + implementation plan

## Phase 2: Milestone A — Foundations
- [x] Tauri 2 app skeleton + Rust workspace (5 crates)
- [x] SQLite storage layer with schema
- [x] File import: hash → copy → decode pipeline
- [x] FFmpeg decode → canonical PCM cache (ADJP format)
- [x] Minimal cpal audio output (engine + SPSC queues)
- [x] Basic React UI (library view + DJ view + crossfader)
- [x] Unit tests (18 passing: buffer, command, transport, cache, db)

## Phase 3: Milestone B — Two Decks + Mixing
- [x] Per-deck DSP: 3-band EQ (biquad) + SVF filter
- [x] Wire EQ/filter into deck signal chain (PCM → EQ → Filter → Gain)
- [x] Handle SetEq/SetFilter commands in engine callback
- [x] Wire TrackLoader into audio callback
- [x] Add EQ/filter values to DeckSnapshot
- [x] Add set_eq/set_filter Tauri commands
- [x] Add UI: gain slider, EQ knobs (HI/MID/LO + kill), filter knob per deck
- [x] Tests (25 total: +7 new for EQ passthrough/kill/gain, filter bypass/LP/HP/clamping)

## Phase 4: Milestone C — Sync + Quantization
## Phase 5: Milestone D — Analysis
## Phase 6: Milestone E — Transitions
## Phase 7: Milestone F — AI Conductor
## Phase 8: Milestone G — Polish + Export
