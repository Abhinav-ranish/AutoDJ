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
}
