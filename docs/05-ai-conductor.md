# Auto-DJ — AI Conductor

## Role

The AI conductor is a **planner**, not a performer. It receives structured context about
the current mix state and library, and outputs a JSON plan of commands that conform to
`docs/03-command-api.md`. The engine validates and executes; the AI never touches audio.

---

## When the AI is Consulted

The AI is called at specific trigger points, not continuously:

| Trigger | Description |
|---------|-------------|
| `NeedNextTrack` | ~60% through the current track (configurable) |
| `SectionBoundary` | When a new section starts on the playing track |
| `UserRequest` | User asks "what should I play next?" |
| `TransitionStart` | To refine transition parameters just before execution |
| `SetStart` | Beginning of a DJ set (to plan first 2–3 tracks) |

---

## AI Input Context

The AI receives a structured JSON context (not raw audio):

```json
{
  "current_state": {
    "mode": "AutoDJ",
    "master_bpm": 128.0,
    "deck_a": {
      "track_id": "trk_abc",
      "title": "Track A Title",
      "artist": "Artist A",
      "bpm": 128.0,
      "key": "Bm",
      "camelot": "10A",
      "current_section": "drop",
      "position_percent": 0.65,
      "remaining_bars": 32,
      "energy": 0.85,
      "sections_remaining": ["breakdown", "outro"]
    },
    "deck_b": {
      "state": "empty"
    },
    "recent_tracks": ["trk_001", "trk_002", "trk_003"],
    "set_duration_minutes": 45,
    "energy_trajectory": "rising"
  },
  "candidates": [
    {
      "track_id": "trk_xyz",
      "title": "Candidate Title",
      "artist": "Candidate Artist",
      "bpm": 126.0,
      "key": "F#m",
      "camelot": "11A",
      "energy_avg": 0.78,
      "compatibility_score": 0.85,
      "tempo_distance": 2.0,
      "key_distance": 1
    }
  ],
  "available_templates": [
    "smooth_beatmix",
    "filter_sweep",
    "echo_drop"
  ],
  "user_preferences": {
    "mix_style": "smooth",
    "energy_preference": "gradual_build",
    "transition_length_bars": 16,
    "allow_key_clash": false
  }
}
```

---

## AI Output Schema

The AI must return valid JSON matching this schema:

```json
{
  "decision": {
    "action": "transition",
    "reasoning": "Track X has compatible key (11A, +1 from current 10A) and similar energy...",
    "confidence": 0.85
  },
  "track_selection": {
    "track_id": "trk_xyz",
    "score_breakdown": {
      "tempo": 0.90,
      "key": 0.85,
      "energy": 0.80,
      "variety": 0.75,
      "vocal_clash": 0.95
    }
  },
  "transition_plan": {
    "template": "smooth_beatmix",
    "params": {
      "duration_bars": 16,
      "incoming_deck": "B",
      "outgoing_deck": "A",
      "incoming_start_beat": 0,
      "bass_swap_bar": 8,
      "crossfader_curve": "equal_power"
    },
    "trigger": {
      "type": "AtSection",
      "section": "outro",
      "offset_bars": -4
    }
  },
  "energy_intent": "maintain",
  "next_look_ahead": {
    "suggested_tracks": ["trk_aaa", "trk_bbb"],
    "reason": "Building toward a peak moment"
  }
}
```

---

## Template System

Transition templates define the **shape** of a transition. Each template has:

1. **Name** (identifier)
2. **Parameters** with types, defaults, and valid ranges
3. **Expansion function** that produces a sequence of primitive commands

### Template: `smooth_beatmix`

Classic 16-bar mix with bass swap.

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `duration_bars` | u32 | 16 | 4–64 | Total transition length |
| `bass_swap_bar` | u32 | 8 | 1–duration | Bar at which to swap bass EQ |
| `crossfader_curve` | enum | equal_power | linear/equal_power/hard_cut | Curve shape |
| `incoming_start_beat` | u32 | 0 | 0–track_length | Where to start incoming track |

### Template: `filter_sweep`

Sweep a high-pass filter up on the outgoing track while bringing in the incoming.

