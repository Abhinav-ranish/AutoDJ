# AutoDJ AI Enrichment & Next Phases Design

**Date**: 2026-03-07
**Approach**: Foundation First — sequential phases with clean dependencies

## Overview

Comprehensive overhaul of AutoDJ's AI capabilities through enriched song data, Gemini-powered classification, and a redesigned decision engine. Includes all remaining handoff items: mix history, audio export, and visual polish.

## Phases 11–17

---

## Phase 11: Fix All Warnings & Code Quality

**Goal**: Zero compiler warnings, clean clippy output.

### Compilation Errors (2)
- `crates/autodj-core/src/eq.rs:49,63` — Replace `0.7071` with `f32::consts::FRAC_1_SQRT_2`

### Unused Code (6)
- `crates/autodj-analysis/src/pipeline.rs:9` — Remove unused import `RegionInfo`
- `crates/autodj-analysis/src/key.rs:110` — Prefix unused variable as `_base`
- `crates/autodj-core/src/engine.rs:356` — Prefix unused param as `_sample_rate`
- `crates/autodj-app/src/commands.rs:13` — Remove unused import `lofty::file::AudioFile`
- `crates/autodj-ai/src/camelot.rs:3` — Remove or use `CAMELOT_KEYS` constant

### Clippy Warnings (15)
- `db.rs:294` — Refactor `save_track_profile` to accept a struct instead of 12 args
- `bpm.rs:52` — Use `*=` operator instead of manual assign
- `bpm.rs:74-87` — Collapse nested if statements
- `key.rs:40` — Use `(20.0..=5000.0).contains(&freq)`
- `key.rs:68` — Use `enumerate()` instead of manual index
- `loudness.rs:39` — Use `enumerate()` for loop counter
- `perceived_tempo.rs` (8 instances) — Use `.contains()` for range checks
- `deck.rs:181` — Remove unnecessary `f64 as f64` cast
- `transport.rs:60`, `engine.rs:314` — Use `is_multiple_of()` (or `%` with note)

---

## Phase 12: Database Schema Expansion & DSP Enrichment

**Goal**: Rich audio features stored per track for AI consumption.

### New DB Columns on `track_profiles`

| Column | Type | Description |
|--------|------|-------------|
| `spectral_centroid` | REAL | Brightness measure (Hz) |
| `spectral_flatness` | REAL | Tonal vs noisy (0.0–1.0) |
| `bass_energy` | REAL | Energy in 20–250 Hz band |
| `mid_energy` | REAL | Energy in 250–4000 Hz band |
| `high_energy` | REAL | Energy in 4000–20000 Hz band |
| `vocal_presence` | REAL | Estimated vocal content (0.0–1.0) |
| `rhythm_complexity` | REAL | Onset pattern irregularity (0.0–1.0) |
| `danceability` | REAL | Computed from onset regularity + bass energy |
| `energy_trajectory` | TEXT | "building" / "steady" / "declining" / "dynamic" |
| `intro_length_ms` | INTEGER | Actual intro duration (energy-based) |
| `outro_length_ms` | INTEGER | Actual outro duration (energy-based) |
| `first_drop_ms` | INTEGER | Position of first major energy peak |

### New DSP Modules in `autodj-analysis`

#### `spectral.rs`
- STFT-based spectral centroid calculation (weighted mean of frequency bins)
- Spectral flatness: geometric mean / arithmetic mean of magnitude spectrum
- 3-band energy distribution: split spectrum at 250 Hz and 4000 Hz crossover points
- Average across full track with optional per-section breakdown

#### `vocal_detection.rs`
- Harmonic-to-noise ratio in vocal frequency range (300 Hz – 3.5 kHz)
- Spectral peak prominence in vocal band vs surrounding bands
- Returns 0.0 (instrumental) to 1.0 (heavily vocal)

