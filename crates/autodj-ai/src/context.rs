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
}

pub fn build_context(input: &ContextInput) -> AiContext {
    let engine_state = input.engine_state;
    let playing_track_profile = input.playing_track_profile;
    let playing_artist = input.playing_artist;
    let playing_title = input.playing_title;
    let available_profiles = input.available_profiles;
    let intent = input.intent;
    let target_energy = input.target_energy;
    let recent_tracks = input.recent_tracks;
    let available_metadata = input.available_metadata;
    // Determine playing deck and time remaining
    // For simplicity, we just use the active deck as the one playing
    let deck_a_playing = engine_state.deck_a.state == "playing";
    let playing_deck = if deck_a_playing { "A" } else { "B" };
    let deck_snapshot = if deck_a_playing { &engine_state.deck_a } else { &engine_state.deck_b };
    
    let time_remaining_ms = deck_snapshot.duration_ms.saturating_sub(deck_snapshot.position_ms);

    let current_state = CurrentStateContext {
        playing_deck: playing_deck.to_string(),
        track_id: playing_track_profile.track_id.clone(),
        artist: playing_artist.to_string(),
        title: playing_title.to_string(),
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
    };

    let preferences = PreferencesContext {
        intent: intent.to_string(),
        target_energy,
    };

    let constraints = ConstraintsContext {
        max_bpm_difference: 15.0,
        allow_key_shift: true,
        recent_tracks: recent_tracks.to_vec(),
    };

    let available_tracks = available_profiles
        .iter()
        .map(|p| {
            let (artist, title) = available_metadata
                .get(&p.track_id)
                .cloned()
                .unwrap_or_else(|| ("Unknown".to_string(), "Unknown".to_string()));
            
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
            }
        })
        .collect();

    AiContext {
        current_state,
        preferences,
        constraints,
        available_tracks,
    }
}