| Parameter | Type | Default | Range |
|-----------|------|---------|-------|
| `duration_bars` | u32 | 8 | 4–32 |
| `filter_start_hz` | f32 | 200 | 20–2000 |
| `filter_end_hz` | f32 | 8000 | 2000–20000 |
| `sweep_curve` | enum | exponential | linear/exponential |

### Template: `echo_drop`

Apply echo/delay on outgoing, cut, drop in incoming from silence.

| Parameter | Type | Default | Range |
|-----------|------|---------|-------|
| `echo_bars` | u32 | 2 | 1–4 |
| `echo_feedback` | f32 | 0.6 | 0.3–0.8 |
| `silence_beats` | u32 | 4 | 0–8 |
| `incoming_start_section` | string | "drop" | section name |

---

## Safety Constraints

1. **JSON validation**: Every AI response is validated against the output schema.
   Invalid JSON → reject, use heuristic scorer instead.
2. **Parameter clamping**: All numerical parameters are clamped to template ranges.
3. **Track existence**: Selected track_id must exist in the library.
4. **Tempo guard**: If BPM difference > 8%, warn (in Assisted mode) or use a
   gentler transition (in AutoDJ mode). Never stretch > 10%.
5. **Key clash**: If `allow_key_clash` is false and key distance > 2 on Camelot
   wheel, reject or pick a different candidate.
6. **Vocal clash detection**: If both tracks have high energy in the 200–4000 Hz
   range at the overlap region, prefer a template with bass-swap or filter.
7. **Rate limiting**: AI is called at most once every 10 seconds to prevent
   runaway API costs.
8. **Timeout**: AI calls timeout after 5 seconds. On timeout, fall back to
   heuristic scorer.

---

## Heuristic Scorer (Fallback)

When AI is unavailable, the **deterministic heuristic scorer** selects the next track:

```
score(candidate) =
    w_tempo  * tempo_compatibility(current, candidate)   +
    w_key    * key_compatibility(current, candidate)      +
    w_energy * energy_match(energy_intent, candidate)     +
    w_vocal  * vocal_clash_penalty(current, candidate)    +
    w_variety * recency_penalty(candidate, recent_tracks)
```

### Weights (configurable)

| Weight | Default | Description |
|--------|---------|-------------|
| `w_tempo` | 0.30 | Prefer similar tempo |
| `w_key` | 0.25 | Prefer compatible key |
| `w_energy` | 0.20 | Match energy trajectory |
| `w_vocal` | 0.10 | Penalize vocal clashes |
| `w_variety` | 0.15 | Penalize recently played |

### Compatibility functions

- **Tempo**: `1.0 - abs(bpm_a - bpm_b) / 20.0`, clamped to [0, 1]
- **Key**: Camelot distance ≤ 1 → 1.0; distance 2 → 0.6; distance > 2 → 0.2
- **Energy**: Depends on intent:
  - "maintain" → 1.0 - abs(energy_a - energy_b)
  - "build" → candidate.energy > current.energy ? 1.0 : 0.3
  - "drop" → candidate.energy < current.energy ? 1.0 : 0.3
- **Vocal clash**: If both tracks have vocals in the overlap → 0.0; else 1.0
  (simplified: check if energy in mid-range is high for both)
- **Recency**: 1.0 if not played in last 20 tracks; 0.0 if in last 5; linear between

---

## AI Prompt Template

```
You are an AI DJ conductor. Your job is to select the next track and plan a
transition for a live DJ set.

CURRENT STATE:
{current_state_json}

CANDIDATE TRACKS (pre-scored by compatibility):
{candidates_json}

AVAILABLE TRANSITION TEMPLATES:
{templates_json}

USER PREFERENCES:
{preferences_json}

RULES:
1. You MUST select a track from the candidates list.
2. You MUST use one of the available templates.
3. All parameters MUST be within the documented ranges.
4. Consider the energy trajectory of the set.
5. Avoid key clashes unless the user allows them.
6. Your response MUST be valid JSON matching this schema:
{output_schema_json}

Respond with ONLY the JSON object, no explanation outside it.
```
