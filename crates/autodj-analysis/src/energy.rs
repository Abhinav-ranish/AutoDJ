//! Energy curve + Drop candidates

use crate::profile::EnergyInfo;

/// Computes the energy curve over the given window size.
pub fn analyze_energy(samples: &[f32], sample_rate: u32, window_seconds: f64) -> EnergyInfo {
    let window_samples = (sample_rate as f64 * window_seconds * 2.0) as usize; // stereo
    if window_samples == 0 || samples.is_empty() {
        return EnergyInfo {
            window_seconds,
            curve: vec![],
            average: 0.0,
            peak: 0.0,
            drop_candidates_ms: vec![],
        };
    }

    let mut curve = Vec::new();
    let mut sum = 0.0;
    let mut peak = 0.0;

    for chunk in samples.chunks(window_samples) {
        let mut sq = 0.0;
        for &s in chunk {
            sq += s * s;
        }
        let rms = (sq / chunk.len() as f32).sqrt() as f64;
        curve.push(rms);
        sum += rms;
        if rms > peak {
            peak = rms;
        }
    }

    // Normalize curve safely
    if peak > 1e-5 {
        for v in &mut curve {
            *v /= peak;
        }
    }

    let average = if !curve.is_empty() { sum / curve.len() as f64 } else { 0.0 };
    if peak > 1e-5 {
        let _ = average / peak; // not strictly needed, just checking
    }

    // Drop detection: look for 2x energy increase over short period
    let mut drop_candidates_ms = Vec::new();
    let lookback_steps = (2.0 / window_seconds).ceil() as usize; // ~2 seconds lookback

    for i in lookback_steps..curve.len() {
        let current = curve[i];
        let mut min_recent = current;
        for j in 1..=lookback_steps {
            if curve[i - j] < min_recent {
                min_recent = curve[i - j];
            }
        }
        
        // If energy doubles after a quiet part, and it's quite loud overall
        if current > min_recent * 2.0 && current > 0.6 {
            let ms = (i as f64 * window_seconds * 1000.0) as u64;
            drop_candidates_ms.push(ms);
        }
    }

    EnergyInfo {
        window_seconds,
        curve,
        average: if peak > 1e-5 { average / peak } else { 0.0 }, // normalized avg
        peak: 1.0, // normalized max
        drop_candidates_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_energy_silence_vs_loud() {
        let sample_rate = 48000;
        let mut samples = vec![0.0; sample_rate as usize * 2]; // 1 sec silence
        samples.extend(vec![0.8; sample_rate as usize * 2]); // 1 sec loud
        
        let info = analyze_energy(&samples, sample_rate, 0.5);
        assert_eq!(info.curve.len(), 4);
        assert!(info.curve[0] < 1e-5);
        assert!(info.curve[1] < 1e-5);
        assert!((info.curve[2] - 1.0).abs() < 1e-2); // normalized max
        assert!((info.curve[3] - 1.0).abs() < 1e-2);
    }
}