#### `rhythm.rs`
- Onset regularity: standard deviation of inter-onset intervals normalized by mean
- Complexity metric: entropy of onset timing relative to beat grid
- Danceability: composite of onset regularity (50%), bass energy (30%), tempo stability (20%)

### Pipeline Updates
- New analyzers run concurrently via rayon alongside existing BPM/key/energy workers
- `TrackProfile` struct gains new fields with `Option<T>` for backwards compatibility
- Section segmentation upgraded: energy-curve-based intro/outro detection replaces fixed 15% heuristic
- `save_track_profile` updated to persist new columns

---

## Phase 13: Gemini-Powered Classification

**Goal**: Subjective qualities (genre, mood, vibe) classified per track by AI.

### New DB Columns on `track_profiles`

| Column | Type | Description |
|--------|------|-------------|
| `genre` | TEXT | Primary genre (house, hip-hop, techno, pop, etc.) |
| `subgenre` | TEXT | Specific subgenre (deep house, trap, minimal techno) |
| `mood` | TEXT | Emotional character (euphoric, melancholic, aggressive, chill, dark, uplifting) |
| `vibe` | TEXT | Set role (anthem, filler, opener, closer, peak_time, warm_up) |
| `mix_notes` | TEXT | AI-generated mixing tips |
| `ai_tags` | TEXT | Comma-separated flexible tags |

### New Module: `autodj-ai/src/classifier.rs`

**Input**: Full `TrackProfile` + ID3 metadata (title, artist, album, genre tag)

**Process**:
1. Build structured prompt with all DSP features: BPM, key, spectral profile, energy curve shape, vocal presence, rhythm complexity, groove type, danceability, band energies
2. Include ID3 metadata: artist, title, album, genre tag
3. Send to Gemini with JSON response schema enforcement
4. Parse response into classification fields
5. Store in `track_profiles` table

**Integration**:
- Runs as post-analysis step after DSP pipeline completes
- Rate-limited (1 call per 10s, matching existing limiter)
- Graceful fallback: if API unavailable, genre/mood columns stay NULL
- Re-classification command available for manual trigger

**Prompt Strategy**:
- Provide all numeric features with labels and ranges
- Ask for structured JSON with predefined enum values for genre/mood/vibe
- Include artist/title for additional context (Gemini can recognize well-known tracks)
- Request mix_notes as free-form DJ-oriented tips

---

## Phase 14: AI Decision Engine Overhaul

**Goal**: Dramatically smarter track selection, transition timing, and template choice.

### Redesigned AI Context (`context.rs`)

The context builder sends **everything** to the LLM:
- Full spectral profile (centroid, flatness, band energies)
- Genre/subgenre/mood/vibe from Gemini classification
- Energy trajectory and curve shape
- Section details (intro/outro lengths, drop positions)
- Vocal presence and rhythm complexity
- Danceability score
- Mix history (last 10 tracks played, transitions used)
- Set position awareness (tracks played, elapsed time, estimated set duration)
- Mix notes from classifier

### Expanded Heuristic Scorer (`scorer.rs`)

New weight distribution:
- Tempo compatibility: 0.20 (was 0.30)
- Key compatibility: 0.25 (was 0.30)
- Energy match: 0.15 (was 0.20)
- Variety: 0.10 (was 0.20)
- Genre compatibility: 0.15 (NEW)
- Mood continuity: 0.10 (NEW)
- Spectral contrast: 0.05 (NEW — avoid two identical-sounding tracks)

Genre compatibility scoring:
- Same genre: 1.0
- Related genres (house↔techno, hip-hop↔trap): 0.7
- Neutral pairs (pop↔house): 0.4
- Clashing genres (metal↔ambient): 0.1

Mood continuity scoring:
- Same mood: 1.0
- Adjacent moods (chill→uplifting): 0.8
- Moderate shift (euphoric→melancholic): 0.4
- Jarring shift (aggressive→chill): 0.2

