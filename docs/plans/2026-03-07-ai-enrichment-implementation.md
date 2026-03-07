# AI Enrichment & Next Phases Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix all warnings, enrich song data with DSP features + Gemini classification, overhaul the AI decision engine, add mix history, audio export, and visual polish.

**Architecture:** Sequential phases building on each other. Phase 11 cleans up warnings, Phase 12 adds DSP analysis modules and DB columns, Phase 13 adds Gemini-powered classification, Phase 14 redesigns the AI context/scorer/planner to use all new data.

**Tech Stack:** Rust (rustfft, rayon, rusqlite, reqwest), React/TypeScript (Zustand), Tauri v2, Google Gemini API

---

## Phase 11: Fix All Warnings & Code Quality

### Task 1: Fix EQ Clippy Errors

**Files:**
- Modify: `crates/autodj-core/src/eq.rs:49,63`

**Step 1: Fix the two approximate constant warnings**

Replace the manual `0.7071` with the standard library constant:

```rust
// In set_lowpass (line 49), change:
let alpha = sin_w0 / (2.0 * 0.7071);
// To:
let alpha = sin_w0 / (2.0 * std::f32::consts::FRAC_1_SQRT_2);

// In set_highpass (line 63), change:
let alpha = sin_w0 / (2.0 * 0.7071);
// To:
let alpha = sin_w0 / (2.0 * std::f32::consts::FRAC_1_SQRT_2);
```

**Step 2: Verify compilation**

Run: `cargo clippy -p autodj-core 2>&1 | head -30`
Expected: No errors from eq.rs

**Step 3: Commit**

```bash
git add crates/autodj-core/src/eq.rs
git commit -m "fix: use FRAC_1_SQRT_2 constant in EQ biquad filters"
```

---

### Task 2: Fix Unused Imports and Variables

**Files:**
- Modify: `crates/autodj-analysis/src/pipeline.rs:9`
- Modify: `crates/autodj-analysis/src/key.rs:110`
- Modify: `crates/autodj-core/src/engine.rs:356`
- Modify: `crates/autodj-app/src/commands.rs:13`
- Modify: `crates/autodj-ai/src/camelot.rs:3-6`

**Step 1: Fix pipeline.rs — remove unused `RegionInfo` import**

```rust
// Line 9, change:
use crate::profile::{RegionInfo, TrackProfile};
// To:
use crate::profile::TrackProfile;
```

Note: `RegionInfo` is used directly as `crate::profile::RegionInfo` on line 102, so removing the import is safe.

**Step 2: Fix key.rs — prefix unused variable `base`**

```rust
// Line 110, change:
let (base, _) = mapping.iter().find(|(k, _)| *k == key).unwrap_or(&("C", "8"));
// To:
let (_base, _) = mapping.iter().find(|(k, _)| *k == key).unwrap_or(&("C", "8"));
```

**Step 3: Fix engine.rs — prefix unused parameter `sample_rate`**

```rust
// Line 356, change:
sample_rate: u32,
// To:
_sample_rate: u32,
```

**Step 4: Fix commands.rs — remove unused `AudioFile` import**

```rust
// Line 13, remove entirely:
use lofty::file::AudioFile;
```

**Step 5: Fix camelot.rs — remove unused `CAMELOT_KEYS` constant**

```rust
// Lines 3-6, remove entirely:
const CAMELOT_KEYS: [&str; 24] = [
    "1A", "1B", "2A", "2B", "3A", "3B", "4A", "4B", "5A", "5B", "6A", "6B",
    "7A", "7B", "8A", "8B", "9A", "9B", "10A", "10B", "11A", "11B", "12A", "12B",
];
```

**Step 6: Verify**

Run: `cargo check --workspace 2>&1 | grep warning`
Expected: Only the remaining clippy-style warnings (range checks, etc.)

**Step 7: Commit**

```bash
git add crates/autodj-analysis/src/pipeline.rs crates/autodj-analysis/src/key.rs crates/autodj-core/src/engine.rs crates/autodj-app/src/commands.rs crates/autodj-ai/src/camelot.rs
git commit -m "fix: remove unused imports, variables, and constants"
```

---

### Task 3: Fix Clippy Style Warnings

**Files:**
- Modify: `crates/autodj-analysis/src/perceived_tempo.rs:133-188`
- Modify: `crates/autodj-analysis/src/bpm.rs:52,74-75`
- Modify: `crates/autodj-analysis/src/key.rs:40,68`
- Modify: `crates/autodj-analysis/src/loudness.rs:38-39`
- Modify: `crates/autodj-core/src/deck.rs:181`
- Modify: `crates/autodj-core/src/transport.rs:60`
- Modify: `crates/autodj-core/src/engine.rs:314`

**Step 1: Fix perceived_tempo.rs — use `.contains()` for 8 range checks**

Replace each manual range check with the idiomatic `.contains()` pattern:

```rust
// Line 133: if bpm >= 80.0 && bpm <= 115.0 {
if (80.0..=115.0).contains(&bpm) {

// Line 146: if bpm >= 120.0 && bpm <= 170.0 && sparsity_ratio > 0.15 {
if (120.0..=170.0).contains(&bpm) && sparsity_ratio > 0.15 {

// Line 152: if bpm >= 80.0 && bpm < 100.0 && ...
if (80.0..100.0).contains(&bpm) && avg_onsets < 2.0 && sparsity_ratio < 0.15 {

// Line 158: if bpm >= 135.0 && bpm <= 155.0 && avg_onsets > 3.0 {
if (135.0..=155.0).contains(&bpm) && avg_onsets > 3.0 {

// Line 164: if bpm >= 130.0 && bpm <= 145.0 && ...
if (130.0..=145.0).contains(&bpm) && avg_onsets > 2.5 && onset_variance > 1.0 {

// Line 170: if bpm >= 118.0 && bpm <= 135.0 && ...
if (118.0..=135.0).contains(&bpm) && energy_variance < 0.002 && sparsity_ratio < 0.05 {

// Line 176: if bpm >= 130.0 && bpm <= 155.0 && ...
if (130.0..=155.0).contains(&bpm) && energy_variance < 0.003 && sparsity_ratio < 0.08 {

// Line 188: if bpm >= 160.0 && bpm <= 185.0 {
if (160.0..=185.0).contains(&bpm) {
```

