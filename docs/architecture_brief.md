# Project Brief

## 1. Overview
- **Project Name**: AutoDJ
- **Purpose**: A high-intelligence, automated DJ mixing system designed for live performance and mix recording.
- **Architecture**: Tauri-based desktop app with a multi-crate Rust backend and a React/TypeScript frontend.
- **Runtime Pieces**: 
    - **Frontend**: UI, Library management, AI configuration.
    - **App Bridge**: Tauri commands, State management (`AppState`).
    - **Audio Engine**: Lock-free real-time engine (`cpal`) running a 44.1/48kHz callback.
    - **AI Conductor**: LLM (Gemini/Groq) + Heuristic Planner.
    - **DSP Pipeline**: Offline analysis of BPM, Key, Energy, and Sections.
- **Stack**: Rust, React, SQLite, FFmpeg (decoding), CPAL (audio).
- **Interaction**: Frontend polls state snapshots (30Hz) and sends commands via SPSC ring buffers.
- **Unique Hook**: "Phrase-aware" transition planning that combines LLM reasoning with strict DSP-derived timing guards.

## 2. Architecture Map
- `autodj-app`: Entry point, Tauri commands, `AppState` (High-level orchestration).
- `autodj-core`: Audio engine heart. `Deck` (playback), `Mixer` (summing), `TransitionExpander` (macro execution), `TransportClock` (beat positioning).
- `autodj-ai`: Intelligence layer. `AiPlanner` (Decision loop), `HeuristicScorer` (Fallback logic), `Classifier` (Gemini-powered tagging).
- `autodj-analysis`: DSP pipeline. `beats`, `bpm`, `key`, `energy`, `sections` (Phase-aware detection).
- `autodj-storage`: Data Persistence. `Database` (SQLite), `Cache` (PCM/Hashed files), `Decoder` (FFmpeg-to-PCM).

## 3. Runtime Flow
- **Startup**: `main.rs` -> `AppState::new` -> Audio Thread spawned -> UI initialized.
- **Import**: `import_files` -> SHA256 Hash -> FFmpeg decode to PCM cache -> DB Metadata entry.
- **Analysis**: `analyze_track` -> PCM buffer read -> DSP pipeline (BPM/Key/Energy) -> `track_profiles` DB table.
- **Planning**: `request_next_track` -> `AiPlanner::plan_next` -> [LLM Query | Heuristic Fallback] -> `AiDecision`.
- **Mixing**: `AiDecision` -> `ExecuteTransition` command -> `TransitionExpander` -> Sequential primitive commands (Gain, EQ, Play) scheduled via `TransportClock`.
- **Recording**: Stereo tap in audio callback -> Lock-free queue -> `WavWriter` thread.

## 4. Key Data Models
- `TrackRecord`: Core metadata (id, hash, artist, title, status).
- `TrackProfile`: Deep analysis results (BPM, Key, Energy curve, Sections, Phrases, Auto-Cues).
- `AiDecision`: The "brain's" output (Wait/Transition/Prepare, Target Track, Timeline Actions).
- `MixAction`: Timestamped low-level command (SetGain at 500ms, SetFilter at 1200ms).
- `EngineStateSnapshot`: 30Hz snapshot of decks (position, levels, EQ, filters, spectrum).

## 5. Important Logic Locations
- **Analysis Pipeline**: `autodj-analysis/src/pipeline.rs` (Orchestrates DSP modules).
- **Perceived Tempo**: `autodj-analysis/src/perceived_tempo.rs` (Half/double time logic).
- **Track Scoring**: `autodj-ai/src/scorer.rs` -> `score()` (Weights: Tempo, Key, Energy, Genre, Mood, Vibe).
- **Decision Loop**: `autodj-ai/src/planner.rs` -> `plan_next()` (The LLM vs Heuristic forking point).
- **Transition Execution**: `autodj-core/src/transition.rs` (Where `smooth_beatmix` becomes a sequence of Gain changes).
- **Audio Callback**: `autodj-core/src/engine.rs` -> `build_output_stream` closure (The real-time inner loop).
- **Safety Fallback**: `autodj-core/src/engine.rs` -> `silence_frame_counter` (Detects dead segments).

## 6. Current Decision Behavior
- **Selection Algorithm**: Uses `HeuristicScorer` with `DjPersonality` (Smooth, Hype, Lounge). Penalizes recently played artists and tracks (3-play replay cap).
- **Timing**: Phrase-aware. `planner.rs` targets `transition_windows` from analysis profiles, quantized to 8-bar boundaries.
- **Entry Points**: `select_entry_point` intelligently chooses `drop`, `intro`, or `first_beat` based on transition template and energy jump.
- **Transition Choice**: Scores ~10 templates (DropSwap, EchoDrop, BassKillSwap, etc.). Default is `smooth_beatmix` if scores are low.
- **LLM vs Heuristic**: LLM is preferred but `planner.rs` aggressively falls back to Heuristics if API fails or parse errors occur.

