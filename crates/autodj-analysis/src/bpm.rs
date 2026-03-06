//! BPM Detection via Onset Autocorrelation

use crate::profile::{TempoCandidate, TempoInfo};
use rustfft::{num_complex::Complex, FftPlanner};

/// Detect BPM using a simple RMS onset envelope + autocorrelation
pub fn detect_bpm(samples: &[f32], sample_rate: u32) -> TempoInfo {
    if samples.is_empty() {
        return TempoInfo {
            bpm: 120.0,
            confidence: 0.0,
            candidates: vec![],
            variable_tempo: false,
        };
    }

    // 10ms hops
    let hop_samples = (sample_rate as f32 * 0.01) as usize * 2; // stereo
    let mut onsets = Vec::new();

    let chunks: Vec<&[f32]> = samples.chunks(hop_samples).collect();
    for chunk in &chunks {
        let mut sq = 0.0;
        for &s in *chunk {
            sq += s * s;
        }
        onsets.push((sq / chunk.len() as f32).sqrt());
    }

    // First derivative of RMS envelope (positive changes only)
    let mut flux = vec![0.0f32; onsets.len()];
    for i in 1..onsets.len() {
        let diff = onsets[i] - onsets[i - 1];
        flux[i] = diff.max(0.0);
    }

    // To compute autocorrelation fast, we use FFT: IFFT(FFT(x) * conj(FFT(x)))
    // For a real track, we only need maybe the first 30 seconds to guess BPM
    // Let's take up to 3000 hops (30 seconds)
    let max_len = flux.len().min(3000);
    let mut signal = vec![Complex { re: 0.0f32, im: 0.0f32 }; max_len.next_power_of_two() * 2];
    for i in 0..max_len {
        signal[i].re = flux[i];
    }

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(signal.len());
    let ifft = planner.plan_fft_inverse(signal.len());

    fft.process(&mut signal);
    for c in &mut signal {
        *c = *c * c.conj();
    }
    ifft.process(&mut signal);

    // Analyze autocorrelation for peaks in the 80 - 160 BPM range
    // 1 hop = 0.01 sec.
    // 80 BPM = 1.33 beats/sec = 0.75 sec/beat = 75 hops
    // 160 BPM = 2.66 beats/sec = 0.375 sec/beat = 37.5 hops
    
    let min_lag = 37; // ~162 BPM
    let max_lag = 75; // ~80 BPM

    let mut best_bpm = 120.0;
    let mut best_score = 0.0;
    let mut candidates = Vec::new();

    if signal.len() > max_lag {
        let max_val = signal[min_lag..=max_lag].iter().map(|c| c.re).fold(0.0f32, |a, b| a.max(b));
        
        for lag in min_lag..=max_lag {
            let score = signal[lag].re;
            // Simple peak picking
            if lag > min_lag && lag < max_lag {
                if score > signal[lag - 1].re && score > signal[lag + 1].re {
                    let bpm = 60.0 / (lag as f64 * 0.01);
                    let normalized_score = (score / max_val.max(1e-5)) as f64;
                    candidates.push(TempoCandidate {
                        bpm,
                        confidence: normalized_score,
                    });
                    if normalized_score > best_score {
                        best_score = normalized_score;
                        best_bpm = bpm;
                    }
                }
            }
        }
    }

    // Sort candidates by confidence
    candidates.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

    TempoInfo {
        bpm: if best_score > 0.0 { best_bpm } else { 120.0 },
        confidence: best_score,
        candidates,
        variable_tempo: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bpm_synthetic_120() {
        let sr = 48000;
        let sec = 4.0;
        let mut samples = vec![0.0f32; (sr as f32 * sec * 2.0) as usize];
        
        // 120 BPM = 2 beats per second = every 0.5 seconds
        // Generate impulsive "kicks" every 0.5 seconds
        let beat_samples = (sr as f64 * 0.5) as usize;
        for i in (0..(samples.len() / 2)).step_by(beat_samples) {
            for j in 0..100 {
                let idx = (i + j) * 2;
                if idx + 1 < samples.len() {
                    samples[idx] = 1.0;
                    samples[idx + 1] = 1.0;
                }
            }
        }

        let info = detect_bpm(&samples, sr);
        // Should detect ~120 BPM
        assert!((info.bpm - 120.0).abs() <= 2.0, "Expected ~120 BPM, got {}", info.bpm);
    }
}
