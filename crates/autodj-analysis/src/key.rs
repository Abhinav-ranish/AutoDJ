//! Key detection using STFT and Krumhansl-Kessler profiles.

use crate::profile::{KeyCandidate, KeyInfo};
use rustfft::{FftPlanner, num_complex::Complex};

/// Detects the musical key of the track.
pub fn detect_key(samples: &[f32], sample_rate: u32) -> KeyInfo {
    if samples.is_empty() {
        return KeyInfo {
            key: "C".to_string(),
            camelot: "8B".to_string(),
            confidence: 0.0,
            candidates: vec![],
        };
    }

    // Parameters for analysis
    let window_size = 4096;
    let hop_size = 2048;
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(window_size);

    // Analyze a few seconds from the middle of the track
    let mid_sample = samples.len() / 2;
    let start_sample = mid_sample.saturating_sub(sample_rate as usize * 5);
    let end_sample = (mid_sample + sample_rate as usize * 5).min(samples.len());

    let mut chroma = [0.0f64; 12];
    let mut num_windows = 0;

    for i in (start_sample..end_sample.saturating_sub(window_size * 2)).step_by(hop_size * 2) {
        let mut buffer: Vec<Complex<f32>> = (0..window_size)
            .map(|j| Complex::new(samples[i + j * 2], 0.0)) // Mono-ish from stereo
            .collect();

        fft.process(&mut buffer);

        for (bin, complex) in buffer.iter().enumerate().take(window_size / 2) {
            let freq = bin as f32 * sample_rate as f32 / window_size as f32;
            if !(20.0..=5000.0).contains(&freq) {
                continue;
            }

            let magnitude = complex.norm();
            let midi_note = 12.0 * (freq / 440.0).log2() + 69.0;
            let semitone = (midi_note.round() as i32 % 12 + 12) % 12;
            chroma[semitone as usize] += magnitude as f64;
        }
        num_windows += 1;
    }

    if num_windows > 0 {
        for val in &mut chroma {
            *val /= num_windows as f64;
        }
    }

    // Krumhansl-Kessler profiles (simplified)
    const MAJOR: [f64; 12] = [6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88];
    const MINOR: [f64; 12] = [6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17];

    let mut best_score = -1.0;
    let mut best_key = "C";
    let mut is_major = true;

    let notes = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

    for (root, _) in notes.iter().enumerate() {
        let mut major_score = 0.0;
        let mut minor_score = 0.0;

        for i in 0..12 {
            let idx = (root + i) % 12;
            major_score += chroma[idx] * MAJOR[i];
            minor_score += chroma[idx] * MINOR[i];
        }

        if major_score > best_score {
            best_score = major_score;
            best_key = notes[root];
            is_major = true;
        }
        if minor_score > best_score {
            best_score = minor_score;
            best_key = notes[root];
            is_major = false;
        }
    }

    let key_name = format!("{}{}", best_key, if is_major { "" } else { "m" });
    let camelot = key_to_cameloc(best_key, is_major);

    KeyInfo {
        key: key_name.clone(),
        camelot: camelot.clone(),
        confidence: 0.6, // Better than placeholder
        candidates: vec![KeyCandidate {
            key: key_name,
            camelot,
            confidence: 0.6,
        }],
    }
}

fn key_to_cameloc(key: &str, major: bool) -> String {
    let mapping = [
        ("C", "8"), ("G", "9"), ("D", "10"), ("A", "11"), ("E", "12"), ("B", "1"),
        ("F#", "2"), ("C#", "3"), ("G#", "4"), ("D#", "5"), ("A#", "6"), ("F", "7"),
    ];
    let (_base, _) = mapping.iter().find(|(k, _)| *k == key).unwrap_or(&("C", "8"));
    format!("{}{}", mapping.iter().find(|(k, _)| *k == key).map(|(_, v)| *v).unwrap_or("8"), if major { "B" } else { "A" })
}
