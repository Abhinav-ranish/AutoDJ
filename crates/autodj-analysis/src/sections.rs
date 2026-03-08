//! Section segmentation using energy curve analysis.

use crate::profile::{AutoCue, PhraseInfo, Section, TransitionWindow};

/// Divide the track into functional sections based on the energy curve.
///
/// Uses sustained energy changes to detect intro/buildup/drop/breakdown/outro
/// boundaries rather than fixed percentages.
pub fn analyze_sections(duration_ms: u64, energy_curve: &[f64]) -> Vec<Section> {
    if duration_ms < 30000 || energy_curve.len() < 4 {
        return vec![Section {
            label: "body".to_string(),
            start_ms: 0,
            end_ms: duration_ms,
            confidence: 0.1,
        }];
    }

    let ms_per_window = duration_ms as f64 / energy_curve.len() as f64;
    let avg_energy: f64 = energy_curve.iter().sum::<f64>() / energy_curve.len() as f64;

    // Find intro end: first point where energy rises above 50% of average
    // and stays there for at least 3 windows
    let intro_threshold = avg_energy * 0.5;
    let intro_end_idx = find_sustained_rise(energy_curve, intro_threshold, 3)
        .unwrap_or(0);

    // Find outro start: last point where energy drops below 50% of average
    // and stays there for at least 3 windows
    let reversed: Vec<f64> = energy_curve.iter().rev().copied().collect();
    let outro_offset = find_sustained_rise(&reversed, intro_threshold, 3)
        .unwrap_or(0);
    let outro_start_idx = energy_curve.len().saturating_sub(outro_offset);

    // Find the first major energy peak (drop) after intro
    let drop_idx = find_first_peak(energy_curve, intro_end_idx, avg_energy * 1.3);

    // Find breakdown: significant energy dip between drop and outro
    let breakdown_idx = if let Some(di) = drop_idx {
        find_energy_dip(energy_curve, di, outro_start_idx, avg_energy * 0.6)
    } else {
        None
    };

    let idx_to_ms = |idx: usize| -> u64 {
        ((idx as f64 * ms_per_window) as u64).min(duration_ms)
    };

    let mut sections = Vec::new();

    // Intro
    if intro_end_idx > 0 {
        sections.push(Section {
            label: "intro".to_string(),
            start_ms: 0,
            end_ms: idx_to_ms(intro_end_idx),
            confidence: 0.7,
        });
    }

    // Buildup (between intro end and drop)
    let body_start = idx_to_ms(intro_end_idx);
    if let Some(di) = drop_idx {
        if di > intro_end_idx + 2 {
            sections.push(Section {
                label: "buildup".to_string(),
                start_ms: body_start,
                end_ms: idx_to_ms(di),
                confidence: 0.6,
            });
        }

        // Drop
        let drop_end = breakdown_idx.unwrap_or(outro_start_idx);
        sections.push(Section {
            label: "drop".to_string(),
            start_ms: idx_to_ms(di),
            end_ms: idx_to_ms(drop_end),
            confidence: 0.7,
        });

        // Breakdown (if detected)
        if let Some(bd) = breakdown_idx {
            sections.push(Section {
                label: "breakdown".to_string(),
                start_ms: idx_to_ms(bd),
                end_ms: idx_to_ms(outro_start_idx),
                confidence: 0.6,
            });
        }
    } else {
        // No clear drop detected - just use "body"
        sections.push(Section {
            label: "body".to_string(),
            start_ms: body_start,
            end_ms: idx_to_ms(outro_start_idx),
            confidence: 0.5,
        });
    }

    // Outro
    if outro_start_idx < energy_curve.len() {
        sections.push(Section {
            label: "outro".to_string(),
            start_ms: idx_to_ms(outro_start_idx),
            end_ms: duration_ms,
            confidence: 0.7,
        });
    }

    // Ensure we cover the full track (fill gaps)
    if sections.is_empty() {
        sections.push(Section {
            label: "body".to_string(),
            start_ms: 0,
            end_ms: duration_ms,
            confidence: 0.3,
        });
    }

    sections
}

