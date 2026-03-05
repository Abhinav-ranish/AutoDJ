# Auto-DJ — Audio Engine

## Real-Time Thread Rules

The audio callback (invoked by cpal at ~5 ms intervals for a 256-sample buffer at 48 kHz)
is the highest-priority code path. It **must**:

1. **Never block** — no mutexes, no I/O, no `println!`, no allocation.
2. **Never panic** — all code paths must be infallible; use saturating/clamping arithmetic.
3. **Be deterministic** — same inputs → same outputs, independent of wall-clock time.
4. **Complete within budget** — < 3 ms for a 256-sample buffer (leaves 2 ms headroom).

### Allowed in the callback

- Read from the SPSC command ring buffer (non-blocking `try_pop`).
- Read from pre-loaded PCM sample buffers (already in memory).
- Write to the SPSC state snapshot ring buffer (non-blocking `try_push`).
- Arithmetic on `f32` sample data.
- Advance internal counters/clocks.

### Forbidden in the callback

- `Mutex::lock`, `RwLock`, `Condvar`, channels that may block.
- `Box::new`, `Vec::push` (unless pre-allocated and bounded).
- File I/O, network I/O.
- Logging (use a lock-free logger if absolutely needed for debugging).
- Spawning threads/tasks.

---

## Mixer Graph

```
Deck A ──▶ [EQ A] ──▶ [Filter A] ──▶ [Gain A] ──┐
                                                    ├──▶ [Crossfader] ──▶ [Master Gain] ──▶ [Limiter] ──▶ OUT
Deck B ──▶ [EQ B] ──▶ [Filter B] ──▶ [Gain B] ──┘
```

### Signal chain (per sample frame, stereo)

1. **Deck**: Read next stereo sample from the track buffer. Apply time-stretch if
   tempo ≠ track's native BPM (via pre-computed rubato output or real-time resampler).
2. **3-band EQ**: Low (< 250 Hz), Mid (250–2500 Hz), High (> 2500 Hz).
   Each band: gain -∞ to +6 dB. Implementation: 2nd-order Linkwitz-Riley crossover filters.
3. **HP/LP Filter**: Resonant, sweepable. Cutoff 20 Hz – 20 kHz, resonance 0.5–4.0.
   Implementation: 2nd-order biquad (SVF topology for stability).
4. **Deck Gain**: -∞ to +12 dB, linear taper.
5. **Crossfader**: Position -1.0 (full A) to +1.0 (full B).
   Curve options: linear, equal-power (default), hard-cut.
   Equal-power: `gain_a = cos(θ)`, `gain_b = sin(θ)` where `θ = (pos+1)/2 * π/2`.
6. **Master Gain**: -∞ to +6 dB.
7. **Limiter**: Brick-wall at -0.3 dBFS. Attack 0.1 ms, release 50 ms.
   Implementation: look-ahead limiter (delay the signal by `attack_samples` and
   compute gain envelope).

---

## Deck Model

```rust
struct Deck {
    /// Currently loaded track (PCM buffer handle)
    track: Option<TrackBuffer>,

    /// Playback position in samples (f64 for sub-sample accuracy)
    position: f64,

    /// Playback speed multiplier (1.0 = native tempo)
    speed: f64,

    /// Transport state
    state: TransportState,  // Playing | Paused | Cueing

    /// Current EQ gains [low, mid, high] in linear amplitude
    eq: [f32; 3],

    /// Filter state
    filter: FilterState,

    /// Deck gain (linear)
    gain: f32,

    /// Cue points (sample positions)
    cue_points: [Option<u64>; 8],

    /// Active loop (start, end) in samples, None if no loop
    active_loop: Option<(u64, u64)>,

    /// Beat grid: sample positions of each beat
    beat_grid: Vec<u64>,   // pre-loaded from analysis, not allocated at runtime

    /// Current beat index (for quantization)
    current_beat: usize,
}
```

### TransportState