**Step 2: Fix bpm.rs — use `*=` and collapse nested if**

```rust
// Line 52, change:
*c = *c * c.conj();
// To:
*c *= c.conj();

// Lines 74-75, collapse nested if:
if lag > min_lag && lag < max_lag
    && score > signal[lag - 1].re && score > signal[lag + 1].re
{
```

**Step 3: Fix key.rs — use `.contains()` and `enumerate()`**

```rust
// Line 40, change:
if freq < 20.0 || freq > 5000.0 {
// To:
if !(20.0..=5000.0).contains(&freq) {

// Line 68, change:
for root in 0..12 {
// To:
for (root, _) in notes.iter().enumerate() {
```

**Step 4: Fix loudness.rs — use proper counter**

```rust
// Lines 37-39, change the chunks iterator and manual count variable.
// Replace:
let chunks = samples.chunks(window_samples);
let mut count = 0;
for chunk in chunks {
    // ... body ...
    count += 1;
}
// With:
for (count, chunk) in samples.chunks(window_samples).enumerate() {
    // ... body unchanged, just remove the `count += 1` line ...
}
```

**Step 5: Fix deck.rs — remove unnecessary cast**

```rust
// Line 181, change:
self.position as f64 / t.total_frames() as f64
// To (position is already f64):
self.position / t.total_frames() as f64
```

Note: Check the type of `self.position` first — if it's `f64`, remove the cast. If it's `usize`, keep it.

**Step 6: Fix transport.rs and engine.rs — modulo operations**

These use `% ==` which is standard Rust. The `is_multiple_of()` method is nightly-only as of Rust 1.83, so we should just suppress the warning or leave as-is. Add `#[allow(clippy::manual_is_multiple_of)]` if clippy fires on them.

**Step 7: Verify zero warnings**

Run: `cargo clippy --workspace 2>&1`
Expected: Zero warnings

**Step 8: Commit**

```bash
git add -A
git commit -m "fix: resolve all clippy warnings across workspace"
```

---

### Task 4: Refactor `save_track_profile` to Use a Struct

**Files:**
- Modify: `crates/autodj-storage/src/db.rs:294-344`
- Modify: `crates/autodj-app/src/commands.rs` (caller)

**Step 1: Create a `ProfileData` struct in db.rs**

Add before `save_track_profile`:

```rust
/// Data needed to save a track profile to the database.
pub struct ProfileData<'a> {
    pub track_id: &'a str,
    pub profile_json: &'a str,
    pub bpm: f64,
    pub bpm_confidence: f64,
    pub perceived_tempo: f64,
    pub groove_type: &'a str,
    pub key_name: &'a str,
    pub camelot: &'a str,
    pub key_confidence: f64,
    pub lufs: f64,
    pub energy_avg: f64,
}
```

**Step 2: Update `save_track_profile` to accept `ProfileData`**

```rust
pub fn save_track_profile(&self, data: &ProfileData) -> Result<(), DbError> {
    // Same body, but use data.track_id, data.profile_json, etc.
}
```

**Step 3: Update the caller in commands.rs**

Find the call to `save_track_profile` and update it to construct a `ProfileData` struct.

**Step 4: Verify**

Run: `cargo clippy --workspace 2>&1`
Expected: No "too many arguments" warning

**Step 5: Commit**

```bash
git add crates/autodj-storage/src/db.rs crates/autodj-app/src/commands.rs
git commit -m "refactor: use ProfileData struct for save_track_profile"
```

---

## Phase 12: Database Schema Expansion & DSP Enrichment

### Task 5: Add New DB Columns via Migration

**Files:**
- Modify: `crates/autodj-storage/src/db.rs`

**Step 1: Add migration SQL to `init_schema`**

In the `migrations` array (line 177), add new column migrations:

```rust
let migrations = [
    "ALTER TABLE track_profiles ADD COLUMN perceived_tempo REAL",
    "ALTER TABLE track_profiles ADD COLUMN groove_type TEXT",
    // New Phase 12 columns
    "ALTER TABLE track_profiles ADD COLUMN spectral_centroid REAL",
    "ALTER TABLE track_profiles ADD COLUMN spectral_flatness REAL",
    "ALTER TABLE track_profiles ADD COLUMN bass_energy REAL",
    "ALTER TABLE track_profiles ADD COLUMN mid_energy REAL",
    "ALTER TABLE track_profiles ADD COLUMN high_energy REAL",
    "ALTER TABLE track_profiles ADD COLUMN vocal_presence REAL",
    "ALTER TABLE track_profiles ADD COLUMN rhythm_complexity REAL",
    "ALTER TABLE track_profiles ADD COLUMN danceability REAL",
    "ALTER TABLE track_profiles ADD COLUMN energy_trajectory TEXT",
    "ALTER TABLE track_profiles ADD COLUMN intro_length_ms INTEGER",
    "ALTER TABLE track_profiles ADD COLUMN outro_length_ms INTEGER",
    "ALTER TABLE track_profiles ADD COLUMN first_drop_ms INTEGER",
];
```

**Step 2: Update `ProfileData` struct to include new fields**

```rust
pub struct ProfileData<'a> {
    // ... existing fields ...
    pub spectral_centroid: Option<f64>,
    pub spectral_flatness: Option<f64>,
    pub bass_energy: Option<f64>,
    pub mid_energy: Option<f64>,
    pub high_energy: Option<f64>,
    pub vocal_presence: Option<f64>,
    pub rhythm_complexity: Option<f64>,
    pub danceability: Option<f64>,
    pub energy_trajectory: Option<&'a str>,
    pub intro_length_ms: Option<i64>,
    pub outro_length_ms: Option<i64>,
    pub first_drop_ms: Option<i64>,
}
```

**Step 3: Update `save_track_profile` SQL to insert new columns**

Extend the INSERT/UPSERT query to include all new columns.

**Step 4: Add new columns to `TrackRecord` and `get_all_tracks` query**

Add `genre`, `mood`, `vibe` to `TrackRecord` struct (will be NULL until Phase 13).

**Step 5: Verify**

Run: `cargo test -p autodj-storage`
Expected: All tests pass

**Step 6: Commit**

```bash
git add crates/autodj-storage/src/db.rs
git commit -m "feat(storage): add DSP enrichment columns to track_profiles schema"
```

---

### Task 6: Create Spectral Analysis Module

