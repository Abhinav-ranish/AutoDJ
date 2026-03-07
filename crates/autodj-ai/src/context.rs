//! Builder for the AI context payload.

use autodj_analysis::profile::TrackProfile;
use autodj_core::engine::EngineStateSnapshot;
use serde::{Deserialize, Serialize};

/// The complete state context sent to the AI Conductor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiContext {
    pub current_state: CurrentStateContext,
    pub preferences: PreferencesContext,
    pub constraints: ConstraintsContext,
    pub available_tracks: Vec<TrackProfileShort>,
    pub set_flow: Option<SetFlowContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentStateContext {
    pub playing_deck: String,
    pub track_id: String,
    pub artist: String,
    pub title: String,
    pub bpm: f64,
    pub perceived_tempo: f64,
    pub groove_type: String,
    pub key: String,
    pub energy: f64,
    pub lufs_loudness: f64,
    pub elapsed_ms: u64,
    pub time_remaining_ms: u64,
    pub duration_ms: u64,
    pub phrase_length_ms: u64,
    // Phase 14: enriched fields
    pub genre: Option<String>,
    pub mood: Option<String>,
    pub vibe: Option<String>,
    pub vocal_presence: Option<f64>,
    pub energy_trajectory: Option<String>,
    pub intro_length_ms: Option<u64>,
    pub outro_length_ms: Option<u64>,
    pub first_drop_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferencesContext {
    pub intent: String,
    pub target_energy: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintsContext {
    pub max_bpm_difference: f64,
    pub allow_key_shift: bool,
    pub recent_tracks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackProfileShort {
    pub id: String,
    pub artist: String,
    pub title: String,
    pub bpm: f64,
    pub perceived_tempo: f64,
    pub groove_type: String,
    pub key: String,
    pub energy: f64,
    pub lufs_loudness: f64,
    pub duration_ms: u64,
    pub phrase_length_ms: u64,
    // Phase 14: enriched fields
    pub genre: Option<String>,
    pub subgenre: Option<String>,
    pub mood: Option<String>,
    pub vibe: Option<String>,
    pub mix_notes: Option<String>,
    pub spectral_centroid: Option<f64>,
    pub vocal_presence: Option<f64>,
    pub danceability: Option<f64>,
    pub energy_trajectory: Option<String>,
    pub intro_length_ms: Option<u64>,
    pub outro_length_ms: Option<u64>,
}

/// Set-level flow tracking for AI decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetFlowContext {
    pub tracks_played: u32,
    pub elapsed_minutes: f64,
    pub recent_genres: Vec<String>,
    pub recent_moods: Vec<String>,
    pub energy_trend: String,
}

/// Classification data for a track (loaded from DB).
#[derive(Debug, Clone, Default)]
pub struct TrackClassificationData {
    pub genre: Option<String>,
    pub subgenre: Option<String>,
    pub mood: Option<String>,
    pub vibe: Option<String>,
    pub mix_notes: Option<String>,
}

/// Input parameters for building AI context.
pub struct ContextInput<'a> {
    pub engine_state: &'a EngineStateSnapshot,
    pub playing_track_profile: &'a TrackProfile,
    pub playing_artist: &'a str,
    pub playing_title: &'a str,
    pub available_profiles: &'a [TrackProfile],
    pub intent: &'a str,
    pub target_energy: f64,
    pub recent_tracks: &'a [String],
    pub available_metadata: &'a std::collections::HashMap<String, (String, String)>,
    pub classifications: &'a std::collections::HashMap<String, TrackClassificationData>,
    pub set_flow: Option<SetFlowContext>,
}

pub fn build_context(input: &ContextInput) -> AiContext {
    let engine_state = input.engine_state;
    let playing_track_profile = input.playing_track_profile;
    let available_profiles = input.available_profiles;
    let available_metadata = input.available_metadata;

    let deck_a_playing = engine_state.deck_a.state == "playing";
    let playing_deck = if deck_a_playing { "A" } else { "B" };
    let deck_snapshot = if deck_a_playing { &engine_state.deck_a } else { &engine_state.deck_b };

    let time_remaining_ms = deck_snapshot.duration_ms.saturating_sub(deck_snapshot.position_ms);

    let playing_class = input.classifications
        .get(&playing_track_profile.track_id)
        .cloned()
        .unwrap_or_default();

    let current_state = CurrentStateContext {
        playing_deck: playing_deck.to_string(),
        track_id: playing_track_profile.track_id.clone(),
        artist: input.playing_artist.to_string(),
        title: input.playing_title.to_string(),
        bpm: playing_track_profile.tempo.bpm,
        perceived_tempo: playing_track_profile.perceived_tempo.as_ref()
            .map(|pt| pt.perceived_tempo)
            .unwrap_or(playing_track_profile.tempo.bpm),
        groove_type: playing_track_profile.perceived_tempo.as_ref()
            .map(|pt| pt.groove_type.clone())
            .unwrap_or_else(|| "standard".to_string()),
        key: playing_track_profile.key.camelot.clone(),
        energy: playing_track_profile.energy.average,
        lufs_loudness: playing_track_profile.loudness.integrated_lufs,
        elapsed_ms: deck_snapshot.position_ms,
        time_remaining_ms,
        duration_ms: deck_snapshot.duration_ms,
        phrase_length_ms: (60_000.0 / playing_track_profile.tempo.bpm * 16.0) as u64,
        genre: playing_class.genre,
        mood: playing_class.mood,
        vibe: playing_class.vibe,
        vocal_presence: playing_track_profile.vocal_presence,
        energy_trajectory: playing_track_profile.energy_trajectory.clone(),
        intro_length_ms: playing_track_profile.intro_length_ms,
        outro_length_ms: playing_track_profile.outro_length_ms,
        first_drop_ms: playing_track_profile.first_drop_ms,
    };

    let preferences = PreferencesContext {
        intent: input.intent.to_string(),
        target_energy: input.target_energy,
    };

    let constraints = ConstraintsContext {
        max_bpm_difference: 15.0,
        allow_key_shift: true,
        recent_tracks: input.recent_tracks.to_vec(),
    };

    let available_tracks = available_profiles
        .iter()
        .map(|p| {
            let (artist, title) = available_metadata
                .get(&p.track_id)
                .cloned()
                .unwrap_or_else(|| ("Unknown".to_string(), "Unknown".to_string()));

            let class = input.classifications
                .get(&p.track_id)
                .cloned()
                .unwrap_or_default();

            TrackProfileShort {
                id: p.track_id.clone(),
                artist,
                title,
                bpm: p.tempo.bpm,
                perceived_tempo: p.perceived_tempo.as_ref()
                    .map(|pt| pt.perceived_tempo)
                    .unwrap_or(p.tempo.bpm),
                groove_type: p.perceived_tempo.as_ref()
                    .map(|pt| pt.groove_type.clone())
                    .unwrap_or_else(|| "standard".to_string()),
                key: p.key.camelot.clone(),
                energy: p.energy.average,
                lufs_loudness: p.loudness.integrated_lufs,
                duration_ms: p.duration_ms,
                phrase_length_ms: (60_000.0 / p.tempo.bpm * 16.0) as u64,
                genre: class.genre,
                subgenre: class.subgenre,
                mood: class.mood,
                vibe: class.vibe,
                mix_notes: class.mix_notes,
                spectral_centroid: p.spectral.as_ref().map(|s| s.centroid),
                vocal_presence: p.vocal_presence,
                danceability: p.rhythm.as_ref().map(|r| r.danceability),
                energy_trajectory: p.energy_trajectory.clone(),
                intro_length_ms: p.intro_length_ms,
                outro_length_ms: p.outro_length_ms,
            }
        })
        .collect();

    AiContext {
        current_state,
        preferences,
        constraints,
        available_tracks,
        set_flow: input.set_flow.clone(),
    }
}