## 7. Current UI State
- **Functional**: Track browser (Search/Filter), Deck status (Waveform/Progress), Mixer (EQ/Filters/Crossfader), AI Decision logs, Recording.
- **Placeholder/Weak**: "Assisted" mode is mostly manual; "AutoDJ" mode is robust but hides complex FX from the user.
- **State Surfacing**: DB-driven library vs Real-time engine snapshots. Disconnect: Engine doesn't know "Artist", only "TrackID".

## 8. Current Architectural Weaknesses
- **Heuristic Monotony**: The heuristic logic heavily biases `smooth_beatmix`, leading to "safe" but predictable transitions.
- **Manual Sync Dependence**: The engine (`autodj-core`) doesn't natively "lock" beats; it relies on `AppState` to compute and send correct `SetSpeed` commands.
- **Vocal Blindness**: Vocal detection exists (`vocal_presence`), but it doesn't prevent overlapping two vocal hooks (Needs phrase-level vocal maps).
- **Stubbed Primitives**: Some turtablism actions (`Scratch`, `VinylBrake`) exist in the schema but are only partially implemented in the engine.
- **Contextual Lag**: The AI planner has limited "future set" awareness; it usually only plans one track ahead.

## 9. File Index for Fast Follow-up
- `crates/autodj-ai/src/planner.rs`: The brain's main loop.
- `crates/autodj-ai/src/scorer.rs`: How tracks are ranked.
- `crates/autodj-core/src/engine.rs`: The audio engine inner loop.
- `crates/autodj-core/src/transition.rs`: Macro expansion for mixing.
- `crates/autodj-analysis/src/pipeline.rs`: The analysis coordinator.
- `crates/autodj-analysis/src/sections.rs`: Song structure logic.
- `crates/autodj-app/src/commands.rs`: Tauri->Engine API.
- `crates/autodj-storage/src/db.rs`: SQLite schema.
- `ui/src/store.ts`: Frontend state & Tauri command wrappers.
- `crates/autodj-ai/src/llm.rs`: Gemini/Groq integration.

## 10. What another AI should inspect next
1.  `crates/autodj-ai/src/planner.rs`: Understand how `MixAction` timelines are built.
2.  `crates/autodj-core/src/transition.rs`: See how macro templates like `echo_drop` are hardcoded.
3.  `crates/autodj-analysis/src/sections.rs`: Evaluate how "Drops" and "Intros" are actually detected.
4.  `crates/autodj-core/src/engine.rs`: Look at the `execute_command` match arm for missing primitives.
5.  `crates/autodj-ai/src/scorer.rs`: Tune the weights for Key vs Tempo.
6.  `crates/autodj-storage/src/db.rs`: Look at `track_profiles` schema for missing DSP metrics.
7.  `crates/autodj-analysis/src/bpm.rs`: Check the accuracy of the autocorrelation/comb-filter logic.
8.  `ui/src/components/AiBar.tsx`: See how the AI's internal reasoning is presented to the user.
9.  `crates/autodj-ai/src/context.rs`: See what data is actually sent to Gemini (limiting factor for LLM intelligence).
10. `crates/autodj-core/src/deck.rs`: Understand the sample-accurate seeking and looping logic.

## 11. Reality Check
- **Is the app actually AI-driven?** 
  Heuristic-dominant. While it calls Gemini/Groq, the `fallback_heuristic` (`planner.rs:261`) handles ~90% of edge cases and is the primary source of transition timing.
- **Does it really use Gemini/Groq in the active loop?** 
  Attempts to, but often gated by `api_key` presence and parse-fail safety. Most "live" decisions are heuristic (`heuristic_only` tag).
- **Does it truly start tracks from intelligent markers?** 
  Yes. `select_entry_point` (`planner.rs:699`) is one of the project's strongest features, correctly skipping silence or targeting drops.
- **Does it actually perform DJ-style actions?** 
  No. It exposes primitives like `EchoOut` or `LoopRoll`, but the heuristic planner over-relies on simple Gain/Filter ramps (`smooth_beatmix`). 
- **Why does it feel like an automated crossfader?** 
  Lack of "Energy-Aware Timing". It transitions when a track reaches "outro" rather than when the energy peaks, and it rarely uses high-impact FX (Echo/Slam) in the default profile.
- **5 highest-impact fixes**:
  1. Phrase-level Vocal Mapping: Stop "Vocal Clash" by detecting hooks vs verses.
  2. Native Beat-Sync in Engine: Resolve the "Speed Drift" by locking phases in the audio thread.
  3. Template Expansion: Move transition definitions from hardcoded Rust to JSON templates.
  4. Real-time FX UI: Allow the user to trigger the AI's performance primitives manually.
  5. LLM Prompt Refinement: Current prompts are generic; they need more DSP "snapshot" data to make expert-level decisions.