### Transition Intelligence

Template selection logic:
- Vocal tracks → longer transitions (16-32 bars), smooth beatmix preferred
- Energy drops → echo drop template
- Building energy → filter sweep
- Same genre/BPM → smooth beatmix
- Large BPM gap (>8) → echo drop (creates natural break)

Section-aware timing:
- Trigger point: when outgoing track reaches outro region (not arbitrary %)
- Align: outgoing outro length with incoming intro length
- Avoid: triggering during vocal sections of outgoing track

### Improved Prompt Engineering

System prompt includes:
- DJ mixing principles (harmonic mixing, energy flow, crowd reading)
- Set flow context ("30 minutes in, energy building, 15 tracks played")
- Genre compatibility rules
- Mood transition guidance
- Track-specific mixing notes from classifier
- Explicit instruction to consider vocal sections, drops, and section boundaries

---

## Phase 15: Mix History & Session Tracking

**Goal**: Record mix history and feed it back to AI for set awareness.

### Implementation
- Wire up existing `sessions` and `mix_history` tables (schema exists)
- Auto-create session on first track play
- Record per-track: start_ms, end_ms, transition_type, ai_decision_json
- Close session on 5-minute idle timeout
- Feed last 10 tracks into AI context builder
- Track implicit user feedback: did user override AI suggestion? (store override count)

### UI
- Session history panel: tracklist with timestamps and transition types
- Session stats: total duration, track count, genre distribution
- Current session indicator in header

---

## Phase 16: Audio Export

**Goal**: Capture live mix output to WAV file.

### Implementation
- Tap master output buffer in audio callback (after limiter)
- Write interleaved f32 samples to ring buffer → writer thread → WAV file
- Start/stop recording via Tauri command
- WAV format: 48 kHz, stereo, 32-bit float
- On stop: flush buffer, write WAV header, embed metadata
- Include tracklist + timestamps as BWF markers or sidecar JSON

### UI
- Record button (red circle) in master section
- Recording indicator: red dot + elapsed time
- File save dialog on stop

---

## Phase 17: Visual Polish & Analysis Improvements

**Goal**: Complete remaining handoff items.

### Album Art
- Extract embedded artwork via `lofty` crate during import
- Cache as PNG in `~/.autodj/cache/art/{track_id}.png`
- Display in deck header (small) and library grid (thumbnail)
- Fallback: genre-colored placeholder

### Beat Grid Refinement
- Replace synthetic beat grid (fixed intervals from BPM) with dynamic programming tracker
- DP tracker follows actual onset positions, handles tempo variations
- Improved downbeat detection using spectral weight
- Higher confidence scores for DP-tracked grids vs synthetic

### Section Analysis Improvement
- Energy-curve-based detection (replaces fixed 15% intro/outro)
- Detect actual transitions in energy: sustained low → ramp = intro/buildup boundary
- Drop detection: sudden energy spike after sustained dip
- Breakdown detection: energy dip within high-energy section
- Store labeled sections with confidence scores

### Library Enhancements
- Genre column with color-coded badges
- Mood column with icon indicators
- AI tags shown as small chips
- Album art thumbnails in list view
- Sort/filter by genre, mood, vibe

---

## Dependencies

```
Phase 11 ──→ Phase 12 ──→ Phase 13 ──→ Phase 14
                                          │
                                          ├──→ Phase 15
                                          ├──→ Phase 16
                                          └──→ Phase 17
```

Phases 15, 16, 17 are independent of each other and can be done in any order after Phase 14.

## Success Criteria

1. `cargo clippy --workspace` produces zero warnings
2. Every imported track gets 12+ new DSP features computed
3. Gemini classifies genre/mood/vibe for tracks when API key is available
4. AI makes genre-aware, mood-continuous, section-timed decisions
5. Mix history is recorded and influences future AI decisions
6. Live mix can be exported as WAV
7. Album art displays in deck and library
