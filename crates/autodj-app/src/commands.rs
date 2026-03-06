use crate::state::AppState;
use autodj_core::buffer::TrackBuffer;
use autodj_core::command::{Command, CommandSource, DeckId, EqBand};
use autodj_core::engine::EngineStateSnapshot;
use autodj_storage::cache::hash_file;
use autodj_storage::db::TrackRecord;
use autodj_storage::decoder;
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

        // Extract title from filename
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string());

        // Insert track record
        let track_id = db
            .insert_track(&file_hash, path_str, title.as_deref(), None)
            .map_err(|e| format!("DB insert: {e}"))?;

        // Copy original file
        state
            .cache
            .copy_original(&path, &track_id)
            .map_err(|e| format!("Copy: {e}"))?;

        // Update status to decoding
        db.update_status(&track_id, "decoding", None)
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
                db.update_status(&track_id, "ready", None)
                    .map_err(|e| format!("DB: {e}"))?;
                imported_ids.push(track_id);
            }
            Err(e) => {
                let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
                db.update_status(&track_id, "error", Some(&e.to_string()))
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
pub fn load_track(
    deck: String,
    track_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let deck_id = parse_deck(&deck)?;

    // Open the PCM cache file
    let pcm_path = state.cache.pcm_path(&track_id);
    if !pcm_path.exists() {
        return Err(format!("PCM cache not found for track {track_id}"));
    }

    let track_buffer = TrackBuffer::open(&pcm_path)
        .map_err(|e| format!("Failed to open track buffer: {e}"))?;

    state.load_track_to_deck(deck_id, Arc::new(track_buffer));

    // Also send a Play command right away
    state.send_command(Command::Play { deck: deck_id }, CommandSource::User);

    Ok(())
}

/// Play a deck.
#[tauri::command]
pub fn play(deck: String, state: State<'_, AppState>) -> Result<(), String> {
    let deck_id = parse_deck(&deck)?;
    state.send_command(Command::Play { deck: deck_id }, CommandSource::User);
    Ok(())
}

/// Pause a deck.
#[tauri::command]
pub fn pause(deck: String, state: State<'_, AppState>) -> Result<(), String> {
    let deck_id = parse_deck(&deck)?;
    state.send_command(Command::Pause { deck: deck_id }, CommandSource::User);
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
pub fn set_deck_gain(deck: String, gain_db: f32, state: State<'_, AppState>) -> Result<(), String> {
    let deck_id = parse_deck(&deck)?;
    state.send_command(
        Command::SetDeckGain { deck: deck_id, gain_db },
        CommandSource::User,
    );
    Ok(())
}

/// Set EQ band gain for a deck.
#[tauri::command]
pub fn set_eq(deck: String, band: String, gain_db: f32, state: State<'_, AppState>) -> Result<(), String> {
    let deck_id = parse_deck(&deck)?;
    let eq_band = match band.to_lowercase().as_str() {
        "low" => EqBand::Low,
        "mid" => EqBand::Mid,
        "high" => EqBand::High,
        _ => return Err(format!("Invalid EQ band: {band}")),
    };
    state.send_command(
        Command::SetEq { deck: deck_id, band: eq_band, gain_db },
        CommandSource::User,
    );
    Ok(())
}

/// Set filter parameters for a deck.
#[tauri::command]
pub fn set_filter(deck: String, cutoff_hz: f32, resonance: f32, state: State<'_, AppState>) -> Result<(), String> {
    let deck_id = parse_deck(&deck)?;
    state.send_command(
        Command::SetFilter { deck: deck_id, cutoff_hz, resonance },
        CommandSource::User,
    );
    Ok(())
}

/// Seek a deck to a position in milliseconds.
#[tauri::command]
pub fn seek(deck: String, position_ms: u64, state: State<'_, AppState>) -> Result<(), String> {
    let deck_id = parse_deck(&deck)?;
    // Convert ms to beats approximately (will be refined with track analysis)
    // For now: beat = ms * bpm / 60000
    let bpm = 128.0; // default; ideally get from engine state
    let beat = (position_ms as f64 * bpm / 60000.0) as u64;
    state.send_command(
        Command::Seek { deck: deck_id, beat },
        CommandSource::User,
    );
    Ok(())
}

/// Set a loop on a deck.
#[tauri::command]
pub fn set_loop(deck: String, length_bars: u32, state: State<'_, AppState>) -> Result<(), String> {
    let deck_id = parse_deck(&deck)?;
    // Start beat = current position (approximate); the engine will handle it
    // For now, use beat 0 — the UI will refine this
    state.send_command(
        Command::SetLoop { deck: deck_id, start_beat: 0, length_bars },
        CommandSource::User,
    );
    Ok(())
}

/// Clear the loop on a deck.
#[tauri::command]
pub fn clear_loop(deck: String, state: State<'_, AppState>) -> Result<(), String> {
    let deck_id = parse_deck(&deck)?;
    state.send_command(
        Command::ClearLoop { deck: deck_id },
        CommandSource::User,
    );
    Ok(())
}

/// Set the tempo (BPM) for a deck.
#[tauri::command]
pub fn set_tempo(deck: String, bpm: f64, state: State<'_, AppState>) -> Result<(), String> {
    let _deck_id = parse_deck(&deck)?;
    state.send_command(
        Command::SetTempo { deck: _deck_id, bpm },
        CommandSource::User,
    );
    Ok(())
}

/// Analyze a track.
#[tauri::command]
pub fn analyze_track(track_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.clone();
    let cache = state.cache.clone();

    // Spawn a background thread to do the analysis
    std::thread::spawn(move || {
        let track = match db.get_track(&track_id) {
            Ok(Some(t)) => t,
            _ => return,
        };

        if track.import_status != "ready" && track.import_status != "analyzed" {
            let _ = db.update_status(&track_id, "analysis_failed", Some("Track not ready"));
            return;
        }

        let pcm_path = cache.cache_dir().join(format!("{}.adjp", track.file_hash));
        let buffer = match autodj_core::buffer::TrackBuffer::open(&pcm_path) {
            Ok(b) => b,
            Err(_) => {
                let _ = db.update_status(&track_id, "analysis_failed", Some("Failed to open PCM buffer"));
                return;
            }
        };

        // Extract all samples into a Vec<f32> for analysis
        let total = buffer.total_frames() * 2;
        let mut samples = Vec::with_capacity(total);
        for i in 0..total {
            samples.push(buffer.get_sample(i));
        }

        let peaks_dir = cache.cache_dir();

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
                let _ = db.save_track_profile(
                    &track_id,
                    &json,
                    profile.tempo.bpm,
                    profile.tempo.confidence,
                    &profile.key.key,
                    &profile.key.camelot,
                    profile.key.confidence,
                    profile.loudness.integrated_lufs,
                    profile.energy.average,
                );
                let _ = db.update_status(&track_id, "analyzed", None);
            }
            Err(e) => {
                let _ = db.update_status(&track_id, "analysis_failed", Some(&e));
            }
        }
    });

    Ok(())
}

/// Get a track profile.
#[tauri::command]
pub fn get_track_profile(track_id: String, state: State<'_, AppState>) -> Result<Option<String>, String> {
    state.db.get_track_profile_json(&track_id).map_err(|e| e.to_string())
}

/// Get the current engine state snapshot.
#[tauri::command]
pub fn get_engine_state(state: State<'_, AppState>) -> Result<Option<EngineStateSnapshot>, String> {
    Ok(state.latest_state())
}

fn parse_deck(deck: &str) -> Result<DeckId, String> {
    match deck {
        "A" | "a" => Ok(DeckId::A),
        "B" | "b" => Ok(DeckId::B),
        _ => Err(format!("Invalid deck: {deck}")),
    }
}
