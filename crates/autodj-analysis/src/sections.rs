//! Section segmentation using energy curve analysis.

use crate::profile::Section;

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
}