**Files:**
- Create: `crates/autodj-analysis/src/spectral.rs`
- Modify: `crates/autodj-analysis/src/lib.rs`

**Step 1: Write tests first**

```rust
// crates/autodj-analysis/src/spectral.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_centroid_pure_sine() {
        // A 1kHz sine wave should have centroid near 1000 Hz
        let sr = 48000;
        let samples: Vec<f32> = (0..sr * 2) // 1 second stereo
            .map(|i| {
                let t = (i / 2) as f32 / sr as f32;
                (2.0 * std::f32::consts::PI * 1000.0 * t).sin()
            })
            .collect();
        let result = analyze_spectral(&samples, sr);
        assert!((result.centroid - 1000.0).abs() < 200.0,
            "Expected centroid near 1000 Hz, got {}", result.centroid);
    }

    #[test]
    fn test_spectral_flatness_tonal_vs_noise() {
        let sr = 48000;
        // Pure sine = very tonal = low flatness
        let sine: Vec<f32> = (0..sr * 2)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * (i / 2) as f32 / sr as f32).sin())
            .collect();
        let sine_result = analyze_spectral(&sine, sr);

        // White noise = high flatness
        let noise: Vec<f32> = (0..sr * 2)
            .map(|_| (rand_simple() * 2.0 - 1.0) as f32)
            .collect();
        let noise_result = analyze_spectral(&noise, sr);

        assert!(sine_result.flatness < noise_result.flatness,
            "Sine flatness {} should be less than noise flatness {}",
            sine_result.flatness, noise_result.flatness);
    }

    #[test]
    fn test_band_energy_distribution() {
        let sr = 48000;
        // Low frequency signal (100 Hz)
        let low: Vec<f32> = (0..sr * 2)
            .map(|i| (2.0 * std::f32::consts::PI * 100.0 * (i / 2) as f32 / sr as f32).sin())
            .collect();
        let result = analyze_spectral(&low, sr);
        assert!(result.bass_energy > result.high_energy,
            "100 Hz signal should have more bass than treble");
    }

    fn rand_simple() -> f64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let val = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut hasher = DefaultHasher::new();
        val.hash(&mut hasher);
        (hasher.finish() as f64) / u64::MAX as f64
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p autodj-analysis spectral`
Expected: FAIL — functions not defined

**Step 3: Implement spectral analysis**

```rust
//! Spectral analysis: centroid, flatness, and band energy distribution.

use rustfft::{num_complex::Complex, FftPlanner};

pub struct SpectralInfo {
    pub centroid: f64,
    pub flatness: f64,
    pub bass_energy: f64,
    pub mid_energy: f64,
    pub high_energy: f64,
}

pub fn analyze_spectral(samples: &[f32], sample_rate: u32) -> SpectralInfo {
    if samples.is_empty() {
        return SpectralInfo { centroid: 0.0, flatness: 0.0, bass_energy: 0.0, mid_energy: 0.0, high_energy: 0.0 };
    }

    let window_size = 4096;
    let hop_size = 2048;
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(window_size);

    let mut total_centroid = 0.0;
    let mut total_flatness = 0.0;
    let mut total_bass = 0.0;
    let mut total_mid = 0.0;
    let mut total_high = 0.0;
    let mut num_windows = 0u64;

    // Analyze across the full track
    for i in (0..samples.len().saturating_sub(window_size * 2)).step_by(hop_size * 2) {
        let mut buffer: Vec<Complex<f32>> = (0..window_size)
            .map(|j| {
                let idx = i + j * 2;
                if idx < samples.len() {
                    Complex::new(samples[idx], 0.0)
                } else {
                    Complex::new(0.0, 0.0)
                }
            })
            .collect();

        fft.process(&mut buffer);

        let half = window_size / 2;
        let mut weighted_sum = 0.0;
        let mut magnitude_sum = 0.0;
        let mut log_sum = 0.0;
        let mut bass = 0.0;
        let mut mid = 0.0;
        let mut high = 0.0;
        let mut bin_count = 0u32;

        for (bin, c) in buffer.iter().enumerate().take(half) {
            let freq = bin as f64 * sample_rate as f64 / window_size as f64;
            if freq < 20.0 { continue; }
            let mag = c.norm() as f64;

            weighted_sum += freq * mag;
            magnitude_sum += mag;
            log_sum += (mag + 1e-10).ln();
            bin_count += 1;

            // Band energy distribution
            if freq <= 250.0 {
                bass += mag * mag;
            } else if freq <= 4000.0 {
                mid += mag * mag;
            } else {
                high += mag * mag;
            }
        }

        if magnitude_sum > 1e-10 {
            total_centroid += weighted_sum / magnitude_sum;
        }

        if bin_count > 0 {
            let geometric_mean = (log_sum / bin_count as f64).exp();
            let arithmetic_mean = magnitude_sum / bin_count as f64;
            if arithmetic_mean > 1e-10 {
                total_flatness += geometric_mean / arithmetic_mean;
            }
        }

        let band_total = bass + mid + high;
        if band_total > 1e-10 {
            total_bass += bass / band_total;
            total_mid += mid / band_total;
            total_high += high / band_total;
        }

        num_windows += 1;
    }

    let n = num_windows.max(1) as f64;
    SpectralInfo {
        centroid: total_centroid / n,
        flatness: (total_flatness / n).clamp(0.0, 1.0),
        bass_energy: total_bass / n,
        mid_energy: total_mid / n,
        high_energy: total_high / n,
    }
}
```

**Step 4: Add module to lib.rs**

```rust
pub mod spectral;
```

**Step 5: Run tests**

Run: `cargo test -p autodj-analysis spectral`
Expected: PASS

**Step 6: Commit**

```bash
git add crates/autodj-analysis/src/spectral.rs crates/autodj-analysis/src/lib.rs
git commit -m "feat(analysis): add spectral analysis module (centroid, flatness, band energy)"
```

---

### Task 7: Create Vocal Detection Module

**Files:**
- Create: `crates/autodj-analysis/src/vocal_detection.rs`
- Modify: `crates/autodj-analysis/src/lib.rs`

**Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vocal_presence_silence() {
        let samples = vec![0.0f32; 48000 * 2];
        let result = detect_vocal_presence(&samples, 48000);
        assert!(result < 0.1, "Silence should have near-zero vocal presence");
    }

    #[test]
    fn test_vocal_presence_returns_bounded_value() {
        let sr = 48000;
        let samples: Vec<f32> = (0..sr * 4) // 2 sec stereo
            .map(|i| (2.0 * std::f32::consts::PI * 800.0 * (i / 2) as f32 / sr as f32).sin())
            .collect();
        let result = detect_vocal_presence(&samples, sr);
        assert!((0.0..=1.0).contains(&result), "Result should be 0.0-1.0, got {}", result);
    }
}
```

**Step 2: Run tests to verify failure**

Run: `cargo test -p autodj-analysis vocal`
Expected: FAIL

**Step 3: Implement vocal detection**

The approach: measure the ratio of spectral energy in the vocal frequency range (300 Hz - 3.5 kHz) vs the total spectrum, combined with harmonic prominence analysis.

```rust
//! Vocal presence detection using spectral analysis in the vocal frequency range.

use rustfft::{num_complex::Complex, FftPlanner};

/// Detect vocal presence in the track. Returns 0.0 (instrumental) to 1.0 (heavily vocal).
pub fn detect_vocal_presence(samples: &[f32], sample_rate: u32) -> f64 {
    if samples.is_empty() { return 0.0; }

    let window_size = 4096;
    let hop_size = 2048;
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(window_size);

    let mut vocal_energy_sum = 0.0;
    let mut total_energy_sum = 0.0;
    let mut harmonic_ratio_sum = 0.0;
    let mut num_windows = 0u64;

    for i in (0..samples.len().saturating_sub(window_size * 2)).step_by(hop_size * 2) {
        let mut buffer: Vec<Complex<f32>> = (0..window_size)
            .map(|j| {
                let idx = i + j * 2;
                Complex::new(if idx < samples.len() { samples[idx] } else { 0.0 }, 0.0)
            })
            .collect();

        fft.process(&mut buffer);

        let half = window_size / 2;
        let mut vocal_energy = 0.0;
        let mut total_energy = 0.0;
        let mut peak_mags: Vec<f64> = Vec::new();

        for (bin, c) in buffer.iter().enumerate().take(half) {
            let freq = bin as f64 * sample_rate as f64 / window_size as f64;
            let mag = c.norm() as f64;
            total_energy += mag * mag;

            // Vocal range: 300 Hz - 3500 Hz
            if (300.0..=3500.0).contains(&freq) {
                vocal_energy += mag * mag;
                peak_mags.push(mag);
            }
        }

        if total_energy > 1e-10 {
            vocal_energy_sum += vocal_energy / total_energy;
        }

        // Harmonic ratio: how peaked (harmonic) vs flat (noise) the vocal band is
        if !peak_mags.is_empty() {
            peak_mags.sort_by(|a, b| b.partial_cmp(a).unwrap());
            let top_3: f64 = peak_mags.iter().take(3).sum();
            let total: f64 = peak_mags.iter().sum();
            if total > 1e-10 {
                harmonic_ratio_sum += top_3 / total;
            }
        }

        total_energy_sum += total_energy;
        num_windows += 1;
    }

    let n = num_windows.max(1) as f64;
    let avg_vocal_ratio = vocal_energy_sum / n;
    let avg_harmonic_ratio = harmonic_ratio_sum / n;

    // Combine vocal band energy ratio with harmonic prominence
    // High vocal ratio + high harmonic ratio = strong vocal presence
    let vocal_score = (avg_vocal_ratio * 0.6 + avg_harmonic_ratio * 0.4).clamp(0.0, 1.0);
    vocal_score
}
```

**Step 4: Add module and run tests**

Run: `cargo test -p autodj-analysis vocal`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/autodj-analysis/src/vocal_detection.rs crates/autodj-analysis/src/lib.rs
git commit -m "feat(analysis): add vocal presence detection module"
```

---

### Task 8: Create Rhythm Analysis Module

**Files:**
- Create: `crates/autodj-analysis/src/rhythm.rs`
- Modify: `crates/autodj-analysis/src/lib.rs`

**Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regular_beats_high_danceability() {
        // Perfect kick pattern = high danceability
        let sr = 48000;
        let mut samples = vec![0.0f32; sr as usize * 4]; // 2 sec stereo
        let beat_interval = (sr as f64 * 0.5) as usize; // 120 BPM
        for i in (0..samples.len() / 2).step_by(beat_interval) {
            for j in 0..50 {
                let idx = (i + j) * 2;
                if idx + 1 < samples.len() {
                    samples[idx] = 0.9;
                    samples[idx + 1] = 0.9;
                }
            }
        }
        let result = analyze_rhythm(&samples, sr, 120.0);
        assert!(result.danceability > 0.5, "Regular beats should be danceable, got {}", result.danceability);
    }

    #[test]
    fn test_silence_low_danceability() {
        let samples = vec![0.0f32; 48000 * 4];
        let result = analyze_rhythm(&samples, 48000, 120.0);
        assert!(result.danceability < 0.3, "Silence should have low danceability");
    }
}
```

**Step 2: Implement rhythm analysis**

```rust
//! Rhythm analysis: onset regularity, complexity, and danceability.

pub struct RhythmInfo {
    pub complexity: f64,
    pub danceability: f64,
}

