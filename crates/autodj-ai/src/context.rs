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
    pub key: String,
    pub energy: f64,
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
    pub key: String,
    pub energy: f64,
    pub duration_ms: u64,
    pub phrase_length_ms: u64,
}

pub fn build_context(
    engine_state: &EngineStateSnapshot,
    playing_track_profile: &TrackProfile,
    playing_artist: &str,
    playing_title: &str,
    available_profiles: &[TrackProfile],
    intent: &str,
    target_energy: f64,
    recent_tracks: &[String],
    available_metadata: &std::collections::HashMap<String, (String, String)>,
) -> AiContext {
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
        key: playing_track_profile.key.camelot.clone(),
        energy: playing_track_profile.energy.average,
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
                key: p.key.camelot.clone(),
                energy: p.energy.average,
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
