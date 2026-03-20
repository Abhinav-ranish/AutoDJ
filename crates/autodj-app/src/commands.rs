use crate::state::AppState;
use crate::state::DjMode;
use autodj_ai::{context::build_context, planner::AiDecision, AiConfig};
use autodj_core::buffer::TrackBuffer;
use autodj_core::command::{Command, CommandSource, DeckId, EqBand, EqMode};
use autodj_core::engine::EngineStateSnapshot;
use autodj_storage::cache::hash_file;
use autodj_storage::db::{ImportStatus, TrackRecord};
use autodj_storage::decoder;
use lofty::prelude::*;
use lofty::tag::Accessor;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

/// Import audio files into the library.
#[tauri::command]
pub async fn import_files(
    paths: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let mut imported_ids = Vec::new();

    for path_str in &paths {
        let path = PathBuf::from(path_str);
        if !path.exists() {
            log::warn!("File does not exist: {}", path_str);
            continue;
        }

        // Hash the file for deduplication
        let file_hash = hash_file(&path).map_err(|e| format!("Hash error: {e}"))?;

        // Check if already imported
        let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
        if db.hash_exists(&file_hash).map_err(|e| format!("DB: {e}"))? {
            log::info!("File already imported: {}", path_str);
            continue;
        }

        // Extract metadata using lofty
        let mut title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string());
        let mut artist = None;
        let mut album_art_data: Option<Vec<u8>> = None;

        if let Ok(tagged_file) = lofty::read_from_path(&path) {
            if let Some(tag) = tagged_file
                .primary_tag()
                .or_else(|| tagged_file.first_tag())
            {
                if let Some(t) = tag.title().as_deref() {
                    title = Some(t.to_string());
                }
                if let Some(a) = tag.artist().as_deref() {
                    artist = Some(a.to_string());
                }
                // Extract album art
                if let Some(picture) = tag.pictures().first() {
                    album_art_data = Some(picture.data().to_vec());
                }
            }
        }

        // Insert track record
        let track_id = db
            .insert_track(&file_hash, path_str, title.as_deref(), artist.as_deref())
            .map_err(|e| format!("DB insert: {e}"))?;

        // Save album art to cache
        if let Some(art_data) = album_art_data {
            let art_path = state.cache.art_path(&track_id);
            if let Err(e) = std::fs::write(&art_path, &art_data) {
                log::warn!("Failed to save album art for {}: {}", track_id, e);
            }
        }

        // Copy original file
        state
            .cache
            .copy_original(&path, &track_id)
            .map_err(|e| format!("Copy: {e}"))?;

        // Update status to decoding
        db.update_status(&track_id, ImportStatus::Decoding, None)
            .map_err(|e| format!("DB: {e}"))?;

        // Drop the DB lock before decoding (takes time)
        drop(db);

        // Decode with FFmpeg
        match decoder::decode_to_pcm(&path, &track_id, &state.cache) {
            Ok(_pcm_path) => {
                let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
                // Get duration from PCM file
                let pcm_path = state.cache.pcm_path(&track_id);
                if let Ok(buf) = TrackBuffer::open(&pcm_path) {
                    db.update_duration(&track_id, buf.duration_ms() as i64).ok();
                }
                db.update_status(&track_id, ImportStatus::Ready, None)
                    .map_err(|e| format!("DB: {e}"))?;
                imported_ids.push(track_id);
            }
            Err(e) => {
                let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
                db.update_status(&track_id, ImportStatus::Error, Some(&e.to_string()))
                    .map_err(|e| format!("DB: {e}"))?;
                log::error!("Decode failed for {}: {}", path_str, e);
            }
        }
    }

    Ok(imported_ids)
}