pub fn analyze_rhythm(samples: &[f32], sample_rate: u32, bpm: f64) -> RhythmInfo {
    if samples.is_empty() || bpm <= 0.0 {
        return RhythmInfo { complexity: 0.0, danceability: 0.0 };
    }

    // Detect onsets via energy delta
    let hop_samples = (sample_rate as f32 * 0.01) as usize * 2; // 10ms hop, stereo
    let mut onset_times: Vec<f64> = Vec::new();
    let mut prev_rms = 0.0f32;

    for (i, chunk) in samples.chunks(hop_samples).enumerate() {
        let rms = (chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len().max(1) as f32).sqrt();
        if rms - prev_rms > 0.03 && rms > 0.01 {
            onset_times.push(i as f64 * 0.01);
        }
        prev_rms = rms;
    }

    if onset_times.len() < 4 {
        return RhythmInfo { complexity: 0.0, danceability: 0.0 };
    }

    // Inter-onset intervals
    let intervals: Vec<f64> = onset_times.windows(2).map(|w| w[1] - w[0]).collect();
    let avg_interval = intervals.iter().sum::<f64>() / intervals.len() as f64;
    let interval_std = (intervals.iter().map(|i| (i - avg_interval).powi(2)).sum::<f64>()
        / intervals.len() as f64).sqrt();

    // Regularity: low std = regular = danceable
    let regularity = if avg_interval > 1e-5 {
        (1.0 - (interval_std / avg_interval).min(1.0)).max(0.0)
    } else { 0.0 };

    // Complexity: higher std = more complex rhythm
    let complexity = (interval_std / avg_interval.max(0.01)).min(1.0);

    // Bass energy proxy (count of strong onsets)
    let strong_onsets = onset_times.len() as f64 / (samples.len() as f64 / (sample_rate as f64 * 2.0));
    let onset_density = (strong_onsets / 10.0).min(1.0); // normalize: 10 onsets/sec = max

    // Danceability = regularity * 0.5 + onset_density * 0.3 + tempo_suitability * 0.2
    let tempo_score = if (90.0..=140.0).contains(&bpm) { 1.0 }
        else if (80.0..=160.0).contains(&bpm) { 0.7 }
        else { 0.3 };

    let danceability = (regularity * 0.5 + onset_density * 0.3 + tempo_score * 0.2).clamp(0.0, 1.0);

    RhythmInfo { complexity, danceability }
}
```

**Step 3: Run tests, commit**

```bash
git add crates/autodj-analysis/src/rhythm.rs crates/autodj-analysis/src/lib.rs
git commit -m "feat(analysis): add rhythm analysis module (complexity, danceability)"
```

---

### Task 9: Update TrackProfile and Pipeline

**Files:**
- Modify: `crates/autodj-analysis/src/profile.rs`
- Modify: `crates/autodj-analysis/src/pipeline.rs`

**Step 1: Add new fields to TrackProfile**

```rust
// In TrackProfile struct, add:
pub spectral: Option<SpectralProfile>,
pub vocal_presence: Option<f64>,
pub rhythm: Option<RhythmProfile>,
pub energy_trajectory: Option<String>,
pub intro_length_ms: Option<u64>,
pub outro_length_ms: Option<u64>,
pub first_drop_ms: Option<u64>,
```

Add the new sub-structs:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectralProfile {
    pub centroid: f64,
    pub flatness: f64,
    pub bass_energy: f64,
    pub mid_energy: f64,
    pub high_energy: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RhythmProfile {
    pub complexity: f64,
    pub danceability: f64,
}
```

**Step 2: Update pipeline to run new analyzers**

In `analyze_track`, add the new analysis steps as parallel rayon jobs:

```rust
// After existing parallel analysis, add:
let (spectral, vocal, rhythm_info) = rayon::join(
    || crate::spectral::analyze_spectral(samples, sample_rate),
    || rayon::join(
        || crate::vocal_detection::detect_vocal_presence(samples, sample_rate),
        || crate::rhythm::analyze_rhythm(samples, sample_rate, tempo.bpm),
    ),
);

// Compute energy trajectory from energy curve
let energy_trajectory = compute_energy_trajectory(&energy_info.curve);

// Compute intro/outro lengths from energy curve
let (intro_length, outro_length) = detect_intro_outro(&energy_info, duration_ms);
let first_drop = energy_info.drop_candidates_ms.first().copied();
```

Add helper functions:

```rust
fn compute_energy_trajectory(curve: &[f64]) -> String {
    if curve.len() < 4 { return "steady".to_string(); }
    let first_quarter: f64 = curve[..curve.len()/4].iter().sum::<f64>() / (curve.len()/4) as f64;
    let last_quarter: f64 = curve[curve.len()*3/4..].iter().sum::<f64>() / (curve.len()/4).max(1) as f64;
    let mid_half: f64 = curve[curve.len()/4..curve.len()*3/4].iter().sum::<f64>() / (curve.len()/2).max(1) as f64;

    if last_quarter > first_quarter * 1.3 { "building".to_string() }
    else if first_quarter > last_quarter * 1.3 { "declining".to_string() }
    else if mid_half > first_quarter * 1.3 && mid_half > last_quarter * 1.3 { "dynamic".to_string() }
    else { "steady".to_string() }
}

fn detect_intro_outro(energy: &EnergyInfo, duration_ms: u64) -> (Option<u64>, Option<u64>) {
    let curve = &energy.curve;
    if curve.len() < 4 { return (None, None); }

    let avg = energy.average;
    let window_ms = (energy.window_seconds * 1000.0) as u64;

    // Intro: first run of below-average energy
    let intro_end = curve.iter().position(|&e| e > avg * 0.5).unwrap_or(0);
    let intro_ms = if intro_end > 0 { Some(intro_end as u64 * window_ms) } else { None };

    // Outro: last run of below-average energy
    let outro_start = curve.iter().rposition(|&e| e > avg * 0.5).unwrap_or(curve.len());
    let outro_ms = if outro_start < curve.len() {
        Some(duration_ms - outro_start as u64 * window_ms)
    } else { None };

    (intro_ms, outro_ms)
}
```

**Step 3: Update TrackProfile construction**

Set the new fields in the `Ok(TrackProfile { ... })` block.

**Step 4: Update save_track_profile call in commands.rs**

Pass new fields from TrackProfile into the ProfileData struct.

**Step 5: Verify**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 6: Commit**

```bash
git add -A
git commit -m "feat(analysis): integrate spectral/vocal/rhythm into pipeline and persist to DB"
```

---

## Phase 13: Gemini-Powered Classification

### Task 10: Create Classifier Module

**Files:**
- Create: `crates/autodj-ai/src/classifier.rs`
- Modify: `crates/autodj-ai/src/lib.rs`

**Step 1: Define the classification output struct**

```rust
//! Gemini-powered track classification for genre, mood, vibe, and mixing notes.

use crate::{AiConfig, AiError};
use crate::llm::LlmClient;
use autodj_analysis::TrackProfile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackClassification {
    pub genre: String,
    pub subgenre: String,
    pub mood: String,
    pub vibe: String,
    pub mix_notes: String,
    pub ai_tags: Vec<String>,
}
```

**Step 2: Implement the classification function**

