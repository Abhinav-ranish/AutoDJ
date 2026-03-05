# Auto-DJ — Storage

## Overview

Storage is split into two layers:

1. **SQLite database** — metadata, preferences, analysis results, state
2. **File cache** — decoded PCM, waveform peaks, original audio files

---

## SQLite Schema

Database file: `~/.autodj/autodj.db`

### tracks

```sql
CREATE TABLE tracks (
    id            TEXT PRIMARY KEY,         -- "trk_" + 12-char random
    file_hash     TEXT NOT NULL UNIQUE,     -- SHA-256 of original file
    original_path TEXT NOT NULL,            -- where the file was imported from
    title         TEXT,
    artist        TEXT,
    album         TEXT,
    genre         TEXT,
    duration_ms   INTEGER,
    file_size     INTEGER,
    mime_type     TEXT,
    import_status TEXT NOT NULL DEFAULT 'pending',
    -- 'pending' → 'decoding' → 'decoded' → 'analyzing' → 'ready' → 'error'
    error_message TEXT,
    created_at    INTEGER NOT NULL,         -- unix timestamp
    updated_at    INTEGER NOT NULL
);

CREATE INDEX idx_tracks_hash ON tracks(file_hash);
CREATE INDEX idx_tracks_status ON tracks(import_status);
```

### track_profiles

```sql
CREATE TABLE track_profiles (
    track_id       TEXT PRIMARY KEY REFERENCES tracks(id),
    profile_json   TEXT NOT NULL,           -- full TrackProfile JSON
    bpm            REAL,                    -- denormalized for queries
    bpm_confidence REAL,
    key_name       TEXT,                    -- e.g., "Bm"
    camelot        TEXT,                    -- e.g., "10A"
    key_confidence REAL,
    lufs           REAL,
    energy_avg     REAL,
    analyzed_at    INTEGER NOT NULL
);

CREATE INDEX idx_profiles_bpm ON track_profiles(bpm);
CREATE INDEX idx_profiles_key ON track_profiles(camelot);
CREATE INDEX idx_profiles_energy ON track_profiles(energy_avg);
```

### playlists

```sql
CREATE TABLE playlists (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE playlist_tracks (
    playlist_id TEXT REFERENCES playlists(id),
    track_id    TEXT REFERENCES tracks(id),
    position    INTEGER NOT NULL,
    PRIMARY KEY (playlist_id, track_id)
);
```

### mix_history

```sql
CREATE TABLE mix_history (
    id              TEXT PRIMARY KEY,
    track_id        TEXT REFERENCES tracks(id),
    started_at      INTEGER NOT NULL,
    ended_at        INTEGER,
    transition_type TEXT,
    transition_json TEXT,                   -- parameters used
    ai_decision_json TEXT,                  -- AI's reasoning (if any)
    session_id      TEXT NOT NULL
);

CREATE INDEX idx_history_session ON mix_history(session_id);
CREATE INDEX idx_history_track ON mix_history(track_id);
```

### preferences

```sql
CREATE TABLE preferences (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
```

### sessions

```sql
CREATE TABLE sessions (
    id         TEXT PRIMARY KEY,
    started_at INTEGER NOT NULL,
    ended_at   INTEGER,
    track_count INTEGER DEFAULT 0,
    total_duration_ms INTEGER DEFAULT 0
);
```

---

## File Cache Layout

```
~/.autodj/
├── autodj.db                          (SQLite database)
├── cache/
│   ├── pcm/
│   │   ├── trk_abc123.pcm            (48kHz stereo f32 LE, raw)
│   │   └── trk_def456.pcm
│   ├── peaks/
│   │   ├── trk_abc123_peaks.bin       (pre-computed waveform peaks)
│   │   └── trk_def456_peaks.bin
│   └── stretched/
│       └── trk_abc123_128bpm.pcm      (pre-stretched to target BPM)
├── originals/
│   ├── trk_abc123.mp3                 (copy of imported file)
│   └── trk_def456.flac
└── logs/
    └── autodj.log
```

---

## Hashing

- Algorithm: SHA-256
- Input: first 1 MB + last 1 MB + file size (for speed on large files)
- Purpose: deduplication (same file imported from different paths)
- If hash collision is detected, skip import and show "already imported" in UI

---

## PCM Cache Format

Raw PCM files with a minimal header:

```
Bytes 0–3:   Magic: "ADJP" (AutoDJ PCM)
Bytes 4–7:   Version: u32 LE (1)
Bytes 8–11:  Sample rate: u32 LE (48000)
Bytes 12–13: Channels: u16 LE (2)
Bytes 14–15: Bit depth: u16 LE (32, for f32)
Bytes 16–23: Total samples: u64 LE (per channel)
Bytes 24–31: Duration ms: u64 LE
Bytes 32+:   Raw f32 LE interleaved sample data
```

### Reading in the audio callback

The PCM file is memory-mapped (`mmap`) at load time. The audio callback reads
directly from the mapped region — zero copy, zero allocation.

---

## Waveform Peaks Format

Binary file with peaks pre-computed at a fixed resolution:

```
Bytes 0–3:    Magic: "ADJW" (AutoDJ Waveform)
Bytes 4–7:    Version: u32 LE (1)
Bytes 8–11:   Samples per pixel: u32 LE (256)
Bytes 12–13:  Channels: u16 LE (2)
Bytes 14–17:  Total pixels: u32 LE
Bytes 18+:    For each pixel: min_l: f32, max_l: f32, min_r: f32, max_r: f32
```

The UI fetches this file and renders peaks directly to a Canvas element.
At 48 kHz / 256 samples per pixel = ~187 pixels per second → a 4-minute track
≈ 45,000 pixels → ~720 KB per track. Acceptable.

---

## Decode Pipeline

```
Original file (MP3/FLAC/WAV/AAC/OGG)
    │
    ▼ spawn ffmpeg
ffmpeg -i input.mp3 -f f32le -ar 48000 -ac 2 -
    │
    ▼ pipe stdout
Read into memory buffer
    │
    ▼ write
PCM cache file (with header)
    │
    ▼ update
SQLite: set import_status = 'decoded'
```

### FFmpeg command

```bash
ffmpeg -i "<input_path>" \
       -f f32le \
       -acodec pcm_f32le \
       -ar 48000 \
       -ac 2 \
       - \
       2>/dev/null
```

Output is piped to stdout and read in chunks. The decoder thread writes the
ADJP header first, then appends sample data. On completion, it updates the
total_samples field in the header.
