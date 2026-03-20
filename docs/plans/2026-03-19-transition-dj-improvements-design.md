# Transition & DJ Feature Improvements

**Date:** 2026-03-19
**Status:** Approved
**Problem:** Transitions sound abrupt — simulated echo effects use gain fades instead of real delay DSP, and the AI's fine-grained MixAction timelines are never executed by the conductor.

## Scope

Two high-impact changes:
1. Tempo-synced delay DSP per deck (replaces simulated echo effects)
2. AI action execution in the conductor (dispatches MixAction timelines as engine commands)

## 1. Tempo-Synced Delay DSP

### New module: `crates/autodj-core/src/delay.rs`

Stereo feedback delay with BPM-synced timing and ping-pong mode.

**Buffer:** Pre-allocated circular buffer for max 2 seconds at 48kHz (96,000 stereo frames). Zero-allocation in the audio callback.

**Parameters:**
- `subdivision: f32` — delay time in beats (0.25 = 1/16th, 0.5 = 1/8th, 1.0 = quarter note)
- `feedback: f32` — feedback amount, clamped to [0.0, 0.85] to prevent runaway
- `wet: f32` — dry/wet mix [0.0, 1.0]
- `ping_pong: bool` — when true, alternates delay taps between L and R channels

**Delay time calculation:**
```
delay_frames = (60.0 / bpm) * subdivision * sample_rate
```

**Processing per frame:**
1. Read delayed sample from `buffer[write_head - delay_frames]`
2. For ping-pong: apply L/R panning based on tap count
3. Output = `dry * input + wet * delayed`
4. Write `input + delayed * feedback` at write head
5. Advance write head

**BPM tracking:** The deck passes current BPM into `process()` so delay time smoothly follows tempo changes without clicks (crossfade between old and new delay times over ~64 samples).

### Signal chain change in `deck.rs`

```
Before: PCM -> EQ -> Filter -> Gain
After:  PCM -> EQ -> Filter -> Delay -> Gain
```

Add `pub delay: StereoDelay` field to `Deck` struct, initialized in `with_sample_rate()`.

### New commands in `command.rs`

```rust
SetDelay { deck: DeckId, subdivision: f32, feedback: f32, wet: f32, ping_pong: bool },
ClearDelay { deck: DeckId },
```

Add to `CommandValidator` with clamping: subdivision [0.0625, 4.0], feedback [0.0, 0.85], wet [0.0, 1.0].

### Template upgrades

**`echo_drop`** (currently simulated gain fade):
- Replace exponential gain ramp with `SetDelay { subdivision: 0.5, feedback: 0.6, wet: 0.8 }`
- After the silence gap + hard cut, `ClearDelay` on the outgoing deck
- Remove "[simulated]" tag

**`fx_echo_slam`** (currently simulated):
- Use real delay tail on outgoing deck instead of gain decay approximation
- Keep the filter slam on incoming deck (that part is already real DSP)
- Remove "[simulated]" tag

### Frontend controls

Add delay controls to `ControlStrip.tsx`:
- Echo ON/OFF toggle button
- Subdivision selector (1/4, 1/2, 3/4, 1 beat)
- Feedback knob

### MixAction integration

Add `SetDelay` and `ClearDelay` to the `MixActionKind` enum in `planner.rs`. Update the system prompt to document the new real delay effect, removing the "[simulated]" notes from echo_drop and fx_echo_slam.

## 2. AI Action Execution

### Problem

The AI planner generates `MixAction` timelines with timestamped commands (SetGain, SetFilter, SetEq, etc.) but the conductor ignores them entirely. It always falls through to `Command::ExecuteTransition { template, params }` which runs template expansion.

### Solution

In `conductor.rs`, before dispatching `ExecuteTransition`, check if the AI decision includes a non-empty action timeline. If yes, convert each action to engine commands and dispatch them directly, skipping template expansion.

### Implementation in `conductor.rs` (Transitioning phase, ~line 852)

```
if let Some(actions) = decision.actions && !actions.is_empty():
    for action in actions:
        command = convert_mix_action_to_command(action.kind, playing_deck, idle_deck)
        schedule = ms_to_schedule_point(action.at_ms, current_position_ms, bpm, sample_rate)
        send_scheduled_command(command, CommandSource::AI, schedule)
    // Don't run ExecuteTransition
else:
    // Existing template expansion path
    send_command(ExecuteTransition { template, params })
```

### MixAction to Command mapping

| MixActionKind | Engine Command |
|---|---|
| `SetGain { deck, gain_db }` | `SetDeckGain` |
| `SetFilter { deck, cutoff_hz, resonance }` | `SetFilter` |
| `SetEqBand { deck, band, gain_db }` | `SetEq` |
| `SetEqKill { deck, band, enabled }` | `SetEqKill` |
| `SetCrossfader { position }` | `SetCrossfader` |
| `SetLoop { deck, length_bars }` | `SetLoop` |
| `ClearLoop { deck }` | `ClearLoop` |
| `BeatRepeat { deck, beats }` | `BeatRepeat` |
| `BeatRepeatOff { deck }` | `BeatRepeatOff` |
| `VinylBrake { deck, duration_ms }` | `VinylBrake` |
| `Spinback { deck, duration_ms }` | `Spinback` |
| `EchoOut { deck, .. }` | `SetDelay` (new) |
| `FilterSlam { deck, .. }` | `SetFilter` (rapid sweep) |
| `ExecuteTemplate { name, params }` | `ExecuteTransition` |

### Timing conversion

Convert `at_ms` (relative to transition start) to sample offset:
```
target_sample = current_sample + (at_ms as f64 / 1000.0 * sample_rate as f64) as u64
```

Use `SchedulePoint::AtBeat` or `Immediate` depending on whether the action time is in the future or now.

### Safety

All dispatched commands go through `CommandValidator` which already clamps all parameters to safe ranges. Additional guard: reject action timelines with more than 64 actions (prevent LLM hallucination flooding).

## Implementation order

1. **Delay DSP** (`delay.rs`) — the foundation
2. **Wire delay into deck** — signal chain + commands
3. **Upgrade echo_drop + fx_echo_slam templates** — use real delay
4. **AI action execution** — conductor changes
5. **Frontend delay controls** — UI
6. **Update system prompt** — remove simulated tags, document delay
7. **Tests** — delay unit tests, template integration tests

## Files to modify

- `crates/autodj-core/src/delay.rs` (NEW)
- `crates/autodj-core/src/deck.rs` — add delay to signal chain
- `crates/autodj-core/src/command.rs` — SetDelay/ClearDelay commands
- `crates/autodj-core/src/engine.rs` — handle new commands, pass BPM to delay
- `crates/autodj-core/src/transition.rs` — upgrade echo_drop, fx_echo_slam
- `crates/autodj-app/src/conductor.rs` — AI action execution
- `crates/autodj-app/src/commands.rs` — Tauri commands for delay
- `crates/autodj-ai/src/planner.rs` — MixActionKind::SetDelay, system prompt
- `ui/src/components/ControlStrip.tsx` — delay UI controls
