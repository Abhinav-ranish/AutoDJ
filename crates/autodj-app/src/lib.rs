//! AutoDJ Tauri application library.

pub mod commands;
pub mod conductor;
pub mod state;

use state::AppState;
use tauri::Emitter;
use tauri::Manager;

/// Run the Tauri application.
///
/// # Errors
/// Returns an error if the app fails to initialize.
pub fn run() {
    dotenvy::dotenv().ok();
    env_logger::init();

    let app_state = match AppState::new() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to initialize app state: {e}");
            std::process::exit(1);
        }
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::import_files,
            commands::get_tracks,
            commands::load_track,
            commands::play,
            commands::pause,
            commands::set_crossfader,
            commands::set_deck_gain,
            commands::set_eq,
            commands::set_filter,
            commands::seek,
            commands::set_loop,
            commands::clear_loop,
            commands::set_tempo,
            commands::set_speed,
            commands::analyze_track,
            commands::get_track_profile,
            commands::get_engine_state,
            commands::execute_transition,
            commands::request_next_track,
            commands::set_mode,
            commands::set_ai_config,
            commands::sync_to_master,
            commands::set_hot_cue,
            commands::get_hot_cues,
            commands::jump_to_hot_cue,
            commands::clear_library,
            commands::classify_track,
            commands::start_recording,
            commands::stop_recording,
            commands::get_album_art,
            commands::set_eq_kill,
            commands::set_eq_mode,
            commands::seek_ms,
            commands::get_auto_cues,
            commands::submit_transition_feedback,
            commands::get_preference,
            commands::set_preference,
            commands::get_all_preferences,
            commands::get_audio_devices,
            commands::create_playlist,
            commands::get_playlists,
            commands::rename_playlist,
            commands::delete_playlist,
            commands::add_track_to_playlist,
            commands::remove_track_from_playlist,
            commands::reorder_playlist,
            commands::get_playlist_tracks,
            commands::get_sessions,
            commands::export_cue_sheet,
            commands::beat_repeat,
            commands::beat_repeat_off,
            commands::set_slip_mode,
            commands::vinyl_brake,
            commands::spinback,
            commands::set_delay,
            commands::clear_delay,
            commands::jump_to_marker,
            commands::test_ai_provider,
            commands::get_ai_debug_info,
            commands::get_conductor_state,
            commands::set_conductor_queue,
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            // Allow the asset protocol to serve files from ~/.autodj
            let state = handle.state::<AppState>();
            let base_dir = state.cache.base_dir().to_path_buf();
            let scope = app.asset_protocol_scope();
            scope.allow_directory(&base_dir, true)
                .unwrap_or_else(|e| log::warn!("Failed to add asset scope: {e}"));

            // Spawn the backend conductor loop (owns AutoDJ state machine)
            conductor::spawn_conductor(handle.clone());

            // Spawn a thread to poll engine state and emit events
            std::thread::spawn(move || {
                loop {
                    let state = handle.state::<AppState>();
                    if let Some(snapshot) = state.latest_state() {
                        let _ = handle.emit("engine-state", snapshot);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(33)); // ~30Hz
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
