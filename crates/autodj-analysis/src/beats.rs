//! Beat tracking via dynamic programming (Ellis 2007 inspired)
//!
//! Replaces the previous synthetic grid approach with onset-based DP tracking.
//! 1. Compute onset strength envelope via spectral flux
//! 2. Run DP beat tracking with tempo prior penalty
//! 3. Backtrack to find optimal beat positions

use crate::profile::BeatInfo;
use rustfft::{num_complex::Complex, FftPlanner};

/// FFT window size for onset detection
const ONSET_FFT_SIZE: usize = 2048;
/// Hop size for onset detection (in mono frames)
const ONSET_HOP: usize = 512;

/// Generates a beat grid using dynamic programming beat tracking.
pub fn track_beats(samples: &[f32], sample_rate: u32, bpm: f64) -> BeatInfo {
    if samples.is_empty() || bpm <= 0.0 {
        return BeatInfo {
            count: 0,
            times_ms: vec![],
            downbeat_indices: vec![],
            confidence: 0.0,
        };
    }

    // Step 1: Compute onset strength envelope via spectral flux
    let onset_env = compute_onset_envelope(samples, sample_rate);
    if onset_env.is_empty() {
        return fallback_synthetic_grid(samples, sample_rate, bpm);
    }

    // Step 2: DP beat tracking with tempo prior
    let hop_duration = ONSET_HOP as f64 / sample_rate as f64;
    let beat_period_frames = (60.0 / bpm) / hop_duration;

    let beats = dp_beat_track(&onset_env, beat_period_frames);
    if beats.len() < 4 {
        return fallback_synthetic_grid(samples, sample_rate, bpm);
    }

    // Step 3: Convert onset frame indices to milliseconds
    let mut times_ms = Vec::with_capacity(beats.len());
    let mut downbeat_indices = Vec::new();

    for (i, &onset_idx) in beats.iter().enumerate() {
        let time_s = onset_idx as f64 * hop_duration;
        let time_ms = (time_s * 1000.0) as u64;
        times_ms.push(time_ms);

        // Assume 4/4 time: every 4th beat is a downbeat
        if i % 4 == 0 {
            downbeat_indices.push(i);
        }
    }

    // Compute confidence based on onset strength at detected beats
    let confidence = compute_beat_confidence(&onset_env, &beats);

    BeatInfo {
        count: times_ms.len() as u64,
        times_ms,
        downbeat_indices,
        confidence,
    }
}

/// Fallback to synthetic grid when DP tracker fails (too few beats, silence, etc.)
fn fallback_synthetic_grid(samples: &[f32], sample_rate: u32, bpm: f64) -> BeatInfo {
    let total_frames = samples.len() / 2;
    let samples_per_beat = (sample_rate as f64 * 60.0 / bpm) as usize;
    let mut times_ms = Vec::new();
    let mut downbeat_indices = Vec::new();

    // Find the first major peak in the first 2 seconds to align the grid
    let search_samples = (sample_rate as usize * 2 * 2).min(samples.len());
    let mut max_val: f32 = 0.0;
    let mut first_beat_sample = 0;

    for i in (0..search_samples).step_by(2) {
        if i + 1 < samples.len() {
            let v = samples[i].abs() + samples[i + 1].abs();
            if v > max_val {
                max_val = v;
                first_beat_sample = i / 2;
            }
        }
    }

    let mut current_frame = first_beat_sample;
    let mut beat_idx = 0;

    while current_frame < total_frames {
        let ms = (current_frame as u64 * 1000) / sample_rate as u64;
        times_ms.push(ms);

        if beat_idx % 4 == 0 {
            downbeat_indices.push(beat_idx);
        }

        beat_idx += 1;
        current_frame += samples_per_beat;
    }

    BeatInfo {
        count: times_ms.len() as u64,
        times_ms,
        downbeat_indices,
        confidence: 0.3, // low confidence for synthetic grid
    }
}

