//! Heuristic scorer for evaluating track transition candidates.

use crate::AiConfig;
use crate::camelot::camelot_distance;
use autodj_analysis::profile::TrackProfile;
use std::cmp::Ordering;

/// Enriched candidate data for scoring (combines profile + classification).
pub struct ScoringCandidate<'a> {
    pub profile: &'a TrackProfile,
    pub genre: Option<&'a str>,
    pub mood: Option<&'a str>,
}

pub struct HeuristicScorer {
    config: AiConfig,
}

impl HeuristicScorer {
    pub fn new(config: AiConfig) -> Self {
        Self { config }
    }

    /// Select the best next track from a list of enriched candidates.
    pub fn select_next<'a>(
        &self,
        current: &ScoringCandidate<'_>,
        candidates: &'a [ScoringCandidate<'a>],
        recent_tracks: &[String],
        intent: &str,
    ) -> Option<&'a TrackProfile> {
        if candidates.is_empty() {
            return None;
        }

        let mut scored: Vec<(&TrackProfile, f64)> = candidates
            .iter()
            .map(|c| (c.profile, self.score(current, c, recent_tracks, intent)))
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        Some(scored[0].0)
    }

    /// Calculate a compatibility score [0.0 - 1.0] for transitioning.
    pub fn score(
        &self,
        current: &ScoringCandidate<'_>,
        candidate: &ScoringCandidate<'_>,
        recent_tracks: &[String],
        intent: &str,
    ) -> f64 {
        let cur = current.profile;
        let cand = candidate.profile;

        let current_tempo = cur.perceived_tempo.as_ref()
            .map(|pt| pt.perceived_tempo)
            .unwrap_or(cur.tempo.bpm);
        let candidate_tempo = cand.perceived_tempo.as_ref()
            .map(|pt| pt.perceived_tempo)
            .unwrap_or(cand.tempo.bpm);

        let s_bpm = self.score_tempo(current_tempo, candidate_tempo);
        let s_key = self.score_key(&cur.key.camelot, &cand.key.camelot);
        let s_energy = self.score_energy(cur.energy.average, cand.energy.average, intent);
        let s_variety = self.score_variety(&cand.track_id, recent_tracks);
        let s_genre = self.score_genre(current.genre, candidate.genre);
        let s_mood = self.score_mood(current.mood, candidate.mood);
        let s_spectral = self.score_spectral(
            cur.spectral.as_ref().map(|s| s.centroid),
            cand.spectral.as_ref().map(|s| s.centroid),
        );

        (s_bpm * self.config.weight_tempo)
            + (s_key * self.config.weight_key)
            + (s_energy * self.config.weight_energy)
            + (s_variety * self.config.weight_variety)
            + (s_genre * self.config.weight_genre)
            + (s_mood * self.config.weight_mood)
            + (s_spectral * self.config.weight_spectral)
    }

    fn score_tempo(&self, bpm_a: f64, bpm_b: f64) -> f64 {
        if bpm_a <= 0.0 || bpm_b <= 0.0 {
            return 0.5;
        }
        let diff = (bpm_a - bpm_b).abs();
        (1.0 - (diff / 20.0)).clamp(0.0, 1.0)
    }

    fn score_key(&self, camelot_a: &str, camelot_b: &str) -> f64 {
        let distance = camelot_distance(camelot_a, camelot_b);
        match distance {
            0 => 1.0,
            1 => 0.8,
            2 => 0.5,
            3 => 0.4,
            _ => 0.1,
        }
    }

    fn score_energy(&self, energy_a: f64, energy_b: f64, intent: &str) -> f64 {
        let diff = energy_b - energy_a;
        match intent.to_lowercase().as_str() {
            "build" => if diff > 0.0 && diff < 0.3 { 1.0 } else { 0.5 },
            "drop" => if diff > 0.3 { 1.0 } else { 0.2 },
            "relax" | "breakdown" => if diff < -0.1 { 1.0 } else { 0.3 },
            _ => if diff.abs() < 0.15 { 1.0 } else { 0.5 },
        }
    }

    fn score_variety(&self, candidate_id: &str, recent_tracks: &[String]) -> f64 {
        if recent_tracks.contains(&candidate_id.to_string()) { 0.0 } else { 1.0 }
    }

    fn score_genre(&self, genre_a: Option<&str>, genre_b: Option<&str>) -> f64 {
        match (genre_a, genre_b) {
            (Some(a), Some(b)) if a.eq_ignore_ascii_case(b) => 1.0,
            (Some(a), Some(b)) if genres_related(a, b) => 0.7,
            (Some(_), Some(_)) => 0.3,
            _ => 0.5,
        }
    }

    fn score_mood(&self, mood_a: Option<&str>, mood_b: Option<&str>) -> f64 {
        match (mood_a, mood_b) {
            (Some(a), Some(b)) if a.eq_ignore_ascii_case(b) => 1.0,
            (Some(a), Some(b)) if moods_adjacent(a, b) => 0.8,
            (Some(_), Some(_)) => 0.4,
            _ => 0.5,
        }
    }

    fn score_spectral(&self, centroid_a: Option<f64>, centroid_b: Option<f64>) -> f64 {
        match (centroid_a, centroid_b) {
            (Some(a), Some(b)) => {
                let diff = (a - b).abs() / a.max(b).max(1.0);
                if diff < 0.1 { 0.6 }
                else if diff < 0.3 { 1.0 }
                else { 0.4 }
            }
            _ => 0.5,
        }
    }
}

