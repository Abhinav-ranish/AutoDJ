# AutoDJ Reality Audit

## 1. What is definitely implemented

### Audio Engine
- **Core Playback**: Sample-accurate stereo playback with linear interpolation and lock-free ring buffer command architecture.
- **DSP Primitives**: 3-band EQ (Classic & Isolator modes), SVF Filter (HP/LP sweep), and a brick-wall limiter with look-ahead envelope detection (`mixer.rs:84-104`).
- **Performance FX**: `SlipMode`, `BeatRepeat` (stutter), `VinylBrake`, and `Spinback` are fully implemented in the engine inner loop (`deck.rs:177-212`).

### Analysis Pipeline
- **Sophisticated DSP**: `pipeline.rs` runs parallelized analysis of BPM, Key (Camelot), LUFS Loudness, and multi-dimensional Spectral data.
- **Structural Detection**: Intelligent detection of Intros, Outros, "Impact Moments," and "Vocal-Free Zones" (`pipeline.rs:163-233`).
- **Phasing**: True phrase grid generation (16/32-beat boundaries) used for quantization.

### AI/LLM
- **Provider Support**: Complete integration for Gemini-2.5-flash and Groq (Llama-3.3) with JSON response formatting (`llm.rs`).
- **Context Engine**: Rich `AiContext` builder that sends engine snapshots, available track profiles, and even "vocal risk levels" to the LLM (`context.rs:162-210`).

### Library & DJ UI
- **Library Management**: SQLite persistence with track metadata, playlists, and session recording history.
- **Real-time UI**: High-frequency (30Hz) telemetry including spectrum data, progress bars, and AI decision reasoning logs.
- **AutoDJ Controller**: An autonomous state machine in `AiBar.tsx` that manages the planning -> loading -> aligning -> transitioning cycle.

## 2. What is partially implemented

- **FX Templates**: `EchoDrop` and `BackspinCut` are *simulated* using gain/filter ramps because dedicated Echo/Reverb FX units are missing from the engine signal chain (`transition.rs:239-240`).
- **AutoDJ Autonomy**: The "Conductor" logic lives in the React UI (`AiBar.tsx`). If the UI is hidden or the browser tab throttles, the "AutoDJ" behavior can lag or fail to trigger.
- **Performance Primitives Usage**: While `VinylBrake` and `Spinback` exist in the engine, they are rarely used by the `AiPlanner` heuristic, which favors volume-based transitions.
- **Manual Sync**: The engine doesn't have a "Hard Sync" lock. It relies on the UI/App layer calculating correct `SetSpeed` multipliers based on BPM differences.

## 3. What is still missing

- **Dedicated FX Units**: Echo, Reverb, Flanger, and Phaser are only "simulated" via gain/filter modulation.
- **Advanced Turntablism**: `Scratch` and `Nudge` actions are stubbed in the command schema but lack physical implementation in `deck.rs`.
- **Global Clock**: The "Master Clock" is a floating value in `AppState`, not a sample-accurate global sync source in the audio thread.
- **Vocal Separation**: The system *detects* vocals but cannot *separate* them (Phase-aware mixing only).

## 4. What was fixed recently

- **Template Selection Fairness**: Heavy bias towards `smooth_beatmix` was reduced (base score 0.50 -> 0.35) to favor more creative transitions like `filter_sweep` or `loop_roll` (`planner.rs:586`).
- **Phrase Alignment Logic**: The AutoDJ loop in `AiBar.tsx` now properly awaits the next phrase boundary before firing transitions (`AiBar.tsx:219-239`).
- **Enhanced Auto-Cueing**: Addition of `best_mix_in` and `best_mix_out` markers derived from energy stability and vocal-free zone overlaps.
- **Zustand Store Migration**: Preferences (like AI config) migrated from `localStorage` to the SQLite `preferences` table for persistence.
- **Vocal Risk Awareness**: `context.rs` now explicitly flags "high vocal risk" to prevent clashing vocal-heavy tracks.

## 5. What still feels misleading

- **"AI-Driven"**: The app is heavily **Heuristic-Driven**. The LLM is a "Consultant" that is frequently bypassed by the `fallback_heuristic` which contains the actual expert-system logic for transition points and template selection.
- **"Echo Drop"**: In the UI, this looks like an FX transition, but in code, it's just a sequence of `SetDeckGain` commands.
- **"Phrase Aware"**: While it *detects* phrases, the engine doesn't "know" it's in a phrase; it's the UI loop that quantizes its commands.

## 6. Overall reality check

- **Heuristic vs AI-Driven?** 90% Heuristic. The LLM handles high-level "vibe" selection, but the Rust heuristic handles 100% of the timing and DSP-tight logic.
- **Intelligent Marker-Aware Mixing?** **YES.** This is the strongest "real" feature. It uses `transition_windows` and `auto_cues` effectively to skip silence and target drops.
- **DJ Actions vs Blend/Fade?** Mostly sophisticated blends. "Performance actions" like `loop_roll` are just becoming real.
- **UI Quality?** **Polished Prototype.** It looks "Premium" (Inter font, sleek gradients), but the underlying orchestration is tightly coupled to the DOM lifecycle.
- **5 Biggest Fixes**:
  1. Phrase-aligned transition triggering.
  2. Integration of "Vocal-Free" zone detection into planners.
  3. Replay caps and anti-repetition scoring.
  4. Multi-provider AI (Gemini/Groq) support.
  5. SQLite-backed preference persistence.
- **5 Biggest Gaps**:
  1. No true Echo/Reverb FX in audio thread.
  2. Orchestration logic stuck in React (`AiBar.tsx`).
  3. No sample-accurate tempo sync lock.
  4. Limited turntablism (scratch/nudge).
  5. No vocal stem separation support yet.

## 7. Fast summary table

| Area | Status | Confidence | Notes |
| :--- | :--- | :--- | :--- |
| **Audio Engine** | Implemented | 100% | High-quality PCM engine; missing native FX. |
| **DSP Analysis** | Implemented | 95% | Extremely rich; foundation of the "marker" intelligence. |
| **AI Planner** | Partial | 90% | Heuristics do the heavy lifting; LLM is advisory. |
| **AutoDJ Loop** | Partial | 85% | Orchestration is in the UI, not the backend service. |
| **Transitions** | Implemented | 95% | 9+ templates exist; timing is phrase-accurate. |
| **UI Design** | Implemented | 95% | Premium look, "AiBar" provides deep reasoning transparency. |

## 8. If I had to brief another AI in 10 bullets

1. The app is a Tauri shell; the "brain" is in `autodj-ai`, the "body" is in `autodj-core`.
2. The code is much more "Heuristic Expert System" than "Generative AI."
3. Transition timing is driven by `transition_windows` generated in `autodj-analysis/sections.rs`.
4. The audio thread uses SPSC ring buffers; look for command processing in `engine.rs`.
5. The "Conductor" (AutoDJ autonomous loop) is in `ui/src/components/AiBar.tsx` (React).
6. Transitions are sequences of primitive commands (`SetGain`, `SetFilter`) scheduled via a `TransportClock`.
7. `planner.rs` is where the "Expert DJ" logic lives—thousands of lines of heuristic scoring.
8. Silence detection and emergency transitions are real and reactive.
9. "Echo" and "Reverb" are fake (volume/filter ramps).
10. The system is "Vocal-Aware" (prefers not to overlap vocals) but not "Vocal-Capable" (cannot mute stems).