/// Compute onset strength envelope using spectral flux (half-wave rectified)
fn compute_onset_envelope(samples: &[f32], sample_rate: u32) -> Vec<f32> {
    let mono_len = samples.len() / 2;
    if mono_len < ONSET_FFT_SIZE {
        return vec![];
    }

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(ONSET_FFT_SIZE);

    let num_frames = (mono_len - ONSET_FFT_SIZE) / ONSET_HOP + 1;
    let mut onset_env = Vec::with_capacity(num_frames);
    let mut prev_mag = vec![0.0f32; ONSET_FFT_SIZE / 2 + 1];
    let mut buffer: Vec<Complex<f32>> = vec![Complex::new(0.0, 0.0); ONSET_FFT_SIZE];

    // Hann window
    let window: Vec<f32> = (0..ONSET_FFT_SIZE)
        .map(|i| {
            0.5 * (1.0
                - (2.0 * std::f32::consts::PI * i as f32 / (ONSET_FFT_SIZE - 1) as f32).cos())
        })
        .collect();

    for frame_idx in 0..num_frames {
        let start = frame_idx * ONSET_HOP;

        // Fill FFT buffer with windowed mono samples
        for i in 0..ONSET_FFT_SIZE {
            let sample_idx = start + i;
            let mono = if sample_idx < mono_len {
                let l = samples[sample_idx * 2];
                let r = if sample_idx * 2 + 1 < samples.len() {
                    samples[sample_idx * 2 + 1]
                } else {
                    l
                };
                (l + r) * 0.5
            } else {
                0.0
            };
            buffer[i] = Complex::new(mono * window[i], 0.0);
        }

        fft.process(&mut buffer);

        // Compute spectral flux (half-wave rectified difference of magnitudes)
        let mut flux = 0.0f32;
        for i in 0..=ONSET_FFT_SIZE / 2 {
            let mag = buffer[i].norm();
            let diff = mag - prev_mag[i];
            if diff > 0.0 {
                flux += diff;
            }
            prev_mag[i] = mag;
        }

        onset_env.push(flux);
    }

    // Normalize onset envelope
    let max_onset = onset_env.iter().cloned().fold(0.0f32, f32::max);
    if max_onset > 0.0 {
        for v in &mut onset_env {
            *v /= max_onset;
        }
    }

    onset_env
}

/// Dynamic programming beat tracking.
///
/// Cost function: C(t) = -onset(t) + alpha * F(t - t_prev, period)
/// where F is a log-Gaussian penalty for deviating from expected beat period.
fn dp_beat_track(onset_env: &[f32], beat_period: f64) -> Vec<usize> {
    let n = onset_env.len();
    if n == 0 {
        return vec![];
    }

    // Alpha controls balance between onset strength and tempo regularity
    let alpha = 680.0f64;
    let sigma = beat_period * 0.4; // tolerance for tempo variation

    // DP score and backpointer arrays
    let mut score = vec![0.0f64; n];
    let mut backptr = vec![0usize; n];

    // Search window: look back between 0.5x and 2x the beat period
    let min_lookback = (beat_period * 0.5).max(1.0) as usize;
    let max_lookback = (beat_period * 2.0).min(n as f64 - 1.0) as usize;

    if max_lookback == 0 || min_lookback >= max_lookback {
        return vec![];
    }

    // Initialize first frames with onset strength only
    for i in 0..n.min(max_lookback + 1) {
        score[i] = onset_env[i] as f64;
    }

    // Forward pass
    let sigma_sq_2 = 2.0 * sigma * sigma;
    for t in (max_lookback + 1)..n {
        let mut best_score = f64::NEG_INFINITY;
        let mut best_prev = 0;

        for lag in min_lookback..=max_lookback {
            let prev = t - lag;
            // Log-Gaussian tempo penalty
            let period_error = lag as f64 - beat_period;
            let penalty = -alpha * (period_error * period_error) / sigma_sq_2;

            let candidate = score[prev] + penalty;
            if candidate > best_score {
                best_score = candidate;
                best_prev = prev;
            }
        }

        score[t] = onset_env[t] as f64 + best_score;
        backptr[t] = best_prev;
    }

    // Backtrack from the best score in the last beat period window
    let search_start = n.saturating_sub(max_lookback + 1);
    let mut best_end = search_start;
    let mut best_end_score = score[search_start];

    for i in search_start..n {
        if score[i] > best_end_score {
            best_end_score = score[i];
            best_end = i;
        }
    }

    // Trace back through the backpointers
    let mut beats = Vec::new();
    let mut t = best_end;
    let mut safety = 0;

    while t > min_lookback && safety < n {
        beats.push(t);
        let prev = backptr[t];
        if prev >= t {
            break;
        }
        t = prev;
        safety += 1;
    }
    // Include the final traced position if valid
    if t < n {
        beats.push(t);
    }

    beats.reverse();
    beats
}

