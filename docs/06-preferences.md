# Auto-DJ — Preferences

## Overview

User preferences control the AI conductor's behaviour, ownership duration, and mix style.
They are stored in SQLite and editable from the UI settings panel.

---

## Preferences Schema

```json
{
  "version": 1,
  "user_id": "default",

  "mode": {
    "current": "Assisted",
    "default_on_start": "Manual"
  },

  "ownership": {
    "hold_duration_bars": 8,
    "auto_resume_ai": true,
    "resume_delay_bars": 2
  },

  "mixing": {
    "style": "smooth",
    "default_transition_bars": 16,
    "max_tempo_shift_percent": 8,
    "allow_key_clash": false,
    "crossfader_curve": "equal_power"
  },

  "energy": {
    "preference": "gradual_build",
    "min_energy": 0.2,
    "max_energy": 1.0,
    "target_set_arc": "build_peak_cool"
  },

  "library": {
    "auto_analyze_on_import": true,
    "preferred_genres": [],
    "blocked_tracks": []
  },

  "audio": {
    "output_device": "default",
    "sample_rate": 48000,
    "buffer_size": 256,
    "master_limiter_ceiling_db": -0.3
  },

  "ai": {
    "enabled": true,
    "api_endpoint": "https://api.openai.com/v1/chat/completions",
    "model": "gpt-4o-mini",
    "api_key_env": "AUTODJ_AI_API_KEY",
    "timeout_seconds": 5,
    "max_calls_per_minute": 6,
    "temperature": 0.3
  },

  "ui": {
    "waveform_color_mode": "frequency",
    "show_ai_reasoning": true,
    "show_beat_markers": true,
    "theme": "dark"
  }
}
```

---

## Mix Style Options

| Style | Description | Default transition | Behaviour |
|-------|-------------|-------------------|-----------|
| `smooth` | Long, seamless transitions | 16-bar beatmix | Prefers bass-swap, equal-power crossfade |
| `punchy` | Shorter, impactful transitions | 8-bar filter sweep or echo-drop | Prefers filter sweeps, harder cuts |
| `minimal` | Clean, simple transitions | 8-bar crossfade | Avoids complex templates |
| `creative` | Varied, uses all templates | Varies | AI picks freely from all templates |

---

## Energy Trajectory Options

| Trajectory | Description |
|-----------|-------------|
| `gradual_build` | Slowly increase energy over the set |
| `constant` | Maintain steady energy |
| `wave` | Alternate high/low energy in waves |
| `build_peak_cool` | Build → peak → cool down (classic DJ arc) |
| `random` | No energy constraints |

---

## Parameter Ownership Duration

The `hold_duration_bars` setting controls how long a user retains ownership of a
parameter after touching it:

- **Default**: 8 bars (~15 seconds at 128 BPM)
- **Range**: 1–64 bars
- **Resume delay**: After ownership expires, wait `resume_delay_bars` before AI
  resumes control (gives user time to react)

### Override behaviour by mode

| Mode | User touches a control | After hold expires |
|------|----------------------|-------------------|
| Manual | Normal control, no AI | Stays with user |
| Assisted | AI pauses proposals for that param | AI resumes proposals |
| AutoDJ | AI stops controlling that param | AI resumes control |

---

## Persistence

Preferences are stored in the `preferences` table in SQLite:

```sql
CREATE TABLE preferences (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL,  -- JSON encoded
    updated_at INTEGER NOT NULL
);
```

On startup, missing keys are filled with defaults. Schema version is checked;
if the stored version is older, a migration runs.
