# AutoDJ Backend DJ Behavior Audit
**Date:** 2026-03-08
**Scope:** autodj-ai (planner, scorer, context, llm) + autodj-core (transition, engine, command)

---

## 1. Current LLM Flow

```
plan_next()
  ├─ config.enabled && api_key.is_some()?
  │   YES → query_llm() → serde_json parse → validate track_id → Ok(decision)
  │         Any failure → fallback_heuristic()  [no reason recorded]
  └─ NO → fallback_heuristic()  [reason not recorded at all]
```

**Gate:** `AiConfig::default()` has `enabled: false`, `api_key: None`.
LLM is off by default and requires explicit opt-in. If LLM is unavailable, this is completely silent.

**After LLM succeeds:** The decision is accepted verbatim. No validation of `actions`, `execute_at_ms`, `template_name`, or action field clamping. Only `track_selection` is validated (must exist in available profiles).

---

## 2. Current Fallback Points

| Point | Condition | Current behavior |
|---|---|---|
| `plan_next():242` | `!enabled \|\| api_key.is_none()` | Silent fallback, `planner_mode="heuristic_only"` |
| `plan_next():248-251` | LLM returned unknown track_id | `log::warn` + fallback |
| `plan_next():255-258` | Any LLM error (HTTP, parse, timeout) | `log::warn` + fallback |
| `fallback_heuristic()` | Always | Returns `planner_mode="heuristic_only"`, no reason |

**No structured fallback reason is ever recorded in `AiDecision`.**
No way to tell from the output why you ended up in heuristic mode.

---

## 3. Current Timing Logic

- **Too-early guard:** `position_frac < 0.20` → return "wait" immediately
- **Too-late guard:** `position_frac > 0.95` OR `time_remaining_ms < 60_000` → "transition_now"
- **Normal:** "prepare_next" → pick best future transition window

**Transition window selection:** Takes highest-scored future `TransitionWindow` from track analysis. Aligns to next phrase boundary.

**Engagement holdback (Phase 38):** If best window is a `drop` anchor and it's < 8 seconds away, delay by 16 bars.

**Problems:**
- No momentum decay detection
- No "section just got good, hold longer" detection beyond the drop holdback
- Emergency too-late collapses to "transition_now" without checking if there's a better window in the near future
- `phrase_length_ms` in context is hardcoded to 16 beats, never reflects actual phrase structure

---

## 4. Transition Template Scoring Bias Problems

### Smooth_beatmix dominance

Base scores: `smooth_beatmix=0.50`, `bass_kill_swap=0.30`, `filter_sweep=0.30`, `break_blend=0.25`, while `drop_swap=0.20`, `echo_drop=0.15`, `loop_roll=0.15`, `backspin_cut=0.10`, `drop_ride=0.10`, `fx_echo_slam=0.10`.

In a typical "same genre, small energy jump, no vocal overlap, slow BPM, near outro" scenario:
- `smooth_beatmix` scores ~0.90 (base 0.5 + genre +0.2 + outro +0.1 + BPM +0.1)
- `bass_kill_swap` scores ~0.50 (base 0.3 + no-genre +0.15 + energy +0.15)
- No other template gets close

**smooth_beatmix wins by default in any calm scenario.** The margin is 0.40+ points, so anti-repetition (which decrements by 0.6^n) still enables smooth_beatmix to win for 2-3 consecutive uses before being displaced.

### Tiebreaker is deterministic, not varied

When top 2 scores are within 0.1, selection uses `elapsed_ms % 2 == 0`. At any given BPM, positions repeat the same modular pattern. For most real tracks this produces the same tiebreaker result every time.

### `fx_echo_slam` is broken

`fx_echo_slam` exists in template scoring and system prompt but NOT in `TransitionExpander`. Its action timeline uses `EchoOut` and `FilterSlam` which have no `Command` counterpart in autodj-core. It will be silently broken if dispatched via `ExecuteTransition`. When dispatched as `MixAction` timeline, the two broken action types will be silently ignored or crash at the app layer.

---

## 5. Expressive Action Gaps

### Fully implemented (Command enum + engine)
- `BeatRepeat` / `BeatRepeatOff` — deck.rs beat_repeat window
- `SetSlipMode` — deck.rs slip_position tracking
- `VinylBrake` — deck.rs brake ramp
- `Spinback` — deck.rs spinback ramp (reversal)
- `SetLoop` / `ClearLoop` — deck.rs loop region
- `SetFilter`, `SetEq`, `SetEqKill`, `SetCrossfader`, `SetDeckGain` — all present
- `JumpToMarker` / `SeekMs` — seek works, marker name ignored

### Stub (command defined, engine does nothing)
- `SyncToMaster` — logs debug only. BPM matching assumed to happen at app layer.

