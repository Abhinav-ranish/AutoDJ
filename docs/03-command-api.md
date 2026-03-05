# Auto-DJ — Command API

## Overview

All interactions with the audio engine — from the UI, the AI conductor, or the heuristic
scorer — go through a single **command interface**. Commands are validated, clamped, tagged
with ownership, and placed into the SPSC ring buffer for the audio callback to consume.

---

## Command Envelope

Every command is wrapped in an envelope:

```json
{
  "id": "cmd_a1b2c3d4",
  "source": "USER | AI | SYSTEM",
  "timestamp_ms": 1709654400000,
  "schedule": { "type": "NextBar" },
  "ownership_claim": true,
  "command": { "type": "...", ... }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `id` | `string` | Unique command ID (UUID or short hash) |
| `source` | `enum` | Who issued the command |
| `timestamp_ms` | `u64` | Wall-clock time of command creation |
| `schedule` | `SchedulePoint` | When to execute (see below) |
| `ownership_claim` | `bool` | If true and source=USER, takes ownership |
| `command` | `Command` | The actual command payload |

---

## SchedulePoint (JSON)

```json
// Immediate
{ "type": "Immediate" }

// Next beat boundary
{ "type": "NextBeat" }

// Next bar (downbeat)
{ "type": "NextBar" }

// Specific beat
{ "type": "AtBeat", "beat": 128 }

// Relative
{ "type": "AfterBars", "bars": 4 }
```

---

## Command Types

### Deck commands

```json
// Load a track onto a deck
{ "type": "LoadTrack", "deck": "A", "track_id": "trk_abc123" }

// Play / Pause
{ "type": "Play", "deck": "A" }
{ "type": "Pause", "deck": "A" }

// Seek to position (in beats, relative to track start)
{ "type": "Seek", "deck": "A", "beat": 64 }

// Set cue point
{ "type": "SetCue", "deck": "A", "slot": 1, "beat": 32 }

// Jump to cue
{ "type": "JumpToCue", "deck": "A", "slot": 1 }

// Set loop
{ "type": "SetLoop", "deck": "A", "start_beat": 64, "length_bars": 4 }

// Clear loop
{ "type": "ClearLoop", "deck": "A" }
```

### Mixer commands

```json
// Crossfader position (-1.0 = full A, +1.0 = full B)
{ "type": "SetCrossfader", "position": 0.0 }

// Deck gain (dB, clamped: -inf to +12)
{ "type": "SetDeckGain", "deck": "A", "gain_db": -3.0 }

// EQ band gain (dB, clamped: -inf to +6)
{ "type": "SetEQ", "deck": "A", "band": "low", "gain_db": 0.0 }

// Filter cutoff (Hz, clamped: 20 to 20000)
{ "type": "SetFilter", "deck": "A", "cutoff_hz": 1000.0, "resonance": 1.0 }

// Master gain
{ "type": "SetMasterGain", "gain_db": 0.0 }
```

### Tempo commands

```json
// Set deck tempo (BPM)
{ "type": "SetTempo", "deck": "A", "bpm": 128.0 }

// Nudge tempo temporarily
{ "type": "NudgeTempo", "deck": "A", "delta_bpm": 0.5, "duration_beats": 4 }

// Sync deck to master clock
{ "type": "SyncToMaster", "deck": "A" }
```

### Transition commands (compound)

```json
{
  "type": "ExecuteTransition",
  "template": "smooth_beatmix",
  "params": {
    "duration_bars": 16,
    "incoming_deck": "B",
    "outgoing_deck": "A",
    "incoming_start_beat": 0,
    "bass_swap_bar": 8,
    "crossfader_curve": "equal_power"
  }
}
```

### System commands

```json
// Request next track (triggers AI/heuristic)
{ "type": "RequestNextTrack" }

// Set mode
{ "type": "SetMode", "mode": "AutoDJ" }

// Resume AI ownership
{ "type": "ResumeAI" }

