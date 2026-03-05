# Auto-DJ — Milestone A + B Walkthrough

## Milestone A: Foundations ✅
- 10 architecture docs under `docs/`
- 5 Rust crates in workspace
- React + Vite + Zustand UI scaffolded
- 18 tests passing (buffer, command, transport, cache, DB)

## Milestone B: Two Decks + Full Mixing ✅

### New DSP Modules

| File | What it does |
|------|-------------|
| [eq.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-core/src/eq.rs) | 3-band EQ: Linkwitz-Riley biquad crossovers at 250/2500 Hz, per-band gain -∞ to +6 dB |
| [filter.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-core/src/filter.rs) | SVF filter: auto LP (>1 kHz cutoff) / HP (≤1 kHz), bypass at 20 kHz, resonance 0.5–4.0 |

### Signal Chain (per frame)

```
PCM sample → 3-Band EQ → SVF Filter → Deck Gain → Crossfader → Master Gain → Limiter → OUT
```

### Test Results

**25 tests passing**, 0 failures

### How to Run

```bash
cd /Users/ragebaiter/Documents/GitHub/AutoDJ
cargo tauri dev
```

## Next: Milestone C — Sync + Quantization
