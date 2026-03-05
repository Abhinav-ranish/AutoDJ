# Auto-DJ — Architecture

## Module Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                        Tauri Shell                              │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                    React UI (WebView)                     │   │
│  │  Decks · Mixer · Library · Waveforms · AI Panel           │   │
│  └──────────────────┬───────────────────────────────────────┘   │
│                     │  Tauri commands / events                   │
│  ┌──────────────────▼───────────────────────────────────────┐   │
│  │                 Rust Backend (tauri::State)                │   │
│  │                                                           │   │
│  │  ┌─────────────┐  ┌──────────────┐  ┌────────────────┐   │   │
│  │  │ Command      │  │ AI Conductor │  │ Analysis       │   │   │
│  │  │ Validator    │  │ (planner)    │  │ Worker Pool    │   │   │
│  │  └──────┬──────┘  └──────┬───────┘  └───────┬────────┘   │   │
│  │         │                │                   │            │   │
│  │         ▼                ▼                   │            │   │
│  │  ┌──────────────────────────────────┐        │            │   │
│  │  │       Command Queue (SPSC)       │        │            │   │
│  │  └──────────────┬───────────────────┘        │            │   │
│  │                 │                             │            │   │
│  │  ┌──────────────▼───────────────────┐        │            │   │
│  │  │        Audio Engine              │        │            │   │
│  │  │  ┌────────┐  ┌────────┐          │        │            │   │
│  │  │  │ Deck A │  │ Deck B │          │        │            │   │
│  │  │  └───┬────┘  └───┬────┘          │        │            │   │
│  │  │      └─────┬──────┘              │        │            │   │
│  │  │      ┌─────▼─────┐              │        │            │   │
│  │  │      │  Mixer     │              │        │            │   │
│  │  │      │ xfader+EQ  │              │        │            │   │
│  │  │      │ +limiter   │              │        │            │   │
│  │  │      └─────┬─────┘              │        │            │   │
│  │  │            │ PCM out             │        │            │   │
│  │  └────────────┼─────────────────────┘        │            │   │
│  │               │                               │            │   │
│  │  ┌────────────▼───────┐  ┌───────────────────▼──────┐    │   │
│  │  │  cpal output       │  │  FFmpeg Decoder          │    │   │
│  │  │  (audio device)    │  │  (spawned process)       │    │   │
│  │  └────────────────────┘  └──────────────────────────┘    │   │
│  │                                                           │   │
│  │  ┌────────────────────────────────────────────────────┐   │   │
│  │  │              SQLite Storage Layer                   │   │   │
│  │  └────────────────────────────────────────────────────┘   │   │
│  └───────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

---

## Thread Model

| Thread | Responsibility | Constraints |
|--------|---------------|-------------|
| **Audio callback** (cpal) | Read command queue, advance transport, mix decks, write PCM to device | No I/O, no alloc, no locks, no panics |
| **Main / Tauri** | Handle UI commands, manage state, dispatch events | Standard async Rust |
| **Decoder pool** (2-3 threads) | Spawn ffmpeg, pipe PCM, write to cache | Disk I/O allowed |
| **Analysis workers** (rayon) | BPM detection, key detection, segmentation | CPU-intensive, can allocate |
| **AI conductor** (async task) | Call LLM API, parse response, validate JSON, enqueue commands | Network I/O, timeouts |

### Inter-thread communication

```
UI ──Tauri cmd──▶ Main thread ──SPSC ring──▶ Audio callback
                                              │
Audio callback ──SPSC ring──▶ Main thread ──Tauri event──▶ UI
                 (state snapshots)
```

- **UI → Engine**: `CommandQueue` — lock-free SPSC ring buffer (`ringbuf` crate).
  Commands are small structs (≤128 bytes) with a `scheduled_at: SchedulePoint` field.
- **Engine → UI**: `StateSnapshot` — engine writes snapshots into a triple-buffer
  (or second SPSC ring). Main thread polls at 30 Hz and emits Tauri events.
- **Decoder → Engine**: Decoded PCM is written to a memory-mapped cache file.
  Engine reads from the cache via a `TrackBuffer` handle (pre-loaded into RAM).

---

## Data Flow

```
                      ┌──────────┐
  User drops file ──▶ │ Importer │
                      └────┬─────┘
                           │ hash + copy
                      ┌────▼─────┐
                      │ Storage  │  (SQLite row + file on disk)
                      └────┬─────┘
                           │ spawn ffmpeg
                      ┌────▼──────────┐
                      │ Decoder       │  → canonical PCM cache (.pcm)
                      └────┬──────────┘
                           │
                      ┌────▼──────────┐
                      │ Analyzer      │  → track_profile.json
                      └────┬──────────┘
                           │
                      ┌────▼──────────┐
                      │ Library UI    │  (shows BPM, key, energy)
                      └────┬──────────┘
                           │ user/AI loads track
                      ┌────▼──────────┐
                      │ Deck A or B   │  (reads from PCM cache)
                      └────┬──────────┘
                           │ mixed output
                      ┌────▼──────────┐
                      │ Speakers      │
                      └───────────────┘
```

---

## Crate Layout

```
autodj/
├── Cargo.toml              (workspace)
├── crates/
│   ├── autodj-core/        Audio engine, mixer, transport, command queue
│   ├── autodj-analysis/    BPM, key, segmentation, waveform peaks
│   ├── autodj-storage/     SQLite, file cache, track_profile I/O
│   ├── autodj-ai/          AI conductor, scoring, template system
│   └── autodj-app/         Tauri application, commands, state management
├── ui/                     React + Vite frontend
│   ├── src/
│   ├── index.html
│   └── package.json
└── docs/
```

---

## Technology Choices

| Concern | Choice | Rationale |
|---------|--------|-----------|
| Desktop shell | Tauri 2 | Small binary, native look, Rust backend |
| Audio I/O | `cpal` | Cross-platform, low-level, well-maintained |
| Lock-free queue | `ringbuf` | Established SPSC ring buffer crate |
| Decoder | FFmpeg (spawned) | Handles every codec; licensing isolated |
| Storage | `rusqlite` | Sync API fits our decoder/analysis threads |
| Time-stretch | `rubato` | Pure Rust sample-rate converter; pitch shift via resample ratio |
| Frontend | React + Vite | Fast HMR, large ecosystem |
| Waveform | Pre-computed peaks → Canvas | Keep UI thread light |

### Deviations from defaults

If any technology choice changes during development, the rationale is documented here:

*(none yet)*

---

## Error / Fallback Strategy

```
Level 0 (happy path)
  AI selects track → transition executes → next track ready

Level 1 (AI unavailable / timeout > 5s)
  Heuristic scorer picks top candidate → deterministic transition template

Level 2 (analysis incomplete)
  Use conservative defaults (120 BPM, no key match, 8-bar crossfade)

Level 3 (decoder failure)
  Skip track, log error, try next in queue

Level 4 (audio device lost)
  Pause transport, attempt reconnect every 2s, notify UI

Level 5 (unrecoverable)
  Graceful shutdown, write crash log, exit
```
