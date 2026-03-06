//! Beat tracking downbeat detection

use crate::profile::BeatInfo;

/// Generates a beat grid given the samples and a known BPM.
/// A real implementation would use DP, but here we synthesize a grid from the 
/// first major onset.
pub fn track_beats(samples: &[f32], sample_rate: u32, bpm: f64) -> BeatInfo {
    if samples.is_empty() || bpm <= 0.0 {
        return BeatInfo {
            count: 0,
            times_ms: vec![],
            downbeat_indices: vec![],
            confidence: 0.0,
        };
    }

    // Find the first major peak (kick drum) in the first 2 seconds to align the grid
    let search_samples = (sample_rate * 2 * 2).min(samples.len() as u32) as usize; // up to 2 sec stereo
    let mut max_val = 0.0;
    let mut first_beat_sample = 0;

    for i in (0..search_samples).step_by(2) {
        let v = samples[i].abs() + samples[i + 1].abs();
        if v > max_val {
            max_val = v;
            first_beat_sample = i / 2;
        }
    }

    let samples_per_beat = (sample_rate as f64 * 60.0 / bpm) as usize;
    let mut times_ms = Vec::new();
    let mut downbeat_indices = Vec::new();
    
    let total_frames = samples.len() / 2;
    let mut current_frame = first_beat_sample;
    let mut beat_idx = 0;

    while current_frame < total_frames {
        let ms = (current_frame as u64 * 1000) / sample_rate as u64;
        times_ms.push(ms);
        
        // Assume 4/4 time
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
        confidence: 0.5, // synthetic grid is medium confidence
    }
}
