# AutoDJ — Gaps, Bugs & Recommendations

## 🔴 Critical Bugs (Break the app)

| # | File | Bug | Fix |
|---|------|-----|-----|
| B1 | `autodj-storage/src/decoder.rs` | `find_ffmpeg()` only checks Unix paths (`/opt/homebrew`, `/usr/local`, `/usr/bin`) + uses `which` — **breaks on Windows** | Add `%ProgramFiles%\ffmpeg\bin\ffmpeg.exe` + use `where` on Windows |
| B2 | `autodj-app/src/state.rs` | `dirs_data_dir()` uses `std::env::var("HOME")` — **`HOME` doesn't exist on Windows** | Use Tauri `app.path().app_data_dir()` or the `dirs` crate |
| ~~B3~~ | `autodj-app/src/commands.rs:analyze_track` | ~~Opens PCM at `cache.cache_dir()`~~ **(FIXED via `base_dir`)** | ~~Use `state.cache.pcm_path(&track_id)`~~ |
| ~~B4~~ | `autodj-app/src/commands.rs:analyze_track` | ~~Calls `state.db.clone()` inside spawned thread~~ **(FIXED via `Arc`)** | ~~Change `AppState.db` to `Arc<Mutex<Database>>`~~ |

## 🟡 Medium Bugs (Wrong behavior)

| # | File | Bug | Fix |
|---|------|-----|-----|
| B5 | `autodj-app/src/commands.rs:seek` | Hardcoded `bpm = 128.0` for ms → beat conversion | Read BPM from current engine state snapshot |
| B6 | `autodj-app/src/commands.rs:set_loop` | `start_beat = 0` always — loop always starts from track beginning | Use current deck `position_ms` from engine state to compute `start_beat` |
| B7 | `autodj-core/src/engine.rs:SyncToMaster` | Command is a no-op (`let _ = deck;`) | After Phase 5 analysis, look up `TrackProfile.tempo.bpm` and adjust `deck.speed` multiplier |

## 🟠 Low / Code Quality Issues

| # | File | Issue |
|---|------|-------|
| C1 | `autodj-analysis/src/pipeline.rs` | Comment says "rayon parallel" but pipeline runs entirely sequentially | Remove misleading comment or add `rayon` |
| C2 | `autodj-core/src/mixer.rs` | No unit tests | Add tests for limiter behavior, equal-power curve |
| C3 | `autodj-app/src/commands.rs` | `parse_deck()` string matching is error-prone | Use `serde` deserialization with `#[serde(rename)]` |
| C4 | `autodj-app` | Import status is raw strings (`"pending"`, `"ready"`, etc.) scattered across crates | Replace with a typed `ImportStatus` enum |
| C5 | `ui/src/store.ts` | All errors go to `console.error` and are silently swallowed | Add a toast / notification system in the UI |
| C6 | `ui/src` | Engine state polled every 33 ms via `setInterval` IPC round-trips | Replace with Tauri `emit`/`listen` events pushed from Rust |

---

## 🟡 Analysis Gaps (Phase 5 — not fully implemented)

| Module | Plan | Reality |
|--------|------|---------|
| `key.rs` | Chromagram + Krumhansl-Kessler profiles | **Stub** — always returns `"C" / "8B"`, confidence 0.1 |
| `sections.rs` | Self-similarity matrix + novelty function | **Time-only heuristic** — splits by 15% intro / body / 15% outro |
| `regions` (safe loops) | Populated from beat grid + energy | **Always empty** (`safe_loops: vec![]`) |
| `beats.rs` | DP beat tracker (Ellis 2007) | Synthetic uniform grid from BPM only — no DP tracker |
| Parallelism | rayon for BPM/Key/Loudness in parallel | Sequential — no rayon used |

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
| **Auto-sync BPM on load** | When a track with a `TrackProfile` is loaded, use `profile.tempo.bpm` to set `deck.speed` automatically |
| **ID3 tag reading on import** | Use `lofty` crate to fill `title`, `artist`, `album`, `artwork` during `import_files` |
| **Hot cues** | Store 8 cue points per deck in DB; jump-to-cue buttons in UI — huge DJ workflow improvement |
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
