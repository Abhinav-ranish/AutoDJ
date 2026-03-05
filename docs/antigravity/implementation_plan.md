# Auto-DJ — Milestone A Implementation Plan

Build a minimal runnable system: import an audio file, decode it with FFmpeg, and play it through cpal.

## 15 Bullet-Point Summary

1. **Rust workspace** — 5 crates: `autodj-core`, `autodj-analysis`, `autodj-storage`, `autodj-ai`, `autodj-app` (Tauri)
2. **Tauri 2 shell** — React + Vite frontend in `ui/`, Rust backend in `crates/autodj-app/`
3. **SQLite schema** — `tracks`, `track_profiles`, `preferences`, sessions/history tables via `rusqlite`
4. **File import pipeline** — drag-drop or file picker → SHA-256 hash → copy to `~/.autodj/originals/`
5. **FFmpeg decode** — spawn ffmpeg, pipe stdout, write canonical PCM (48 kHz/stereo/f32) to `~/.autodj/cache/pcm/`
6. **PCM cache** — custom ADJP binary format with minimal header + raw f32 interleaved samples
7. **cpal audio output** — stable callback reading PCM from memory-mapped cache file
8. **SPSC command queue** — `ringbuf` crate for lock-free UI→engine command passing
9. **State snapshots** — engine pushes read-only state to main thread at 30 Hz via second ring buffer
10. **Deck model** — single deck initially (play/pause, position tracking, gain)
11. **Basic React UI** — library list, import button, single deck with play/pause and waveform placeholder
12. **Command API foundation** — `LoadTrack`, `Play`, `Pause`, `SetDeckGain` commands with validation
13. **Error handling** — `thiserror` for library crates, `anyhow` in the app binary, fallback ladder
14. **No AI yet** — AI integration deferred to Milestone F; scaffolding only
15. **Tests** — unit tests for PCM cache read/write, command validation, and SPSC queue

---

## Proposed Changes

### Workspace Root

#### [NEW] [Cargo.toml](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/Cargo.toml)
Rust workspace manifest listing all 5 crates.

---

### autodj-core (Audio Engine)

#### [NEW] [lib.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-core/src/lib.rs)
Re-exports engine, deck, mixer, command, transport modules.

#### [NEW] [engine.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-core/src/engine.rs)
`AudioEngine` struct: owns cpal stream, SPSC queues, deck, mixer. Processes commands in the audio callback.

#### [NEW] [deck.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-core/src/deck.rs)
`Deck` struct with play/pause/seek, position tracking, gain. Reads from `TrackBuffer`.

#### [NEW] [mixer.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-core/src/mixer.rs)
Simple pass-through mixer (single deck for now). Master gain + basic limiter.

#### [NEW] [command.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-core/src/command.rs)
Command enum, SchedulePoint, CommandValidator, ownership model types.

#### [NEW] [transport.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-core/src/transport.rs)
TransportClock struct (BPM, beat/bar counting). Stub for Milestone C.

#### [NEW] [buffer.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-core/src/buffer.rs)
`TrackBuffer` — wraps memory-mapped PCM data with sample-accurate reading.

#### [NEW] [Cargo.toml](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-core/Cargo.toml)
Dependencies: `cpal`, `ringbuf`, `memmap2`, `thiserror`.

---

### autodj-storage (SQLite + File Cache)

#### [NEW] [lib.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-storage/src/lib.rs)
Re-exports db, cache, decoder modules.

#### [NEW] [db.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-storage/src/db.rs)
`Database` struct wrapping `rusqlite::Connection`. Schema init, track CRUD, pref get/set.

#### [NEW] [cache.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-storage/src/cache.rs)
PCM cache management (ADJP format read/write), file hashing (SHA-256).

#### [NEW] [decoder.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-storage/src/decoder.rs)
FFmpeg spawner: `decode_to_pcm(input_path) → Result<PathBuf>`. Pipes ffmpeg stdout to ADJP file.

#### [NEW] [Cargo.toml](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-storage/Cargo.toml)
Dependencies: `rusqlite` (bundled), `sha2`, `thiserror`, `serde`, `serde_json`.

---

### autodj-analysis (Stub)

#### [NEW] [lib.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-analysis/src/lib.rs)
Stub module — re-exports placeholder types. Actual analysis in Milestone D.

---

### autodj-ai (Stub)

#### [NEW] [lib.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-ai/src/lib.rs)
Stub module — placeholder types. AI integration in Milestone F.

---

### autodj-app (Tauri Application)

#### [NEW] [main.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-app/src/main.rs)
Tauri entry point. Initializes DB, engine, and registers commands.

#### [NEW] [commands.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-app/src/commands.rs)
Tauri command handlers: `import_files`, `get_tracks`, `play`, `pause`, `load_track`, `set_deck_gain`.

#### [NEW] [state.rs](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-app/src/state.rs)
`AppState` struct holding `Database`, `AudioEngine`, shared state for Tauri.

#### [NEW] [Cargo.toml](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-app/Cargo.toml)
Dependencies: `tauri`, `autodj-core`, `autodj-storage`, `serde`, `serde_json`.

#### [NEW] [tauri.conf.json](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/crates/autodj-app/tauri.conf.json)
Tauri configuration: window size, title, security settings, build config.

---

### React UI

#### [NEW] [package.json](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/ui/package.json)
React + Vite + TypeScript + Zustand + @tauri-apps/api.

#### [NEW] [index.html](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/ui/index.html)
Entry HTML with root div.

#### [NEW] [App.tsx](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/ui/src/App.tsx)
Main app layout: Library view + single Deck view.

#### [NEW] [store.ts](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/ui/src/store.ts)
Zustand store with DJState interface, Tauri event listeners.

#### [NEW] [index.css](file:///Users/ragebaiter/Documents/GitHub/AutoDJ/ui/src/index.css)
Dark theme design system with CSS variables.

---

## Verification Plan

### Automated Tests

Run all Rust tests:
```bash
cd /Users/ragebaiter/Documents/GitHub/AutoDJ && cargo test --workspace
```

Tests to write:
1. `autodj-storage::cache` — write ADJP file, read back, verify header + samples match
2. `autodj-storage::decoder` — decode a bundled test WAV, verify PCM output
3. `autodj-core::command` — validate clamping behaviour for out-of-range values
4. `autodj-core::command` — verify invalid commands are rejected

### Manual Verification

1. Run the app: `cd /Users/ragebaiter/Documents/GitHub/AutoDJ && cargo tauri dev`
2. Import an MP3 using the import button
3. Verify the track appears in the library list
4. Click on the track → load to deck
5. Click play → audio should be audible through the default audio device
6. Click pause → audio stops

> [!IMPORTANT]
> FFmpeg must be installed on the system for the decode pipeline to work. On macOS: `brew install ffmpeg`.
