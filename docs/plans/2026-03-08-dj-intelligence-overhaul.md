# DJ Intelligence Overhaul — Phases 46-49

## Problem Statement

The AI DJ has several weaknesses that make transitions feel mechanical:

1. **Songs don't vibe-match** — The 7-dimension scorer (tempo/key/energy/variety/genre/mood/spectral) doesn't score the classified "vibe" field (opener/anthem/filler/closer/peak_time) at all. The incoming song may be technically compatible but feels wrong.

2. **Always starts from the beginning** — Entry point selection (`select_entry_point` in planner.rs:545-576) can jump to markers (intro/drop/break/first_beat) but doesn't find the section in the incoming track that best matches the current vibe. It should compare section-to-section.

3. **Performance FX never fire** — `EchoOut`, `ReverbTail`, `FilterSlam`, `VinylBrake`, `Spinback` are defined as `MixActionKind` variants (planner.rs:135-139) but `build_heuristic_timeline` (planner.rs:645-875) never emits any of them. Templates only use EQ kills, filter sweeps, and gain fades.

4. **Can't replay songs** — `score_variety` (scorer.rs:224-226) hard-returns 0.0 for any recently played track. No anthem callbacks possible.

5. **Basic auto-cues** — Only first_beat, intro, drop, break, outro. No "best mix-in point", vocal-free zones, or impact moments.

---

## Phase 46: Performance FX-Driven Transitions

**Goal**: Wire existing FX into transition templates.

### Implementation

#### 1. FX Injection Helper

**File**: `crates/autodj-ai/src/planner.rs`

Add `inject_transition_fx(actions: &mut Vec<MixAction>, template: &str, outgoing_deck: &str, bar_ms: u64, personality: DjPersonality)` after `build_heuristic_timeline`.

FX rules per template:
| Template | FX Added | Condition |
|----------|----------|-----------|
| `echo_drop` | `EchoOut { beats: 4.0, feedback: 0.6 }` | Always (replaces gain-only fade) |
| `backspin_cut` | `Spinback` or `VinylBrake` | Aggressive/Hype → spinback; Lounge/Smooth → vinyl brake |
| `drop_swap` | `FilterSlam { direction: "up" }` on incoming | At cut point for impact |
| `loop_roll` | `BeatRepeat` (4→2→1→0.5) + `SetSlipMode` | Replace SetLoop approach |
| `break_blend` | `ReverbTail { decay_seconds: 2.0 }` on outgoing | During fade-out |
| `drop_ride` | `FilterSlam { direction: "down" }` on outgoing | At bass swap |
| `smooth_beatmix` | None (FX-free by default) | Only subtle reverb with Aggressive personality |
| `bass_kill_swap` | None | Same as smooth_beatmix |

#### 2. Personality-Gated FX Density

| Personality | FX Density | Templates With FX |
|-------------|-----------|-------------------|
| Lounge | 0.1 | break_blend only |
| Smooth | 0.3 | echo_drop, break_blend |
| OpenFormat | 0.6 | All except smooth_beatmix |
| Hype | 0.8 | All templates |
| Festival/Aggressive | 1.0 | All templates, stronger params |

#### 3. Cooldown & Safety

**File**: `crates/autodj-ai/src/planner.rs` (struct `AiPlanner`)

- Add `recent_fx: Vec<String>` — skip FX if same one used in last 2 transitions
- Safety: no EchoOut + ReverbTail on same deck at same `at_ms`; no VinylBrake + Spinback simultaneously; no FilterSlam + active SetFilter sweep

#### 4. New Template: `fx_echo_slam`

4-bar echo out (increasing feedback) on outgoing → 1 bar silence → filter slam up on incoming with hard cut to full volume. Scored high when: energy_jump > 0.25, personality is Hype/Festival/Aggressive, genre change, near a drop.

---

## Phase 47: Vibe-Matched Transitions & Section Seeking

**Goal**: Score tracks on vibe compatibility and seek to matching sections.

### Implementation

#### 1. Vibe Scoring

**File**: `crates/autodj-ai/src/scorer.rs`

Add `score_vibe(vibe_a: Option<&str>, vibe_b: Option<&str>, set_position: Option<f64>) -> f64`:

Vibe flow rules:
```
opener → warm_up/filler: 1.0
warm_up → filler/peak_time: 1.0
filler → anthem/peak_time: 0.9
anthem → anthem: 0.7 (exciting but tiring)
peak_time → closer/filler: 0.8
closer → opener: 0.5 (only if set ending)
same vibe: 0.6
mismatched (opener → closer mid-set): 0.2
```

Set position modifiers: Peak arc → anthem/peak_time bonus; Warmup arc → opener/warm_up bonus.

#### 2. Weight Redistribution

**File**: `crates/autodj-ai/src/lib.rs`

Add `weight_vibe: 0.10`. New totals: tempo 0.20, key 0.20, energy 0.15, variety 0.05, genre 0.15, mood 0.10, spectral 0.05, vibe 0.10 = 1.0

#### 3. Enhanced Spectral Scoring

**File**: `crates/autodj-ai/src/scorer.rs`

