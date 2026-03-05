# Auto-DJ — Track Analysis

## Overview

Every imported track is analysed to produce a **TrackProfile** — a JSON document
containing all the musical metadata the engine and AI need to make mixing decisions.
Analysis is performed offline (not in the audio callback) by a pool of worker threads.

---

## TrackProfile Schema

```json
{
  "version": 1,
  "track_id": "trk_abc123",
  "file_hash": "sha256:abcdef...",
  "analysis_timestamp": "2025-03-05T12:00:00Z",

  "tempo": {
    "bpm": 128.0,
    "confidence": 0.92,
    "candidates": [
      { "bpm": 128.0, "confidence": 0.92 },
      { "bpm": 64.0, "confidence": 0.45 },
      { "bpm": 256.0, "confidence": 0.30 }
    ],
    "variable_tempo": false
  },

  "beats": {
    "count": 512,
    "times_ms": [0, 468, 937, 1406, "..."],
    "downbeat_indices": [0, 4, 8, 12, "..."],
    "confidence": 0.88
  },

  "key": {
    "key": "Bm",
    "camelot": "10A",
    "confidence": 0.78,
    "candidates": [
      { "key": "Bm", "camelot": "10A", "confidence": 0.78 },
      { "key": "D",  "camelot": "10B", "confidence": 0.55 }
    ]
  },

  "loudness": {
    "integrated_lufs": -8.2,
    "true_peak_db": -0.3,
    "rms_db": -10.5,
    "dynamic_range_db": 8.1
  },

  "energy": {
    "window_seconds": 0.5,
    "curve": [0.12, 0.15, 0.14, 0.35, 0.72, 0.88, "..."],
    "average": 0.45,
    "peak": 0.95,
    "drop_candidates_ms": [64000, 128000]
  },

  "regions": {
    "intro": { "start_ms": 0, "end_ms": 16000, "confidence": 0.85 },
    "outro": { "start_ms": 224000, "end_ms": 240000, "confidence": 0.80 },
    "safe_loops": [
      { "start_beat": 0,  "length_bars": 8, "quality": 0.90 },
      { "start_beat": 64, "length_bars": 4, "quality": 0.85 }
    ]
  },

  "sections": [
    { "label": "intro",     "start_ms": 0,      "end_ms": 16000,  "confidence": 0.85 },
    { "label": "buildup",   "start_ms": 16000,   "end_ms": 48000,  "confidence": 0.70 },
    { "label": "drop",      "start_ms": 48000,   "end_ms": 80000,  "confidence": 0.88 },
    { "label": "breakdown", "start_ms": 80000,   "end_ms": 112000, "confidence": 0.75 },
    { "label": "drop",      "start_ms": 112000,  "end_ms": 176000, "confidence": 0.85 },
    { "label": "outro",     "start_ms": 176000,  "end_ms": 240000, "confidence": 0.80 }
  ],

  "waveform": {
    "peaks_file": "trk_abc123_peaks.bin",
    "samples_per_pixel": 256,
    "channels": 2
  },

  "duration_ms": 240000,
  "sample_rate": 48000,
  "channels": 2
}
```

---

## Algorithms

### BPM Detection

**Approach**: Onset detection + autocorrelation.

1. Compute spectral flux onset detection function (per 10ms hop).
2. Autocorrelate the onset function over a 4–8 second window.
3. Find peaks in the autocorrelation → candidate tempos.
4. Score candidates by peak height and musical plausibility (prefer 80–160 BPM range).
5. Output primary BPM + confidence + candidate list.

**Fallback**: If confidence < 0.5, try alternative: peak-pick in the onset function
and compute inter-onset intervals → histogram → mode.

**Implementation plan (alpha)**: Use external CLI tool `aubio tempo` if available,
else implement a minimal version in Rust using:
- `rustfft` for spectral analysis
- Custom onset detection function
- Autocorrelation via FFT

### Beat Tracking

**Approach**: Dynamic programming beat tracker.

