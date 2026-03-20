//! Spotify integration: OAuth PKCE, Web API client, spotDL orchestration,
//! and metadata enrichment.

use base64::Engine;
use rand::Rng;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const SPOTIFY_AUTH_URL: &str = "https://accounts.spotify.com/authorize";
const SPOTIFY_TOKEN_URL: &str = "https://accounts.spotify.com/api/token";
const REDIRECT_URI: &str = "http://localhost:9876/callback";
const SCOPES: &str = "playlist-read-private playlist-read-collaborative user-library-read";

// ---------------------------------------------------------------------------
// PKCE helpers
// ---------------------------------------------------------------------------

fn generate_code_verifier() -> String {
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..64).map(|_| rng.gen()).collect();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bytes)
}

fn generate_code_challenge(verifier: &str) -> String {
    let hash = Sha256::digest(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash)
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

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

    // -----------------------------------------------------------------------
    // OAuth PKCE flow
    // -----------------------------------------------------------------------

    /// Run the full OAuth PKCE flow: open browser, wait for callback, exchange
    /// code for tokens, and fetch the user profile.
    pub async fn connect(&self) -> Result<SpotifyTokens, String> {
        let code_verifier = generate_code_verifier();
        let code_challenge = generate_code_challenge(&code_verifier);

        let auth_url = format!(
            "{}?client_id={}&response_type=code&redirect_uri={}&scope={}&code_challenge_method=S256&code_challenge={}",
            SPOTIFY_AUTH_URL,
            urlencoding::encode(&self.client_id),
            urlencoding::encode(REDIRECT_URI),
            urlencoding::encode(SCOPES),
            urlencoding::encode(&code_challenge),
        );

        open::that(&auth_url).map_err(|e| format!("Failed to open browser: {e}"))?;

        let code = Self::wait_for_callback().await?;
        let mut tokens = self.exchange_code(&code, &code_verifier).await?;

        let (name, id) = self.get_user_profile(&tokens.access_token).await?;
        tokens.user_name = name;
        tokens.user_id = id;

        Ok(tokens)
    }

    /// Start a tiny HTTP server on port 9876 and wait up to 120 s for the
    /// Spotify redirect with the authorization code.
    async fn wait_for_callback() -> Result<String, String> {
        // Run the blocking tiny_http server on a dedicated thread.
        tokio::task::spawn_blocking(|| {
            let server = tiny_http::Server::http("127.0.0.1:9876")
                .map_err(|e| format!("Failed to start callback server: {e}"))?;

            // 120-second timeout
            let deadline = std::time::Instant::now() + Duration::from_secs(120);

            loop {
                let remaining = deadline
                    .checked_duration_since(std::time::Instant::now())
                    .unwrap_or(Duration::ZERO);

                if remaining.is_zero() {
                    return Err("Timed out waiting for Spotify callback".to_string());
                }

                let request = match server.recv_timeout(remaining) {
                    Ok(Some(req)) => req,
                    Ok(None) => continue,
                    Err(e) => return Err(format!("Callback server error: {e}")),
                };

                let url = request.url().to_string();

                // Respond with a friendly "Connected!" page regardless.
                let html = r#"<html><body style="font-family:sans-serif;text-align:center;padding:60px"><h1>Connected!</h1><p>You can close this tab and return to AutoDJ.</p></body></html>"#;
                let response = tiny_http::Response::from_string(html)
                    .with_header(
                        "Content-Type: text/html"
                            .parse::<tiny_http::Header>()
                            .unwrap(),
                    );
                let _ = request.respond(response);

                // Extract the authorization code from the query string.
                if let Some(query) = url.split('?').nth(1) {
                    for param in query.split('&') {
                        if let Some(code) = param.strip_prefix("code=") {
                            return Ok(code.to_string());
                        }
                    }
                    // Check for error parameter.
                    for param in query.split('&') {
                        if let Some(err) = param.strip_prefix("error=") {
                            return Err(format!("Spotify auth error: {err}"));
                        }
                    }
                }
            }
        })
        .await
        .map_err(|e| format!("Callback task panicked: {e}"))?
    }

    /// Exchange an authorization code for access/refresh tokens.
    async fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
    ) -> Result<SpotifyTokens, String> {
        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", REDIRECT_URI),
            ("client_id", &self.client_id),
            ("code_verifier", code_verifier),
        ];

        let resp = self
            .client
            .post(SPOTIFY_TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Token request failed: {e}"))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Token exchange failed: {body}"));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse token response: {e}"))?;

        let expires_in = json["expires_in"].as_u64().unwrap_or(3600);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Ok(SpotifyTokens {
            access_token: json["access_token"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            refresh_token: json["refresh_token"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            expires_at: now + expires_in,
            user_name: String::new(),
            user_id: String::new(),
        })
    }

    /// Refresh an expired access token.
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<SpotifyTokens, String> {
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &self.client_id),
        ];

        let resp = self
            .client
            .post(SPOTIFY_TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Refresh request failed: {e}"))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Token refresh failed: {body}"));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse refresh response: {e}"))?;

        let expires_in = json["expires_in"].as_u64().unwrap_or(3600);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Spotify may or may not return a new refresh token.
        let new_refresh = json["refresh_token"]
            .as_str()
            .unwrap_or(refresh_token)
            .to_string();

        let access_token = json["access_token"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        let (user_name, user_id) = self.get_user_profile(&access_token).await?;

        Ok(SpotifyTokens {
            access_token,
            refresh_token: new_refresh,
            expires_at: now + expires_in,
            user_name,
            user_id,
        })
    }

    /// Fetch the current user's display name and ID.
    async fn get_user_profile(&self, access_token: &str) -> Result<(String, String), String> {
        let resp = self
            .client
            .get("https://api.spotify.com/v1/me")
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| format!("User profile request failed: {e}"))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("User profile fetch failed: {body}"));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse user profile: {e}"))?;

        let display_name = json["display_name"]
            .as_str()
            .unwrap_or("Unknown")
            .to_string();
        let user_id = json["id"].as_str().unwrap_or_default().to_string();

        Ok((display_name, user_id))
    }

    // -----------------------------------------------------------------------
    // Spotify Web API — playlists & tracks
    // -----------------------------------------------------------------------

    /// Fetch all playlists for the current user, handling pagination.
    pub async fn get_playlists(
        &self,
        access_token: &str,
    ) -> Result<Vec<SpotifyPlaylist>, String> {
        let mut playlists = Vec::new();
        let mut url = "https://api.spotify.com/v1/me/playlists?limit=50".to_string();

        loop {
            let resp = self
                .client
                .get(&url)
                .bearer_auth(access_token)
                .send()
                .await
                .map_err(|e| format!("Playlists request failed: {e}"))?;

            if !resp.status().is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(format!("Playlists fetch failed: {body}"));
            }

            let json: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| format!("Failed to parse playlists: {e}"))?;

            if let Some(items) = json["items"].as_array() {
                for item in items {
                    let image_url = item["images"]
                        .as_array()
                        .and_then(|imgs| imgs.first())
                        .and_then(|img| img["url"].as_str())
                        .map(|s| s.to_string());

                    playlists.push(SpotifyPlaylist {
                        id: item["id"].as_str().unwrap_or_default().to_string(),
                        name: item["name"].as_str().unwrap_or_default().to_string(),
                        image_url,
                        track_count: item["tracks"]["total"].as_u64().unwrap_or(0) as u32,
                        owner: item["owner"]["display_name"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string(),
                    });
                }
            }

            match json["next"].as_str() {
                Some(next_url) => url = next_url.to_string(),
                None => break,
            }
        }

        Ok(playlists)
    }

    /// Fetch all tracks in a playlist, handling pagination.
    pub async fn get_playlist_tracks(
        &self,
        access_token: &str,
        playlist_id: &str,
    ) -> Result<Vec<SpotifyTrack>, String> {
        let mut tracks = Vec::new();
        let mut url = format!(
            "https://api.spotify.com/v1/playlists/{}/tracks?limit=100",
            playlist_id
        );

        loop {
            let resp = self
                .client
                .get(&url)
                .bearer_auth(access_token)
                .send()
                .await
                .map_err(|e| format!("Playlist tracks request failed: {e}"))?;

            if !resp.status().is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(format!("Playlist tracks fetch failed: {body}"));
            }

            let json: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| format!("Failed to parse playlist tracks: {e}"))?;

            if let Some(items) = json["items"].as_array() {
                for item in items {
                    let track = &item["track"];

                    // Skip local files or null tracks.
                    if track.is_null() || track["id"].is_null() {
                        continue;
                    }

                    let artist_name = track["artists"]
                        .as_array()
                        .and_then(|a| a.first())
                        .and_then(|a| a["name"].as_str())
                        .unwrap_or("Unknown")
                        .to_string();

                    let artist_id = track["artists"]
                        .as_array()
                        .and_then(|a| a.first())
                        .and_then(|a| a["id"].as_str())
                        .unwrap_or_default()
                        .to_string();

                    let spotify_url = track["external_urls"]["spotify"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string();

                    tracks.push(SpotifyTrack {
                        id: track["id"].as_str().unwrap_or_default().to_string(),
                        name: track["name"].as_str().unwrap_or_default().to_string(),
                        artist: artist_name,
                        album: track["album"]["name"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string(),
                        duration_ms: track["duration_ms"].as_u64().unwrap_or(0),
                        spotify_url,
                        artist_id,
                    });
                }
            }

            match json["next"].as_str() {
                Some(next_url) => url = next_url.to_string(),
                None => break,
            }
        }

        Ok(tracks)
    }

    /// Fetch an artist's genres.
    pub async fn get_artist_genres(
        &self,
        access_token: &str,
        artist_id: &str,
    ) -> Result<Vec<String>, String> {
        let url = format!("https://api.spotify.com/v1/artists/{}", artist_id);

        let resp = self
            .client
            .get(&url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| format!("Artist request failed: {e}"))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Artist fetch failed: {body}"));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse artist: {e}"))?;

        let genres = json["genres"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        Ok(genres)
    }
}