Upgrade `score_spectral()` to compare:
- `bass_energy` ratios (similar low-end character)
- `flatness` (tonal vs noisy — clean deep house shouldn't follow industrial)
- `high_energy` ratios (bright vs dark feel)

Multi-dimensional spectral distance instead of centroid-only.

#### 4. Section-to-Section Entry Point Selection

**File**: `crates/autodj-ai/src/planner.rs`

Upgrade `select_entry_point` to accept `current_profile` + `current_elapsed_ms`. For each candidate section in incoming track, compute:
- Energy similarity: `1.0 - |current_section_energy - candidate_section_energy|`
- Spectral similarity: compare band ratios
- Label compatibility: outro→intro = ideal, drop→drop = high energy, breakdown→break = smooth

Select the incoming section with highest combined score.

**File**: `crates/autodj-analysis/src/sections.rs`

Add `section_average_energy(curve, duration_ms, start_ms, end_ms) -> f64`.

#### 5. Context Expansion

**File**: `crates/autodj-ai/src/context.rs`

Expose `spectral_bass_energy`, `spectral_flatness`, `spectral_high_energy`, `ai_tags` in `TrackProfileShort`.

---

## Phase 48: Track Recycling & Anthem Callbacks

**Goal**: Allow song replays and anthem callbacks.

### Implementation

#### 1. Soft Variety Scoring

**File**: `crates/autodj-ai/src/scorer.rs`

Replace binary variety scoring with distance-based decay:
```rust
fn score_variety(&self, candidate_id: &str, recent_tracks: &[String],
                 allow_replay: bool, candidate_vibe: Option<&str>) -> f64 {
    if let Some(pos) = recent_tracks.iter().position(|id| id == candidate_id) {
        if allow_replay {
            let is_banger = matches!(candidate_vibe, Some("anthem") | Some("peak_time"));
            let min_distance = if is_banger { 3 } else { 6 };
            if pos >= min_distance {
                return 0.4 + 0.1 * ((pos - min_distance) as f64).min(6.0);
            }
            return 0.0; // too recent
        }
        return 0.0;
    }
    1.0
}
```

#### 2. Config

**File**: `crates/autodj-ai/src/lib.rs`

- `allow_replay: bool` (default: false)
- `anthem_callback_distance: usize` (default: 5)

#### 3. Anthem Callback Bonus

**File**: `crates/autodj-ai/src/scorer.rs` (`select_next_extended`)

At peak set position (>0.6) with `allow_replay: true`:
- Anthems past cooldown get +0.15 bonus
- At closing (>0.85), earliest anthem gets closing callback bonus

#### 4. Play Count Tracking

**File**: `crates/autodj-ai/src/planner.rs`

- `track_play_counts: HashMap<String, usize>` on `AiPlanner`
- Hard cap: no track played more than 3 times per session
- Callback must enter at different section than previous play

#### 5. Quality Evaluation Update

**File**: `crates/autodj-ai/src/quality.rs`

- `RepeatedTrack` severity 0.8 → 0.4 when replay allowed
- New `ExcessiveReplay` issue for >3 plays

---

## Phase 49: Advanced Auto-Cueing & Mix Point Intelligence

**Goal**: Intelligent mix-in/mix-out points, vocal-free zones, impact moments.

### Implementation

#### 1. Vocal Curve

**File**: `crates/autodj-analysis/src/vocal_detection.rs`

Add `detect_vocal_curve(samples, sample_rate) -> Vec<f64>` — per-window vocal presence values using same FFT logic as `detect_vocal_presence` but returning full curve.

#### 2. Vocal-Free Zones

**File**: `crates/autodj-analysis/src/sections.rs`

Add `detect_vocal_free_zones(vocal_curve, duration_ms, threshold: 0.3) -> Vec<(u64, u64)>` — consecutive windows below threshold for 4+ windows.

#### 3. Impact Moments

**File**: `crates/autodj-analysis/src/sections.rs`

Add `detect_impact_moments(energy_curve, duration_ms) -> Vec<u64>` — energy derivative spikes (delta > 0.3 of curve range), deduplicated within 4 seconds.

#### 4. Best Mix Points

**File**: `crates/autodj-analysis/src/sections.rs`

Add `compute_best_mix_points(sections, energy_curve, vocal_free_zones, duration_ms, bpm) -> (Option<AutoCue>, Option<AutoCue>)`:

**Best mix-in**: prefer vocal-free zone in first 30%, phrase-aligned, low-to-moderate energy, end of intro.
**Best mix-out**: prefer outro start or last vocal-free zone, in last 30%, pre-energy-decline, phrase-aligned.

#### 5. Pipeline Integration

**File**: `crates/autodj-analysis/src/pipeline.rs`

After existing `generate_auto_cues`, run vocal curve → vocal-free zones → impact moments → best mix points. Append new cue types.

New cue types: `best_mix_in`, `best_mix_out`, `vocal_free_start`, `vocal_free_end`, `impact`.

#### 6. AI Integration

**File**: `crates/autodj-ai/src/planner.rs`

- Entry point: prefer `best_mix_in` if available with high confidence
- Vocal clash: if outgoing has vocals, prefer `vocal_free_start` entry
- Impact templates: `drop_swap`/`echo_drop` prefer `impact` entry

**File**: `crates/autodj-analysis/src/sections.rs` (transition windows)

- `best_mix_out` → window score 0.9
- `vocal_free_start` → window score 0.75
- `impact` → window score 0.55

---

## Implementation Order

**46 → 47 → 48 → 49** (each independently deliverable)

- Phase 46 (FX) is self-contained — only touches planner.rs templates
- Phase 47 (vibe) establishes vibe scoring that Phase 48 builds on
- Phase 48 (replay) uses vibe data from 47 for smarter callbacks
- Phase 49 (cueing) provides better cue data for 47's section-seeking

## Verification Per Phase

- `cargo test --workspace` passes
- `cargo clippy --workspace` clean
- Manual listening test with 10+ track playlist
- AI debug panel shows new features exercised
