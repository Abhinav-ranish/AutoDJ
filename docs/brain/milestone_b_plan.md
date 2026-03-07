# Milestone B — Two Decks + Full Mixing

## Goal

Wire up per-deck EQ (3-band), HP/LP filter, and deck gain controls in both the Rust audio engine and the React UI. Handle all remaining `SetEq`, `SetFilter`, and `SetDeckGain` commands in the audio callback. Add UI knobs and sliders for every mixer parameter.

## What Already Works (from Milestone A)

- Dual decks (A/B) with play/pause/load
- Crossfader with equal-power curve
- Brick-wall limiter on master
- Deck gain in the engine (but no UI knob for it)
- `SetEq` / `SetFilter` / `SetDeckGain` command enum variants exist + validation/clamping
- TrackLoader ring buffer for loading PCM into decks

## What's Missing

| Area | Gap |
|------|-----|
| **DSP** | No EQ or filter structs — deck just outputs raw PCM × gain |
| **Engine** | `SetEq` / `SetFilter` commands logged as "not implemented" in callback |
| **Mixer** | `mix()` receives post-gain frames but no EQ/filter processing |
| **Tauri** | No `set_eq` or `set_filter` command handlers |
| **UI** | No gain knobs, no EQ knobs, no filter knobs on either deck |
| **State** | `DeckSnapshot` doesn't report EQ/filter/gain values |
| **TrackLoader** | Not wired into audio callback — loads never reach decks at runtime |

---

## Proposed Changes

### autodj-core

#### [NEW] `crates/autodj-core/src/eq.rs`
3-band parametric EQ using 2nd-order biquad filters (Linkwitz-Riley crossovers):
- `ThreeBandEq` struct holding 6 biquad filter states (LP + HP per crossover × stereo)
- `set_gain(band, linear_gain)` — sets per-band gain
- `process(l, r) -> (f32, f32)` — processes one stereo frame (zero-allocation)
- Crossover frequencies: 250 Hz (low/mid), 2500 Hz (mid/high)

#### [NEW] `crates/autodj-core/src/filter.rs`
State Variable Filter (SVF) for HP/LP sweep:
- `SvfFilter` struct with cutoff, resonance, and mode (lowpass/highpass/bypass)
- `set_params(cutoff_hz, resonance)` — update coefficients
- `process(l, r) -> (f32, f32)` — processes one stereo frame
- Auto-selects LP when cutoff < 20000 Hz, HP when cutoff approaches 20 Hz, bypass at center

#### [MODIFY] `crates/autodj-core/src/deck.rs`
- Add `eq: ThreeBandEq` and `filter: SvfFilter` fields
- `next_frame()` signal chain: read PCM → EQ → filter → gain → output
- Add `set_eq(band, gain_db)` and `set_filter(cutoff, resonance)` methods
- Expose EQ/filter state for snapshot reporting

#### [MODIFY] `crates/autodj-core/src/engine.rs`
- Handle `SetEq` and `SetFilter` commands in `execute_command()`
- Wire `TrackLoadReceiver` into the audio callback (check for pending loads each buffer)
- Add EQ/filter values to `DeckSnapshot`

#### [MODIFY] `crates/autodj-core/src/lib.rs`
- Add `pub mod eq;` and `pub mod filter;`

---

### autodj-app

#### [MODIFY] `crates/autodj-app/src/commands.rs`
- Add `set_eq(deck, band, gain_db)` Tauri command
- Add `set_filter(deck, cutoff_hz, resonance)` Tauri command

#### [MODIFY] `crates/autodj-app/src/lib.rs`
- Register new `set_eq` and `set_filter` commands

---

### React UI

#### [MODIFY] `ui/src/store.ts`
- Add `setEq(deck, band, gainDb)` and `setFilter(deck, cutoffHz, resonance)` actions
- Update `DeckSnapshot` interface with `eq_low`, `eq_mid`, `eq_high`, `filter_cutoff`, `filter_resonance`

#### [MODIFY] `ui/src/App.tsx`
- Add per-deck vertical gain slider
- Add per-deck EQ knobs (Low / Mid / High) with kill buttons
- Add per-deck filter knob (bipolar: left = HP, center = bypass, right = LP)
- Visual feedback: current EQ/filter values displayed numerically

---

## Verification Plan

### Automated Tests
```bash
cargo test --workspace
```

New tests:
1. `eq::tests` — verify 3-band EQ gain at DC and Nyquist
2. `filter::tests` — verify SVF filter passes signal in bypass, attenuates at cutoff
3. `deck::tests` — verify signal chain (PCM → EQ → filter → gain)

### Manual Verification
1. `cargo tauri dev` from project root
2. Import an audio file → load on Deck A → play
3. Sweep EQ low knob to -∞ → bass should disappear
4. Sweep filter knob → hear HP/LP sweep
5. Adjust deck gain slider → volume changes
6. Load second track on Deck B → crossfade between them