// Force-release ownership
{ "type": "ReleaseOwnership", "parameter": "crossfader" }
```

---

## Validation Rules

Every command passes through a **CommandValidator** before entering the queue:

1. **Range clamping**: Numerical values are clamped to valid ranges.
   - `gain_db`: min `-96.0` (treated as -∞), max `+12.0`
   - `crossfader`: `-1.0` to `+1.0`
   - `bpm`: `60.0` to `200.0`
   - `cutoff_hz`: `20.0` to `20000.0`
   - `resonance`: `0.5` to `4.0`
   - `length_bars`: `1` to `64`
   - `duration_bars`: `1` to `64`
2. **Track existence**: `LoadTrack` requires the track to exist in storage and have
   a decoded PCM cache.
3. **Beat alignment**: `SetLoop.start_beat` must align to a beat boundary.
4. **Template validation**: `ExecuteTransition` template name must be registered.
5. **Mode guard**: In `Manual` mode, AI-sourced commands are rejected (except suggestions).
   In `Assisted` mode, AI commands require `user_approved: true` flag.

### Invalid command handling

- Clamped values → command executes with clamped value; original value logged.
- Invalid references → command rejected, error event emitted to UI.
- No crash, ever.

---

## Ownership Model

Each controllable parameter has an **owner**:

```rust
enum Owner {
    System,  // engine defaults
    AI,      // AI conductor is driving
    User,    // human has grabbed control
}

struct OwnedParameter {
    owner: Owner,
    /// When user took ownership (beat number), None if not user-owned
    user_since: Option<u64>,
    /// Duration of user ownership in bars (configurable, default 8)
    user_hold_bars: u32,
}
```

### Parameters with ownership tracking

| Parameter | Granularity |
|-----------|------------|
| `crossfader` | Global |
| `deck_gain` | Per deck |
| `eq_low` | Per deck |
| `eq_mid` | Per deck |
| `eq_high` | Per deck |
| `filter` | Per deck |
| `tempo` | Per deck |
| `transport` | Per deck |

### Ownership rules

1. **USER always wins**: A user command immediately takes ownership regardless of
   current owner.
2. **AI defers**: If a parameter is user-owned, AI commands targeting that parameter
   are silently dropped (logged, not errored).
3. **Timeout**: User ownership expires after `user_hold_bars` bars (configurable,
   default 8). Ownership reverts to `AI` if in AutoDJ mode, else `System`.
4. **Explicit release**: User can explicitly release ownership via `ResumeAI` or
   `ReleaseOwnership`.
5. **Mode change**: Switching to `Manual` reverts all ownership to `User`.
   Switching to `AutoDJ` reverts non-user-held parameters to `AI`.

### State diagram

```
         User touches param
SYSTEM ──────────────────────▶ USER
   ▲                              │
   │  mode=Manual                 │ timeout (N bars)
   │                              ▼
   │                            AI (if AutoDJ)
   │                              │
   │  AI unavailable              │ User touches again
   └──────────────────────────────┘
```

---

## Command Expansion

Compound commands (e.g., `ExecuteTransition`) are **expanded** into a sequence of
primitive commands by the `TransitionExpander`:

```
ExecuteTransition("smooth_beatmix", 16 bars) expands to:
  [bar  0] LoadTrack(B, "trk_xyz")
  [bar  0] Play(B)
  [bar  0] SetDeckGain(B, -inf)
  [bar  0] SetCrossfader(0.0)  // centered
  [bar  0→16] Interpolate(SetDeckGain(B), -inf → 0 dB, linear)
  [bar  8] SetEQ(A, low, -inf)   // bass swap
  [bar  8] SetEQ(B, low, 0 dB)
  [bar 16] SetCrossfader(1.0)    // fully on B
  [bar 16] Pause(A)
```

The expanded commands are individually scheduled and can be partially overridden
by the user (e.g., user grabs crossfader mid-transition → remaining crossfader
commands dropped, other commands continue).
