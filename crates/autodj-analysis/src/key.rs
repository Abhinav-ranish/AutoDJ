//! Key detection (Placeholder)

use crate::profile::{KeyCandidate, KeyInfo};

/// Detects the musical key of the track.
/// Full implementation requires STFT chromagram and Krumhansl-Kessler profiles.
/// Returning a placeholder for now.
pub fn detect_key(_samples: &[f32], _sample_rate: u32) -> KeyInfo {
    KeyInfo {
        key: "C".to_string(),
        camelot: "8B".to_string(),
        confidence: 0.1,
        candidates: vec![
            KeyCandidate {
                key: "C".to_string(),
                camelot: "8B".to_string(),
                confidence: 0.1,
            }
        ],
    }
}
