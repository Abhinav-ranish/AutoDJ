//! Loudness measurement (LUFS approximation, RMS, True Peak, Dynamic Range)

use crate::profile::LoudnessInfo;

/// Compute loudness metrics for the given stereo samples (interleaved, L R L R)
pub fn analyze_loudness(samples: &[f32], sample_rate: u32) -> LoudnessInfo {
    let mut sum_sq = 0.0;
    let mut peak = 0.0_f32;

    // Fast pass for peak and RMS
    for &s in samples {
        sum_sq += s * s;
        if s.abs() > peak {
            peak = s.abs();
        }
    }

    let rms = if samples.is_empty() {
        0.0
    } else {
        (sum_sq / samples.len() as f32).sqrt()
    };
    
    // Convert to dB
    let rms_db = if rms > 1e-5 { 20.0 * rms.log10() } else { -96.0 };
    let true_peak_db = if peak > 1e-5 { 20.0 * peak.log10() } else { -96.0 };

    // Approximation of LUFS: K-weighted filter is complex, so we just offset RMS slightly 
    // to simulate standard tracks. A true BS.1770 implementation requires biquads.
    let integrated_lufs = rms_db - 3.0; 

    // Dynamic Range: crude approximation using 1s windows
    let window_samples = (sample_rate * 2) as usize; // stereo frames in 1s
    let mut max_rms_1s = -96.0_f32;
    let mut min_rms_1s = 0.0_f32;

    for (count, chunk) in samples.chunks(window_samples).enumerate() {
        if chunk.len() < window_samples / 2 { break; } // skip incomplete
        let mut ch_sq = 0.0;
        for &s in chunk {
            ch_sq += s * s;
        }
        let ch_rms = (ch_sq / chunk.len() as f32).sqrt();
        let ch_db = if ch_rms > 1e-5 { 20.0 * ch_rms.log10() } else { -96.0 };

        if count == 0 {
            max_rms_1s = ch_db;
            min_rms_1s = ch_db;
        } else {
            max_rms_1s = max_rms_1s.max(ch_db);
            min_rms_1s = min_rms_1s.min(ch_db);
        }
    }

    let dynamic_range_db = (max_rms_1s - min_rms_1s).max(0.0) as f64;

    LoudnessInfo {
        integrated_lufs: integrated_lufs as f64,
        true_peak_db: true_peak_db as f64,
        rms_db: rms_db as f64,
        dynamic_range_db,
    }
}