### Schema-only (MixActionKind exists, no Command, cannot reach engine)
- `EchoOut` — no Command::EchoOut. Emitted in action timelines but engine cannot execute.
- `ReverbTail` — no reverb effect anywhere in autodj-core.
- `FilterSlam` — no Command::FilterSlam. Direction semantics undefined at engine level.
- `Scratch` — defined in MixActionKind but never emitted by any code path.

### Planner access problems
- `BeatRepeat` is only injected in `loop_roll` transitions via `inject_transition_fx()`
- `VinylBrake` only in `backspin_cut` transitions
- `Spinback` only in `backspin_cut` transitions
- These are available and implemented but narrowly accessed

---

## 6. Current Vocal-Awareness Limitations

**Track selection:** `vocal_presence` is NOT a scoring dimension in `HeuristicScorer`. The 8 dimensions are: tempo, key, energy, variety, genre, mood, spectral, vibe. Vocal awareness only applies AFTER a track is selected.

**Post-selection:**
- `vocal_overlap` (both tracks >0.5 vocal) penalizes `smooth_beatmix` (-0.3) and `bass_kill_swap` (-0.2)
- `vocal_overlap` boosts `break_blend` (+0.4) — correct behavior
- Entry point selection uses `vocal_free_start` cue if outgoing track is vocal-heavy

**Gap:** It's possible to select a high-vocal track against a high-vocal current track via the 8-dimensional heuristic when tempo/key/mood/vibe alignment is strong, then get a break_blend or awkward transition. Vocal incompatibility should influence track selection, not just template selection.

---

## 7. Key Anti-Repetition Bugs

**Artist repetition is silently bypassed:**
`ScoringCandidate.artist` is always `None` in `fallback_heuristic()` because the metadata HashMap (mapping track_id → (artist, title)) is not passed in. The `recent_artists` ring buffer is maintained and the 0.3× penalty is coded in the scorer, but it never fires.

**Replay cap is unenforced:**
`track_at_replay_cap()` is defined but never called in `plan_next()` or `fallback_heuristic()`. Any filtering must happen at the app layer before passing `available_profiles`.

**Template window too small:**
Anti-repetition window is 6 templates. At 0.6^2 = 0.36 multiplier after 2 uses, smooth_beatmix at 0.90 becomes 0.32 — below bass_kill_swap at 0.50, which means diversity happens, but only after 2 repeated uses.

---

## 8. Top 5 Backend Fixes With Rationale

### Fix 1: Structured fallback reason in AiDecision
**Why:** Without knowing WHY you're in heuristic mode, it's impossible to distinguish "LLM off by config" from "LLM crashed" from "LLM returned garbage". Debugging requires guessing. Add `fallback_reason: Option<String>` to `AiDecision` with standardized codes.

### Fix 2: Fix artist anti-repetition (always None)
**Why:** The artist diversity code is a no-op. The metadata is available in `AiContext.available_tracks` (which has `artist` per track). Thread it into `ScoringCandidate` so the 0.3× penalty actually fires.

### Fix 3: Reduce smooth_beatmix dominance
**Why:** Starting at 0.50 with +0.20+0.10+0.10 reachable bonuses means smooth_beatmix scores 0.60-0.90 in all calm scenarios. Lower base to 0.35 and tighten the bonus conditions so it only achieves 0.7+ when genuinely ideal. Let bass_kill_swap and filter_sweep compete fairly.

### Fix 4: Add vocal_presence as a track selection scoring dimension
**Why:** Currently the system selects the best track ignoring vocal compatibility, then has to work around a bad choice at template selection time. Pre-empting vocal clashes at selection time produces better musical outcomes.

### Fix 5: Improve transition timing with groove hold / momentum detection
**Why:** The existing logic either waits (too early) or transitions at the best upcoming window. It doesn't model "this section just got interesting, ride it longer" or "this track is winding down, push forward". Adding simple momentum signals produces more DJ-like timing choices.

---

## Implementation Plan

### Files to change

| File | Changes |
|---|---|
| `crates/autodj-ai/src/lib.rs` | Add `weight_vocal` to AiConfig |
| `crates/autodj-ai/src/planner.rs` | Fallback reasons, artist fix, timing, template scoring, diversification |
| `crates/autodj-ai/src/scorer.rs` | Vocal dimension, score_vocal_compatibility() |
| `crates/autodj-ai/src/context.rs` | Add recent_templates, vocal_risk, timing guidance, ranked windows |

### Risk areas
- `planner.rs` is 1,297 lines with many interdependencies. Changes to template scoring affect quality.rs fixture pass rates.
- Anti-repetition changes may produce different template ordering in tests.
- Adding `fallback_reason` field to `AiDecision` requires updating all construction sites including quality.rs fixtures.
- `weight_vocal` adds a 9th scoring dimension — check that weights still sum to ~1.0 in practice (they're tunable anyway).

### Test plan
- Run `cargo test -p autodj-ai` after each phase
- Add new unit tests for: fallback reason tagging, artist anti-repetition, vocal scoring, timing hold logic, template competition balance
- Validate quality.rs fixtures still pass (they test different things)
