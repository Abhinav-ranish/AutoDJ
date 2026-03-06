//! AutoDJ Tauri application library.

pub mod commands;
pub mod state;

use state::AppState;

/// Run the Tauri application.
///
/// # Errors
/// Returns an error if the app fails to initialize.
pub fn run() {
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
            commands::analyze_track,
            commands::get_track_profile,
            commands::get_engine_state,
            commands::execute_transition,
            commands::request_next_track,
            commands::set_mode,
            commands::set_ai_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
