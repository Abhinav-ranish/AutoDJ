//! Spectral analysis: centroid, flatness, and band energy distribution.

use rustfft::{num_complex::Complex, FftPlanner};

/// Result of spectral analysis across a track.
pub struct SpectralInfo {
    pub centroid: f64,
    pub flatness: f64,
    pub bass_energy: f64,
    pub mid_energy: f64,
    pub high_energy: f64,
}

/// Analyze the spectral characteristics of a track.
///
/// Computes average spectral centroid (brightness), spectral flatness (tonality vs noise),
/// and relative energy in bass (<250 Hz), mid (250-4000 Hz), and high (>4000 Hz) bands.
pub fn analyze_spectral(samples: &[f32], sample_rate: u32) -> SpectralInfo {
    if samples.is_empty() {
        return SpectralInfo {
            centroid: 0.0,
            flatness: 0.0,
            bass_energy: 0.0,
            mid_energy: 0.0,
            high_energy: 0.0,
        };
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

    // Iterate over the track in windowed frames (skip every other sample for stereo → mono)
    for i in (0..samples.len().saturating_sub(window_size * 2)).step_by(hop_size * 2) {
        let mut buffer: Vec<Complex<f32>> = (0..window_size)
            .map(|j| {
                let idx = i + j * 2; // step by 2 for stereo interleaved → left channel
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
        let mut weighted_sum = 0.0;
        let mut magnitude_sum = 0.0;
        let mut log_sum = 0.0;
        let mut bass = 0.0;
        let mut mid = 0.0;
        let mut high = 0.0;
        let mut bin_count = 0u32;

        for (bin, c) in buffer.iter().enumerate().take(half) {
            let freq = bin as f64 * sample_rate as f64 / window_size as f64;
            if freq < 20.0 {
                continue;
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_centroid_pure_sine() {
        // A 1kHz sine wave should have centroid near 1000 Hz
        // With Hann windowing and short test signal, allow some tolerance
        let sr = 48000u32;
        let samples: Vec<f32> = (0..sr as usize * 4) // 2 seconds stereo for more windows
            .map(|i| {
                let t = (i / 2) as f32 / sr as f32;
                (2.0 * std::f32::consts::PI * 1000.0 * t).sin()
            })
            .collect();
        let result = analyze_spectral(&samples, sr);
        assert!(
            (result.centroid - 1000.0).abs() < 500.0,
            "Expected centroid near 1000 Hz, got {}",
            result.centroid
        );
    }

    #[test]
    fn test_spectral_flatness_tonal_vs_noise() {
        let sr = 48000u32;
        // Pure sine = very tonal = low flatness
        let sine: Vec<f32> = (0..sr as usize * 2)
            .map(|i| {
                (2.0 * std::f32::consts::PI * 440.0 * (i / 2) as f32 / sr as f32).sin()
            })
            .collect();
        let sine_result = analyze_spectral(&sine, sr);

        // Pseudo-noise = high flatness
        let noise: Vec<f32> = (0..sr as usize * 2)
            .map(|i| rand_simple(i as u64) as f32 * 2.0 - 1.0)
            .collect();
        let noise_result = analyze_spectral(&noise, sr);

        assert!(
            sine_result.flatness < noise_result.flatness,
            "Sine flatness {} should be less than noise flatness {}",
            sine_result.flatness,
            noise_result.flatness
        );
    }

    #[test]
    fn test_band_energy_distribution() {
        let sr = 48000u32;
        // Low frequency signal (100 Hz)
        let low: Vec<f32> = (0..sr as usize * 2)
            .map(|i| {
                (2.0 * std::f32::consts::PI * 100.0 * (i / 2) as f32 / sr as f32).sin()
            })
            .collect();
        let result = analyze_spectral(&low, sr);
        assert!(
            result.bass_energy > result.high_energy,
            "100 Hz signal should have more bass ({}) than treble ({})",
            result.bass_energy,
            result.high_energy
        );
    }

    #[test]
    fn test_empty_samples() {
        let result = analyze_spectral(&[], 48000);
        assert_eq!(result.centroid, 0.0);
        assert_eq!(result.flatness, 0.0);
    }

    fn rand_simple(seed: u64) -> f64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);
        (hasher.finish() as f64) / u64::MAX as f64
    }
}
