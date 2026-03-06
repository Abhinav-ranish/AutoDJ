//! Section segmentation

use crate::profile::Section;

/// Divide the track into functional sections.
/// For this implementation, we use a simple heuristic (intro, body, outro)
/// based on the total duration.
pub fn analyze_sections(duration_ms: u64) -> Vec<Section> {
    if duration_ms < 30000 {
        // Too short for meaningful sections
        return vec![Section {
            label: "body".to_string(),
            start_ms: 0,
            end_ms: duration_ms,
            confidence: 0.1,
        }];
    }

    let intro_len_ms = (duration_ms as f64 * 0.15).min(30000.0) as u64;
    let outro_len_ms = (duration_ms as f64 * 0.15).min(30000.0) as u64;

    vec![
        Section {
            label: "intro".to_string(),
            start_ms: 0,
            end_ms: intro_len_ms,
            confidence: 0.5,
        },
        Section {
            label: "body".to_string(),
            start_ms: intro_len_ms,
            end_ms: duration_ms - outro_len_ms,
            confidence: 0.5,
        },
        Section {
            label: "outro".to_string(),
            start_ms: duration_ms - outro_len_ms,
            end_ms: duration_ms,
            confidence: 0.5,
        },
    ]
}
