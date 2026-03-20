//! App state shared across Tauri commands.

use crate::conductor::ConductorState;
use autodj_ai::{planner::AiPlanner, AiConfig};
use autodj_core::buffer::TrackBuffer;
use autodj_core::command::{Command, CommandSource, DeckId};
use autodj_core::engine::{AudioEngine, EngineStateSnapshot, TrackLoader};
use autodj_core::recorder::{self, RecordConsumer, RecordingFlag};
use autodj_storage::cache::CacheManager;
use autodj_storage::db::Database;
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
    pub current_session_id: std::sync::RwLock<Option<String>>,
    pub recording_flag: RecordingFlag,
    pub record_consumer: Mutex<Option<RecordConsumer>>,
    pub data_dir: PathBuf,
    pub sample_rate: u32,
    /// Backend conductor state — owns the AutoDJ loop.
    pub conductor: tokio::sync::RwLock<ConductorState>,
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
        let db = Database::open(&db_path).map_err(|e| format!("Failed to open database: {e}"))?;

        // Initialize cache
        let cache =
            CacheManager::new(&data_dir).map_err(|e| format!("Failed to init cache: {e}"))?;

        // Create track loader FIRST (engine needs the receiver)
        let (loader, load_receiver) = autodj_core::engine::create_track_loader();

        // Create recording ring buffer and flag
        let (rec_producer, rec_consumer) = recorder::create_record_ring();
        let recording_flag = recorder::create_recording_flag();

        // Initialize audio engine with the track load receiver and recorder
        let (engine, stream) = AudioEngine::new(
            load_receiver,
            Some(rec_producer),
            Some(recording_flag.clone()),
        )?;
        let sample_rate = engine.sample_rate;

        Box::leak(Box::new(stream));

        Ok(Self {
            db: Arc::new(Mutex::new(db)),
            cache: Arc::new(cache),
            engine_sender: Mutex::new(engine.command_sender),
            engine_receiver: Mutex::new(engine.state_receiver),
            track_loader: Mutex::new(loader),
            ai_planner: tokio::sync::RwLock::new(AiPlanner::new(ai_config_from_env())),
            dj_mode: std::sync::RwLock::new(DjMode::Manual),
            current_session_id: std::sync::RwLock::new(None),
            recording_flag,
            record_consumer: Mutex::new(Some(rec_consumer)),
            data_dir,
            sample_rate,
            conductor: tokio::sync::RwLock::new(ConductorState::default()),
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
        track_id: String,
        track: Arc<TrackBuffer>,
    ) -> bool {
        self.track_loader
            .lock()
            .map(|mut loader| loader.load(deck, track_id, track))
            .unwrap_or(false)
    }
}

/// Get the data directory (~/.autodj).
fn dirs_data_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".autodj")
}

/// Build an AiConfig from environment variables (AI_PROVIDER, GEMINI_API_KEY, GROQ_API_KEY, OLLAMA_MODEL).
fn ai_config_from_env() -> AiConfig {
    let provider = std::env::var("AI_PROVIDER")
        .unwrap_or_else(|_| "gemini".to_string())
        .to_lowercase();

    let (api_key, model, endpoint, enabled) = match provider.as_str() {
        "groq" => (
            std::env::var("GROQ_API_KEY")
                .ok()
                .filter(|k| !k.is_empty() && !k.starts_with("gsk_...")),
            "llama-3.1-8b-instant".to_string(),
            Some("https://api.groq.com/openai/v1".to_string()),
            None, // enabled derived from api_key presence
        ),
        "ollama" => {
            let model = std::env::var("OLLAMA_MODEL")
                .unwrap_or_else(|_| "qwen3:4b".to_string());
            let endpoint = std::env::var("OLLAMA_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:11434/v1".to_string());
            (
                None, // no API key needed for local Ollama
                model,
                Some(endpoint),
                Some(true), // always enabled — Ollama is local, no key needed
            )
        }
        _ => (
            std::env::var("GEMINI_API_KEY")
                .ok()
                .filter(|k| !k.is_empty() && !k.starts_with("AIzaSy...")),
            "gemini-2.5-flash".to_string(),
            Some("https://generativelanguage.googleapis.com/v1beta/models".to_string()),
            None,
        ),
    };

    let mut config = AiConfig {
        enabled: enabled.unwrap_or(api_key.is_some()),
        provider: provider.clone(),
        model,
        api_key,
        endpoint_url: endpoint,
        ..AiConfig::default()
    };

    // Set default rate limit for Groq (tokens per minute)
    if provider == "groq" {
        config.rate_limit_tpm = Some(13_000);
    }

    config
}