/// Find the first index where the signal rises above threshold
/// and stays above it for at least `sustain` consecutive windows.
fn find_sustained_rise(curve: &[f64], threshold: f64, sustain: usize) -> Option<usize> {
    let mut count = 0;
    for (i, &val) in curve.iter().enumerate() {
        if val >= threshold {
            count += 1;
            if count >= sustain {
                return Some(i - sustain + 1);
            }
        } else {
            count = 0;
        }
    }
    None
}

/// Find the first major energy peak after a given index.
fn find_first_peak(curve: &[f64], start: usize, threshold: f64) -> Option<usize> {
    for i in start..curve.len().saturating_sub(1) {
        if curve[i] >= threshold && curve[i] >= curve[i.saturating_sub(1)] {
            return Some(i);
        }
    }
    None
}

/// Find a significant energy dip between two indices.
fn find_energy_dip(
    curve: &[f64],
    start: usize,
    end: usize,
    threshold: f64,
) -> Option<usize> {
    if start >= end || end > curve.len() {
        return None;
    }
    // Look for the first sustained drop below threshold
    let mut below_count = 0;
    for i in start..end {
        if curve[i] < threshold {
            below_count += 1;
            if below_count >= 3 {
                return Some(i - 2);
            }
        } else {
            below_count = 0;
        }
    }
    None
}

// ─── Phase 19: Transition Windows ───────────────────────────────

/// Generate candidate transition windows from section boundaries and markers.
pub fn generate_transition_windows(
    sections: &[Section],
    duration_ms: u64,
    _intro_length_ms: Option<u64>,
    outro_length_ms: Option<u64>,
    first_drop_ms: Option<u64>,
    bpm: f64,
) -> Vec<TransitionWindow> {
    let mut windows = Vec::new();
    let bar_ms = if bpm > 0.0 { 60_000.0 / bpm * 4.0 } else { 2000.0 };

    // 1. Outro start is the safest transition point
    if let Some(outro_ms) = outro_length_ms {
        let outro_start = duration_ms.saturating_sub(outro_ms);
        if outro_start > 0 {
            windows.push(TransitionWindow {
                anchor: "outro_start".to_string(),
                start_ms: outro_start,
                end_ms: (outro_start + (bar_ms * 4.0) as u64).min(duration_ms),
                score: 1.0,
            });
        }
    }

    // From section boundaries
    for section in sections {
        match section.label.as_str() {
            "outro" => {
                windows.push(TransitionWindow {
                    anchor: "outro_section".to_string(),
                    start_ms: section.start_ms,
                    end_ms: (section.start_ms + (bar_ms * 4.0) as u64).min(section.end_ms),
                    score: 0.95,
                });
            }
            "breakdown" => {
                windows.push(TransitionWindow {
                    anchor: "break_start".to_string(),
                    start_ms: section.start_ms,
                    end_ms: (section.start_ms + (bar_ms * 4.0) as u64).min(section.end_ms),
                    score: 0.8,
                });
            }
            "drop" => {
                // Pre-drop windows (8 and 16 bars before the drop)
                let pre_16 = section.start_ms.saturating_sub((bar_ms * 16.0) as u64);
                let pre_8 = section.start_ms.saturating_sub((bar_ms * 8.0) as u64);
                if pre_16 > 0 {
                    windows.push(TransitionWindow {
                        anchor: "pre_drop_16".to_string(),
                        start_ms: pre_16,
                        end_ms: pre_16 + (bar_ms * 2.0) as u64,
                        score: 0.6,
                    });
                }
                if pre_8 > 0 {
                    windows.push(TransitionWindow {
                        anchor: "pre_drop_8".to_string(),
                        start_ms: pre_8,
                        end_ms: pre_8 + (bar_ms * 2.0) as u64,
                        score: 0.65,
                    });
                }
                // Post-drop window (8 bars after the drop starts)
                let post_8 = section.start_ms + (bar_ms * 8.0) as u64;
                if post_8 < section.end_ms {
                    windows.push(TransitionWindow {
                        anchor: "post_drop_8".to_string(),
                        start_ms: post_8,
                        end_ms: (post_8 + (bar_ms * 2.0) as u64).min(section.end_ms),
                        score: 0.5,
                    });
                }
            }
            _ => {}
        }
    }

    // 2. First drop — good for echo_drop transitions
    if let Some(drop_ms) = first_drop_ms {
        let pre = drop_ms.saturating_sub((bar_ms * 8.0) as u64);
        if pre > 0 && !windows.iter().any(|w| w.anchor == "pre_drop_8" && (w.start_ms as i64 - pre as i64).unsigned_abs() < bar_ms as u64) {
            windows.push(TransitionWindow {
                anchor: "pre_first_drop".to_string(),
                start_ms: pre,
                end_ms: pre + (bar_ms * 2.0) as u64,
                score: 0.7,
            });
        }
    }

    // Sort by score descending
    windows.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    windows
}