```rust
pub async fn classify_track(
    profile: &TrackProfile,
    artist: &str,
    title: &str,
    id3_genre: Option<&str>,
    config: &AiConfig,
) -> Result<TrackClassification, AiError> {
    let api_key = config.api_key.as_ref()
        .ok_or_else(|| AiError::LlmError("API key not configured".to_string()))?;

    let llm = LlmClient::new();

    let system_prompt = r#"You are a music classification AI. Given audio analysis features and metadata, classify the track.
Return valid JSON with these exact fields:
- genre: primary genre (house, hip-hop, techno, pop, r&b, drum-and-bass, dubstep, ambient, rock, latin, afrobeats, etc.)
- subgenre: specific subgenre (deep house, tech house, trap, lo-fi hip-hop, minimal techno, etc.)
- mood: emotional character (euphoric, melancholic, aggressive, chill, dark, uplifting, dreamy, energetic, sensual, mysterious)
- vibe: set role (anthem, filler, opener, closer, peak_time, warm_up, breakdown_tool, transition_tool)
- mix_notes: brief DJ mixing tips (e.g. "16-bar vocal intro, mix over 32 bars; avoid mixing during vocal hook at 1:30")
- ai_tags: array of 3-5 descriptive tags (e.g. ["vocal", "bass-heavy", "festival", "progressive"])
"#;

    let features = serde_json::json!({
        "artist": artist,
        "title": title,
        "id3_genre": id3_genre,
        "bpm": profile.tempo.bpm,
        "key": profile.key.key,
        "camelot": profile.key.camelot,
        "groove_type": profile.perceived_tempo.as_ref().map(|pt| &pt.groove_type),
        "perceived_tempo": profile.perceived_tempo.as_ref().map(|pt| pt.perceived_tempo),
        "energy_avg": profile.energy.average,
        "energy_peak": profile.energy.peak,
        "loudness_lufs": profile.loudness.integrated_lufs,
        "dynamic_range_db": profile.loudness.dynamic_range_db,
        "spectral": profile.spectral.as_ref().map(|s| serde_json::json!({
            "centroid_hz": s.centroid,
            "flatness": s.flatness,
            "bass_energy": s.bass_energy,
            "mid_energy": s.mid_energy,
            "high_energy": s.high_energy,
        })),
        "vocal_presence": profile.vocal_presence,
        "rhythm": profile.rhythm.as_ref().map(|r| serde_json::json!({
            "complexity": r.complexity,
            "danceability": r.danceability,
        })),
        "energy_trajectory": profile.energy_trajectory,
        "duration_ms": profile.duration_ms,
        "intro_length_ms": profile.intro_length_ms,
        "outro_length_ms": profile.outro_length_ms,
    });

    let context = serde_json::to_string(&features)
        .map_err(|e| AiError::ParseError(format!("Failed to serialize: {e}")))?;

    let response = llm.query(system_prompt, &context, config).await?;

    let classification: TrackClassification = serde_json::from_str(&response)
        .map_err(|e| AiError::ParseError(format!("Failed to parse classification: {e}")))?;

    Ok(classification)
}
```

**Step 3: Add module to lib.rs**

```rust
pub mod classifier;
```

**Step 4: Commit**

```bash
git add crates/autodj-ai/src/classifier.rs crates/autodj-ai/src/lib.rs
git commit -m "feat(ai): add Gemini-powered track classifier (genre/mood/vibe)"
```

---

### Task 11: Add Classification DB Columns and Integration

**Files:**
- Modify: `crates/autodj-storage/src/db.rs`
- Modify: `crates/autodj-app/src/commands.rs`
- Modify: `crates/autodj-storage/src/db.rs` (TrackRecord)

**Step 1: Add DB migration columns for classification**

```rust
// Add to migrations array:
"ALTER TABLE track_profiles ADD COLUMN genre TEXT",
"ALTER TABLE track_profiles ADD COLUMN subgenre TEXT",
"ALTER TABLE track_profiles ADD COLUMN mood TEXT",
"ALTER TABLE track_profiles ADD COLUMN vibe TEXT",
"ALTER TABLE track_profiles ADD COLUMN mix_notes TEXT",
"ALTER TABLE track_profiles ADD COLUMN ai_tags TEXT",
```

**Step 2: Add a `save_classification` method to Database**

```rust
pub fn save_classification(
    &self,
    track_id: &str,
    genre: &str,
    subgenre: &str,
    mood: &str,
    vibe: &str,
    mix_notes: &str,
    ai_tags: &str,
) -> Result<(), DbError> {
    self.conn.execute(
        "UPDATE track_profiles SET genre=?2, subgenre=?3, mood=?4, vibe=?5, mix_notes=?6, ai_tags=?7 WHERE track_id=?1",
        params![track_id, genre, subgenre, mood, vibe, mix_notes, ai_tags],
    )?;
    Ok(())
}
```

**Step 3: Add classify_track Tauri command**

```rust
#[tauri::command]
pub async fn classify_track(
    track_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Get profile, metadata, call classifier, save to DB
}
```

**Step 4: Wire into analysis pipeline or post-analysis step**

After `analyze_track` completes, if Gemini API key is configured, call `classify_track` automatically.

**Step 5: Update TrackRecord to include genre/mood fields for library display**

**Step 6: Commit**

```bash
git add -A
git commit -m "feat(ai): integrate Gemini classification into import pipeline and DB"
```

---

## Phase 14: AI Decision Engine Overhaul

### Task 12: Expand AI Context with Full Track Data

**Files:**
- Modify: `crates/autodj-ai/src/context.rs`

**Step 1: Add rich fields to `TrackProfileShort`**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackProfileShort {
    pub id: String,
    pub artist: String,
    pub title: String,
    pub bpm: f64,
    pub perceived_tempo: f64,
    pub groove_type: String,
    pub key: String,
    pub energy: f64,
    pub lufs_loudness: f64,
    pub duration_ms: u64,
    pub phrase_length_ms: u64,
    // New enriched fields
    pub genre: Option<String>,
    pub subgenre: Option<String>,
    pub mood: Option<String>,
    pub vibe: Option<String>,
    pub mix_notes: Option<String>,
    pub spectral_centroid: Option<f64>,
    pub vocal_presence: Option<f64>,
    pub danceability: Option<f64>,
    pub energy_trajectory: Option<String>,
    pub intro_length_ms: Option<u64>,
    pub outro_length_ms: Option<u64>,
}
```

**Step 2: Add `CurrentStateContext` enrichments**

```rust
// Add to CurrentStateContext:
pub genre: Option<String>,
pub mood: Option<String>,
pub vibe: Option<String>,
pub vocal_presence: Option<f64>,
pub energy_trajectory: Option<String>,
pub intro_length_ms: Option<u64>,
pub outro_length_ms: Option<u64>,
pub first_drop_ms: Option<u64>,
```

**Step 3: Add set flow context**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetFlowContext {
    pub tracks_played: u32,
    pub elapsed_minutes: f64,
    pub recent_genres: Vec<String>,
    pub recent_moods: Vec<String>,
    pub energy_trend: String, // "ascending", "peak", "descending", "steady"
}
```

