//! Rhythm analysis: onset regularity, complexity, and danceability.

/// Result of rhythm analysis.
pub struct RhythmInfo {
    pub complexity: f64,
    pub danceability: f64,
}

/// Analyze rhythm characteristics of a track.
///
/// Uses onset detection via energy deltas to measure:
/// - **Complexity**: variance of inter-onset intervals (0.0 = metronomic, 1.0 = chaotic)
/// - **Danceability**: combination of regularity, onset density, and tempo suitability (0.0–1.0)
pub fn analyze_rhythm(samples: &[f32], sample_rate: u32, bpm: f64) -> RhythmInfo {
    if samples.is_empty() || bpm <= 0.0 {
        return RhythmInfo {
            complexity: 0.0,
            danceability: 0.0,
        };
    }

    // Detect onsets via energy delta (10ms hop, stereo interleaved)
    let hop_samples = (sample_rate as f32 * 0.01) as usize * 2;
    if hop_samples == 0 {
        return RhythmInfo {
            complexity: 0.0,
            danceability: 0.0,
        };
    }

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
        return RhythmInfo {
            complexity: 0.0,
            danceability: 0.0,
        };
    }

    // Inter-onset intervals
    let intervals: Vec<f64> = onset_times.windows(2).map(|w| w[1] - w[0]).collect();
    let avg_interval = intervals.iter().sum::<f64>() / intervals.len() as f64;
    let interval_std = (intervals
        .iter()
        .map(|i| (i - avg_interval).powi(2))
        .sum::<f64>()
        / intervals.len() as f64)
        .sqrt();

    // Regularity: low std = regular = danceable
    let regularity = if avg_interval > 1e-5 {
        (1.0 - (interval_std / avg_interval).min(1.0)).max(0.0)
    } else {
        0.0
    };

    // Complexity: higher std relative to mean = more complex rhythm
    let complexity = (interval_std / avg_interval.max(0.01)).min(1.0);

    // Onset density (onsets per second of audio)
    let duration_secs = samples.len() as f64 / (sample_rate as f64 * 2.0);
    let strong_onsets = onset_times.len() as f64 / duration_secs;
    let onset_density = (strong_onsets / 10.0).min(1.0); // normalize: 10 onsets/sec = max

    // Tempo suitability for dancing
    let tempo_score = if (90.0..=140.0).contains(&bpm) {
        1.0
    } else if (80.0..=160.0).contains(&bpm) {
        0.7
    } else {
        0.3
    };

    let danceability =
        (regularity * 0.5 + onset_density * 0.3 + tempo_score * 0.2).clamp(0.0, 1.0);

    RhythmInfo {
        complexity,
        danceability,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regular_beats_high_danceability() {
        // Perfect kick pattern = high danceability
        let sr = 48000u32;
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
        assert!(
            result.danceability > 0.3,
            "Regular beats should be danceable, got {}",
            result.danceability
        );
    }

    #[test]
    fn test_silence_low_danceability() {
        let samples = vec![0.0f32; 48000 * 4];
        let result = analyze_rhythm(&samples, 48000, 120.0);
        assert!(
            result.danceability < 0.3,
            "Silence should have low danceability, got {}",
            result.danceability
        );
    }

    #[test]
    fn test_empty_samples() {
        let result = analyze_rhythm(&[], 48000, 120.0);
        assert_eq!(result.complexity, 0.0);
        assert_eq!(result.danceability, 0.0);
    }

    #[test]
    fn test_zero_bpm() {
        let samples = vec![0.5f32; 48000 * 4];
        let result = analyze_rhythm(&samples, 48000, 0.0);
        assert_eq!(result.danceability, 0.0);
    }
}