// ─── Phase 19: Phrase Grid ──────────────────────────────────────

/// Build a phrase grid from the beat grid and BPM.
/// Produces boundaries at every `phrase_bars` bars.
pub fn build_phrase_grid(
    bpm: f64,
    duration_ms: u64,
    beat_times_ms: &[u64],
    phrase_bars: u32,
) -> PhraseInfo {
    let phrase_beats = phrase_bars * 4; // 4/4 time
    let mut boundaries = Vec::new();

    if !beat_times_ms.is_empty() {
        // Use actual beat times for accuracy
        for (i, &t) in beat_times_ms.iter().enumerate() {
            if i % phrase_beats as usize == 0 {
                boundaries.push(t);
            }
        }
    } else if bpm > 0.0 {
        // Synthetic phrase grid from BPM
        let phrase_ms = 60_000.0 / bpm * phrase_beats as f64;
        let mut t = 0.0;
        while (t as u64) < duration_ms {
            boundaries.push(t as u64);
            t += phrase_ms;
        }
    }

    PhraseInfo {
        phrase_bars,
        boundaries_ms: boundaries,
    }
}

/// Find the next phrase boundary at or after `now_ms`.
pub fn next_phrase_boundary(phrases: &PhraseInfo, now_ms: u64) -> Option<u64> {
    phrases.boundaries_ms.iter().find(|&&b| b >= now_ms).copied()
}

// ─── Phase 19: Auto-Cue Generation ─────────────────────────────