Add `pub set_flow: Option<SetFlowContext>` to `AiContext`.

**Step 4: Update `build_context` to populate new fields**

Pass classification data from the DB when building the context. The caller (commands.rs) will need to query classification data.

**Step 5: Commit**

```bash
git add crates/autodj-ai/src/context.rs
git commit -m "feat(ai): expand AI context with genre/mood/spectral/set-flow data"
```

---

### Task 13: Expand Heuristic Scorer

**Files:**
- Modify: `crates/autodj-ai/src/scorer.rs`
- Modify: `crates/autodj-ai/src/lib.rs` (AiConfig)

**Step 1: Add new weight fields to AiConfig**

```rust
// In AiConfig, add:
pub weight_genre: f64,    // default 0.15
pub weight_mood: f64,     // default 0.10
pub weight_spectral: f64, // default 0.05
```

Update Default impl with rebalanced weights:
- weight_tempo: 0.20
- weight_key: 0.25
- weight_energy: 0.15
- weight_variety: 0.10
- weight_genre: 0.15
- weight_mood: 0.10
- weight_spectral: 0.05

**Step 2: Add scoring functions**

```rust
fn score_genre(&self, genre_a: Option<&str>, genre_b: Option<&str>) -> f64 {
    match (genre_a, genre_b) {
        (Some(a), Some(b)) if a == b => 1.0,
        (Some(a), Some(b)) if genres_related(a, b) => 0.7,
        (Some(_), Some(_)) => 0.3,
        _ => 0.5, // Unknown = neutral
    }
}

fn score_mood(&self, mood_a: Option<&str>, mood_b: Option<&str>) -> f64 {
    match (mood_a, mood_b) {
        (Some(a), Some(b)) if a == b => 1.0,
        (Some(a), Some(b)) if moods_adjacent(a, b) => 0.8,
        (Some(_), Some(_)) => 0.4,
        _ => 0.5,
    }
}

fn score_spectral(&self, centroid_a: Option<f64>, centroid_b: Option<f64>) -> f64 {
    match (centroid_a, centroid_b) {
        (Some(a), Some(b)) => {
            let diff = (a - b).abs() / a.max(b).max(1.0);
            // Some contrast is good (0.1-0.3), identical or extreme is bad
            if diff < 0.1 { 0.6 } // too similar
            else if diff < 0.3 { 1.0 } // good contrast
            else { 0.4 } // too different
        }
        _ => 0.5,
    }
}
```

**Step 3: Add genre/mood compatibility helpers**

```rust
fn genres_related(a: &str, b: &str) -> bool {
    let groups: &[&[&str]] = &[
        &["house", "deep house", "tech house", "progressive house"],
        &["techno", "minimal techno", "industrial techno"],
        &["hip-hop", "trap", "drill", "boom bap"],
        &["drum-and-bass", "jungle", "breakbeat"],
        &["pop", "dance-pop", "electro-pop"],
        &["r&b", "neo-soul", "funk"],
        &["latin", "reggaeton", "afrobeats"],
    ];
    groups.iter().any(|group| group.contains(&a) && group.contains(&b))
}

fn moods_adjacent(a: &str, b: &str) -> bool {
    let adjacency: &[(&str, &[&str])] = &[
        ("euphoric", &["uplifting", "energetic"]),
        ("uplifting", &["euphoric", "chill", "dreamy"]),
        ("energetic", &["euphoric", "aggressive"]),
        ("chill", &["uplifting", "dreamy", "sensual"]),
        ("dark", &["aggressive", "mysterious"]),
        ("aggressive", &["dark", "energetic"]),
        ("melancholic", &["dreamy", "mysterious"]),
        ("dreamy", &["chill", "melancholic", "uplifting"]),
        ("sensual", &["chill", "mysterious"]),
        ("mysterious", &["dark", "melancholic", "sensual"]),
    ];
    adjacency.iter().any(|(mood, adj)| (*mood == a && adj.contains(&b)) || (*mood == b && adj.contains(&a)))
}
```

**Step 4: Update `score()` to use new dimensions**

The scorer needs access to classification data. Update the scorer to accept enriched profile data (either through the TrackProfile or a separate struct).

**Step 5: Verify and commit**

```bash
git add crates/autodj-ai/src/scorer.rs crates/autodj-ai/src/lib.rs
git commit -m "feat(ai): expand heuristic scorer with genre/mood/spectral dimensions"
```

---

### Task 14: Improve AI Prompt Engineering

**Files:**
- Modify: `crates/autodj-ai/src/planner.rs`

**Step 1: Rewrite the system prompt**

Replace the one-liner with a comprehensive DJ knowledge prompt:

```rust
const DJ_SYSTEM_PROMPT: &str = r#"You are Auto-DJ, an expert AI club DJ with deep knowledge of mixing techniques.

## Your Job
Analyze the current state, available tracks, and set flow to make the best mixing decision.

## Mixing Principles
1. HARMONIC MIXING: Prefer tracks within 1-2 Camelot positions. Same key (0) is ideal, adjacent (±1) is good, relative major/minor (A↔B) works well.
2. TEMPO MATCHING: Keep BPM within ±8 of current. Perceived tempo matters more than raw BPM (trap at 140 feels like 70).
3. ENERGY FLOW: Build energy gradually, maintain peaks, provide relief. Don't jump between extremes without a breakdown.
4. GENRE COHERENCE: Stay in related genres. House→techno is smooth. House→hip-hop needs careful transition (echo drop or breakdown).
5. MOOD CONTINUITY: Gradual mood shifts. Euphoric→uplifting is smooth. Euphoric→melancholic is jarring unless deliberate.
6. VOCAL AWARENESS: Never overlap two vocal-heavy tracks. If current track has vocals, next should be instrumental or have vocals in different section.

## Transition Templates
- smooth_beatmix: 16-32 bars, bass swap, best for same-genre/tempo tracks
- filter_sweep: 8-16 bars, high-pass sweep + incoming fade, good for energy builds
- echo_drop: 4-8 bars, echo feedback → silence → drop, best for genre/tempo changes or big energy moments

## Timing Rules
- Use section boundaries (intro/outro) for transitions, not arbitrary positions
- Match outgoing outro length with incoming intro length when possible
- execute_at_ms should target the start of the outgoing track's outro section
- Never trigger during a vocal hook or drop buildup

## Output
Return valid JSON conforming to the AiDecision schema. Include detailed reasoning.
"#;
```

