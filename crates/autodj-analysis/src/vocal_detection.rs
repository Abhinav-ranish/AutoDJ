//! Vocal presence detection using spectral analysis in the vocal frequency range.

use rustfft::{num_complex::Complex, FftPlanner};

/// Detect vocal presence in the track.
///
/// Returns a value from 0.0 (instrumental) to 1.0 (heavily vocal).
/// Uses the ratio of energy in the vocal frequency range (300-3500 Hz) vs the total spectrum,
/// combined with harmonic prominence analysis.
pub fn detect_vocal_presence(samples: &[f32], sample_rate: u32) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }

    let window_size = 4096;
    let hop_size = 2048;
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(window_size);

    let mut vocal_energy_sum = 0.0;
    let mut harmonic_ratio_sum = 0.0;
    let mut num_windows = 0u64;

    for i in (0..samples.len().saturating_sub(window_size * 2)).step_by(hop_size * 2) {
        let mut buffer: Vec<Complex<f32>> = (0..window_size)
            .map(|j| {
                let idx = i + j * 2;
                let sample = if idx < samples.len() {
                    samples[idx]
                } else {
                    0.0
                };
                // Apply Hann window to reduce spectral leakage
                let hann = 0.5
                    * (1.0
                        - (2.0 * std::f32::consts::PI * j as f32 / (window_size - 1) as f32)
                            .cos());
                Complex::new(sample * hann, 0.0)
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
            peak_mags.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
            let top_3: f64 = peak_mags.iter().take(3).sum();
            let total: f64 = peak_mags.iter().sum();
            if total > 1e-10 {
                harmonic_ratio_sum += top_3 / total;
            }
        }

        num_windows += 1;
    }

    let n = num_windows.max(1) as f64;
    let avg_vocal_ratio = vocal_energy_sum / n;
    let avg_harmonic_ratio = harmonic_ratio_sum / n;

    // Combine vocal band energy ratio with harmonic prominence
    (avg_vocal_ratio * 0.6 + avg_harmonic_ratio * 0.4).clamp(0.0, 1.0)
}

/// Phase 49: Compute per-window vocal presence values.
///
/// Returns a Vec of vocal presence values (0.0–1.0), one per analysis window.
/// Window size matches the energy curve windows for easy alignment.
pub fn detect_vocal_curve(samples: &[f32], sample_rate: u32, window_seconds: f64) -> Vec<f64> {
    if samples.is_empty() || window_seconds <= 0.0 {
        return Vec::new();
    }

    let window_samples = (sample_rate as f64 * window_seconds) as usize * 2; // stereo
    if window_samples == 0 {
        return Vec::new();
    }

    let fft_size = 4096;
    let hop_size = 2048;
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);

    let mut results = Vec::new();

    for chunk_start in (0..samples.len()).step_by(window_samples) {
        let chunk_end = (chunk_start + window_samples).min(samples.len());
        let chunk = &samples[chunk_start..chunk_end];

        if chunk.len() < fft_size * 2 {
            // Too short for FFT, use 0
            results.push(0.0);
            continue;
        }

        let mut vocal_energy_sum = 0.0;
        let mut num_frames = 0u32;

        for i in (0..chunk.len().saturating_sub(fft_size * 2)).step_by(hop_size * 2) {
            let mut buffer: Vec<Complex<f32>> = (0..fft_size)
                .map(|j| {
                    let idx = i + j * 2;
                    let sample = if idx < chunk.len() { chunk[idx] } else { 0.0 };
                    let hann = 0.5
                        * (1.0
                            - (2.0 * std::f32::consts::PI * j as f32 / (fft_size - 1) as f32)
                                .cos());
                    Complex::new(sample * hann, 0.0)
                })
                .collect();

            fft.process(&mut buffer);

            let half = fft_size / 2;
            let mut vocal_energy = 0.0;
            let mut total_energy = 0.0;

            for (bin, c) in buffer.iter().enumerate().take(half) {
                let freq = bin as f64 * sample_rate as f64 / fft_size as f64;
                let mag = c.norm() as f64;
                total_energy += mag * mag;
                if (300.0..=3500.0).contains(&freq) {
                    vocal_energy += mag * mag;
                }
            }

            if total_energy > 1e-10 {
                vocal_energy_sum += vocal_energy / total_energy;
            }
            num_frames += 1;
        }

        let value = if num_frames > 0 {
            (vocal_energy_sum / num_frames as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };
        results.push(value);
    }

    results
}

