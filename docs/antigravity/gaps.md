# AutoDJ — Gaps, Bugs & Recommendations

## 🔴 Critical Bugs (Break the app)

| # | File | Bug | Fix |
|---|------|-----|-----|
| ~~B1~~ | `autodj-storage/src/decoder.rs` | ~~`find_ffmpeg()` only checks Unix paths~~ | **(FIXED)** |
| ~~B2~~ | `autodj-app/src/state.rs` | ~~`dirs_data_dir()` uses `std::env::var("HOME")`~~ | **(FIXED via `dirs` crate)** |
| ~~B3~~ | `autodj-app/src/commands.rs:analyze_track` | ~~Opens PCM at `cache.cache_dir()`~~ **(FIXED via `base_dir`)** | ~~Use `state.cache.pcm_path(&track_id)`~~ |
| ~~B4~~ | `autodj-app/src/commands.rs:analyze_track` | ~~Calls `state.db.clone()` inside spawned thread~~ **(FIXED via `Arc`)** | ~~Change `AppState.db` to `Arc<Mutex<Database>>`~~ |

## 🟡 Medium Bugs (Wrong behavior)

| # | File | Bug | Fix |
|---|------|-----|-----|
| B5 | `autodj-app/src/commands.rs:seek` | Hardcoded `bpm = 128.0` for ms → beat conversion | **(FIXED)** |
| B6 | `autodj-app/src/commands.rs:set_loop` | `start_beat = 0` always — loop always starts from track beginning | **(FIXED)** |
| B7 | `autodj-core/src/engine.rs:SyncToMaster` | Command is a no-op (`let _ = deck;`) | **(FIXED via interpolation + SetSpeed)** |

## ✅ Low / Code Quality Issues (Implemented)

| # | File | Issue | Status |
|---|------|-------|--------|
| C1 | `autodj-analysis/src/pipeline.rs` | sequential pipeline | **(FIXED via rayon)** |
| C2 | `autodj-core/src/mixer.rs` | No unit tests | **(FIXED)** |
| C3 | `autodj-app/src/commands.rs` | error-prone matching | **(FIXED via DeckId)** |
| C4 | `autodj-storage/src/db.rs` | raw status strings | **(FIXED via ImportStatus)** |
| C5 | `ui/src/store.ts` | errors swallowed | **(FIXED via toast)** |
| C6 | `ui/src` | IPC polling | **(FIXED via events)** |

---

## 🟡 Analysis Gaps (Phase 5 — not fully implemented)

| Module | Plan | Reality |
|--------|------|---------|
| `key.rs` | Chromagram + Krumhansl-Kessler profiles | **FIXED** — implemented basic STFT chromagram matching |
| `sections.rs` | Self-similarity matrix + novelty function | **Time-only heuristic** — splits by 15% intro / body / 15% outro |
| `regions` (safe loops) | Populated from beat grid + energy | **Always empty** (`safe_loops: vec![]`) |
| `beats.rs` | DP beat tracker (Ellis 2007) | Synthetic uniform grid from BPM only — no DP tracker |
| Parallelism | rayon for BPM/Key/Loudness in parallel | **FIXED** — implemented in `pipeline.rs` |

---

## ✅ Phase 6 — Transitions (Implemented)

Transitions are largely implemented over the recent commits.
- `autodj-core/src/transition.rs` — `TransitionTemplate` trait + `TransitionExpander` registry
- `SmoothBeatmix`, `FilterSweep`, `EchoDrop` templates
- `ExecuteTransition` variant in `Command` enum and handler in engine
- Tauri: `execute_transition` command
- UI: Transition panel with template choice, duration, and triggering
- **Remaining Gap**: Core engine integration tests for the `expand()` macro commands.

---

## ✅ Phase 7 — AI Conductor (Implemented)

The AI engine and corresponding UI are fully plumbed in!
- `autodj-ai/src/camelot.rs` — Camelot wheel distance function
- `autodj-ai/src/scorer.rs` — `HeuristicScorer` (tempo/key/energy/variety weights)
- `autodj-ai/src/context.rs` — `AiContext` builder
- `autodj-ai/src/llm.rs` — `LlmClient` (with response parsing logic)
- `autodj-ai/src/planner.rs` — `AiPlanner` delegating to LLM or Scorer
- `AppState`: tracks `AiPlanner` + `DjMode`
- Tauri commands: `request_next_track`, `set_mode`, `set_ai_config`
- UI: Mode switcher (Manual/Assisted/AutoDJ), AI status panel, track suggestions
- **Remaining Gap**: Testing the LLM edge cases, rate limits, and scoring heuristics.

---

## ❌ Phase 8 — Polish (Not Planned in Detail)

- Waveform canvas — render `{file_hash}_peaks.bin` on `<canvas>` with playhead + loop region overlay
- Mix history — populate `mix_history` table on track transitions
- Session tracking — create/close sessions; record duration + track count
- Audio export — record master output to WAV (capture mix buffer or cpal loopback)
- Settings screen — persist DJ preferences (crossfader curve, limiter ceiling, AI weights)
- Drag-and-drop import — Tauri `drag-drop` event

---

## 💡 Recommendations

### High Value / Low Effort
| Feature | Notes |
|---------|-------|
| **Auto-sync BPM on load** | When a track with a `TrackProfile` is loaded, use `profile.tempo.bpm` to set `deck.speed` automatically — **Implemented** |
| **ID3 tag reading on import** | Use `lofty` crate to fill `title`, `artist`, `album`, `artwork` during `import_files` — **Implemented** |
| **Hot cues** | Store 8 cue points per deck in DB; jump-to-cue buttons in UI — **Implemented DB + Backend Commands** |
| **Beat nudge buttons** | ±0.5% tempo adjust buttons (`<<` / `>>`) for fine sync without full BPM match |
| **Pitch range slider** | ±6 / ±10 / ±16% tempo range selector |
| **Keyboard shortcuts** | Space = play/pause, Z/X = loop toggle, Q/W = EQ kill |
| **Drag-and-drop import** | Tauri `drag-drop` event — very low effort, big UX win |

### High Value / Medium Effort
| Feature | Notes |
|---------|-------|
| **Waveform canvas with playhead** | Load `_peaks.bin` → render on `<canvas>`; draw playhead + loop region |
| **Album art in library + deck** | Store artwork path in DB during import; display in UI |
| **Mix recording** | Write master output samples to a growable buffer; export as WAV |

### High Value / High Effort
| Feature | Notes |
|---------|-------|
| **MIDI controller support** | Use `midir` crate; map MIDI CC/note events to `Command` sends |
| **Key shift / harmonic pitch lock** | Pitch-shift without tempo change (rubber-band or phase vocoder) |
| **Playlist / crate management** | DB schema additions + side-panel UI for organizing tracks |
