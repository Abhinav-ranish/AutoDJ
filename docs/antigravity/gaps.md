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
| B8 | `ui/src/App.tsx` | Deck BPM incorrectly guesses track metadata based on duration | **(FIXED via `track_id` prop passing through `DeckSnapshot`)** |
| B9 | `ui/src/App.tsx` | Gemini API key input isn't properly masked and state isn't remembered on reload | **(FIXED via `localStorage` init + UI refactor)** |

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
- `autodj-ai/src/context.rs` — `AiContext` builder with millisecond precision
- `autodj-ai/src/llm.rs` — `LlmClient` targeting Gemini `generateContent`
- `autodj-ai/src/planner.rs` — `AiPlanner` delegating to LLM or Scorer, parsing `execute_at_ms` json boundaries
- `AppState`: tracks `AiPlanner` + `DjMode`
- Tauri commands: `request_next_track`, `set_mode`, `set_ai_config`
- UI: Mode switcher (Manual/Assisted/AutoDJ), AI status panel, track suggestions, and settings modal
- Infrastructure: Added `dotenvy` for native `.env` variable ingestion (`GEMINI_API_KEY`)

---

## ✅ Phase 8 — Polish (Implemented)

Many polish items were completed successfully!
- **Waveform canvas** — ✅ Rendering using profile energy vectors with playhead overlays
- **ID3 Parsing** — ✅ Leveraged `lofty` crate during import to extract metadata
- **Hot Cues** — ✅ Core backend commands (`set_hot_cue`, `jump_to_hot_cue`) and DB schemas mapped
- **Tempo Control** — ✅ UI sliders with dynamic pitch range (±6/8/10/16%) and fine-tune nudge buttons (`«` / `»`)
- **Drag-and-Drop** — ✅ Tauri drop events integrated for file imports
- **Keyboard Shortcuts** — ✅ Global shortcuts for transport/loops/EQ kills
- *Pending*: Mix history, audio export to WAV, album art extraction

---

## ✅ Phase 10 — Perceived Tempo Engine (Implemented)

- `autodj-analysis/src/perceived_tempo.rs` — Onset density analysis + groove heuristics
- `PerceivedTempoInfo` added to `TrackProfile` and SQLite `track_profiles` table
- AI `scorer.rs` now scores transitions using `perceived_tempo` instead of raw BPM
- AI `context.rs` sends `perceived_tempo` and `groove_type` to Gemini
- **Remaining Gap**: Replace basic variance heuristics with ML-based drum pattern classifier

---

## 💡 Recommendations

### High Value / Low Effort
| Feature | Notes |
|---------|-------|
| ~~**Auto-sync BPM on load**~~ | ✅ Implemented |
| ~~**ID3 tag reading on import**~~ | ✅ Implemented via `lofty` |
| ~~**Hot cues**~~ | ✅ Implemented DB + Backend Commands |
| ~~**Beat nudge buttons**~~ | ✅ Implemented UI buttons (`<<` / `>>`) |
| ~~**Pitch range slider**~~ | ✅ Implemented |
| ~~**Keyboard shortcuts**~~ | ✅ Implemented |
| ~~**Drag-and-drop import**~~ | ✅ Implemented |

### High Value / Medium Effort
| Feature | Notes |
|---------|-------|
| ~~**Waveform canvas with playhead**~~ | ✅ Implemented |
| **Album art in library + deck** | Store artwork path in DB during import; display in UI |
| **Mix recording** | Write master output samples to a growable buffer; export as WAV |

### High Value / High Effort
| Feature | Notes |
|---------|-------|
| **MIDI controller support** | Use `midir` crate; map MIDI CC/note events to `Command` sends |
| **Key shift / harmonic pitch lock** | Pitch-shift without tempo change (rubber-band or phase vocoder) |
| **Playlist / crate management** | DB schema additions + side-panel UI for organizing tracks |