/// Phase 49: A vocal-free zone in the track.
#[derive(Debug, Clone)]
pub struct VocalFreeZone {
    /// Start window index.
    pub start_idx: usize,
    /// End window index (exclusive).
    pub end_idx: usize,
}

/// Phase 49: Detect zones where vocals are absent for at least `min_windows` consecutive windows.
pub fn detect_vocal_free_zones(
    vocal_curve: &[f64],
    threshold: f64,
    min_windows: usize,
) -> Vec<VocalFreeZone> {
    let mut zones = Vec::new();
    let mut run_start: Option<usize> = None;

    for (i, &v) in vocal_curve.iter().enumerate() {
        if v < threshold {
            if run_start.is_none() {
                run_start = Some(i);
            }
        } else {
            if let Some(start) = run_start {
                if i - start >= min_windows {
                    zones.push(VocalFreeZone {
                        start_idx: start,
                        end_idx: i,
                    });
                }
            }
            run_start = None;
        }
    }

    // Handle trailing zone
    if let Some(start) = run_start {
        if vocal_curve.len() - start >= min_windows {
            zones.push(VocalFreeZone {
                start_idx: start,
                end_idx: vocal_curve.len(),
            });
        }
    }

    zones
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vocal_presence_silence() {
        let samples = vec![0.0f32; 48000 * 2];
        let result = detect_vocal_presence(&samples, 48000);
        assert!(
            result < 0.1,
            "Silence should have near-zero vocal presence, got {}",
            result
        );
    }

    #[test]
    fn test_vocal_presence_returns_bounded_value() {
        let sr = 48000u32;
        let samples: Vec<f32> = (0..sr as usize * 4) // 2 sec stereo
            .map(|i| {
                (2.0 * std::f32::consts::PI * 800.0 * (i / 2) as f32 / sr as f32).sin()
            })
            .collect();
        let result = detect_vocal_presence(&samples, sr);
        assert!(
            (0.0..=1.0).contains(&result),
            "Result should be 0.0-1.0, got {}",
            result
        );
    }

    #[test]
    fn test_empty_samples() {
        let result = detect_vocal_presence(&[], 48000);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_vocal_curve_empty() {
        let curve = detect_vocal_curve(&[], 48000, 0.5);
        assert!(curve.is_empty());
    }

    #[test]
    fn test_vocal_curve_returns_values() {
        let sr = 48000u32;
        // 4 seconds stereo silence
        let samples = vec![0.0f32; sr as usize * 2 * 4];
        let curve = detect_vocal_curve(&samples, sr, 0.5);
        assert!(!curve.is_empty(), "Should produce at least one window");
        for &v in &curve {
            assert!((0.0..=1.0).contains(&v), "Values should be 0.0-1.0, got {}", v);
        }
    }

    #[test]
    fn test_vocal_free_zones_detection() {
        // Simulate: vocal, silent, silent, silent, silent, vocal
        let curve = vec![0.5, 0.1, 0.05, 0.08, 0.1, 0.6];
        let zones = detect_vocal_free_zones(&curve, 0.15, 4);
        assert_eq!(zones.len(), 1);
        assert_eq!(zones[0].start_idx, 1);
        assert_eq!(zones[0].end_idx, 5);
    }

    #[test]
    fn test_vocal_free_zones_too_short() {
        let curve = vec![0.5, 0.1, 0.05, 0.6];
        let zones = detect_vocal_free_zones(&curve, 0.15, 4);
        assert!(zones.is_empty(), "2 windows < min 4, should find no zones");
    }

    #[test]
    fn test_vocal_free_zones_trailing() {
        let curve = vec![0.5, 0.1, 0.05, 0.08, 0.02];
        let zones = detect_vocal_free_zones(&curve, 0.15, 4);
        assert_eq!(zones.len(), 1);
        assert_eq!(zones[0].start_idx, 1);
        assert_eq!(zones[0].end_idx, 5);
    }
}
