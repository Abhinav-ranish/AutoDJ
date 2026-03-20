# Transition & DJ Improvements Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace simulated echo effects with real tempo-synced delay DSP, and make the conductor execute AI-generated MixAction timelines instead of only using template expansion.

**Architecture:** Per-deck stereo delay with circular buffer inserted after the SVF filter in the deck signal chain. AI action execution bridges MixActionKind to engine Commands in the conductor's transitioning phase, skipping template expansion when actions are present.

**Tech Stack:** Rust (autodj-core, autodj-ai, autodj-app), TypeScript/React (ui)

---

### Task 1: Create the StereoDelay DSP module

**Files:**
- Create: `crates/autodj-core/src/delay.rs`
- Modify: `crates/autodj-core/src/lib.rs` — add `pub mod delay;`

**Step 1: Write delay unit tests**

Create `crates/autodj-core/src/delay.rs` with the test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bypass_when_wet_zero() {
        let mut delay = StereoDelay::new(48000.0);
        // With wet=0, output should equal input
        let (l, r) = delay.process(1.0, 1.0, 120.0);
        assert!((l - 1.0).abs() < 0.001);
        assert!((r - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_delay_produces_output_after_delay_time() {
        let mut delay = StereoDelay::new(48000.0);
        delay.set_params(1.0, 0.0, 1.0, false); // 1 beat, no feedback, full wet
        let bpm = 120.0; // 1 beat = 0.5s = 24000 frames

        // Feed an impulse
        delay.process(1.0, 1.0, bpm);
        // Fill with silence until just before delay time
        for _ in 1..24000 {
            delay.process(0.0, 0.0, bpm);
        }
        // At exactly delay_frames, the impulse should appear
        let (l, _r) = delay.process(0.0, 0.0, bpm);
        assert!(l.abs() > 0.5, "Expected delayed impulse, got {l}");
    }

    #[test]
    fn test_feedback_produces_multiple_taps() {
        let mut delay = StereoDelay::new(48000.0);
        delay.set_params(1.0, 0.5, 1.0, false); // feedback=0.5
        let bpm = 120.0;

        // Feed impulse
        delay.process(1.0, 1.0, bpm);
        // Silence for 2 full delay periods (48000 frames)
        for _ in 1..48000 {
            delay.process(0.0, 0.0, bpm);
        }
        // Second tap should be at ~0.5 amplitude (feedback=0.5)
        let (l, _) = delay.process(0.0, 0.0, bpm);
        assert!(l.abs() > 0.2, "Expected feedback tap, got {l}");
    }

    #[test]
    fn test_feedback_clamped_to_max() {
        let mut delay = StereoDelay::new(48000.0);
        delay.set_params(1.0, 0.99, 1.0, false); // try to exceed max
        assert!(delay.feedback <= 0.85);
    }

    #[test]
    fn test_clear_resets_buffer() {
        let mut delay = StereoDelay::new(48000.0);
        delay.set_params(1.0, 0.5, 1.0, false);
        delay.process(1.0, 1.0, 120.0);
        delay.clear();
        // After clear, no delayed output
        for _ in 0..48000 {
            let (l, r) = delay.process(0.0, 0.0, 120.0);
            assert!(l.abs() < 0.001 && r.abs() < 0.001);
        }
    }

    #[test]
    fn test_ping_pong_alternates_channels() {
        let mut delay = StereoDelay::new(48000.0);
        delay.set_params(1.0, 0.5, 1.0, true); // ping_pong=true
        let bpm = 120.0;

        // Feed impulse into both channels
        delay.process(1.0, 1.0, bpm);
        // First tap: should favor one channel
        for _ in 1..24000 {
            delay.process(0.0, 0.0, bpm);
        }
        let (l1, r1) = delay.process(0.0, 0.0, bpm);
        // One channel should be louder than the other
        assert!((l1.abs() - r1.abs()).abs() > 0.1,
            "Expected ping-pong panning, got L={l1} R={r1}");
    }
}
```

**Step 2: Write the StereoDelay implementation above the tests**

```rust
//! Tempo-synced stereo delay with feedback and ping-pong mode.
//!
//! Designed for the audio callback: zero allocation, no I/O.
//! Delay time is derived from BPM and a subdivision value (in beats).

/// Maximum delay time in seconds (2 seconds at any sample rate).
const MAX_DELAY_SECONDS: f32 = 2.0;

/// Maximum feedback to prevent runaway oscillation.
const MAX_FEEDBACK: f32 = 0.85;

/// Stereo delay with circular buffer.
pub struct StereoDelay {
    buffer_l: Vec<f32>,
    buffer_r: Vec<f32>,
    write_head: usize,
    buffer_size: usize,
    sample_rate: f32,

    /// Delay time in beats (0.25 = 1/16th, 1.0 = quarter note).
    pub subdivision: f32,
    /// Feedback amount [0.0, 0.85].
    pub feedback: f32,
    /// Dry/wet mix [0.0, 1.0].
    pub wet: f32,
    /// Ping-pong mode: alternates taps between L and R.
    pub ping_pong: bool,

    /// Whether the delay is active.
    pub active: bool,

    /// Internal tap counter for ping-pong panning.
    tap_counter: u32,
    /// Cached delay in frames (updated when BPM changes).
    cached_delay_frames: usize,
    cached_bpm: f64,
}

impl StereoDelay {
    /// Create a new delay, pre-allocating the circular buffer.
    pub fn new(sample_rate: f32) -> Self {
        let buffer_size = (sample_rate * MAX_DELAY_SECONDS) as usize;
        Self {
            buffer_l: vec![0.0; buffer_size],
            buffer_r: vec![0.0; buffer_size],
            write_head: 0,
            buffer_size,
            sample_rate,
            subdivision: 1.0,
            feedback: 0.0,
            wet: 0.0,
            ping_pong: false,
            active: false,
            tap_counter: 0,
            cached_delay_frames: 0,
            cached_bpm: 0.0,
        }
    }

    /// Set delay parameters. Activates the delay.
    pub fn set_params(&mut self, subdivision: f32, feedback: f32, wet: f32, ping_pong: bool) {
        self.subdivision = subdivision.clamp(0.0625, 4.0);
        self.feedback = feedback.clamp(0.0, MAX_FEEDBACK);
        self.wet = wet.clamp(0.0, 1.0);
        self.ping_pong = ping_pong;
        self.active = true;
        self.cached_bpm = 0.0; // force recalculation
    }

    /// Clear the delay buffer and deactivate.
    pub fn clear(&mut self) {
        self.buffer_l.fill(0.0);
        self.buffer_r.fill(0.0);
        self.active = false;
        self.wet = 0.0;
        self.feedback = 0.0;
        self.tap_counter = 0;
    }

    /// Process one stereo frame. BPM is passed per-frame for tempo tracking.
    pub fn process(&mut self, in_l: f32, in_r: f32, bpm: f64) -> (f32, f32) {
        if !self.active || self.wet <= 0.0 {
            // Still advance write head to keep buffer clean
            self.buffer_l[self.write_head] = in_l;
            self.buffer_r[self.write_head] = in_r;
            self.write_head = (self.write_head + 1) % self.buffer_size;
            return (in_l, in_r);
        }

        // Recalculate delay frames when BPM changes
        if (bpm - self.cached_bpm).abs() > 0.01 {
            self.cached_bpm = bpm;
            let beat_duration_s = 60.0 / bpm;
            let delay_s = beat_duration_s * self.subdivision as f64;
            self.cached_delay_frames = (delay_s * self.sample_rate as f64) as usize;
            self.cached_delay_frames = self.cached_delay_frames.min(self.buffer_size - 1).max(1);
        }

        let delay_frames = self.cached_delay_frames;

        // Read from delay buffer
        let read_pos = if self.write_head >= delay_frames {
            self.write_head - delay_frames
        } else {
            self.buffer_size - (delay_frames - self.write_head)
        };

        let delayed_l = self.buffer_l[read_pos];
        let delayed_r = self.buffer_r[read_pos];

        // Apply ping-pong panning
        let (tap_l, tap_r) = if self.ping_pong {
            // Detect taps by checking if delayed signal crosses a threshold
            // Simple approach: use tap_counter based on delay period
            let pan = if (self.write_head / delay_frames.max(1)) % 2 == 0 {
                (1.0, 0.3) // pan left
            } else {
                (0.3, 1.0) // pan right
            };
            (delayed_l * pan.0, delayed_r * pan.1)
        } else {
            (delayed_l, delayed_r)
        };

        // Mix: dry + wet
        let dry = 1.0 - self.wet;
        let out_l = in_l * dry + tap_l * self.wet;
        let out_r = in_r * dry + tap_r * self.wet;

        // Write input + feedback into buffer
        self.buffer_l[self.write_head] = in_l + tap_l * self.feedback;
        self.buffer_r[self.write_head] = in_r + tap_r * self.feedback;

        self.write_head = (self.write_head + 1) % self.buffer_size;

        (out_l, out_r)
    }
}
```

**Step 3: Add module to lib.rs**

In `crates/autodj-core/src/lib.rs`, add `pub mod delay;`.

**Step 4: Run tests**

Run: `cargo test -p autodj-core delay`
Expected: All 6 delay tests pass.

**Step 5: Commit**

```
git add crates/autodj-core/src/delay.rs crates/autodj-core/src/lib.rs
git commit -m "feat(core): add tempo-synced stereo delay DSP with ping-pong mode"
```

---

### Task 2: Wire delay into the deck signal chain

**Files:**
- Modify: `crates/autodj-core/src/deck.rs` — add delay field and process call
- Modify: `crates/autodj-core/src/command.rs` — add SetDelay/ClearDelay commands
- Modify: `crates/autodj-core/src/engine.rs` — handle new commands, pass BPM

**Step 1: Add delay field to Deck struct**

In `crates/autodj-core/src/deck.rs`:

Add import at top: `use crate::delay::StereoDelay;`

Add field to `Deck` struct (after `pub filter: SvfFilter`):
```rust
/// Tempo-synced delay effect.
pub delay: StereoDelay,
```

Initialize in `with_sample_rate()`: `delay: StereoDelay::new(sample_rate),`

In `load_track()`, add: `self.delay.clear();`

Add method:
```rust
/// Set delay parameters and activate.
pub fn set_delay(&mut self, subdivision: f32, feedback: f32, wet: f32, ping_pong: bool) {
    self.delay.set_params(subdivision, feedback, wet, ping_pong);
}

/// Clear the delay effect.
pub fn clear_delay(&mut self) {
    self.delay.clear();
}
```

**Step 2: Insert delay into signal chain in `next_frame()`**

In `deck.rs`, the signal chain at line ~241-244 currently reads:
```rust
let (l, r) = self.eq.process(l, r);
let (l, r) = self.filter.process(l, r);
(l * self.gain, r * self.gain)
```

Change to:
```rust
let (l, r) = self.eq.process(l, r);
let (l, r) = self.filter.process(l, r);
let (l, r) = self.delay.process(l, r, self.speed * 120.0); // BPM placeholder, real BPM set by engine
(l * self.gain, r * self.gain)
```

Note: We need to pass the actual BPM. Add a `pub bpm: f64` field to `Deck` (default 120.0), set by the engine from `transport.bpm()`. Change the delay process call to use `self.bpm`:
```rust
let (l, r) = self.delay.process(l, r, self.bpm);
```

**Step 3: Add commands to `command.rs`**

Add to the `Command` enum (after `Spinback`):
```rust
/// Tempo-synced delay effect.
SetDelay { deck: DeckId, subdivision: f32, feedback: f32, wet: f32, ping_pong: bool },
/// Clear delay effect.
ClearDelay { deck: DeckId },
```

Add validation in `CommandValidator::validate()`:
```rust
Command::SetDelay { deck, subdivision, feedback, wet, ping_pong } => {
    Some(Command::SetDelay {
        deck,
        subdivision: subdivision.clamp(0.0625, 4.0),
        feedback: feedback.clamp(0.0, 0.85),
        wet: wet.clamp(0.0, 1.0),
        ping_pong,
    })
}
Command::ClearDelay { deck } => Some(Command::ClearDelay { deck }),
```

**Step 4: Handle commands in `engine.rs`**

In `execute_command()` (around line 595), add before the catch-all:
```rust
Command::SetDelay { deck, subdivision, feedback, wet, ping_pong } => {
    Self::get_deck_mut(*deck, deck_a, deck_b).set_delay(*subdivision, *feedback, *wet, *ping_pong);
}
Command::ClearDelay { deck } => {
    Self::get_deck_mut(*deck, deck_a, deck_b).clear_delay();
}
```

Also in the audio callback, update each deck's BPM from the transport clock before rendering. In the per-frame loop (around line 383), before `let a = deck_a.next_frame();`, add:
```rust
deck_a.bpm = transport.bpm;
deck_b.bpm = transport.bpm;
```

**Step 5: Run tests**

Run: `cargo test -p autodj-core`
Expected: All existing tests pass + delay tests pass.

**Step 6: Commit**

```
git add crates/autodj-core/src/deck.rs crates/autodj-core/src/command.rs crates/autodj-core/src/engine.rs
git commit -m "feat(core): wire delay into deck signal chain with SetDelay/ClearDelay commands"
```

---

### Task 3: Upgrade echo_drop and fx_echo_slam templates

**Files:**
- Modify: `crates/autodj-core/src/transition.rs:293-382` — echo_drop
- Modify: `crates/autodj-core/src/transition.rs:1248-1380` — fx_echo_slam

**Step 1: Rewrite echo_drop template**

Replace the simulated gain ramp in `EchoDrop::expand()` with real delay commands:

```rust
impl TransitionTemplate for EchoDrop {
    fn expand(&self, params: &Value) -> Result<Vec<ScheduledCommand>, String> {
        let incoming_deck_str = params["incoming_deck"].as_str().unwrap_or("B");
        let incoming_deck = if incoming_deck_str == "A" { DeckId::A } else { DeckId::B };
        let outgoing_deck = if incoming_deck == DeckId::A { DeckId::B } else { DeckId::A };
        let silence_beats = params["silence_beats"].as_u64().unwrap_or(4) as u32;

        let mut commands = Vec::new();

        // Activate delay on outgoing deck (1/2 beat echo, moderate feedback)
        commands.push(ScheduledCommand {
            command: Command::SetDelay {
                deck: outgoing_deck,
                subdivision: 0.5,
                feedback: 0.6,
                wet: 0.8,
                ping_pong: false,
            },
            source: CommandSource::System,
            schedule: SchedulePoint::NextBar,
            target_sample: None,
        });

        // Fade out outgoing gain over 2 beats (the delay tail carries the echo)
        commands.push(ScheduledCommand {
            command: Command::SetDeckGain { deck: outgoing_deck, gain_db: -6.0 },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBeats(1),
            target_sample: None,
        });
        commands.push(ScheduledCommand {
            command: Command::SetDeckGain { deck: outgoing_deck, gain_db: -96.0 },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBeats(2),
            target_sample: None,
        });

        // Pause outgoing and clear delay
        commands.push(ScheduledCommand {
            command: Command::Pause { deck: outgoing_deck },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBeats(3),
            target_sample: None,
        });
        commands.push(ScheduledCommand {
            command: Command::ClearDelay { deck: outgoing_deck },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBeats(3),
            target_sample: None,
        });

        // Drop incoming after silence gap
        let drop_beat = silence_beats;
        let final_cf = if incoming_deck == DeckId::B { 1.0 } else { -1.0 };
        commands.push(ScheduledCommand {
            command: Command::SetCrossfader { position: final_cf },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBeats(drop_beat),
            target_sample: None,
        });
        commands.push(ScheduledCommand {
            command: Command::SetDeckGain { deck: incoming_deck, gain_db: 0.0 },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBeats(drop_beat),
            target_sample: None,
        });
        commands.push(ScheduledCommand {
            command: Command::Play { deck: incoming_deck },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBeats(drop_beat),
            target_sample: None,
        });

        // Reset outgoing gain for next use
        commands.push(ScheduledCommand {
            command: Command::SetDeckGain { deck: outgoing_deck, gain_db: 0.0 },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBeats(drop_beat + 1),
            target_sample: None,
        });

        Ok(commands)
    }
}
```

**Step 2: Rewrite fx_echo_slam template**

In `FxEchoSlam::expand()`, replace the simulated gain decay with real delay on the outgoing deck. Keep the filter slam on the incoming deck (that's already real DSP). Replace the exponential gain ramp loop with:

```rust
// Real delay tail on outgoing deck
commands.push(ScheduledCommand {
    command: Command::SetDelay {
        deck: outgoing_deck,
        subdivision: 0.75,
        feedback: 0.55,
        wet: 0.9,
        ping_pong: false,
    },
    source: CommandSource::System,
    schedule: SchedulePoint::NextBar,
    target_sample: None,
});

// Fade outgoing gain over echo_beats
for beat in 1..=echo_beats {
    let progress = beat as f32 / echo_beats as f32;
    let out_gain = -12.0 - 48.0 * progress * progress;
    commands.push(ScheduledCommand {
        command: Command::SetDeckGain { deck: outgoing_deck, gain_db: out_gain },
        source: CommandSource::System,
        schedule: SchedulePoint::AfterBeats(beat),
        target_sample: None,
    });
}

// Clear delay when pausing outgoing
commands.push(ScheduledCommand {
    command: Command::ClearDelay { deck: outgoing_deck },
    source: CommandSource::System,
    schedule: SchedulePoint::AfterBeats(echo_beats),
    target_sample: None,
});
```

**Step 3: Update the is_simulated_template function in conductor.rs**

In `crates/autodj-app/src/conductor.rs`, the function at ~line 939:
```rust
fn is_simulated_template(template: &str) -> bool {
    matches!(template, "echo_drop" | "backspin_cut" | "fx_echo_slam")
}
```
Change to (remove echo_drop and fx_echo_slam, they now use real DSP):
```rust
fn is_simulated_template(template: &str) -> bool {
    matches!(template, "backspin_cut")
}
```

**Step 4: Update system prompt in planner.rs**

In `crates/autodj-ai/src/planner.rs`, in the `DJ_SYSTEM_PROMPT` constant (~line 48-57), update the template descriptions:
- `echo_drop` line: Remove `[simulated]`, change description to: `echo_drop: 4-6 bars, tempo-synced delay tail → silence → hard cut (real delay DSP)`
- `fx_echo_slam` line: Remove `[simulated]`, change to: `fx_echo_slam: delay tail on outgoing → filter slam on incoming (real delay + filter DSP)`

**Step 5: Run tests**

Run: `cargo test`
Expected: All tests pass. The existing `test_echo_drop_emits_echo_out_fx` test in planner.rs may need updating if it checks for specific gain values.

**Step 6: Commit**

```
git add crates/autodj-core/src/transition.rs crates/autodj-app/src/conductor.rs crates/autodj-ai/src/planner.rs
git commit -m "feat(core): upgrade echo_drop and fx_echo_slam to use real delay DSP"
```

---

### Task 4: AI action execution in the conductor

**Files:**
- Modify: `crates/autodj-app/src/conductor.rs:852-920` — transitioning phase

**Step 1: Add a `mix_action_to_command` helper function**

Add a new function in `conductor.rs` (before the `is_simulated_template` function):

```rust
/// Convert an AI MixActionKind into an engine Command.
/// Returns None for unimplemented action types (ReverbTail, Scratch).
fn mix_action_to_command(
    kind: &MixActionKind,
    playing_deck: DeckId,
    idle_deck: DeckId,
) -> Option<Command> {
    // Helper to resolve "A"/"B"/"incoming"/"outgoing" deck strings
    let resolve_deck = |s: &str| -> DeckId {
        match s.to_uppercase().as_str() {
            "A" => DeckId::A,
            "B" => DeckId::B,
            "INCOMING" | "IDLE" => idle_deck,
            _ => playing_deck, // "OUTGOING", "PLAYING", or default
        }
    };

    match kind {
        MixActionKind::SetGain { deck, gain_db } => {
            Some(Command::SetDeckGain { deck: resolve_deck(deck), gain_db: *gain_db })
        }
        MixActionKind::SetFilter { deck, cutoff_hz, resonance } => {
            Some(Command::SetFilter { deck: resolve_deck(deck), cutoff_hz: *cutoff_hz, resonance: *resonance })
        }
        MixActionKind::SetEqBand { deck, band, gain_db } => {
            let eq_band = match band.to_lowercase().as_str() {
                "low" | "lo" => autodj_core::command::EqBand::Low,
                "mid" => autodj_core::command::EqBand::Mid,
                _ => autodj_core::command::EqBand::High,
            };
            Some(Command::SetEq { deck: resolve_deck(deck), band: eq_band, gain_db: *gain_db })
        }
        MixActionKind::SetEqKill { deck, band, enabled } => {
            let eq_band = match band.to_lowercase().as_str() {
                "low" | "lo" => autodj_core::command::EqBand::Low,
                "mid" => autodj_core::command::EqBand::Mid,
                _ => autodj_core::command::EqBand::High,
            };
            Some(Command::SetEqKill { deck: resolve_deck(deck), band: eq_band, enabled: *enabled })
        }
        MixActionKind::SetCrossfader { position } => {
            Some(Command::SetCrossfader { position: *position })
        }
        MixActionKind::Play { deck } => {
            Some(Command::Play { deck: resolve_deck(deck) })
        }
        MixActionKind::Pause { deck } => {
            Some(Command::Pause { deck: resolve_deck(deck) })
        }
        MixActionKind::BeatRepeat { deck, beats } => {
            Some(Command::BeatRepeat { deck: resolve_deck(deck), beats: *beats })
        }
        MixActionKind::BeatRepeatOff { deck } => {
            Some(Command::BeatRepeatOff { deck: resolve_deck(deck) })
        }
        MixActionKind::VinylBrake { deck, duration_ms } => {
            Some(Command::VinylBrake { deck: resolve_deck(deck), duration_ms: *duration_ms })
        }
        MixActionKind::Spinback { deck, duration_ms } => {
            Some(Command::Spinback { deck: resolve_deck(deck), duration_ms: *duration_ms })
        }
        MixActionKind::SetSlipMode { deck, enabled } => {
            Some(Command::SetSlipMode { deck: resolve_deck(deck), enabled: *enabled })
        }
        MixActionKind::EchoOut { deck, beats, feedback } => {
            Some(Command::SetDelay {
                deck: resolve_deck(deck),
                subdivision: *beats,
                feedback: *feedback,
                wet: 0.8,
                ping_pong: false,
            })
        }
        MixActionKind::FilterSlam { deck, direction } => {
            let cutoff = if direction == "up" { 20000.0 } else { 200.0 };
            Some(Command::SetFilter {
                deck: resolve_deck(deck),
                cutoff_hz: cutoff,
                resonance: 2.0,
            })
        }
        MixActionKind::SetLoop { deck, length_bars } => {
            Some(Command::SetLoop {
                deck: resolve_deck(deck),
                start_beat: 0, // current beat
                length_bars: *length_bars,
            })
        }
        MixActionKind::ClearLoop { deck } => {
            Some(Command::ClearLoop { deck: resolve_deck(deck) })
        }
        MixActionKind::ExecuteTemplate { template_name, params } => {
            Some(Command::ExecuteTransition {
                template: template_name.clone(),
                params: params.clone(),
            })
        }
        MixActionKind::SetTempo { deck, bpm } => {
            Some(Command::SetTempo { deck: resolve_deck(deck), bpm: *bpm })
        }
        MixActionKind::LoadTrack { deck, track_id } => {
            Some(Command::LoadTrack { deck: resolve_deck(deck), track_id: track_id.clone() })
        }
        // Unimplemented effects — skip silently
        MixActionKind::ReverbTail { .. } | MixActionKind::Scratch { .. } => None,
        MixActionKind::JumpToMarker { deck, marker, position_ms } => {
            Some(Command::JumpToMarker {
                deck: resolve_deck(deck),
                marker: marker.clone(),
                position_ms: *position_ms,
            })
        }
    }
}
```

**Step 2: Dispatch AI actions in the transitioning phase**

In the transitioning phase (around line 852-920), currently the code unconditionally runs `ExecuteTransition`. Change the logic to check for AI actions first.

Find the section that starts with:
```rust
// ── Transitioning phase ─────────────────────────────────────────
let (template_name, duration_bars, transition_ms) = {
```

Before the `ExecuteTransition` dispatch (around line 909-919), wrap it in a conditional:

```rust
// Check if AI provided a direct action timeline
let has_ai_actions = {
    let cond = app_state.conductor.read().await;
    cond.last_decision
        .as_ref()
        .and_then(|d| d.actions.as_ref())
        .map(|a| !a.is_empty())
        .unwrap_or(false)
};

if has_ai_actions {
    // Dispatch AI actions directly — skip template expansion
    let actions = {
        let cond = app_state.conductor.read().await;
        cond.last_decision
            .as_ref()
            .and_then(|d| d.actions.clone())
            .unwrap_or_default()
    };

    let max_actions = 64; // Safety: prevent LLM hallucination flooding
    for action in actions.iter().take(max_actions) {
        if let Some(command) = mix_action_to_command(&action.kind, playing_deck, idle_deck) {
            // Convert at_ms to a schedule point
            let schedule = if action.at_ms == 0 {
                SchedulePoint::Immediate
            } else {
                // Convert ms offset to beats
                let bpm = es.master_bpm.max(60.0);
                let beats_offset = (action.at_ms as f64 / 1000.0) * (bpm / 60.0);
                SchedulePoint::AfterBeats(beats_offset.round().max(1.0) as u32)
            };
            app_state.send_command(command, CommandSource::AI);
        }
    }

    log::info!(
        "[conductor] Dispatched {} AI actions (skipped template expansion)",
        actions.len().min(max_actions)
    );
} else {
    // Existing template expansion path
    let deck_str = if idle_deck == DeckId::A { "A" } else { "B" };
    app_state.send_command(
        Command::ExecuteTransition {
            template: template_name.clone(),
            params: serde_json::json!({
                "incoming_deck": deck_str,
                "duration_bars": duration_bars,
            }),
        },
        CommandSource::System,
    );
}
```

Note: This requires importing `MixActionKind` and `SchedulePoint` at the top of conductor.rs:
```rust
use autodj_ai::planner::MixActionKind;
use autodj_core::command::SchedulePoint;
```

But `send_command` on AppState doesn't accept a SchedulePoint — it sends Immediate. We need to use `send_scheduled_command` or handle scheduling differently. Check if AppState has a method for scheduled commands. If not, for now we can send all commands as Immediate (the at_ms timing is approximate anyway in the conductor's transition flow). The templates already handle timing via SchedulePoint internally.

Actually, looking at the code again, the `send_command` method sends with `Immediate` scheduling. For the AI action path, we should convert `at_ms` to actual delays using `tokio::time::sleep` between batches of actions at the same timestamp, since we're in an async context. This is simpler and more reliable than trying to schedule in the audio thread:

```rust
if has_ai_actions {
    let actions = { /* ... get actions ... */ };
    let max_actions = 64;
    let mut sorted_actions = actions.clone();
    sorted_actions.sort_by_key(|a| a.at_ms);

    let start = std::time::Instant::now();
    let mut last_ms = 0u64;

    for action in sorted_actions.iter().take(max_actions) {
        // Wait for the right time
        if action.at_ms > last_ms {
            let wait = std::time::Duration::from_millis(action.at_ms - last_ms);
            tokio::time::sleep(wait).await;
            last_ms = action.at_ms;
        }

        if let Some(command) = mix_action_to_command(&action.kind, playing_deck, idle_deck) {
            app_state.send_command(command, CommandSource::AI);
        }
    }
}
```

**Step 3: Add the `at_ms` field to MixAction if not present**

Check that `MixAction` has `at_ms: u64` — it should already exist from the planner definition. Verify in `planner.rs`:

```rust
pub struct MixAction {
    pub at_ms: u64,
    pub kind: MixActionKind,
}
```

**Step 4: Run tests**

Run: `cargo test`
Expected: All existing tests pass. The conductor tests don't test the transitioning phase directly (they test state management).

**Step 5: Commit**

```
git add crates/autodj-app/src/conductor.rs
git commit -m "feat(app): execute AI MixAction timelines directly in conductor"
```

---

### Task 5: Add SetDelay/ClearDelay to MixActionKind and update system prompt

**Files:**
- Modify: `crates/autodj-ai/src/planner.rs` — add MixActionKind variants, update system prompt

**Step 1: Add SetDelay and ClearDelay to MixActionKind**

In `crates/autodj-ai/src/planner.rs`, add to the `MixActionKind` enum (after `Scratch`):

```rust
/// Tempo-synced delay effect (real DSP).
SetDelay {
    deck: String,
    subdivision: f32,
    feedback: f32,
    wet: f32,
    ping_pong: bool,
},
/// Clear delay effect.
ClearDelay {
    deck: String,
},
```

**Step 2: Update the system prompt**

In `DJ_SYSTEM_PROMPT`, update the Available Engine Actions section to include:
```
- set_delay { deck, subdivision, feedback, wet, ping_pong } — tempo-synced delay (subdivision in beats)
- clear_delay { deck } — stop delay effect
```

Update the template descriptions:
- `echo_drop`: Remove `[simulated]`, update: `echo_drop: 4-6 bars, tempo-synced delay tail → silence → hard cut`
- `fx_echo_slam`: Remove `[simulated]`, update: `fx_echo_slam: delay tail + gain fade → filter slam in (real delay DSP)`

Under "App-layer approximations" section, remove `echo_out` and `filter_slam` from the approximation list (echo is now real, filter_slam uses real SVF filter). Keep `reverb_tail` and `scratch` as not implemented.

**Step 3: Run tests**

Run: `cargo test -p autodj-ai`
Expected: All AI tests pass.

**Step 4: Commit**

```
git add crates/autodj-ai/src/planner.rs
git commit -m "feat(ai): add SetDelay/ClearDelay to MixActionKind, update system prompt for real delay"
```

---

### Task 6: Add Tauri commands for delay control

**Files:**
- Modify: `crates/autodj-app/src/commands.rs` — add set_delay and clear_delay commands

**Step 1: Add Tauri commands**

Add near the other deck effect commands:

```rust
#[tauri::command]
pub fn set_delay(
    deck: String,
    subdivision: f32,
    feedback: f32,
    wet: f32,
    ping_pong: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let deck_id = match deck.to_uppercase().as_str() {
        "A" => DeckId::A,
        "B" => DeckId::B,
        _ => return Err("Invalid deck".to_string()),
    };
    state.send_command(
        Command::SetDelay { deck: deck_id, subdivision, feedback, wet, ping_pong },
        CommandSource::User,
    );
    Ok(())
}

#[tauri::command]
pub fn clear_delay(
    deck: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let deck_id = match deck.to_uppercase().as_str() {
        "A" => DeckId::A,
        "B" => DeckId::B,
        _ => return Err("Invalid deck".to_string()),
    };
    state.send_command(
        Command::ClearDelay { deck: deck_id },
        CommandSource::User,
    );
    Ok(())
}
```

**Step 2: Register commands in main.rs**

Find the `invoke_handler` macro call and add `set_delay` and `clear_delay` to the list.

**Step 3: Run build**

Run: `cargo build`
Expected: Build succeeds.

**Step 4: Commit**

```
git add crates/autodj-app/src/commands.rs crates/autodj-app/src/main.rs
git commit -m "feat(app): add Tauri commands for delay effect control"
```

---

### Task 7: Frontend delay controls

**Files:**
- Modify: `ui/src/components/ControlStrip.tsx` — add echo toggle and subdivision selector

**Step 1: Add delay UI controls**

In `ControlStrip.tsx`, add an echo section near the other performance controls (beat repeat, spinback, etc.). Add:

- An "ECHO" toggle button that calls `invoke('set_delay', { deck, subdivision: 0.5, feedback: 0.6, wet: 0.8, ping_pong: false })` / `invoke('clear_delay', { deck })`
- A subdivision selector with options: 1/4, 1/2, 3/4, 1 beat
- Style consistently with existing controls

Add state:
```typescript
const [echoActive, setEchoActive] = useState(false);
const [echoSubdivision, setEchoSubdivision] = useState(0.5);
```

Add handler:
```typescript
const toggleEcho = async () => {
    if (echoActive) {
        await invoke('clear_delay', { deck: deckId });
        setEchoActive(false);
    } else {
        await invoke('set_delay', {
            deck: deckId,
            subdivision: echoSubdivision,
            feedback: 0.6,
            wet: 0.8,
            pingPong: false,
        });
        setEchoActive(true);
    }
};
```

**Step 2: Run TypeScript check**

Run: `cd ui && npx tsc --noEmit`
Expected: No errors.

**Step 3: Commit**

```
git add ui/src/components/ControlStrip.tsx
git commit -m "feat(ui): add echo delay toggle and subdivision controls"
```

---

### Task 8: Final integration test

**Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests pass.

**Step 2: Run TypeScript check**

Run: `cd ui && npx tsc --noEmit`
Expected: No errors.

**Step 3: Run cargo clippy**

Run: `cargo clippy -- -D warnings 2>&1 | head -30`
Expected: No warnings.

**Step 4: Final commit if any fixes needed**

```
git commit -m "chore: fix clippy warnings and finalize delay + AI action execution"
```