```
         ┌────────────┐
         │   Empty     │  (no track loaded)
         └─────┬──────┘
               │ load_track
         ┌─────▼──────┐
    ┌───▶│   Paused    │◀──────┐
    │    └─────┬──────┘       │
    │          │ play          │ pause / reach end
    │    ┌─────▼──────┐       │
    │    │  Playing    │───────┘
    │    └─────┬──────┘
    │          │ cue (while playing)
    │    ┌─────▼──────┐
    └────│  Cueing     │  (jumped to cue, paused)
         └────────────┘
```

---

## Transport Clock

The engine maintains a **master transport clock** that all quantised scheduling is
relative to.

```rust
struct TransportClock {
    /// Current sample position (monotonically increasing)
    sample_pos: u64,

    /// Master BPM (set by the currently-dominant deck or user)
    bpm: f64,

    /// Samples per beat = sample_rate * 60.0 / bpm
    samples_per_beat: f64,

    /// Current beat number (since transport start)
    beat_number: u64,

    /// Current bar number (assuming 4/4)
    bar_number: u64,

    /// Position within the current beat (0.0 – 1.0)
    beat_phase: f64,

    /// Time signature (default 4/4)
    beats_per_bar: u32,
}
```

### Clock advancement (called every buffer)

```
for each sample in buffer:
    sample_pos += 1
    beat_phase = (sample_pos % samples_per_beat) / samples_per_beat
    if beat_phase wrapped around:
        beat_number += 1
        if beat_number % beats_per_bar == 0:
            bar_number += 1
```

---

## Scheduling Model

Commands carry a `SchedulePoint`:

```rust
enum SchedulePoint {
    Immediate,           // execute on next buffer
    NextBeat,            // quantise to next beat boundary
    NextBar,             // quantise to next bar (downbeat)
    NextDownbeat,        // alias for NextBar in 4/4
    AtBeat(u64),         // specific beat number
    AtBar(u64),          // specific bar number
    AfterBeats(u32),     // relative: N beats from now
    AfterBars(u32),      // relative: N bars from now
}
```

The audio callback maintains a **ScheduledCommand** min-heap sorted by target sample
position. Each buffer iteration, it pops all commands whose target ≤ current sample_pos
and executes them.

### Quantisation resolution

- All loop points snap to the nearest beat boundary.
- Transition start/end snap to bar boundaries (downbeats).
- EQ/filter changes can be immediate (for manual use) or scheduled (for AI plans).

---

## Pre-allocated Buffers

To avoid runtime allocation:

- **Mix buffer**: `[f32; BUFFER_SIZE * 2]` — stereo interleaved, stack-allocated.
- **Deck read buffers**: `[f32; BUFFER_SIZE * 2]` per deck — pre-allocated.
- **Command heap**: Fixed-capacity `BinaryHeap<ScheduledCommand>` with max 256 entries.
  If full, oldest non-essential commands are dropped.

---

## Time-Stretching Strategy

For the alpha, we use a simple approach:

1. **Pre-compute**: When a track is loaded and its target tempo differs from native BPM,
   pre-stretch the entire track using `rubato` (offline, in the decoder thread).
   Store the stretched version in the PCM cache.
2. **Runtime fallback**: If tempo changes dynamically (e.g., user nudges), apply
   real-time resampling with `rubato`'s `SincFixedIn` resampler. This is allocation-free
   once the resampler is initialised.

Trade-off: pre-stretching uses more disk/RAM but guarantees zero runtime cost.
We accept this for alpha.

---

## FX (Optional, post-MVP)

Two simple send effects, mixed via a wet/dry knob:

1. **Echo-out**: Feedback delay (tempo-synced, e.g., 3/4 beat).
   Implementation: circular delay buffer, feedback coefficient.
2. **Reverb throw**: Simple Freeverb-style algorithm.
   Implementation: allpass + comb filter network.

These are applied *after* the mixer and *before* the limiter.