/// Check if two genres belong to the same family.
fn genres_related(a: &str, b: &str) -> bool {
    let groups: &[&[&str]] = &[
        &["house", "deep house", "tech house", "progressive house", "electro house"],
        &["techno", "minimal techno", "industrial techno", "acid techno"],
        &["hip-hop", "trap", "drill", "boom bap", "lo-fi hip-hop"],
        &["drum-and-bass", "jungle", "breakbeat", "liquid dnb"],
        &["pop", "dance-pop", "electro-pop", "synth-pop"],
        &["r&b", "neo-soul", "funk", "disco"],
        &["latin", "reggaeton", "afrobeats", "dancehall"],
        &["ambient", "downtempo", "chillout", "lo-fi"],
        &["dubstep", "riddim", "future bass", "bass music"],
        &["trance", "psytrance", "progressive trance", "uplifting trance"],
    ];
    let a_low = a.to_lowercase();
    let b_low = b.to_lowercase();
    groups.iter().any(|group| {
        group.iter().any(|g| *g == a_low) && group.iter().any(|g| *g == b_low)
    })
}

/// Check if two moods are adjacent on the mood wheel.
fn moods_adjacent(a: &str, b: &str) -> bool {
    let adjacency: &[(&str, &[&str])] = &[
        ("euphoric", &["uplifting", "energetic"]),
        ("uplifting", &["euphoric", "chill", "dreamy"]),
        ("energetic", &["euphoric", "aggressive"]),
        ("chill", &["uplifting", "dreamy", "sensual"]),
        ("dark", &["aggressive", "mysterious"]),
        ("aggressive", &["dark", "energetic"]),
        ("melancholic", &["dreamy", "mysterious"]),
        ("dreamy", &["chill", "melancholic", "uplifting"]),
        ("sensual", &["chill", "mysterious"]),
        ("mysterious", &["dark", "melancholic", "sensual"]),
    ];
    let a_low = a.to_lowercase();
    let b_low = b.to_lowercase();
    adjacency.iter().any(|(mood, adj)| {
        (*mood == a_low && adj.contains(&b_low.as_str()))
            || (*mood == b_low && adj.contains(&a_low.as_str()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genres_related() {
        assert!(genres_related("house", "deep house"));
        assert!(genres_related("techno", "minimal techno"));
        assert!(!genres_related("house", "hip-hop"));
        assert!(genres_related("hip-hop", "trap"));
    }

    #[test]
    fn test_moods_adjacent() {
        assert!(moods_adjacent("euphoric", "uplifting"));
        assert!(moods_adjacent("dark", "aggressive"));
        assert!(!moods_adjacent("euphoric", "dark"));
        assert!(moods_adjacent("chill", "dreamy"));
    }

    #[test]
    fn test_genres_case_insensitive() {
        assert!(genres_related("House", "Deep House"));
        assert!(genres_related("TECHNO", "minimal techno"));
    }
}