/// Compute confidence based on how well beats align with onsets
/// and how regular the inter-beat intervals are.
fn compute_beat_confidence(onset_env: &[f32], beats: &[usize]) -> f64 {
    if beats.is_empty() || onset_env.is_empty() {
        return 0.0;
    }

    // Average onset strength at beat positions
    let mut total_onset = 0.0f64;
    let mut beat_count = 0;

    for &beat_idx in beats {
        if beat_idx < onset_env.len() {
            total_onset += onset_env[beat_idx] as f64;
            beat_count += 1;
        }
    }

    if beat_count == 0 {
        return 0.0;
    }

    let avg = total_onset / beat_count as f64;

    // Check regularity (coefficient of variation of inter-beat intervals)
    if beats.len() >= 3 {
        let intervals: Vec<f64> = beats.windows(2).map(|w| (w[1] - w[0]) as f64).collect();
        let mean_interval = intervals.iter().sum::<f64>() / intervals.len() as f64;
        if mean_interval > 0.0 {
            let variance = intervals
                .iter()
                .map(|&i| (i - mean_interval).powi(2))
                .sum::<f64>()
                / intervals.len() as f64;
            let cv = variance.sqrt() / mean_interval;

            // Low CV = regular beats = higher confidence
            let regularity = 1.0 / (1.0 + cv * 5.0);

            (avg * 0.5 + regularity * 0.5).min(1.0)
        } else {
            (avg * 0.5).min(1.0)
        }
    } else {
        (avg * 0.5).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beats_synthetic_120bpm() {
        let sr = 48000u32;
        let duration_sec = 4.0;
        let total_samples = (sr as f64 * duration_sec * 2.0) as usize;
        let mut samples = vec![0.0f32; total_samples];

        // Generate kicks at 120 BPM (every 0.5 seconds)
        let beat_frames = (sr as f64 * 0.5) as usize;
        for beat in 0..8 {
            let start = beat * beat_frames;
            for j in 0..200 {
                let idx = (start + j) * 2;
                if idx + 1 < samples.len() {
                    let amp = 1.0 - (j as f32 / 200.0); // decaying impulse
                    samples[idx] = amp;
                    samples[idx + 1] = amp;
                }
            }
        }

        let info = track_beats(&samples, sr, 120.0);
        assert!(info.count >= 4, "Expected at least 4 beats, got {}", info.count);
        assert!(
            info.confidence > 0.2,
            "Expected confidence > 0.2, got {}",
            info.confidence
        );

        // Check that beat positions are roughly 500ms apart
        if info.times_ms.len() >= 2 {
            let intervals: Vec<i64> = info
                .times_ms
                .windows(2)
                .map(|w| w[1] as i64 - w[0] as i64)
                .collect();
            let avg_interval =
                intervals.iter().sum::<i64>() as f64 / intervals.len() as f64;
            assert!(
                (avg_interval - 500.0).abs() < 100.0,
                "Expected ~500ms intervals, got {} ms avg",
                avg_interval
            );
        }
    }

    #[test]
    fn test_beats_empty_input() {
        let info = track_beats(&[], 48000, 120.0);
        assert_eq!(info.count, 0);
        assert_eq!(info.confidence, 0.0);
    }

    #[test]
    fn test_beats_invalid_bpm() {
        let samples = vec![0.0f32; 96000];
        let info = track_beats(&samples, 48000, 0.0);
        assert_eq!(info.count, 0);
    }

    #[test]
    fn test_onset_envelope_short_input() {
        // Input shorter than FFT window should return empty
        let samples = vec![0.0f32; 1000];
        let env = compute_onset_envelope(&samples, 48000);
        assert!(env.is_empty());
    }

    #[test]
    fn test_dp_beat_track_empty() {
        let beats = dp_beat_track(&[], 50.0);
        assert!(beats.is_empty());
    }
}