/// Get tracks with optional search, filter, and sort.
#[tauri::command]
pub fn get_tracks(
    query: Option<String>,
    genre: Option<String>,
    mood: Option<String>,
    bpm_min: Option<f64>,
    bpm_max: Option<f64>,
    key_filter: Option<String>,
    sort_by: Option<String>,
    sort_order: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<TrackRecord>, String> {
    let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;

    // If no filters applied, use the fast path
    if query.is_none()
        && genre.is_none()
        && mood.is_none()
        && bpm_min.is_none()
        && bpm_max.is_none()
        && key_filter.is_none()
        && sort_by.is_none()
    {
        return db.get_all_tracks().map_err(|e| format!("DB: {e}"));
    }

    db.search_tracks(
        query.as_deref(),
        genre.as_deref(),
        mood.as_deref(),
        bpm_min,
        bpm_max,
        key_filter.as_deref(),
        sort_by.as_deref(),
        sort_order.as_deref(),
    )
    .map_err(|e| format!("DB: {e}"))
}

/// Load a track onto a deck.
#[tauri::command]
pub async fn load_track(
    deck: DeckId,
    track_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let deck_id = deck;

    // Open the PCM cache file
    let pcm_path = state.cache.pcm_path(&track_id);
    if !pcm_path.exists() {
        return Err(format!("PCM cache not found for track {track_id}"));
    }

    let track_buffer =
        TrackBuffer::open(&pcm_path).map_err(|e| format!("Failed to open track buffer: {e}"))?;

    state.load_track_to_deck(deck_id, track_id.clone(), Arc::new(track_buffer));

    // Auto-sync BPM if profile exists
    if let Some(snap) = state.latest_state() {
        let master_bpm = snap.master_bpm;
        let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
        if let Ok(Some(track)) = db.get_track(&track_id) {
            if let Some(track_bpm) = track.bpm {
                if track_bpm > 0.0 {
                    let speed = master_bpm / track_bpm;
                    state.send_command(
                        Command::SetSpeed {
                            deck: deck_id,
                            speed,
                        },
                        CommandSource::User,
                    );
                }
            }
        }
    }

    // Record track play in session history
    {
        let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
        let mut session = state
            .current_session_id
            .write()
            .map_err(|_| "Session lock error")?;
        if session.is_none() {
            if let Ok(sid) = db.create_session() {
                *session = Some(sid);
            }
        }
        if let Some(ref sid) = *session {
            let _ = db.record_track_play(sid, &track_id, None, None);
        }
    }

    // Also send a Play command right away
    state.send_command(Command::Play { deck: deck_id }, CommandSource::User);

    Ok(())
}

/// Play a deck.
#[tauri::command]
pub fn play(deck: DeckId, state: State<'_, AppState>) -> Result<(), String> {
    state.send_command(Command::Play { deck }, CommandSource::User);
    Ok(())
}

/// Pause a deck.
#[tauri::command]
pub fn pause(deck: DeckId, state: State<'_, AppState>) -> Result<(), String> {
    state.send_command(Command::Pause { deck }, CommandSource::User);
    Ok(())
}

/// Set crossfader position.
#[tauri::command]
pub fn set_crossfader(position: f32, state: State<'_, AppState>) -> Result<(), String> {
    state.send_command(Command::SetCrossfader { position }, CommandSource::User);
    Ok(())
}

/// Set deck gain.
#[tauri::command]
pub fn set_deck_gain(deck: DeckId, gain_db: f32, state: State<'_, AppState>) -> Result<(), String> {
    state.send_command(Command::SetDeckGain { deck, gain_db }, CommandSource::User);
    Ok(())
}

/// Set EQ band gain for a deck.
#[tauri::command]
pub fn set_eq(
    deck: DeckId,
    band: EqBand,
    gain_db: f32,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.send_command(
        Command::SetEq {
            deck,
            band,
            gain_db,
        },
        CommandSource::User,
    );
    Ok(())
}

/// Set filter parameters for a deck.
#[tauri::command]
pub fn set_filter(
    deck: DeckId,
    cutoff_hz: f32,
    resonance: f32,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.send_command(
        Command::SetFilter {
            deck,
            cutoff_hz,
            resonance,
        },
        CommandSource::User,
    );
    Ok(())
}

/// Seek a deck to a position in milliseconds.
#[tauri::command]
pub fn seek(deck: DeckId, position_ms: u64, state: State<'_, AppState>) -> Result<(), String> {
    // Convert ms to beats based on actual deck BPM if available, else fallback to master
    let bpm = state.latest_state().map(|s| s.master_bpm).unwrap_or(128.0);

    let beat = (position_ms as f64 * bpm / 60000.0) as u64;
    state.send_command(Command::Seek { deck, beat }, CommandSource::User);
    Ok(())
}

/// Set a loop on a deck.
#[tauri::command]
pub fn set_loop(deck: DeckId, length_bars: u32, state: State<'_, AppState>) -> Result<(), String> {
    // Calculate current beat from engine position using master BPM for now
    let start_beat = state
        .latest_state()
        .map(|s| {
            let info = if deck == DeckId::A {
                &s.deck_a
            } else {
                &s.deck_b
            };
            let bpm = s.master_bpm;
            (info.position_ms as f64 * bpm / 60000.0) as u64
        })
        .unwrap_or(0);

    state.send_command(
        Command::SetLoop {
            deck,
            start_beat,
            length_bars,
        },
        CommandSource::User,
    );
    Ok(())
}

/// Clear the loop on a deck.
#[tauri::command]
pub fn clear_loop(deck: DeckId, state: State<'_, AppState>) -> Result<(), String> {
    state.send_command(Command::ClearLoop { deck }, CommandSource::User);
    Ok(())
}

/// Set the tempo (BPM) for a deck.
#[tauri::command]
pub fn set_tempo(deck: DeckId, bpm: f64, state: State<'_, AppState>) -> Result<(), String> {
    state.send_command(Command::SetTempo { deck, bpm }, CommandSource::User);
    Ok(())
}

/// Sync a deck's tempo to the master clock.
#[tauri::command]
pub async fn sync_to_master(
    deck: DeckId,
    track_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let snapshot = state.latest_state().ok_or("Engine state unavailable")?;
    let master_bpm = snapshot.master_bpm;

    let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
    let track_json = db
        .get_track_profile_json(&track_id)
        .map_err(|e| format!("DB error: {e}"))?
        .ok_or_else(|| format!("No profile found for track {track_id}"))?;

    let profile: autodj_analysis::profile::TrackProfile =
        serde_json::from_str(&track_json).map_err(|e| format!("De-serialization error: {e}"))?;

    let track_bpm = profile.tempo.bpm;
    if track_bpm <= 0.0 {
        return Err("Invalid track BPM (0)".to_string());
    }

    let speed = master_bpm / track_bpm;
    state.send_command(Command::SetSpeed { deck, speed }, CommandSource::User);

    Ok(())
}

#[tauri::command]
pub fn set_speed(deck: DeckId, speed: f64, state: State<'_, AppState>) -> Result<(), String> {
    state.send_command(Command::SetSpeed { deck, speed }, CommandSource::User);
    Ok(())
}

/// Analyze a track.
#[tauri::command]
pub async fn analyze_track(track_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.clone();
    let cache = state.cache.clone();

    // Run heavy CPU work in spawn_blocking
    tauri::async_runtime::spawn_blocking(move || {
        let track = match db.lock().unwrap().get_track(&track_id) {
            Ok(Some(t)) => t,
            _ => return Err("Track not found".to_string()),
        };

        if track.import_status != ImportStatus::Ready
            && track.import_status != ImportStatus::Analyzed
        {
            let _ = db.lock().unwrap().update_status(
                &track_id,
                ImportStatus::AnalysisFailed,
                Some("Track not ready"),
            );
            return Err("Track not ready".to_string());
        }

        let pcm_path = cache.pcm_path(&track_id);
        let buffer = match autodj_core::buffer::TrackBuffer::open(&pcm_path) {
            Ok(b) => b,
            Err(_) => {
                let _ = db.lock().unwrap().update_status(
                    &track_id,
                    ImportStatus::AnalysisFailed,
                    Some("Failed to open PCM buffer"),
                );
                return Err("Failed to open PCM buffer".to_string());
            }
        };

        // Extract all samples into a Vec<f32> for analysis
        let total = buffer.total_frames() * 2;
        let mut samples = Vec::with_capacity(total);
        for i in 0..total {
            samples.push(buffer.get_sample(i));
        }

        let peaks_dir = cache.base_dir();

        match autodj_analysis::analyze_track(
            track_id.clone(),
            track.file_hash.clone(),
            &samples,
            buffer.header.sample_rate,
            buffer.header.channels,
            peaks_dir,
        ) {
            Ok(profile) => {
                let json = serde_json::to_string(&profile).unwrap_or_default();
                let db_guard = db.lock().unwrap();
                let energy_traj_str = profile.energy_trajectory.clone();
                let data = autodj_storage::db::ProfileData {
                    track_id: &track_id,
                    profile_json: &json,
                    bpm: profile.tempo.bpm,
                    bpm_confidence: profile.tempo.confidence,
                    perceived_tempo: profile
                        .perceived_tempo
                        .as_ref()
                        .map(|p| p.perceived_tempo)
                        .unwrap_or(profile.tempo.bpm),
                    groove_type: profile
                        .perceived_tempo
                        .as_ref()
                        .map(|p| p.groove_type.as_str())
                        .unwrap_or("standard"),
                    key_name: &profile.key.key,
                    camelot: &profile.key.camelot,
                    key_confidence: profile.key.confidence,
                    lufs: profile.loudness.integrated_lufs,
                    energy_avg: profile.energy.average,
                    spectral_centroid: profile.spectral.as_ref().map(|s| s.centroid),
                    spectral_flatness: profile.spectral.as_ref().map(|s| s.flatness),
                    bass_energy: profile.spectral.as_ref().map(|s| s.bass_energy),
                    mid_energy: profile.spectral.as_ref().map(|s| s.mid_energy),
                    high_energy: profile.spectral.as_ref().map(|s| s.high_energy),
                    vocal_presence: profile.vocal_presence,
                    rhythm_complexity: profile.rhythm.as_ref().map(|r| r.complexity),
                    danceability: profile.rhythm.as_ref().map(|r| r.danceability),
                    energy_trajectory: energy_traj_str.as_deref(),
                    intro_length_ms: profile.intro_length_ms.map(|v| v as i64),
                    outro_length_ms: profile.outro_length_ms.map(|v| v as i64),
                    first_drop_ms: profile.first_drop_ms.map(|v| v as i64),
                };
                let _ = db_guard.save_track_profile(&data);
                let _ = db_guard.update_status(&track_id, ImportStatus::Analyzed, None);
                Ok(())
            }
            Err(e) => {
                let _ = db.lock().unwrap().update_status(
                    &track_id,
                    ImportStatus::AnalysisFailed,
                    Some(&e.to_string()),
                );
                Err(e.to_string())
            }
        }
    })
    .await
    .map_err(|e| format!("Spawn blocking error: {e}"))??;

    Ok(())
}

/// Execute a transition template.
#[tauri::command]
pub fn execute_transition(
    template: String,
    params: serde_json::Value,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.send_command(
        Command::ExecuteTransition { template, params },
        CommandSource::User,
    );
    Ok(())
}

/// Get a track profile.
#[tauri::command]
pub fn get_track_profile(
    track_id: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    state
        .db
        .lock()
        .map_err(|e| format!("DB lock error: {e}"))?
        .get_track_profile_json(&track_id)
        .map_err(|e| e.to_string())
}

/// Get the current engine state snapshot.
#[tauri::command]
pub fn get_engine_state(state: State<'_, AppState>) -> Result<Option<EngineStateSnapshot>, String> {
    Ok(state.latest_state())
}

/// Set the DJ mode (manual, assisted, autodj).
#[tauri::command]
pub fn set_mode(mode: DjMode, state: State<'_, AppState>) -> Result<(), String> {
    let mut current_mode = state
        .dj_mode
        .write()
        .map_err(|_| "Failed to lock dj_mode")?;
    *current_mode = mode;
    Ok(())
}

/// Configure the AI conductor.
#[tauri::command]
pub async fn set_ai_config(config: AiConfig, state: State<'_, AppState>) -> Result<(), String> {
    let mut planner = state.ai_planner.write().await;
    planner.set_config(config);
    Ok(())
}

/// Request the next track from the AI Planner.
#[tauri::command]
pub async fn request_next_track(state: State<'_, AppState>) -> Result<AiDecision, String> {
    let snapshot = state.latest_state().ok_or("Engine state unavailable")?;

    let (
        playing_profile,
        playing_artist,
        playing_title,
        avail_profiles,
        avail_metadata,
        classifications,
        recent_tracks,
    ) = {
        let db = state.db.lock().map_err(|e| format!("DB Lock error: {e}"))?;
        let tracks = db.get_all_tracks().map_err(|e| format!("DB error: {e}"))?;

        let mut profiles = Vec::new();
        let mut metadata = std::collections::HashMap::new();

        // Try to find the currently playing track by deck track_id
        let deck_a_playing = snapshot.deck_a.state == "playing";
        let playing_id = if deck_a_playing {
            snapshot.deck_a.track_id.clone()
        } else {
            snapshot.deck_b.track_id.clone()
        };

        let mut current_profile = None;
        let mut cur_artist = "Unknown".to_string();
        let mut cur_title = "Unknown".to_string();

        for track in &tracks {
            if let Ok(Some(json)) = db.get_track_profile_json(&track.id) {
                if let Ok(profile) =
                    serde_json::from_str::<autodj_analysis::profile::TrackProfile>(&json)
                {
                    let artist = track
                        .artist
                        .clone()
                        .unwrap_or_else(|| "Unknown".to_string());
                    let title = track.title.clone().unwrap_or_else(|| "Unknown".to_string());
                    metadata.insert(track.id.clone(), (artist.clone(), title.clone()));

                    if playing_id.as_deref() == Some(&track.id) {
                        current_profile = Some(profile.clone());
                        cur_artist = artist;
                        cur_title = title;
                    }

                    profiles.push(profile);
                }
            }
        }

        // Fallback: use first profile if playing track not identified
        if current_profile.is_none() {
            if let Some(first) = profiles.first() {
                current_profile = Some(first.clone());
                if let Some((a, t)) = metadata.get(&first.track_id) {
                    cur_artist = a.clone();
                    cur_title = t.clone();
                }
            }
        }

        let p = current_profile.ok_or("No current track profile found!")?;

        // Load classifications
        let mut class_map = std::collections::HashMap::new();
        if let Ok(class_rows) = db.get_all_classifications() {
            for (tid, genre, subgenre, mood, vibe, mix_notes) in class_rows {
                class_map.insert(
                    tid,
                    autodj_ai::context::TrackClassificationData {
                        genre: Some(genre),
                        subgenre: Some(subgenre),
                        mood: Some(mood),
                        vibe: Some(vibe),
                        mix_notes: Some(mix_notes),
                    },
                );
            }
        }

        // Get recent track history for variety scoring
        let recent = db.get_recent_track_ids(10).unwrap_or_default();

        (
            p, cur_artist, cur_title, profiles, metadata, class_map, recent,
        )
    };

    // Get recent_templates from planner for anti-repetition context
    let recent_templates: Vec<String> = {
        let planner = state.ai_planner.read().await;
        planner.recent_templates().to_vec()
    };

    let context = build_context(&autodj_ai::context::ContextInput {
        engine_state: &snapshot,
        playing_track_profile: &playing_profile,
        playing_artist: &playing_artist,
        playing_title: &playing_title,
        available_profiles: &avail_profiles,
        intent: "maintain",
        target_energy: 0.5,
        recent_tracks: &recent_tracks,
        recent_templates: &recent_templates,
        available_metadata: &avail_metadata,
        classifications: &classifications,
        set_flow: None,
    });

    let planner = state.ai_planner.read().await;
    let decision = planner
        .plan_next(
            &context,
            &playing_profile,
            &avail_profiles,
            &classifications,
        )
        .await
        .map_err(|e| format!("AI Planning Error: {e}"))?;

    // Record the decision in mix history if a track was selected
    if let Some(ref track_id) = decision.track_selection {
        let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
        let session_id = state
            .current_session_id
            .read()
            .map_err(|_| "Session lock error".to_string())?;
        if let Some(ref sid) = *session_id {
            let decision_json = serde_json::to_string(&decision).ok();
            let _ = db.record_track_play(
                sid,
                track_id,
                decision
                    .transition_plan
                    .as_ref()
                    .map(|t| t.template_name.as_str()),
                decision_json.as_deref(),
            );
        }
    }

    Ok(decision)
}

/// Set a hot cue for a track.
#[tauri::command]
pub fn set_hot_cue(
    track_id: String,
    cue_index: u32,
    position_ms: u64,
    label: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
    db.save_hot_cue(&track_id, cue_index, position_ms, label.as_deref())
        .map_err(|e| format!("DB error: {e}"))
}

/// Get all hot cues for a track.
#[tauri::command]
pub fn get_hot_cues(
    track_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<(u32, u64, Option<String>)>, String> {
    let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
    db.get_hot_cues(&track_id)
        .map_err(|e| format!("DB error: {e}"))
}

/// Jump to a hot cue on a deck.
#[tauri::command]
pub fn jump_to_hot_cue(
    deck: DeckId,
    track_id: String,
    cue_index: u32,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
    let cues = db
        .get_hot_cues(&track_id)
        .map_err(|e| format!("DB error: {e}"))?;

    if let Some((_, position_ms, _)) = cues.iter().find(|(idx, _, _)| *idx == cue_index) {
        // Convert ms to beats
        let bpm = state.latest_state().map(|s| s.master_bpm).unwrap_or(128.0);

        let beat = (*position_ms as f64 * bpm / 60000.0) as u64;
        state.send_command(Command::Seek { deck, beat }, CommandSource::User);
        Ok(())
    } else {
        Err(format!(
            "Hot cue {cue_index} not found for track {track_id}"
        ))
    }
}

/// Classify a track using Gemini AI (genre, mood, vibe, mix notes).
#[tauri::command]
pub async fn classify_track(
    track_id: String,
    state: State<'_, AppState>,
) -> Result<autodj_ai::classifier::TrackClassification, String> {
    let db = state.db.clone();

    // Get track metadata and profile
    let (artist, title, id3_genre, profile) = {
        let db_guard = db.lock().map_err(|e| format!("DB lock: {e}"))?;
        let track = db_guard
            .get_track(&track_id)
            .map_err(|e| format!("DB error: {e}"))?
            .ok_or_else(|| format!("Track {track_id} not found"))?;

        let profile_json = db_guard
            .get_track_profile_json(&track_id)
            .map_err(|e| format!("DB error: {e}"))?
            .ok_or_else(|| format!("No profile for track {track_id} — analyze first"))?;

        let profile: autodj_analysis::TrackProfile =
            serde_json::from_str(&profile_json).map_err(|e| format!("Profile parse error: {e}"))?;

        (
            track.artist.unwrap_or_else(|| "Unknown".to_string()),
            track.title.unwrap_or_else(|| "Unknown".to_string()),
            None::<String>, // ID3 genre not stored separately yet
            profile,
        )
    };

    // Build classification input from profile
    let input = autodj_ai::classifier::ClassificationInput {
        artist: &artist,
        title: &title,
        id3_genre: id3_genre.as_deref(),
        bpm: profile.tempo.bpm,
        perceived_tempo: profile.perceived_tempo.as_ref().map(|p| p.perceived_tempo),
        groove_type: profile
            .perceived_tempo
            .as_ref()
            .map(|p| p.groove_type.as_str()),
        key: &profile.key.key,
        camelot: &profile.key.camelot,
        energy_avg: profile.energy.average,
        lufs: profile.loudness.integrated_lufs,
        spectral_centroid: profile.spectral.as_ref().map(|s| s.centroid),
        spectral_flatness: profile.spectral.as_ref().map(|s| s.flatness),
        bass_energy: profile.spectral.as_ref().map(|s| s.bass_energy),
        mid_energy: profile.spectral.as_ref().map(|s| s.mid_energy),
        high_energy: profile.spectral.as_ref().map(|s| s.high_energy),
        vocal_presence: profile.vocal_presence,
        rhythm_complexity: profile.rhythm.as_ref().map(|r| r.complexity),
        danceability: profile.rhythm.as_ref().map(|r| r.danceability),
        energy_trajectory: profile.energy_trajectory.as_deref(),
        duration_ms: profile.duration_ms,
        intro_length_ms: profile.intro_length_ms,
        outro_length_ms: profile.outro_length_ms,
    };

    // Get AI config
    let ai_config = {
        let planner = state.ai_planner.read().await;
        planner.config().clone()
    };

    // Call Gemini classifier
    let classification = autodj_ai::classifier::classify_track(&input, &ai_config)
        .await
        .map_err(|e| format!("Classification error: {e}"))?;

    // Throttle classification requests for rate-limited providers
    if ai_config.provider == "groq" {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }

    // Save to DB
    let tags_json =
        serde_json::to_string(&classification.ai_tags).unwrap_or_else(|_| "[]".to_string());
    let db_guard = db.lock().map_err(|e| format!("DB lock: {e}"))?;
    db_guard
        .save_classification(
            &track_id,
            &classification.genre,
            &classification.subgenre,
            &classification.mood,
            &classification.vibe,
            &classification.mix_notes,
            &tags_json,
        )
        .map_err(|e| format!("DB save error: {e}"))?;

    Ok(classification)
}

/// Clear all tracks from the library and cache.
#[tauri::command]
pub fn clear_library(state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
    db.clear_all_tracks().map_err(|e| format!("DB: {e}"))?;

    state.cache.clear_all().map_err(|e| format!("Cache: {e}"))?;
    Ok(())
}

/// Start recording the master output to a WAV file.
#[tauri::command]
pub fn start_recording(filename: String, state: State<'_, AppState>) -> Result<(), String> {
    use autodj_core::recorder::WavWriter;
    use std::sync::atomic::Ordering;

    // Check if already recording
    if state.recording_flag.load(Ordering::Relaxed) {
        return Err("Already recording".to_string());
    }

    // Take the consumer out of state (can only record once at a time)
    let consumer = state
        .record_consumer
        .lock()
        .map_err(|e| format!("Lock error: {e}"))?
        .take()
        .ok_or("Recording consumer not available (already in use?)")?;

    // Build output path in data_dir/recordings/
    let recordings_dir = state.data_dir.join("recordings");
    std::fs::create_dir_all(&recordings_dir)
        .map_err(|e| format!("Failed to create recordings dir: {e}"))?;
    let output_path = recordings_dir.join(&filename);

    let flag = state.recording_flag.clone();
    let sample_rate = state.sample_rate;

    // Set recording flag BEFORE spawning thread
    flag.store(true, Ordering::Relaxed);

    // Spawn writer thread
    std::thread::spawn(move || {
        let writer = WavWriter::new(output_path, sample_rate);
        match writer.run(consumer, flag) {
            Ok(path) => log::info!("Recording saved to: {}", path.display()),
            Err(e) => log::error!("Recording failed: {e}"),
        }
    });

    Ok(())
}

/// Stop recording and finalize the WAV file.
#[tauri::command]
pub fn stop_recording(state: State<'_, AppState>) -> Result<(), String> {
    use std::sync::atomic::Ordering;

    if !state.recording_flag.load(Ordering::Relaxed) {
        return Err("Not currently recording".to_string());
    }

    // Clear the flag - the writer thread will drain remaining samples and exit
    state.recording_flag.store(false, Ordering::Relaxed);

    // Note: The consumer was moved to the writer thread in start_recording.
    // It will be dropped when the writer thread finishes.
    // Recording can only happen once per app session (consumer is consumed).
    // A future improvement could recreate the ring buffer pair.

    Ok(())
}

/// Get the album art path for a track, if it exists.
#[tauri::command]
pub fn get_album_art(
    track_id: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    if state.cache.art_exists(&track_id) {
        Ok(Some(
            state
                .cache
                .art_path(&track_id)
                .to_string_lossy()
                .to_string(),
        ))
    } else {
        Ok(None)
    }
}

/// Set EQ kill switch for a deck band.
#[tauri::command]
pub fn set_eq_kill(
    deck: DeckId,
    band: EqBand,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.send_command(
        Command::SetEqKill {
            deck,
            band,
            enabled,
        },
        CommandSource::User,
    );
    Ok(())
}

/// Set EQ mode (classic/isolator) for a deck.
#[tauri::command]
pub fn set_eq_mode(deck: DeckId, mode: EqMode, state: State<'_, AppState>) -> Result<(), String> {
    state.send_command(Command::SetEqMode { deck, mode }, CommandSource::User);
    Ok(())
}

/// Seek a deck to exact millisecond position.
#[tauri::command]
pub fn seek_ms(deck: DeckId, position_ms: u64, state: State<'_, AppState>) -> Result<(), String> {
    state.send_command(Command::SeekMs { deck, position_ms }, CommandSource::User);
    Ok(())
}

/// Get auto-cues for a track from DB.
#[tauri::command]
pub fn get_auto_cues(
    track_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<(String, u64, f64)>, String> {
    let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
    db.get_auto_cues(&track_id)
        .map_err(|e| format!("DB error: {e}"))
}

/// Submit feedback about a transition.
#[tauri::command]
pub fn submit_transition_feedback(
    from_track_id: String,
    to_track_id: String,
    template_used: String,
    rating: i32,
    notes: Option<String>,
    decision_json: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
    let session_id = state
        .current_session_id
        .read()
        .map_err(|_| "Session lock error".to_string())?;
    let sid = session_id.as_deref().unwrap_or("unknown");
    db.save_transition_feedback(
        sid,
        &from_track_id,
        &to_track_id,
        &template_used,
        rating,
        notes.as_deref(),
        decision_json.as_deref(),
    )
    .map_err(|e| format!("DB error: {e}"))
}

// ─── Phase 30: DJ Performance Engine ──────────────────────────

/// Beat repeat: loop the current beat region.
#[tauri::command]
pub fn beat_repeat(deck: String, beats: f32, state: State<'_, AppState>) -> Result<(), String> {
    let deck_id = parse_deck(&deck)?;
    state.send_command(
        Command::BeatRepeat {
            deck: deck_id,
            beats,
        },
        CommandSource::User,
    );
    Ok(())
}

/// Stop beat repeat.
#[tauri::command]
pub fn beat_repeat_off(deck: String, state: State<'_, AppState>) -> Result<(), String> {
    let deck_id = parse_deck(&deck)?;
    state.send_command(
        Command::BeatRepeatOff { deck: deck_id },
        CommandSource::User,
    );
    Ok(())
}

/// Toggle slip mode on a deck.
#[tauri::command]
pub fn set_slip_mode(
    deck: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let deck_id = parse_deck(&deck)?;
    state.send_command(
        Command::SetSlipMode {
            deck: deck_id,
            enabled,
        },
        CommandSource::User,
    );
    Ok(())
}

/// Vinyl brake effect.
#[tauri::command]
pub fn vinyl_brake(
    deck: String,
    duration_ms: u32,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let deck_id = parse_deck(&deck)?;
    state.send_command(
        Command::VinylBrake {
            deck: deck_id,
            duration_ms,
        },
        CommandSource::User,
    );
    Ok(())
}

/// Spinback effect.
#[tauri::command]
pub fn spinback(deck: String, duration_ms: u32, state: State<'_, AppState>) -> Result<(), String> {
    let deck_id = parse_deck(&deck)?;
    state.send_command(
        Command::Spinback {
            deck: deck_id,
            duration_ms,
        },
        CommandSource::User,
    );
    Ok(())
}

/// Jump to a named marker position.
#[tauri::command]
pub fn jump_to_marker(
    deck: String,
    marker: String,
    position_ms: u64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let deck_id = parse_deck(&deck)?;
    state.send_command(
        Command::JumpToMarker {
            deck: deck_id,
            marker,
            position_ms,
        },
        CommandSource::User,
    );
    Ok(())
}

// ─── Phase 33: AI Provider Health Check ─────────────────────

/// Test the currently configured AI provider by sending a minimal request.
#[tauri::command]
pub async fn test_ai_provider(state: State<'_, AppState>) -> Result<String, String> {
    let planner = state.ai_planner.read().await;
    let config = planner.config().clone();
    drop(planner);

    // Ollama is local inference — no API key needed
    if config.provider != "ollama"
        && (config.api_key.is_none() || config.api_key.as_deref() == Some(""))
    {
        return Err("No API key configured".to_string());
    }

    let client = autodj_ai::llm::LlmClient::new();
    match client
        .query(
            "You are a test assistant. Reply with exactly: {\"status\": \"ok\"}",
            "health check",
            &config,
        )
        .await
    {
        Ok(resp) => Ok(format!(
            "Provider '{}' responded: {}",
            config.provider, resp
        )),
        Err(e) => Err(format!("Provider '{}' failed: {}", config.provider, e)),
    }
}

// ─── Phase 39: Debug & Explainability ───────────────────────

/// Get detailed debug info about the last AI decision and planner state.
#[tauri::command]
pub async fn get_ai_debug_info(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let snapshot = state.latest_state().ok_or("Engine state unavailable")?;
    let planner = state.ai_planner.read().await;
    let config = planner.config().clone();
    drop(planner);

    let deck_a_playing = snapshot.deck_a.state == "playing";
    let playing_deck = if deck_a_playing { "A" } else { "B" };
    let playing_snap = if deck_a_playing {
        &snapshot.deck_a
    } else {
        &snapshot.deck_b
    };

    let active = {
        let mode = state
            .dj_mode
            .read()
            .map_err(|_| "Mode lock error".to_string())?;
        *mode == DjMode::Autodj
    };

    // Include structured conductor state in debug info
    let conductor = state.conductor.read().await;
    let conductor_snapshot = conductor.snapshot(active);
    drop(conductor);
    let conductor_phase = format!("{:?}", conductor_snapshot.phase).to_lowercase();
    let conductor_trigger = conductor_snapshot.trigger_reason.clone();
    let conductor_message = conductor_snapshot.message.clone();
    let conductor_error = conductor_snapshot.error.clone();
    let decision_source = conductor_snapshot.decision_source.clone();
    let fallback_reason = conductor_snapshot.fallback_reason.clone();
    let template_chosen = conductor_snapshot.template_chosen.clone();
    let template_scores = conductor_snapshot.template_scores.clone();
    let entry_marker = conductor_snapshot.entry_marker.clone();
    let confidence = conductor_snapshot.confidence;
    let reasoning_tags = conductor_snapshot.reasoning_tags.clone();
    let conductor_trigger_meta = conductor_snapshot.trigger_meta.clone();
    let conductor_decision_meta = conductor_snapshot.decision_meta.clone();

    Ok(serde_json::json!({
        "provider": config.provider,
        "model": config.model,
        "enabled": config.enabled,
        "has_api_key": config.api_key.is_some() && config.api_key.as_deref() != Some(""),
        "playing_deck": playing_deck,
        "position_ms": playing_snap.position_ms,
        "duration_ms": playing_snap.duration_ms,
        "position_fraction": playing_snap.position_fraction,
        "master_bpm": snapshot.master_bpm,
        "beat_number": snapshot.beat_number,
        "bar_number": snapshot.bar_number,
        "silence_frames": snapshot.silence_frames,
        "crossfader": snapshot.crossfader,
        // Conductor runtime state
        "conductor_phase": conductor_phase,
        "conductor_trigger": conductor_trigger,
        "conductor_message": conductor_message,
        "conductor_error": conductor_error,
        "decision_source": decision_source,
        "fallback_reason": fallback_reason,
        "template_chosen": template_chosen,
        "template_scores": template_scores,
        "entry_marker": entry_marker,
        "confidence": confidence,
        "reasoning_tags": reasoning_tags,
        "conductor_trigger_meta": conductor_trigger_meta,
        "conductor_decision_meta": conductor_decision_meta,
    }))
}

/// Get the current conductor snapshot.
#[tauri::command]
pub async fn get_conductor_state(
    state: State<'_, AppState>,
) -> Result<crate::conductor::ConductorSnapshot, String> {
    let active = {
        let mode = state.dj_mode.read().map_err(|_| "Mode lock error")?;
        *mode == DjMode::Autodj
    };

    let cond = state.conductor.read().await;
    let mut snap = cond.snapshot(active);

    // Resolve pending track title
    if let Some(ref tid) = snap.pending_track_id {
        if let Ok(db) = state.db.lock() {
            if let Ok(Some(track)) = db.get_track(tid) {
                snap.pending_track_title = track.title;
            }
        }
    }

    Ok(snap)
}

/// Set the conductor's playlist queue for AutoDJ mode.
#[tauri::command]
pub async fn set_conductor_queue(
    track_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut cond = state.conductor.write().await;
    cond.queue = track_ids;
    cond.queue_index = 0;
    Ok(())
}

fn parse_deck(s: &str) -> Result<DeckId, String> {
    match s.to_uppercase().as_str() {
        "A" => Ok(DeckId::A),
        "B" => Ok(DeckId::B),
        _ => Err(format!("Invalid deck: {}", s)),
    }
}

// ─── Preferences ──────────────────────────────────────────────

/// Get a preference value.
#[tauri::command]
pub fn get_preference(key: String, state: State<'_, AppState>) -> Result<Option<String>, String> {
    let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
    db.get_preference(&key).map_err(|e| format!("DB: {e}"))
}

/// Set a preference value.
#[tauri::command]
pub fn set_preference(
    key: String,
    value: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
    db.set_preference(&key, &value)
        .map_err(|e| format!("DB: {e}"))
}

/// Get all preferences.
#[tauri::command]
pub fn get_all_preferences(state: State<'_, AppState>) -> Result<Vec<(String, String)>, String> {
    let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
    db.get_all_preferences().map_err(|e| format!("DB: {e}"))
}

/// List available audio output devices.
#[tauri::command]
pub fn get_audio_devices() -> Result<Vec<String>, String> {
    use cpal::traits::{DeviceTrait, HostTrait};
    let host = cpal::default_host();
    let devices = host
        .output_devices()
        .map_err(|e| format!("Failed to enumerate devices: {e}"))?;
    let names: Vec<String> = devices.filter_map(|d| d.name().ok()).collect();
    Ok(names)
}

// ─── Playlists ──────────────────────────────────────────────

/// Create a new playlist.
#[tauri::command]
pub fn create_playlist(
    name: String,
    description: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
    db.create_playlist(&name, description.as_deref())
        .map_err(|e| format!("DB: {e}"))
}

/// Get all playlists.
#[tauri::command]
pub fn get_playlists(
    state: State<'_, AppState>,
) -> Result<Vec<autodj_storage::db::PlaylistRecord>, String> {
    let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
    db.get_all_playlists().map_err(|e| format!("DB: {e}"))
}

/// Rename a playlist.
#[tauri::command]
pub fn rename_playlist(
    playlist_id: String,
    name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
    db.rename_playlist(&playlist_id, &name)
        .map_err(|e| format!("DB: {e}"))
}

/// Delete a playlist.
#[tauri::command]
pub fn delete_playlist(playlist_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
    db.delete_playlist(&playlist_id)
        .map_err(|e| format!("DB: {e}"))
}

/// Add a track to a playlist.
#[tauri::command]
pub fn add_track_to_playlist(
    playlist_id: String,
    track_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
    db.add_track_to_playlist(&playlist_id, &track_id)
        .map_err(|e| format!("DB: {e}"))
}

/// Remove a track from a playlist.
#[tauri::command]
pub fn remove_track_from_playlist(
    playlist_id: String,
    track_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
    db.remove_track_from_playlist(&playlist_id, &track_id)
        .map_err(|e| format!("DB: {e}"))
}

/// Reorder tracks in a playlist.
#[tauri::command]
pub fn reorder_playlist(
    playlist_id: String,
    track_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
    db.reorder_playlist(&playlist_id, &track_ids)
        .map_err(|e| format!("DB: {e}"))
}

/// Get tracks in a playlist.
#[tauri::command]
pub fn get_playlist_tracks(
    playlist_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<TrackRecord>, String> {
    let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
    db.get_playlist_tracks(&playlist_id)
        .map_err(|e| format!("DB: {e}"))
}

/// Get all sessions for cue sheet export.
#[tauri::command]
pub fn get_sessions(
    state: State<'_, AppState>,
) -> Result<Vec<autodj_storage::db::SessionRecord>, String> {
    let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
    db.get_all_sessions().map_err(|e| format!("DB: {e}"))
}

/// Export a session as a cue sheet string.
#[tauri::command]
pub fn export_cue_sheet(session_id: String, state: State<'_, AppState>) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
    let entries = db
        .get_session_mix_history(&session_id)
        .map_err(|e| format!("DB: {e}"))?;
    if entries.is_empty() {
        return Err("No mix history for this session".to_string());
    }
    let session_start = entries.first().map(|e| e.started_at).unwrap_or(0);
    let mut lines = Vec::new();
    lines.push("# AutoDJ Set — Cue Sheet".to_string());
    lines.push(String::new());
    for (i, entry) in entries.iter().enumerate() {
        let offset_secs = (entry.started_at - session_start).max(0);
        let mins = offset_secs / 60;
        let secs = offset_secs % 60;
        let artist = entry.artist.as_deref().unwrap_or("Unknown");
        let title = entry.title.as_deref().unwrap_or("Unknown");
        let transition = entry.transition_type.as_deref().unwrap_or("");
        let trans_str = if transition.is_empty() {
            String::new()
        } else {
            format!(" [{}]", transition)
        };
        lines.push(format!(
            "{:02}. [{:02}:{:02}] {} — {}{}",
            i + 1,
            mins,
            secs,
            artist,
            title,
            trans_str
        ));
    }
    Ok(lines.join("\n"))
}
