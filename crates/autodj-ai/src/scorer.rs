//! Heuristic scorer for evaluating track transition candidates.

use crate::AiConfig;
use crate::camelot::camelot_distance;
use autodj_analysis::profile::TrackProfile;
use std::cmp::Ordering;

pub struct HeuristicScorer {
    config: AiConfig,
}

impl HeuristicScorer {
    pub fn new(config: AiConfig) -> Self {
        Self { config }
    }

    /// Select the best next track from a list of candidates.
    pub fn select_next<'a>(
        &self,
        current: &TrackProfile,
        candidates: &'a [TrackProfile],
        recent_tracks: &[String],
        intent: &str,
    ) -> Option<&'a TrackProfile> {
        if candidates.is_empty() {
            return None;
        }

        let mut scored_candidates: Vec<(&TrackProfile, f64)> = candidates
            .iter()
            .map(|c| (c, self.score(current, c, recent_tracks, intent)))
            .collect();

        // Sort descending
        scored_candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));

        Some(scored_candidates[0].0)
    }

    /// Calculate a compatibility score [0.0 - 1.0] for transitioning from `current` to `candidate`.
    pub fn score(
        &self,
        current: &TrackProfile,
        candidate: &TrackProfile,
        recent_tracks: &[String],
        intent: &str,
    ) -> f64 {
        // Prefer perceived tempo over raw BPM for groove-aware matching
        let current_tempo = current.perceived_tempo.as_ref()
            .map(|pt| pt.perceived_tempo)
            .unwrap_or(current.tempo.bpm);
        let candidate_tempo = candidate.perceived_tempo.as_ref()
            .map(|pt| pt.perceived_tempo)
            .unwrap_or(candidate.tempo.bpm);

        let score_bpm = self.score_tempo(current_tempo, candidate_tempo);
        let score_key = self.score_key(&current.key.camelot, &candidate.key.camelot);
        let score_energy = self.score_energy(current.energy.average, candidate.energy.average, intent);
        let score_variety = self.score_variety(&candidate.track_id, recent_tracks);

        (score_bpm * self.config.weight_tempo)
            + (score_key * self.config.weight_key)
            + (score_energy * self.config.weight_energy)
            + (score_variety * self.config.weight_variety)
    }

    fn score_tempo(&self, bpm_a: f64, bpm_b: f64) -> f64 {
        if bpm_a <= 0.0 || bpm_b <= 0.0 {
            return 0.5; // Unknown
        }
        let diff = (bpm_a - bpm_b).abs();
        // Perfect score for 0 diff, drops to 0 score at 20+ diff
        (1.0 - (diff / 20.0)).clamp(0.0, 1.0)
    }

    fn score_key(&self, camelot_a: &str, camelot_b: &str) -> f64 {
        let distance = camelot_distance(camelot_a, camelot_b);
        match distance {
            0 => 1.0, // Same key
            1 => 0.8, // Adjacent (+/- 1 hr, or major/minor mode change)
            2 => 0.5, // Energy boost (+2 hr)
            3 => 0.4, // Related
            _ => 0.1, // Key clash
        }
    }

    fn score_energy(&self, energy_a: f64, energy_b: f64, intent: &str) -> f64 {
        let diff = energy_b - energy_a;
        match intent.to_lowercase().as_str() {
            "build" => {
                if diff > 0.0 && diff < 0.3 { 1.0 } else { 0.5 }
            }
            "drop" => {
                if diff > 0.3 { 1.0 } else { 0.2 }
            }
            "relax" | "breakdown" => {
                if diff < -0.1 { 1.0 } else { 0.3 }
            }
            "maintain" | _ => {
                if diff.abs() < 0.15 { 1.0 } else { 0.5 }
            }
        }
    }

    fn score_variety(&self, candidate_id: &str, recent_tracks: &[String]) -> f64 {
        if recent_tracks.contains(&candidate_id.to_string()) {
            0.0 // Do not repeat tracks!
        } else {
            1.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock profile creation helper would go here
}
