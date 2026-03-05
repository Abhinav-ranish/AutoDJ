# Auto-DJ — Tauri UI

## Overview

React SPA in Tauri WebView. Communicates via Tauri commands (UI→Rust) and events (Rust→UI).

---

## Screens

### 1. Main DJ View

Two decks side-by-side with waveforms, EQ/filter/gain knobs, transport controls,
crossfader, and collapsible AI panel showing proposals.

### 2. Library View

Track browser with search, sort by BPM/key/energy, import status indicators,
drag-and-drop import area.

### 3. Settings View

Preferences editor (see `docs/06-preferences.md`).

### 4. Session History

Past mixes with track list and transition details.

---

## State Model (Zustand)

```typescript
interface DJState {
  mode: 'Manual' | 'Assisted' | 'AutoDJ';
  masterBpm: number;
  masterBeatNumber: number;
  decks: { A: DeckState; B: DeckState };
  crossfader: number;
  masterGain: number;
  limiterActive: boolean;
  aiProposal: TransitionProposal | null;
  aiStatus: 'idle' | 'thinking' | 'error' | 'disabled';
  tracks: Track[];
}

interface DeckState {
  trackId: string | null;
  title: string;
  artist: string;
  bpm: number;
  key: string;
  camelot: string;
  state: 'empty' | 'loading' | 'paused' | 'playing' | 'cueing';
  positionMs: number;
  durationMs: number;
  gain: number;
  eq: { low: number; mid: number; high: number };
  filterCutoff: number;
  filterResonance: number;
  activeLoop: { startBeat: number; lengthBars: number } | null;
  cuePoints: (number | null)[];
  ownership: Record<string, 'SYSTEM' | 'AI' | 'USER'>;
}
```

---

## Tauri Bindings

### Commands (UI → Rust)

Deck: `load_track`, `play`, `pause`, `seek`, `set_cue`, `jump_to_cue`, `set_loop`, `clear_loop`
Mixer: `set_crossfader`, `set_deck_gain`, `set_eq`, `set_filter`, `set_master_gain`
Tempo: `set_tempo`, `sync_to_master`
Mode: `set_mode`, `resume_ai`
Library: `import_files`, `get_tracks`, `delete_track`
AI: `accept_proposal`, `deny_proposal`, `request_suggestion`
Prefs: `get_preferences`, `set_preferences`

### Events (Rust → UI)

- `state_update` — partial DJState at 30 Hz
- `track_imported` — import status
- `analysis_complete` — TrackProfile result
- `ai_proposal` — transition proposal from AI
- `error` — error details

---

## Waveform Rendering

Canvas-based. Peaks file fetched on track load, rendered to offscreen canvas once,
visible portion blitted each frame with playhead overlay. Budget: initial render <50ms,
per-frame <2ms.

---

## Theme

Dark professional DJ aesthetic. Deep blacks (#0a0a0f), electric blue (#00d4ff)
and warm amber (#ff9500) accents. Beat-synced pulse on BPM display.
Waveform colored by frequency content.