1. Use the onset detection function from BPM detection.
2. Apply a dynamic programming tracker (similar to Ellis 2007) that finds
   beat positions that:
   - Align with onsets
   - Maintain regular spacing (target BPM)
3. Identify downbeats by finding the strongest periodic accent at the bar level.

**Fallback**: If beat tracking fails, create a synthetic beat grid from BPM
starting at the first detected onset.

### Key Detection

**Approach**: Chromagram + key profile correlation.

1. Compute chromagram (12-bin pitch class profile) using STFT.
2. Correlate with Krumhansl-Kessler key profiles (12 major + 12 minor).
3. Top correlation → key + confidence.
4. Map to Camelot wheel.

**Camelot mapping table**:

| Key | Camelot | Key | Camelot |
|-----|---------|-----|---------|
| Ab minor | 1A | B major | 1B |
| Eb minor | 2A | F# major | 2B |
| Bb minor | 3A | Db major | 3B |
| F minor | 4A | Ab major | 4B |
| C minor | 5A | Eb major | 5B |
| G minor | 6A | Bb major | 6B |
| D minor | 7A | F major | 7B |
| A minor | 8A | C major | 8B |
| E minor | 9A | G major | 9B |
| B minor | 10A | D major | 10B |
| F# minor | 11A | A major | 11B |
| C# minor | 12A | E major | 12B |

### Loudness

- **Integrated LUFS**: ITU-R BS.1770-4 algorithm over full track.
- **True peak**: Oversample 4x, find absolute max.
- **RMS**: Root mean square of all samples.
- **Dynamic range**: Difference between loudest and quietest 1-second windows.

### Energy Curve

1. Divide track into windows (default 0.5s).
2. Per window: RMS energy, weighted by perceptual curve (A-weighting).
3. Normalize to 0.0–1.0 range.
4. **Drop candidates**: Points where energy increases > 2x within 2 seconds
   after a local minimum.

### Section Segmentation

**Approach**: Structural similarity matrix.

1. Compute feature vectors per beat (MFCCs + chromagram + energy).
2. Build self-similarity matrix.
3. Detect boundaries using novelty function (checkerboard kernel convolution).
4. Label sections by energy profile:
   - Low energy, start → "intro"
   - Rising energy → "buildup"
   - High energy → "drop"
   - Dropping energy → "breakdown"
   - Low energy, end → "outro"

**Fallback**: If segmentation confidence low, divide into equal thirds
(intro / body / outro).

### Safe Loop Regions

1. For each downbeat, score a loop of N bars (N ∈ {1, 2, 4, 8}):
   - Spectral similarity between start and end (should be high).
   - No major transient at the loop boundary.
   - Energy stability (low variance within the loop).
2. Return top 4 loop regions.

---

## Confidence Rules

| Analysis | High (≥0.8) | Medium (0.5–0.8) | Low (<0.5) |
|----------|------------|-------------------|------------|
| BPM | Use directly | Use with ±1 BPM tolerance | Fall back to 120 BPM |
| Key | Use for matching | Use but allow wider compatibility | Ignore key matching |
| Beats | Use beat grid | Use but mark "approximate" | Synthetic grid from BPM |
| Sections | Use for transition planning | Use boundaries, generic labels | Divide into thirds |

---

## Analysis Pipeline

```
Track imported
    │
    ▼
Decode to PCM (if not cached)
    │
    ▼ (rayon worker)
┌───────────────────┐
│ 1. BPM detection   │
│ 2. Beat tracking    │  (depends on BPM)
│ 3. Key detection    │  (independent)
│ 4. Loudness         │  (independent)
│ 5. Energy curve     │  (independent)
│ 6. Segmentation     │  (depends on beats + energy)
│ 7. Safe loops       │  (depends on beats)
│ 8. Waveform peaks   │  (independent)
└───────────┬─────────┘
            │
            ▼
Write track_profile.json
Update SQLite row (status = analyzed)
Emit event to UI
```

Steps 1, 3, 4, 5, 8 run concurrently. Steps 2, 6, 7 depend on step 1.
