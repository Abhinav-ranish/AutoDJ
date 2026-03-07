use crate::state::AppState;
use autodj_core::buffer::TrackBuffer;
use autodj_core::command::{Command, CommandSource, DeckId, EqBand};
use autodj_core::engine::EngineStateSnapshot;
use autodj_storage::cache::hash_file;
use autodj_storage::db::{TrackRecord, ImportStatus};
use autodj_storage::decoder;
use autodj_ai::{AiConfig, planner::AiDecision, context::build_context};
use crate::state::DjMode;
use std::path::PathBuf;
use std::sync::Arc;
use lofty::prelude::*;
use lofty::file::AudioFile;
use lofty::tag::Accessor;
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

        if let Ok(tagged_file) = lofty::read_from_path(&path) {
            if let Some(tag) = tagged_file.primary_tag().or_else(|| tagged_file.first_tag()) {
                if let Some(t) = tag.title().as_deref() {
                    title = Some(t.to_string());
                }
                if let Some(a) = tag.artist().as_deref() {
                    artist = Some(a.to_string());
                }
            }
        }

        // Insert track record
        let track_id = db
            .insert_track(&file_hash, path_str, title.as_deref(), artist.as_deref())
            .map_err(|e| format!("DB insert: {e}"))?;

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

/// Get all tracks in the library.
#[tauri::command]
pub fn get_tracks(state: State<'_, AppState>) -> Result<Vec<TrackRecord>, String> {
    let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
    db.get_all_tracks().map_err(|e| format!("DB: {e}"))
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

    let track_buffer = TrackBuffer::open(&pcm_path)
        .map_err(|e| format!("Failed to open track buffer: {e}"))?;

    state.load_track_to_deck(deck_id, track_id.clone(), Arc::new(track_buffer));

    // Auto-sync BPM if profile exists
    if let Some(snap) = state.latest_state() {
        let master_bpm = snap.master_bpm;
        let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
        if let Ok(Some(track)) = db.get_track(&track_id) {
            if let Some(track_bpm) = track.bpm {
                if track_bpm > 0.0 {
                    let speed = master_bpm / track_bpm;
                    state.send_command(Command::SetSpeed { deck: deck_id, speed }, CommandSource::User);
                }
            }
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
    state.send_command(
        Command::SetCrossfader { position },
        CommandSource::User,
    );
    Ok(())
}

/// Set deck gain.
#[tauri::command]
pub fn set_deck_gain(deck: DeckId, gain_db: f32, state: State<'_, AppState>) -> Result<(), String> {
    state.send_command(
        Command::SetDeckGain { deck, gain_db },
        CommandSource::User,
    );
    Ok(())
}

/// Set EQ band gain for a deck.
#[tauri::command]
pub fn set_eq(deck: DeckId, band: EqBand, gain_db: f32, state: State<'_, AppState>) -> Result<(), String> {
    state.send_command(
        Command::SetEq { deck, band, gain_db },
        CommandSource::User,
    );
    Ok(())
}

/// Set filter parameters for a deck.
#[tauri::command]
pub fn set_filter(deck: DeckId, cutoff_hz: f32, resonance: f32, state: State<'_, AppState>) -> Result<(), String> {
    state.send_command(
        Command::SetFilter { deck, cutoff_hz, resonance },
        CommandSource::User,
    );
    Ok(())
}

/// Seek a deck to a position in milliseconds.
#[tauri::command]
pub fn seek(deck: DeckId, position_ms: u64, state: State<'_, AppState>) -> Result<(), String> {
    // Convert ms to beats based on actual deck BPM if available, else fallback to master
    let bpm = state.latest_state()
        .map(|s| s.master_bpm)
        .unwrap_or(128.0);
        
    let beat = (position_ms as f64 * bpm / 60000.0) as u64;
    state.send_command(
        Command::Seek { deck, beat },
        CommandSource::User,
    );
    Ok(())
}

/// Set a loop on a deck.
#[tauri::command]
pub fn set_loop(deck: DeckId, length_bars: u32, state: State<'_, AppState>) -> Result<(), String> {
    // Calculate current beat from engine position using master BPM for now
    let start_beat = state.latest_state()
        .map(|s| {
            let info = if deck == DeckId::A { &s.deck_a } else { &s.deck_b };
            let bpm = s.master_bpm;
            (info.position_ms as f64 * bpm / 60000.0) as u64
        })
        .unwrap_or(0);

    state.send_command(
        Command::SetLoop { deck, start_beat, length_bars },
        CommandSource::User,
    );
    Ok(())
}

/// Clear the loop on a deck.
#[tauri::command]
pub fn clear_loop(deck: DeckId, state: State<'_, AppState>) -> Result<(), String> {
    state.send_command(
        Command::ClearLoop { deck },
        CommandSource::User,
    );
    Ok(())
}

/// Set the tempo (BPM) for a deck.
#[tauri::command]
pub fn set_tempo(deck: DeckId, bpm: f64, state: State<'_, AppState>) -> Result<(), String> {
    state.send_command(
        Command::SetTempo { deck, bpm },
        CommandSource::User,
    );
    Ok(())
}

/// Sync a deck's tempo to the master clock.
#[tauri::command]
pub async fn sync_to_master(deck: DeckId, track_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let snapshot = state.latest_state().ok_or("Engine state unavailable")?;
    let master_bpm = snapshot.master_bpm;

    let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
    let track_json = db.get_track_profile_json(&track_id)
        .map_err(|e| format!("DB error: {e}"))?
        .ok_or_else(|| format!("No profile found for track {track_id}"))?;
    
    let profile: autodj_analysis::profile::TrackProfile = serde_json::from_str(&track_json)
        .map_err(|e| format!("De-serialization error: {e}"))?;
    
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

        if track.import_status != ImportStatus::Ready && track.import_status != ImportStatus::Analyzed {
            let _ = db.lock().unwrap().update_status(&track_id, ImportStatus::AnalysisFailed, Some("Track not ready"));
            return Err("Track not ready".to_string());
        }

        let pcm_path = cache.pcm_path(&track_id);
        let buffer = match autodj_core::buffer::TrackBuffer::open(&pcm_path) {
            Ok(b) => b,
            Err(_) => {
                let _ = db.lock().unwrap().update_status(&track_id, ImportStatus::AnalysisFailed, Some("Failed to open PCM buffer"));
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
                let _ = db_guard.save_track_profile(
                    &track_id,
                    &json,
                    profile.tempo.bpm,
                    profile.tempo.confidence,
                    profile.perceived_tempo.as_ref().map(|p| p.perceived_tempo).unwrap_or(profile.tempo.bpm),
                    profile.perceived_tempo.as_ref().map(|p| p.groove_type.as_str()).unwrap_or("standard"),
                    &profile.key.key,
                    &profile.key.camelot,
                    profile.key.confidence,
                    profile.loudness.integrated_lufs,
                    profile.energy.average,
                );
                let _ = db_guard.update_status(&track_id, ImportStatus::Analyzed, None);
                Ok(())
            }
            Err(e) => {
                let _ = db.lock().unwrap().update_status(&track_id, ImportStatus::AnalysisFailed, Some(&e.to_string()));
                Err(e.to_string())
            }
        }
    }).await.map_err(|e| format!("Spawn blocking error: {e}"))??;

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
pub fn get_track_profile(track_id: String, state: State<'_, AppState>) -> Result<Option<String>, String> {
    state.db.lock().map_err(|e| format!("DB lock error: {e}"))?.get_track_profile_json(&track_id).map_err(|e| e.to_string())
}

/// Get the current engine state snapshot.
#[tauri::command]
pub fn get_engine_state(state: State<'_, AppState>) -> Result<Option<EngineStateSnapshot>, String> {
    Ok(state.latest_state())
}

/// Set the DJ mode (manual, assisted, autodj).
#[tauri::command]
pub fn set_mode(mode: DjMode, state: State<'_, AppState>) -> Result<(), String> {
    let mut current_mode = state.dj_mode.write().map_err(|_| "Failed to lock dj_mode")?;
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
    // 1. Get engine properties snapshot
    let snapshot = state.latest_state().ok_or("Engine state unavailable")?;
    
    // 2. Fetch tracks and lock DB to assemble AiContext
    let (playing_profile, avail_profiles, avail_metadata) = {
        let db = state.db.lock().map_err(|e| format!("DB Lock error: {e}"))?;
        let tracks = db.get_all_tracks().map_err(|e| format!("DB error: {e}"))?;
        
        let mut profiles = Vec::new();
        let mut metadata = std::collections::HashMap::new();

        let deck_a_playing = snapshot.deck_a.state == "playing";
        let _playing_id = if deck_a_playing { "UNKNOWN_A" } else { "UNKNOWN_B" }; // TODO track currently loaded ID natively

        let mut current_profile = None;

        for track in tracks {
            if let Ok(Some(json)) = db.get_track_profile_json(&track.id) {
                if let Ok(profile) = serde_json::from_str::<autodj_analysis::profile::TrackProfile>(&json) {
                    metadata.insert(track.id.clone(), (
                        track.artist.clone().unwrap_or_else(|| "Unknown".to_string()),
                        track.title.clone().unwrap_or_else(|| "Unknown".to_string())
                    ));
                    
                    // Hack for demonstration: just pick the first track as playing if none specified
                    if current_profile.is_none() {
                        current_profile = Some(profile.clone());
                    }

                    profiles.push(profile);
                }
            }
        }
        
        let p = current_profile.ok_or("No current track profile found!")?;
        (p, profiles, metadata)
    };
    
    // 3. Assemble Context
    let context = build_context(
        &snapshot,
        &playing_profile,
        "Artist",
        "Title",
        &avail_profiles,
        "maintain", // Default intent
        0.5,
        &[],
        &avail_metadata
    );

    // 4. Await LLM AI Plan
    let planner = state.ai_planner.read().await;
    let decision = planner.plan_next(&context, &playing_profile, &avail_profiles)
        .await
        .map_err(|e| format!("AI Planning Error: {e}"))?;
    
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
    let cues = db.get_hot_cues(&track_id).map_err(|e| format!("DB error: {e}"))?;
    
    if let Some((_, position_ms, _)) = cues.iter().find(|(idx, _, _)| *idx == cue_index) {
        // Convert ms to beats
        let bpm = state.latest_state()
            .map(|s| s.master_bpm)
            .unwrap_or(128.0);
            
        let beat = (*position_ms as f64 * bpm / 60000.0) as u64;
        state.send_command(
            Command::Seek { deck, beat },
            CommandSource::User,
        );
        Ok(())
    } else {
        Err(format!("Hot cue {cue_index} not found for track {track_id}"))
    }
}

/// Clear all tracks from the library and cache.
#[tauri::command]
pub fn clear_library(state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
    db.clear_all_tracks().map_err(|e| format!("DB: {e}"))?;
    
    state.cache.clear_all().map_err(|e| format!("Cache: {e}"))?;
    Ok(())
}