**Step 2: Update heuristic fallback to use section data**

```rust
// In fallback_heuristic, improve transition timing:
// Instead of: context.current_state.duration_ms.saturating_sub(10_000)
// Use outro start if available:
let execute_at = context.current_state.outro_start_ms
    .unwrap_or_else(|| context.current_state.duration_ms.saturating_sub(
        context.current_state.outro_length_ms.unwrap_or(10_000)));
```

**Step 3: Improve template selection logic**

```rust
// Consider vocal presence and genre for template choice:
let template_name = if energy_jump > 0.3 {
    "echo_drop"
} else if genre_change {
    "filter_sweep"
} else if vocal_overlap_risk {
    "filter_sweep" // gives time for vocal to clear
} else {
    "smooth_beatmix"
};
```

**Step 4: Commit**

```bash
git add crates/autodj-ai/src/planner.rs
git commit -m "feat(ai): comprehensive DJ prompt engineering and section-aware transitions"
```

---

### Task 15: Update Frontend Store and UI for Enriched Data

**Files:**
- Modify: `ui/src/store.ts`
- Modify: `ui/src/App.tsx`

**Step 1: Add genre/mood fields to TrackRecord interface**

```typescript
export interface TrackRecord {
    // ... existing fields ...
    genre: string | null;
    mood: string | null;
    vibe: string | null;
}
```

**Step 2: Update library table to show genre and mood columns**

Add genre and mood columns to the library table in App.tsx.

**Step 3: Commit**

```bash
git add ui/src/store.ts ui/src/App.tsx
git commit -m "feat(ui): display genre, mood, and vibe in library table"
```

---

## Phase 15: Mix History & Session Tracking

### Task 16: Wire Up Mix History Recording

**Files:**
- Modify: `crates/autodj-storage/src/db.rs`
- Modify: `crates/autodj-app/src/commands.rs`
- Modify: `crates/autodj-app/src/state.rs`

**Step 1: Add session management methods to Database**

```rust
pub fn create_session(&self) -> Result<String, DbError> { /* insert into sessions */ }
pub fn end_session(&self, session_id: &str) -> Result<(), DbError> { /* update ended_at */ }
pub fn record_track_play(&self, session_id: &str, track_id: &str, transition_type: Option<&str>, ai_decision: Option<&str>) -> Result<(), DbError> { /* insert into mix_history */ }
pub fn get_recent_track_ids(&self, limit: usize) -> Result<Vec<String>, DbError> { /* query mix_history ORDER BY started_at DESC */ }
```

**Step 2: Add session state to AppState**

```rust
pub current_session_id: std::sync::RwLock<Option<String>>,
```

**Step 3: Record plays in load_track and execute_transition commands**

**Step 4: Feed recent tracks into AI context**

**Step 5: Commit**

```bash
git add -A
git commit -m "feat: wire up mix history recording and session tracking"
```

---

## Phase 16: Audio Export

### Task 17: Implement WAV Recording

**Files:**
- Create: `crates/autodj-core/src/recorder.rs`
- Modify: `crates/autodj-core/src/engine.rs`
- Modify: `crates/autodj-core/src/lib.rs`
- Modify: `crates/autodj-app/src/commands.rs`

**Step 1: Create recorder module with ring buffer capture**

```rust
//! Audio recorder: captures master output to WAV file.
pub struct AudioRecorder {
    file: Option<std::io::BufWriter<std::fs::File>>,
    samples_written: u64,
    sample_rate: u32,
}
```

**Step 2: Tap master output in engine callback**

After the limiter output, push samples to the recorder's ring buffer.

**Step 3: Add Tauri commands**

```rust
#[tauri::command]
pub async fn start_recording(path: String, state: State<'_, AppState>) -> Result<(), String> { }
#[tauri::command]
pub async fn stop_recording(state: State<'_, AppState>) -> Result<String, String> { }
```

**Step 4: Add UI recording button**

**Step 5: Commit**

```bash
git add -A
git commit -m "feat: add WAV recording of live mix output"
```

---

## Phase 17: Visual Polish & Analysis Improvements

### Task 18: Album Art Extraction

**Files:**
- Modify: `crates/autodj-app/src/commands.rs`
- Modify: `crates/autodj-storage/src/cache.rs`

**Step 1: Extract album art via lofty during import**

```rust
// In import_files, after tag reading:
if let Some(picture) = tag.pictures().first() {
    let art_dir = cache.base_dir().join("cache").join("art");
    std::fs::create_dir_all(&art_dir)?;
    let art_path = art_dir.join(format!("{}.png", track_id));
    std::fs::write(&art_path, picture.data())?;
}
```

**Step 2: Add Tauri command to get album art path**

**Step 3: Display in UI**

**Step 4: Commit**

```bash
git add -A
git commit -m "feat: extract and display album art from imported tracks"
```

---

### Task 19: Improved Section Analysis

**Files:**
- Modify: `crates/autodj-analysis/src/sections.rs`

**Step 1: Replace fixed 15% heuristic with energy-based detection**

Use the energy curve to detect actual intro/buildup/drop/breakdown/outro boundaries based on sustained energy changes rather than fixed percentages.

**Step 2: Update section labels and confidence scores**

**Step 3: Commit**

```bash
git add crates/autodj-analysis/src/sections.rs
git commit -m "feat(analysis): energy-based section detection replacing fixed heuristic"
```

---

## Verification

After all tasks complete:

1. `cargo clippy --workspace` — zero warnings
2. `cargo test --workspace` — all tests pass
3. `cargo tauri dev` — app launches and runs
4. Import a track → verify DSP features computed
5. If Gemini API key set → verify genre/mood classification
6. Request AI next track → verify enriched context in decision
7. Check library → genre/mood columns visible
