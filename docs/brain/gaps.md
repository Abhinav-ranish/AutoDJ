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

## ✅ Phase 11 — Code Quality Cleanup (Completed)

All code quality items resolved in commits `ddfdfa4`–`817eff6`:
- `FRAC_1_SQRT_2` constant in EQ biquad filters
- Removed all unused imports, variables, and constants
- Zero clippy warnings across workspace
- Extracted `ProfileData` and `ContextInput` structs for cleaner APIs

---

## 🔵 Phases 12–17 — AI Enrichment

| Phase | Feature | Status |
|-------|---------|--------|
| 12 | DSP Enrichment (spectral, vocal, rhythm analysis) | **Complete** |
| 13 | Gemini Classification (genre/mood/vibe per track) | **Complete** |
| 14 | AI Decision Engine Overhaul (7-weight scorer, section-aware transitions) | **Complete** |
| 15 | Mix History & Session Feedback (wire up existing DB tables) | **Complete** |
| 16 | Audio Export (WAV recording of live mix) | **Complete** |
| 17 | Visual Polish (album art, energy-based sections, genre/mood badges) | **Complete** |

**All AI Enrichment phases complete.** 61 tests passing, zero warnings.

### Known gaps remaining:
- `beats.rs` — synthetic grid only, no DP tracker (future improvement)
- Album art display in deck panels — extracted and cached but not shown in deck UI
- Recording limited to one session per app launch (consumer is consumed)
- Batch classification pipeline (per-track available but no bulk UI)

---

## ✅ Phases 18–20 — Autonomous Mixing & AI Performance Controls

| Phase | Feature | Status |
|-------|---------|--------|
| 18 | Autonomous AutoDJ Loop (position-aware controller, auto-classify, chaining) | **Complete** |
| 19 | DJ-Style Live Mixing (section-aware windows, phrase alignment, 4 new templates) | **Complete** |
| 20 | AI Performance Controls (EQ isolator/kills, decision schema v2, action timelines) | **Complete** |

**All 31 tests passing, zero warnings.** TypeScript compiles clean.

### Phase 19 Implementation Details:
- `TransitionWindow` struct with anchor/start/end/score fields (`profile.rs`)
- `generate_transition_windows()` computes safe mix windows from section markers (`sections.rs`)
- `build_phrase_grid()` + `next_phrase_boundary()` for 8-bar phrase alignment (`sections.rs`)
- `generate_auto_cues()` produces `CUE_INTRO`, `CUE_DROP`, `CUE_BREAK`, `CUE_OUTRO`, `CUE_FIRST_BEAT` (`sections.rs`)
- `auto_cues` DB table + `save_auto_cues()`/`get_auto_cues()` methods (`db.rs`)
- 4 new transition templates: `DropSwap`, `LoopRoll`, `BassKillSwap`, `BreakBlend` (`transition.rs`)
- AutoDJ controller overhauled: section-aware windowed triggers, phrase boundary alignment, `aligning` phase in FSM (`App.tsx`)

### Phase 20 Implementation Details:
- LR4 cascade isolator mode + per-band kill switches in EQ (`eq.rs`)
- `SetEqKill`, `SetEqMode`, `SeekMs` engine commands (`command.rs`, `engine.rs`)
- `MixAction` struct + `MixActionKind` enum (12 action types) in AI schema (`planner.rs`)
- `build_heuristic_timeline()` generates full action sequences without LLM (`planner.rs`)
- Expanded DJ system prompt with all 7 templates + safety constraints (`planner.rs`)
- `transition_feedback` DB table for user learning hooks (`db.rs`)
- AI timeline preview panel + reasoning tags + confidence badge (`App.tsx`)
- 5 new Tauri commands: `set_eq_kill`, `set_eq_mode`, `seek_ms`, `get_auto_cues`, `submit_transition_feedback`

### Known gaps deferred from Phase 19/20:
- `BackspinExit` transition template (style-gated)
- Regression test for "unsafe" zone scheduling avoidance
- 4-band EQ mode (LOW/LOWMID/HIMID/HIGH)
- Per-deck EQ presets
- Action rate limiting (>N actions per 8 bars)
- Preference bias from feedback in scorer
- Live AI overlay on deck UI + "Lock controls" toggles

---

## ✅ Phases 21–25 — Library, Settings, Polish, Batch Ops, Safety

| Phase | Feature | Status |
|-------|---------|--------|
| 21 | Library Search, Sort & Filter (SQL search, column sort, filter pills) | **Complete** |
| 22 | Preferences & Settings (DB persistence, audio devices, settings panel) | **Complete** |
| 23 | Visual Polish & UX (album art in decks, hot cue buttons, context menu) | **Complete** |
| 24 | Batch Operations & AI Polish (Classify All with progress) | **Complete** |
| 25 | Safety & Reliability (RMS silence detection, emergency recovery, underrun counter) | **Complete** |

**65 tests passing, zero warnings.** TypeScript compiles clean.

### Phase 21 Implementation Details:
- `search_tracks` Tauri command with SQL `LIKE` search on title/artist
- BPM range, key, genre, mood filter parameters
- Column-header click-to-sort (toggles ASC/DESC)
- Search bar + filter pills UI in library toolbar
- Clear filters button to reset all

### Phase 22 Implementation Details:
- `get_preference()`/`set_preference()` DB methods for key-value preferences
- AI config persisted to SQL (with localStorage migration on first load)
- `get_audio_devices` command via `cpal` host device enumeration
- `SettingsPanel` component with 3 tabs: AI, Audio, AutoDJ
- Preferences restored on app launch via `initStore`

### Phase 23 Implementation Details:
- Album art display in deck headers (36x36 thumbnail via `convertFileSrc`)
- 4 color-coded hot cue buttons (red/green/blue/yellow) — set on click if empty, jump if set
- Right-click context menu on library rows (Load to A/B, Re-analyze, Classify)

### Phase 24 Implementation Details:
- "Classify All" button in library toolbar (visible when API key configured)
- Sequential batch classification with progress display ("Classifying X/Y")
- Skips already-classified and unanalyzed tracks

### Phase 25 Implementation Details:
- RMS-based silence detection in audio callback (~30Hz sampling aligned with snapshots)
- Silence threshold: 0.0001 (~-80 dB), 90 consecutive frames (~3s) triggers emergency
- `master_rms`, `silence_frames`, `underrun_count` fields added to `EngineStateSnapshot`
- AutoDJ controller monitors silence_frames and triggers emergency track load

### Known gaps deferred from Phases 21–25:
- Waveform hot cue markers (vertical lines at cue positions)
- Mix history view (browse past sessions)
- Responsive layout improvements
- Inline edit/modal for AI classifications
- AI feedback loop (thumb up/down)
- Energy arc goal parameter
- End-of-track detection by audio signal
- Crash-safe session recovery

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
| ~~**Album art in library + deck**~~ | ✅ Extracted on import, cached; deck display pending |
| ~~**Mix recording**~~ | ✅ WAV recording with ring buffer tap + writer thread |

### High Value / High Effort
| Feature | Notes |
|---------|-------|
| **MIDI controller support** | Use `midir` crate; map MIDI CC/note events to `Command` sends |
| **Key shift / harmonic pitch lock** | Pitch-shift without tempo change (rubber-band or phase vocoder) |
| **Playlist / crate management** | DB schema additions + side-panel UI for organizing tracks |

## Phase 50 Status ✅
- Fixed/finished in this phase: backend conductor truth visibility, fallback provenance, runtime memory feedback, and `fx_echo_slam` runtime support.
- Remaining gaps listed above are still the active follow-up set.