// ---------------------------------------------------------------------------
// Metadata mapper
// ---------------------------------------------------------------------------

/// Map Spotify audio features and artist genres to an `SpotifyEnrichment`
/// suitable for DJ classification.
pub fn map_spotify_to_classification(
    genres: &[String],
    valence: f64,
    energy: f64,
    danceability: f64,
) -> SpotifyEnrichment {
    // --- Genre mapping ---
    let genre_lower: Vec<String> = genres.iter().map(|g| g.to_lowercase()).collect();

    let edm_keywords = [
        "techno",
        "house",
        "trance",
        "drum and bass",
        "dnb",
        "dubstep",
        "electro",
        "hardstyle",
        "garage",
        "breakbeat",
        "jungle",
        "ambient",
        "downtempo",
    ];
    let hiphop_keywords = ["hip hop", "hip-hop", "rap", "trap", "grime"];
    let rnb_keywords = ["r&b", "rnb", "soul", "neo soul"];
    let pop_keywords = ["pop", "dance pop", "electropop"];
    let rock_keywords = ["rock", "indie", "punk", "metal", "alternative"];

    let (primary_genre, subgenre) = if let Some(g) = genre_lower
        .iter()
        .find(|g| edm_keywords.iter().any(|kw| g.contains(kw)))
    {
        ("electronic".to_string(), g.clone())
    } else if let Some(g) = genre_lower
        .iter()
        .find(|g| hiphop_keywords.iter().any(|kw| g.contains(kw)))
    {
        ("hip-hop".to_string(), g.clone())
    } else if let Some(g) = genre_lower
        .iter()
        .find(|g| rnb_keywords.iter().any(|kw| g.contains(kw)))
    {
        ("r&b".to_string(), g.clone())
    } else if let Some(g) = genre_lower
        .iter()
        .find(|g| pop_keywords.iter().any(|kw| g.contains(kw)))
    {
        ("pop".to_string(), g.clone())
    } else if let Some(g) = genre_lower
        .iter()
        .find(|g| rock_keywords.iter().any(|kw| g.contains(kw)))
    {
        ("rock".to_string(), g.clone())
    } else {
        let first = genre_lower.first().cloned().unwrap_or("unknown".into());
        (first.clone(), first)
    };

    // --- Mood from valence ---
    let mood = if valence > 0.75 {
        "euphoric"
    } else if valence > 0.55 {
        "uplifting"
    } else if valence > 0.4 {
        "energetic"
    } else if valence > 0.25 {
        "melancholic"
    } else {
        "dark"
    };

    // --- Vibe from energy + danceability ---
    let vibe = if energy > 0.7 && danceability > 0.7 {
        "anthem"
    } else if energy > 0.7 {
        "peak_time"
    } else if energy > 0.4 {
        "warm_up"
    } else {
        "closer"
    };

    // --- Mix notes ---
    let mix_notes = format!(
        "{} {} track with {} energy. Suited for {} sets.",
        mood,
        primary_genre,
        if energy > 0.7 {
            "high"
        } else if energy > 0.4 {
            "mid"
        } else {
            "low"
        },
        vibe.replace('_', " "),
    );

    // --- AI tags ---
    let mut ai_tags: Vec<String> = vec![
        primary_genre.clone(),
        mood.to_string(),
        vibe.to_string(),
    ];
    if danceability > 0.7 {
        ai_tags.push("danceable".to_string());
    }
    if energy > 0.8 {
        ai_tags.push("high-energy".to_string());
    }
    if valence < 0.3 {
        ai_tags.push("moody".to_string());
    }

    SpotifyEnrichment {
        genre: primary_genre,
        subgenre,
        mood: mood.to_string(),
        vibe: vibe.to_string(),
        mix_notes,
        ai_tags,
    }
}

