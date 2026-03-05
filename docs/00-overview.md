# Auto-DJ — Overview

## Product Goals

Auto-DJ is a cross-platform desktop application that plays the role of an intelligent DJ.
It manages a music library, analyses tracks, beat-matches and mixes them on two virtual decks,
and optionally lets an AI "conductor" drive the set — while the human user can grab control
at any moment.

### Target audience (alpha)

A handful of friends who want continuous, well-mixed music at parties or work sessions
without babysitting a playlist. Licensing is ignored for this alpha.

### Core value props

1. **Always-on safety** — a limiter, silence detection, and fallback ladder guarantee
   that *something reasonable* is always playing, even if the AI errors out.
2. **Musical correctness** — transitions are quantised to beats/bars; tempo and key
   compatibility are enforced.
3. **Three modes on a spectrum** — from fully manual to fully autonomous, switchable
   at any time.

---

## Constraints

| Constraint | Detail |
|---|---|
| Real-time thread is sacred | No I/O, no allocations, no locks, no LLM calls in the audio callback. |
| Deterministic scheduling | Every command is quantised to the next beat, bar, or downbeat. |
| AI as planner | The LLM emits JSON command plans; it never touches DSP. |
| Canonical PCM | All audio is decoded to 48 kHz / stereo / f32 before playback. |
| Offline-first | The app works fully without an internet connection (AI falls back to heuristic scorer). |

---

## Modes

### 1. Manual
- User loads tracks, triggers play, adjusts EQ/filter/crossfader.
- AI may offer non-intrusive suggestions (e.g., "Track X matches well").
- User ignores or accepts suggestions at will.

### 2. Assisted
- AI proposes a next-track and a transition plan.
- UI shows the proposal; user accepts, modifies, or rejects.
- If user doesn't respond within a configurable window the proposal auto-expires
  (does **not** auto-execute — silence is safer than a bad transition).

### 3. AutoDJ
- AI selects tracks and executes transitions autonomously.
- User sees real-time preview of upcoming plan.
- **Grab control**: touching any parameter (crossfader, EQ, filter, tempo) instantly
  transfers ownership to the user for a configurable duration (default: 8 bars).
  After that, AI resumes unless the user explicitly holds.

---

## UX Principles

1. **Glanceable** — current BPM, key, energy, and mode visible at all times.
2. **One-click override** — any AI action can be cancelled with a single click or
   by touching the relevant control.
3. **Transparent AI** — the AI's reasoning (selected template, score breakdown) is
   shown in a collapsible panel so the user understands *why*.
4. **No surprise silence** — the fallback ladder ensures continuous playback.
5. **Low latency UI** — state snapshots from the engine are pushed at ~30 fps;
   waveform rendering uses pre-computed peaks.

---

## Assumptions (alpha)

- Users have a local collection of audio files (MP3, FLAC, WAV, AAC, OGG).
- The machine has ≥ 4 cores and ≥ 8 GB RAM.
- An `ffmpeg` binary is either bundled or available on `$PATH`.
- For AI features an OpenAI-compatible API key is configured (optional).