/// Generate canonical cue markers from section analysis.
pub fn generate_auto_cues(
    sections: &[Section],
    first_drop_ms: Option<u64>,
    beat_times_ms: &[u64],
) -> Vec<AutoCue> {
    let mut cues = Vec::new();

    // CUE_FIRST_BEAT — the very first beat
    if let Some(&first_beat) = beat_times_ms.first() {
        cues.push(AutoCue {
            cue_type: "first_beat".to_string(),
            position_ms: first_beat,
            confidence: 0.9,
        });
    } else {
        cues.push(AutoCue {
            cue_type: "first_beat".to_string(),
            position_ms: 0,
            confidence: 0.5,
        });
    }

    for section in sections {
        match section.label.as_str() {
            "intro" => {
                cues.push(AutoCue {
                    cue_type: "intro".to_string(),
                    position_ms: section.start_ms,
                    confidence: section.confidence,
                });
            }
            "drop" => {
                cues.push(AutoCue {
                    cue_type: "drop".to_string(),
                    position_ms: section.start_ms,
                    confidence: section.confidence,
                });
            }
            "breakdown" => {
                cues.push(AutoCue {
                    cue_type: "break".to_string(),
                    position_ms: section.start_ms,
                    confidence: section.confidence,
                });
            }
            "outro" => {
                cues.push(AutoCue {
                    cue_type: "outro".to_string(),
                    position_ms: section.start_ms,
                    confidence: section.confidence,
                });
            }
            _ => {}
        }
    }

    // If first_drop_ms was detected but no drop section exists, add it
    if let Some(drop_ms) = first_drop_ms {
        if !cues.iter().any(|c| c.cue_type == "drop") {
            cues.push(AutoCue {
                cue_type: "drop".to_string(),
                position_ms: drop_ms,
                confidence: 0.6,
            });
        }
    }

    cues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_track() {
        let sections = analyze_sections(20000, &[0.5, 0.5]);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].label, "body");
    }

    #[test]
    fn test_energy_based_sections() {
        // Simulate: quiet intro, buildup, loud drop, quiet outro
        let mut curve = Vec::new();
        // Intro: low energy (10 windows)
        for _ in 0..10 { curve.push(0.1); }
        // Buildup (5 windows)
        for i in 0..5 { curve.push(0.2 + i as f64 * 0.15); }
        // Drop: high energy (15 windows)
        for _ in 0..15 { curve.push(0.9); }
        // Breakdown (5 windows)
        for _ in 0..5 { curve.push(0.2); }
        // Outro: low energy (10 windows)
        for _ in 0..10 { curve.push(0.1); }

        let sections = analyze_sections(225_000, &curve);
        assert!(sections.len() >= 3, "Expected at least 3 sections, got {}", sections.len());

        let labels: Vec<&str> = sections.iter().map(|s| s.label.as_str()).collect();
        assert!(labels.contains(&"intro"), "Missing intro: {:?}", labels);
        assert!(labels.contains(&"drop") || labels.contains(&"body"), "Missing drop/body: {:?}", labels);
        assert!(labels.contains(&"outro"), "Missing outro: {:?}", labels);
    }

    #[test]
    fn test_sustained_rise() {
        let curve = [0.1, 0.1, 0.5, 0.6, 0.7, 0.8];
        let idx = find_sustained_rise(&curve, 0.4, 3);
        assert_eq!(idx, Some(2));
    }

    #[test]
    fn test_transition_windows_from_sections() {
        let sections = vec![
            Section { label: "intro".into(), start_ms: 0, end_ms: 30000, confidence: 0.7 },
            Section { label: "drop".into(), start_ms: 60000, end_ms: 120000, confidence: 0.7 },
            Section { label: "outro".into(), start_ms: 180000, end_ms: 240000, confidence: 0.7 },
        ];
        let windows = generate_transition_windows(
            &sections, 240000, Some(60000), Some(60000), Some(60000), 128.0,
        );
        assert!(!windows.is_empty(), "Expected transition windows");
        // Outro start should be scored highest
        assert_eq!(windows[0].anchor, "outro_start");
    }

    #[test]
    fn test_phrase_grid_synthetic() {
        let phrases = build_phrase_grid(120.0, 120000, &[], 8);
        assert_eq!(phrases.phrase_bars, 8);
        // At 120 BPM, 8 bars = 32 beats = 16s. So 120s / 16s = ~7.5 boundaries
        assert!(phrases.boundaries_ms.len() >= 7);
        assert_eq!(phrases.boundaries_ms[0], 0);
    }

    #[test]
    fn test_next_phrase_boundary() {
        let phrases = PhraseInfo {
            phrase_bars: 8,
            boundaries_ms: vec![0, 16000, 32000, 48000],
        };
        assert_eq!(next_phrase_boundary(&phrases, 5000), Some(16000));
        assert_eq!(next_phrase_boundary(&phrases, 16000), Some(16000));
        assert_eq!(next_phrase_boundary(&phrases, 49000), None);
    }

    #[test]
    fn test_auto_cues_from_sections() {
        let sections = vec![
            Section { label: "intro".into(), start_ms: 0, end_ms: 30000, confidence: 0.7 },
            Section { label: "drop".into(), start_ms: 60000, end_ms: 120000, confidence: 0.7 },
            Section { label: "outro".into(), start_ms: 180000, end_ms: 240000, confidence: 0.7 },
        ];
        let cues = generate_auto_cues(&sections, Some(60000), &[0, 500, 1000]);
        let types: Vec<&str> = cues.iter().map(|c| c.cue_type.as_str()).collect();
        assert!(types.contains(&"first_beat"));
        assert!(types.contains(&"intro"));
        assert!(types.contains(&"drop"));
        assert!(types.contains(&"outro"));
    }
}