// ---------------------------------------------------------------------------
// spotDL orchestration
// ---------------------------------------------------------------------------

/// Check whether spotDL is installed and return its version string.
pub fn check_spotdl() -> Result<String, String> {
    let output = std::process::Command::new("spotdl")
        .arg("--version")
        .output()
        .map_err(|_| {
            "spotDL not found. Install it with: pip install spotdl".to_string()
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "spotDL check failed: {stderr}. Install with: pip install spotdl"
        ));
    }

    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(version)
}

/// Download a Spotify track using spotDL and return the path to the
/// downloaded MP3 file.
pub async fn download_track(
    spotify_url: &str,
    output_dir: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    // Ensure output directory exists.
    tokio::fs::create_dir_all(output_dir)
        .await
        .map_err(|e| format!("Failed to create output directory: {e}"))?;

    let output = tokio::process::Command::new("spotdl")
        .arg("download")
        .arg(spotify_url)
        .arg("--output")
        .arg(output_dir)
        .arg("--format")
        .arg("mp3")
        .arg("--bitrate")
        .arg("320k")
        .output()
        .await
        .map_err(|e| format!("Failed to run spotdl: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("spotdl download failed: {stderr}"));
    }

    // Find the newest .mp3 file in the output directory.
    let mut newest: Option<(std::path::PathBuf, std::time::SystemTime)> = None;
    let mut read_dir = tokio::fs::read_dir(output_dir)
        .await
        .map_err(|e| format!("Failed to read output directory: {e}"))?;

    while let Some(entry) = read_dir
        .next_entry()
        .await
        .map_err(|e| format!("Failed to read directory entry: {e}"))?
    {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("mp3") {
            let meta = entry
                .metadata()
                .await
                .map_err(|e| format!("Failed to read file metadata: {e}"))?;
            let modified = meta.modified().unwrap_or(UNIX_EPOCH);
            if newest.as_ref().map_or(true, |(_, t)| modified > *t) {
                newest = Some((path, modified));
            }
        }
    }

    newest
        .map(|(path, _)| path)
        .ok_or_else(|| "No MP3 file found after spotdl download".to_string())
}
