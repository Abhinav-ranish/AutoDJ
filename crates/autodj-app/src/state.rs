//! App state shared across Tauri commands.

use autodj_core::buffer::TrackBuffer;
use autodj_core::command::{Command, CommandSource, DeckId};
use autodj_core::engine::{AudioEngine, EngineStateSnapshot, TrackLoader};
use autodj_storage::cache::CacheManager;
use autodj_storage::db::Database;
use autodj_ai::{AiConfig, planner::AiPlanner};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DjMode {
    Manual,
    Assisted,
    Autodj,
}

/// Shared application state managed by Tauri.
pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub cache: Arc<CacheManager>,
    pub engine_sender: Mutex<autodj_core::engine::EngineCommandSender>,
    pub engine_receiver: Mutex<autodj_core::engine::EngineStateReceiver>,
    pub track_loader: Mutex<TrackLoader>,
    pub ai_planner: tokio::sync::RwLock<AiPlanner>,
    pub dj_mode: std::sync::RwLock<DjMode>,
    pub data_dir: PathBuf,
    pub sample_rate: u32,
}

impl AppState {
    /// Initialize application state.
    pub fn new() -> Result<Self, String> {
        // Determine data directory
        let data_dir = dirs_data_dir();
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| format!("Failed to create data dir: {e}"))?;

        // Initialize database
        let db_path = data_dir.join("autodj.db");
        let db = Database::open(&db_path)
            .map_err(|e| format!("Failed to open database: {e}"))?;

        // Initialize cache
        let cache = CacheManager::new(&data_dir)
            .map_err(|e| format!("Failed to init cache: {e}"))?;

        // Create track loader FIRST (engine needs the receiver)
        let (loader, load_receiver) = autodj_core::engine::create_track_loader();

        // Initialize audio engine with the track load receiver
        let (engine, stream) = AudioEngine::new(load_receiver)?;
        let sample_rate = engine.sample_rate;

        Box::leak(Box::new(stream));

        Ok(Self {
            db: Arc::new(Mutex::new(db)),
            cache: Arc::new(cache),
            engine_sender: Mutex::new(engine.command_sender),
            engine_receiver: Mutex::new(engine.state_receiver),
            track_loader: Mutex::new(loader),
            ai_planner: tokio::sync::RwLock::new(AiPlanner::new(AiConfig::default())),
            dj_mode: std::sync::RwLock::new(DjMode::Manual),
            data_dir,
            sample_rate,
        })
    }

    /// Send a command to the audio engine.
    pub fn send_command(&self, command: Command, source: CommandSource) -> bool {
        self.engine_sender
            .lock()
            .map(|mut sender| sender.send(command, source))
            .unwrap_or(false)
    }

    /// Get the latest engine state snapshot.
    pub fn latest_state(&self) -> Option<EngineStateSnapshot> {
        self.engine_receiver
            .lock()
            .ok()
            .and_then(|mut recv| recv.latest())
    }

    /// Load a track buffer onto a deck.
    pub fn load_track_to_deck(
        &self,
        deck: DeckId,
        track: Arc<TrackBuffer>,
    ) -> bool {
        self.track_loader
            .lock()
            .map(|mut loader| loader.load(deck, track))
            .unwrap_or(false)
    }
}

/// Get the data directory (~/.autodj).
/// Get the data directory (~/.autodj or %APPDATA%\autodj).
fn dirs_data_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".autodj")
}
