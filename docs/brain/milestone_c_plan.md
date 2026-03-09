# Milestone C — Sync + Quantization

## Goal

Add tempo sync, beat-quantized command scheduling, and looping. After this milestone, commands can execute on beat/bar boundaries, decks can sync to master BPM, and users can set beat-aligned loops.

## What Already Exists

| Component | Status |
|-----------|--------|
| `TransportClock` | Tracks beat/bar numbers, `next_beat_sample()`, `next_bar_sample()` |
| `SchedulePoint` enum | `Immediate`, `NextBeat`, `NextBar`, `AtBeat`, `AfterBeats`, `AfterBars` |
| `ScheduledCommand` struct | Has `schedule` and `target_sample` fields |
| Command ring buffer | Consumes commands immediately in callback |

## What's Missing

| Gap | Detail |
|-----|--------|
| **Schedule resolution** | Commands always execute immediately — `SchedulePoint` is ignored |
| **Command heap** | No min-heap to hold pending scheduled commands until their target sample |
| **Loop system** | No loop fields on Deck, no `SetLoop`/`ClearLoop` command handling |
| **Tempo sync** | No `SyncToMaster` command handling, no deck-to-master BPM matching |
| **Seek** | No `Seek` command handling |
| **UI** | No loop controls, no sync button, no tempo display per deck |

---

## Proposed Changes

### autodj-core

#### [MODIFY] `crates/autodj-core/src/engine.rs`
- Add a pre-allocated `Vec<ScheduledCommand>` (max 256) as a simple sorted pending queue
- On each audio buffer: resolve `SchedulePoint` → target sample using `TransportClock`, insert into pending queue
- Each sample: drain all pending commands whose `target_sample <= sample_pos` and execute them
- Support `EngineCommandSender::send_scheduled()` with explicit `SchedulePoint`
- Handle `Seek`, `SetLoop`, `ClearLoop`, `SyncToMaster` commands

#### [MODIFY] `crates/autodj-core/src/deck.rs`
- Add `loop_start: Option<usize>` and `loop_end: Option<usize>` fields (frame positions)
- In `next_frame()`: when position reaches `loop_end`, jump back to `loop_start`
- Add `set_loop(start_frame, end_frame)`, `clear_loop()`, `seek(frame)` methods
- Add loop state to snapshot reporting

#### [MODIFY] `crates/autodj-core/src/transport.rs`
- Add `resolve_schedule(&self, schedule: &SchedulePoint) -> u64` method
  - Converts `NextBeat` → `next_beat_sample()`, `NextBar` → `next_bar_sample()`, etc.
- Add `beat_to_frame(beat: u64) -> u64` for seeking by beat number

#### [MODIFY] `crates/autodj-core/src/command.rs`
- No structural changes needed — all command variants already exist

#### [MODIFY] `crates/autodj-core/src/engine.rs` (DeckSnapshot)
- Add `loop_active: bool`, `loop_start_ms: u64`, `loop_end_ms: u64` fields
- Add `bpm: f64` field per deck (for tempo display)

---

### autodj-app

#### [MODIFY] `crates/autodj-app/src/commands.rs`
- Add `seek(deck, position_ms)` command
- Add `set_loop(deck, start_ms, length_beats)` command
- Add `clear_loop(deck)` command
- Add `sync_to_master(deck)` command
- Add `set_tempo(deck, bpm)` command

#### [MODIFY] `crates/autodj-app/src/lib.rs`
- Register new commands

---

### React UI

#### [MODIFY] `ui/src/store.ts`
- Add actions: `seek`, `setLoop`, `clearLoop`, `syncToMaster`, `setTempo`
- Update `DeckSnapshot` with loop/bpm fields

#### [MODIFY] `ui/src/App.tsx`
- Add per-deck: SYNC button, tempo +/- nudge, loop toggle (4/8/16 bars), seek-on-click

---

## Verification Plan

### Automated Tests
```bash
cargo test --workspace
```

New tests:
1. `transport::tests::test_resolve_schedule_next_beat` — verify beat resolution
2. `transport::tests::test_resolve_schedule_next_bar` — verify bar resolution
3. `deck::tests::test_loop_playback` — verify position wraps at loop end
4. `deck::tests::test_seek` — verify seek moves position
5. `engine::tests::test_scheduled_command` — verify delayed command execution (if testable outside audio thread)

### Manual Verification
1. `cargo tauri dev` → load track → set 4-bar loop → hear it repeat
2. Load second track → press SYNC → tempos match
3. Schedule a command on NextBar → verify it executes on the downbeat

## Milestone C Status ✅
- Finished.
