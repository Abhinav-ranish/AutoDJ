//! Spotify integration: OAuth PKCE, Web API client, spotDL orchestration,
//! and metadata enrichment.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Spotify OAuth tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: u64,
    pub user_name: String,
    pub user_id: String,
}

impl SpotifyTokens {
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now >= self.expires_at.saturating_sub(60)
    }
}

/// A Spotify playlist summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyPlaylist {
    pub id: String,
    pub name: String,
    pub image_url: Option<String>,
    pub track_count: u32,
    pub owner: String,
}

/// A track from a Spotify playlist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyTrack {
    pub id: String,
    pub name: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    pub spotify_url: String,
    pub artist_id: String,
}

/// Spotify enrichment data for classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyEnrichment {
    pub genre: String,
    pub subgenre: String,
    pub mood: String,
    pub vibe: String,
    pub mix_notes: String,
    pub ai_tags: Vec<String>,
}

/// Spotify API client with OAuth token management.
pub struct SpotifyClient {
    client: Client,
    client_id: String,
}

impl SpotifyClient {
    pub fn new(client_id: &str) -> Self {
        Self {
            client: Client::new(),
            client_id: client_id.to_string(),
        }
    }
}
