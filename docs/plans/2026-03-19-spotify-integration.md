# Spotify Integration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add Spotify OAuth PKCE integration for browsing playlists, downloading tracks via spotDL, and enriching library metadata with Spotify genre/mood/energy data.

**Architecture:** A new `spotify.rs` module in `autodj-app` handles OAuth PKCE and Spotify Web API calls. spotDL (external Python tool) downloads audio files which feed into the existing import pipeline. Spotify metadata supplements our audio analysis for genre/mood/vibe classification.

**Tech Stack:** Rust (reqwest for Spotify API, tiny-http for OAuth callback), TypeScript/React (playlist browser UI), spotDL (external, user-installed)

---

### Task 1: Add dependencies and create Spotify module skeleton

**Files:**
- Modify: `crates/autodj-app/Cargo.toml`
- Create: `crates/autodj-app/src/spotify.rs`
- Modify: `crates/autodj-app/src/lib.rs`

**Step 1: Add dependencies to Cargo.toml**

In `crates/autodj-app/Cargo.toml`, add after the `dotenvy` line (line 24):
```toml
reqwest = { version = "0.12", features = ["json"] }
tiny_http = "0.12"
sha2 = "0.10"
base64 = "0.22"
rand = "0.8"
```

Note: `reqwest` is already a workspace dependency (used by `autodj-ai`), but `autodj-app` doesn't have it directly. `tiny_http` is for the OAuth callback server. `sha2`/`base64`/`rand` are for PKCE code generation.

**Step 2: Create spotify.rs skeleton**

Create `crates/autodj-app/src/spotify.rs`:

```rust
//! Spotify integration: OAuth PKCE, Web API client, spotDL orchestration,
//! and metadata enrichment.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Spotify OAuth tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: u64, // Unix timestamp
    pub user_name: String,
    pub user_id: String,
}

impl SpotifyTokens {
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now >= self.expires_at.saturating_sub(60) // 60s buffer
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
```

**Step 3: Register module in lib.rs**

In `crates/autodj-app/src/lib.rs`, add after line 5 (`pub mod state;`):
```rust
pub mod spotify;
```

**Step 4: Run check**

Run: `cargo check -p autodj-app`
Expected: Compiles cleanly.

**Step 5: Commit**

```
git add crates/autodj-app/Cargo.toml crates/autodj-app/src/spotify.rs crates/autodj-app/src/lib.rs
git commit -m "feat(app): add Spotify module skeleton and dependencies"
```

---

### Task 2: Implement OAuth PKCE flow

**Files:**
- Modify: `crates/autodj-app/src/spotify.rs`

**Step 1: Write PKCE helper functions**

Add to `spotify.rs` after the `SpotifyClient::new()`:

```rust
use base64::Engine;
use rand::Rng;
use sha2::{Digest, Sha256};

/// Generate a random PKCE code verifier (43-128 chars, URL-safe).
fn generate_code_verifier() -> String {
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..64).map(|_| rng.gen()).collect();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bytes)
}

/// Derive the S256 code challenge from a verifier.
fn generate_code_challenge(verifier: &str) -> String {
    let hash = Sha256::digest(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash)
}
```

**Step 2: Implement the OAuth connect flow**

Add to `SpotifyClient`:

```rust
const SPOTIFY_AUTH_URL: &str = "https://accounts.spotify.com/authorize";
const SPOTIFY_TOKEN_URL: &str = "https://accounts.spotify.com/api/token";
const REDIRECT_URI: &str = "http://localhost:9876/callback";
const SCOPES: &str = "playlist-read-private playlist-read-collaborative user-library-read";

impl SpotifyClient {
    /// Start the OAuth PKCE flow. Opens browser and waits for callback.
    /// Returns tokens on success.
    pub async fn connect(&self) -> Result<SpotifyTokens, String> {
        let code_verifier = generate_code_verifier();
        let code_challenge = generate_code_challenge(&code_verifier);

        // Build authorization URL
        let auth_url = format!(
            "{}?client_id={}&response_type=code&redirect_uri={}&scope={}&code_challenge_method=S256&code_challenge={}",
            SPOTIFY_AUTH_URL,
            urlencoding::encode(&self.client_id),
            urlencoding::encode(REDIRECT_URI),
            urlencoding::encode(SCOPES),
            urlencoding::encode(&code_challenge),
        );

        // Open in system browser
        if open::that(&auth_url).is_err() {
            return Err("Failed to open browser for Spotify login".to_string());
        }

        // Start local HTTP server to catch the redirect
        let auth_code = Self::wait_for_callback().await?;

        // Exchange code for tokens
        self.exchange_code(&auth_code, &code_verifier).await
    }

    /// Spin up a tiny HTTP server on localhost:9876, wait for the OAuth callback.
    async fn wait_for_callback() -> Result<String, String> {
        let server = tiny_http::Server::http("127.0.0.1:9876")
            .map_err(|e| format!("Failed to start OAuth callback server: {e}"))?;

        log::info!("[spotify] Waiting for OAuth callback on localhost:9876...");

        // Wait up to 120 seconds for the callback
        let timeout = std::time::Duration::from_secs(120);
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > timeout {
                return Err("Spotify login timed out (120s)".to_string());
            }

            if let Ok(Some(request)) = server.try_recv() {
                let url = request.url().to_string();
                if url.starts_with("/callback") {
                    // Parse ?code=... from URL
                    let code = url
                        .split('?')
                        .nth(1)
                        .and_then(|query| {
                            query.split('&').find_map(|param| {
                                let mut parts = param.splitn(2, '=');
                                if parts.next() == Some("code") {
                                    parts.next().map(|v| v.to_string())
                                } else {
                                    None
                                }
                            })
                        })
                        .ok_or_else(|| "No auth code in callback URL".to_string())?;

                    // Respond with success page
                    let response = tiny_http::Response::from_string(
                        "<html><body><h2>Connected to Spotify!</h2><p>You can close this tab.</p></body></html>"
                    ).with_header(
                        tiny_http::Header::from_bytes("Content-Type", "text/html").unwrap()
                    );
                    let _ = request.respond(response);

                    return Ok(code);
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    /// Exchange authorization code for access + refresh tokens.
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
            .map_err(|e| format!("Token exchange failed: {e}"))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Token exchange HTTP error: {text}"));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Token parse error: {e}"))?;

        let access_token = body["access_token"]
            .as_str()
            .ok_or("Missing access_token")?
            .to_string();
        let refresh_token = body["refresh_token"]
            .as_str()
            .ok_or("Missing refresh_token")?
            .to_string();
        let expires_in = body["expires_in"].as_u64().unwrap_or(3600);

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Fetch user profile
        let (user_name, user_id) = self.get_user_profile(&access_token).await?;

        Ok(SpotifyTokens {
            access_token,
            refresh_token,
            expires_at: now + expires_in,
            user_name,
            user_id,
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
            .map_err(|e| format!("Token refresh failed: {e}"))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Token refresh error: {text}"));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Token refresh parse error: {e}"))?;

        let access_token = body["access_token"]
            .as_str()
            .ok_or("Missing access_token")?
            .to_string();
        let new_refresh = body["refresh_token"]
            .as_str()
            .unwrap_or(refresh_token)
            .to_string();
        let expires_in = body["expires_in"].as_u64().unwrap_or(3600);

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let (user_name, user_id) = self.get_user_profile(&access_token).await?;

        Ok(SpotifyTokens {
            access_token,
            refresh_token: new_refresh,
            expires_at: now + expires_in,
            user_name,
            user_id,
        })
    }

    /// Fetch the current user's profile.
    async fn get_user_profile(&self, access_token: &str) -> Result<(String, String), String> {
        let resp = self
            .client
            .get("https://api.spotify.com/v1/me")
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| format!("Profile fetch failed: {e}"))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Profile parse error: {e}"))?;

        let display_name = body["display_name"]
            .as_str()
            .unwrap_or("Unknown")
            .to_string();
        let user_id = body["id"].as_str().unwrap_or("").to_string();

        Ok((display_name, user_id))
    }
}
```

**Step 3: Add `open` and `urlencoding` dependencies**

In `crates/autodj-app/Cargo.toml`, add:
```toml
open = "5"
urlencoding = "2.1"
```

**Step 4: Run check**

Run: `cargo check -p autodj-app`
Expected: Compiles cleanly.

**Step 5: Commit**

```
git add crates/autodj-app/src/spotify.rs crates/autodj-app/Cargo.toml
git commit -m "feat(app): implement Spotify OAuth PKCE flow with token refresh"
```

---

### Task 3: Implement Spotify API client (playlists + audio features)

**Files:**
- Modify: `crates/autodj-app/src/spotify.rs`

**Step 1: Add playlist and track fetching methods**

Add to `SpotifyClient`:

```rust
impl SpotifyClient {
    /// Get the current user's playlists.
    pub async fn get_playlists(&self, access_token: &str) -> Result<Vec<SpotifyPlaylist>, String> {
        let mut playlists = Vec::new();
        let mut url = "https://api.spotify.com/v1/me/playlists?limit=50".to_string();

        loop {
            let resp = self
                .client
                .get(&url)
                .bearer_auth(access_token)
                .send()
                .await
                .map_err(|e| format!("Playlist fetch failed: {e}"))?;

            let body: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| format!("Playlist parse error: {e}"))?;

            if let Some(items) = body["items"].as_array() {
                for item in items {
                    let image_url = item["images"]
                        .as_array()
                        .and_then(|imgs| imgs.first())
                        .and_then(|img| img["url"].as_str())
                        .map(|s| s.to_string());

                    playlists.push(SpotifyPlaylist {
                        id: item["id"].as_str().unwrap_or("").to_string(),
                        name: item["name"].as_str().unwrap_or("Untitled").to_string(),
                        image_url,
                        track_count: item["tracks"]["total"].as_u64().unwrap_or(0) as u32,
                        owner: item["owner"]["display_name"]
                            .as_str()
                            .unwrap_or("Unknown")
                            .to_string(),
                    });
                }
            }

            // Pagination
            if let Some(next) = body["next"].as_str() {
                url = next.to_string();
            } else {
                break;
            }
        }

        Ok(playlists)
    }

    /// Get all tracks from a playlist.
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
                .map_err(|e| format!("Playlist tracks fetch failed: {e}"))?;

            let body: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| format!("Playlist tracks parse error: {e}"))?;

            if let Some(items) = body["items"].as_array() {
                for item in items {
                    let track = &item["track"];
                    if track.is_null() {
                        continue;
                    }

                    let artist = track["artists"]
                        .as_array()
                        .and_then(|a| a.first())
                        .and_then(|a| a["name"].as_str())
                        .unwrap_or("Unknown");

                    let artist_id = track["artists"]
                        .as_array()
                        .and_then(|a| a.first())
                        .and_then(|a| a["id"].as_str())
                        .unwrap_or("");

                    let spotify_url = track["external_urls"]["spotify"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();

                    tracks.push(SpotifyTrack {
                        id: track["id"].as_str().unwrap_or("").to_string(),
                        name: track["name"].as_str().unwrap_or("Unknown").to_string(),
                        artist: artist.to_string(),
                        album: track["album"]["name"]
                            .as_str()
                            .unwrap_or("Unknown")
                            .to_string(),
                        duration_ms: track["duration_ms"].as_u64().unwrap_or(0),
                        spotify_url,
                        artist_id: artist_id.to_string(),
                    });
                }
            }

            if let Some(next) = body["next"].as_str() {
                url = next.to_string();
            } else {
                break;
            }
        }

        Ok(tracks)
    }

    /// Get genres for an artist.
    pub async fn get_artist_genres(
        &self,
        access_token: &str,
        artist_id: &str,
    ) -> Result<Vec<String>, String> {
        if artist_id.is_empty() {
            return Ok(Vec::new());
        }

        let url = format!("https://api.spotify.com/v1/artists/{}", artist_id);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| format!("Artist fetch failed: {e}"))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Artist parse error: {e}"))?;

        let genres = body["genres"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|g| g.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        Ok(genres)
    }
}
```

**Step 2: Add the metadata mapper**

```rust
/// Map Spotify genres and features to our classification taxonomy.
pub fn map_spotify_to_classification(
    genres: &[String],
    valence: f64,
    energy: f64,
    danceability: f64,
) -> SpotifyEnrichment {
    // Genre: pick the most specific genre from Spotify's list
    let genre = genres
        .iter()
        .find(|g| {
            // Prefer specific EDM subgenres
            ["house", "techno", "drum and bass", "dubstep", "trance", "ambient",
             "hip hop", "r&b", "pop", "rock", "latin", "afrobeats"]
                .iter()
                .any(|k| g.to_lowercase().contains(k))
        })
        .or(genres.first())
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());

    let subgenre = genres
        .iter()
        .find(|g| g != &&genre)
        .cloned()
        .unwrap_or_default();

    // Mood from valence
    let mood = if valence > 0.75 {
        "euphoric"
    } else if valence > 0.55 {
        "uplifting"
    } else if valence > 0.4 {
        "energetic"
    } else if valence > 0.25 {
        "melancholic"
    } else if valence > 0.1 {
        "dark"
    } else {
        "mysterious"
    };

    // Vibe from energy + danceability
    let vibe = if energy > 0.8 && danceability > 0.7 {
        "anthem"
    } else if energy > 0.7 {
        "peak_time"
    } else if energy > 0.5 && danceability > 0.6 {
        "filler"
    } else if energy > 0.3 {
        "warm_up"
    } else if energy < 0.3 {
        "closer"
    } else {
        "transition_tool"
    };

    let mix_notes = format!(
        "Energy {:.0}%, Danceability {:.0}%, Valence {:.0}% — {}",
        energy * 100.0,
        danceability * 100.0,
        valence * 100.0,
        if energy > 0.7 { "high energy, mix during buildup" }
        else if energy < 0.3 { "low energy, good for cooldown" }
        else { "mid energy, versatile mixing" }
    );

    let mut ai_tags: Vec<String> = Vec::new();
    if energy > 0.7 { ai_tags.push("high-energy".to_string()); }
    if danceability > 0.7 { ai_tags.push("danceable".to_string()); }
    if valence > 0.7 { ai_tags.push("feel-good".to_string()); }
    if valence < 0.3 { ai_tags.push("moody".to_string()); }
    for g in genres.iter().take(2) {
        ai_tags.push(g.clone());
    }

    SpotifyEnrichment {
        genre,
        subgenre,
        mood: mood.to_string(),
        vibe: vibe.to_string(),
        mix_notes,
        ai_tags,
    }
}
```

**Step 3: Run check**

Run: `cargo check -p autodj-app`
Expected: Compiles cleanly.

**Step 4: Commit**

```
git add crates/autodj-app/src/spotify.rs
git commit -m "feat(app): implement Spotify API client (playlists, artists, metadata mapper)"
```

---

### Task 4: Add spotDL download orchestration

**Files:**
- Modify: `crates/autodj-app/src/spotify.rs`

**Step 1: Add spotDL check and download functions**

```rust
use std::process::Command as StdCommand;

/// Check if spotDL is installed and available.
pub fn check_spotdl() -> Result<String, String> {
    let output = StdCommand::new("spotdl")
        .arg("--version")
        .output()
        .map_err(|_| "spotDL is not installed. Install with: pip install spotdl".to_string())?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(version)
    } else {
        Err("spotDL is not working correctly. Try: pip install --upgrade spotdl".to_string())
    }
}

/// Download a single track using spotDL.
/// Returns the path to the downloaded file.
pub async fn download_track(
    spotify_url: &str,
    output_dir: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create download dir: {e}"))?;

    let output = tokio::process::Command::new("spotdl")
        .arg("download")
        .arg(spotify_url)
        .arg("--output")
        .arg(output_dir.to_str().unwrap_or("."))
        .arg("--format")
        .arg("mp3")
        .arg("--bitrate")
        .arg("320k")
        .output()
        .await
        .map_err(|e| format!("spotDL failed to start: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("spotDL download failed: {stderr}"));
    }

    // Find the downloaded file (spotDL names it {Artist} - {Title}.mp3)
    let stdout = String::from_utf8_lossy(&output.stdout);
    log::info!("[spotify] spotDL output: {}", stdout);

    // Look for the most recently created file in the output dir
    let mut newest: Option<(std::path::PathBuf, std::time::SystemTime)> = None;
    if let Ok(entries) = std::fs::read_dir(output_dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if let Ok(modified) = meta.modified() {
                    if newest.as_ref().map_or(true, |(_, t)| modified > *t) {
                        if entry.path().extension().map_or(false, |e| e == "mp3") {
                            newest = Some((entry.path(), modified));
                        }
                    }
                }
            }
        }
    }

    newest
        .map(|(path, _)| path)
        .ok_or_else(|| "spotDL completed but no MP3 file found".to_string())
}
```

**Step 2: Run check**

Run: `cargo check -p autodj-app`
Expected: Compiles cleanly.

**Step 3: Commit**

```
git add crates/autodj-app/src/spotify.rs
git commit -m "feat(app): add spotDL download orchestration"
```

---

### Task 5: Add Tauri commands for Spotify

**Files:**
- Modify: `crates/autodj-app/src/commands.rs`
- Modify: `crates/autodj-app/src/lib.rs`

**Step 1: Add Spotify Tauri commands to commands.rs**

Add at the end of `commands.rs`, before the closing tests module (if any):

```rust
// ── Spotify Integration ────────────────────────────────────────────

#[tauri::command]
pub async fn spotify_connect(
    client_id: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    use crate::spotify::SpotifyClient;

    let client = SpotifyClient::new(&client_id);
    let tokens = client.connect().await?;

    // Persist tokens in DB preferences
    {
        let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
        db.set_preference("spotify_access_token", &tokens.access_token)
            .map_err(|e| format!("DB: {e}"))?;
        db.set_preference("spotify_refresh_token", &tokens.refresh_token)
            .map_err(|e| format!("DB: {e}"))?;
        db.set_preference("spotify_token_expiry", &tokens.expires_at.to_string())
            .map_err(|e| format!("DB: {e}"))?;
        db.set_preference("spotify_user_name", &tokens.user_name)
            .map_err(|e| format!("DB: {e}"))?;
        db.set_preference("spotify_user_id", &tokens.user_id)
            .map_err(|e| format!("DB: {e}"))?;
        db.set_preference("spotify_client_id", &client_id)
            .map_err(|e| format!("DB: {e}"))?;
    }

    Ok(serde_json::json!({
        "connected": true,
        "user_name": tokens.user_name,
        "user_id": tokens.user_id,
    }))
}

#[tauri::command]
pub async fn spotify_disconnect(state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
    let _ = db.set_preference("spotify_access_token", "");
    let _ = db.set_preference("spotify_refresh_token", "");
    let _ = db.set_preference("spotify_token_expiry", "0");
    let _ = db.set_preference("spotify_user_name", "");
    let _ = db.set_preference("spotify_user_id", "");
    Ok(())
}

#[tauri::command]
pub async fn spotify_status(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
    let user_name = db.get_preference("spotify_user_name").ok().flatten().unwrap_or_default();
    let refresh_token = db.get_preference("spotify_refresh_token").ok().flatten().unwrap_or_default();
    let connected = !refresh_token.is_empty();

    Ok(serde_json::json!({
        "connected": connected,
        "user_name": if connected { user_name } else { String::new() },
    }))
}

/// Helper: get a valid access token (refreshing if expired).
async fn get_valid_spotify_token(state: &AppState) -> Result<String, String> {
    let (access_token, refresh_token, expiry, client_id) = {
        let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
        let at = db.get_preference("spotify_access_token").ok().flatten().unwrap_or_default();
        let rt = db.get_preference("spotify_refresh_token").ok().flatten().unwrap_or_default();
        let exp = db.get_preference("spotify_token_expiry").ok().flatten()
            .and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
        let cid = db.get_preference("spotify_client_id").ok().flatten().unwrap_or_default();
        (at, rt, exp, cid)
    };

    if refresh_token.is_empty() {
        return Err("Not connected to Spotify".to_string());
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if now < expiry.saturating_sub(60) && !access_token.is_empty() {
        return Ok(access_token);
    }

    // Refresh the token
    let client = crate::spotify::SpotifyClient::new(&client_id);
    let tokens = client.refresh_token(&refresh_token).await?;

    // Persist new tokens
    {
        let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
        db.set_preference("spotify_access_token", &tokens.access_token)
            .map_err(|e| format!("DB: {e}"))?;
        db.set_preference("spotify_refresh_token", &tokens.refresh_token)
            .map_err(|e| format!("DB: {e}"))?;
        db.set_preference("spotify_token_expiry", &tokens.expires_at.to_string())
            .map_err(|e| format!("DB: {e}"))?;
    }

    Ok(tokens.access_token)
}

#[tauri::command]
pub async fn spotify_get_playlists(
    state: State<'_, AppState>,
) -> Result<Vec<crate::spotify::SpotifyPlaylist>, String> {
    let token = get_valid_spotify_token(&state).await?;
    let client_id = {
        let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
        db.get_preference("spotify_client_id").ok().flatten().unwrap_or_default()
    };
    let client = crate::spotify::SpotifyClient::new(&client_id);
    client.get_playlists(&token).await
}

#[tauri::command]
pub async fn spotify_get_playlist_tracks(
    playlist_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::spotify::SpotifyTrack>, String> {
    let token = get_valid_spotify_token(&state).await?;
    let client_id = {
        let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
        db.get_preference("spotify_client_id").ok().flatten().unwrap_or_default()
    };
    let client = crate::spotify::SpotifyClient::new(&client_id);
    client.get_playlist_tracks(&token, &playlist_id).await
}

#[tauri::command]
pub async fn spotify_import_playlist(
    playlist_id: String,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    let token = get_valid_spotify_token(&state).await?;
    let client_id = {
        let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
        db.get_preference("spotify_client_id").ok().flatten().unwrap_or_default()
    };
    let client = crate::spotify::SpotifyClient::new(&client_id);

    let tracks = client.get_playlist_tracks(&token, &playlist_id).await?;
    let download_dir = state.data_dir.join("downloads");
    let total = tracks.len();
    let mut imported = 0u32;
    let mut skipped = 0u32;
    let mut failed = 0u32;

    for (i, track) in tracks.iter().enumerate() {
        // Emit progress
        let _ = app_handle.emit("spotify-import-progress", serde_json::json!({
            "current": i + 1,
            "total": total,
            "track": format!("{} - {}", track.artist, track.name),
            "status": "downloading",
        }));

        // Check if already in library (simple artist+title match)
        {
            let db = state.db.lock().map_err(|e| format!("DB lock: {e}"))?;
            let existing = db.get_all_tracks().map_err(|e| format!("DB: {e}"))?;
            let already_exists = existing.iter().any(|t| {
                t.artist.as_deref().unwrap_or("") == track.artist
                    && t.title.as_deref().unwrap_or("") == track.name
            });
            if already_exists {
                log::info!("[spotify] Skipping existing: {} - {}", track.artist, track.name);
                skipped += 1;
                continue;
            }
        }

        // Download via spotDL
        match crate::spotify::download_track(&track.spotify_url, &download_dir).await {
            Ok(file_path) => {
                log::info!("[spotify] Downloaded: {:?}", file_path);

                // Feed into existing import pipeline
                let path_str = file_path.to_string_lossy().to_string();
                match import_files(vec![path_str], state.clone()).await {
                    Ok(_) => {
                        imported += 1;

                        // Enrich with Spotify metadata
                        let genres = client
                            .get_artist_genres(&token, &track.artist_id)
                            .await
                            .unwrap_or_default();

                        let enrichment = crate::spotify::map_spotify_to_classification(
                            &genres,
                            0.5, // TODO: fetch audio features for valence
                            0.5, // TODO: fetch audio features for energy
                            0.5, // TODO: fetch audio features for danceability
                        );

                        // Find the track we just imported and save classification
                        if let Ok(db) = state.db.lock() {
                            let all_tracks = db.get_all_tracks().unwrap_or_default();
                            if let Some(imported_track) = all_tracks.iter().find(|t| {
                                t.artist.as_deref().unwrap_or("") == track.artist
                                    && t.title.as_deref().unwrap_or("") == track.name
                            }) {
                                let tags_json = serde_json::to_string(&enrichment.ai_tags)
                                    .unwrap_or_else(|_| "[]".to_string());
                                let _ = db.save_classification(
                                    &imported_track.id,
                                    &enrichment.genre,
                                    &enrichment.subgenre,
                                    &enrichment.mood,
                                    &enrichment.vibe,
                                    &enrichment.mix_notes,
                                    &tags_json,
                                );
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("[spotify] Import failed for {}: {}", track.name, e);
                        failed += 1;
                    }
                }
            }
            Err(e) => {
                log::warn!("[spotify] Download failed for {}: {}", track.name, e);
                failed += 1;
            }
        }
    }

    // Emit completion
    let _ = app_handle.emit("spotify-import-progress", serde_json::json!({
        "current": total,
        "total": total,
        "status": "complete",
        "imported": imported,
        "skipped": skipped,
        "failed": failed,
    }));

    Ok(serde_json::json!({
        "imported": imported,
        "skipped": skipped,
        "failed": failed,
        "total": total,
    }))
}

#[tauri::command]
pub fn spotify_check_spotdl() -> Result<String, String> {
    crate::spotify::check_spotdl()
}
```

**Step 2: Register commands in lib.rs**

In `crates/autodj-app/src/lib.rs`, add to the `invoke_handler!` list (after `commands::set_conductor_queue,` on line 92):
```rust
commands::spotify_connect,
commands::spotify_disconnect,
commands::spotify_status,
commands::spotify_get_playlists,
commands::spotify_get_playlist_tracks,
commands::spotify_import_playlist,
commands::spotify_check_spotdl,
```

**Step 3: Run check**

Run: `cargo check -p autodj-app`
Expected: Compiles cleanly.

**Step 4: Commit**

```
git add crates/autodj-app/src/commands.rs crates/autodj-app/src/lib.rs
git commit -m "feat(app): add Tauri commands for Spotify connect, playlists, import"
```

---

### Task 6: Add Spotify settings tab to frontend

**Files:**
- Modify: `ui/src/App.tsx`
- Modify: `ui/src/store.ts`

**Step 1: Add Spotify state and actions to the store**

In `ui/src/store.ts`, add to the DJStore interface (after `aiConfig` around line 235):
```typescript
spotifyStatus: { connected: boolean; user_name: string } | null;
```

Add to store initialization (after `aiConfig: null` around line 351):
```typescript
spotifyStatus: null,
```

Add actions to the interface (near other action definitions):
```typescript
fetchSpotifyStatus: () => Promise<void>;
spotifyConnect: (clientId: string) => Promise<void>;
spotifyDisconnect: () => Promise<void>;
```

Add implementations (near other action implementations):
```typescript
fetchSpotifyStatus: async () => {
    try {
        const status = await invoke<{ connected: boolean; user_name: string }>("spotify_status");
        set({ spotifyStatus: status });
    } catch (e) {
        set({ spotifyStatus: { connected: false, user_name: "" } });
    }
},

spotifyConnect: async (clientId: string) => {
    try {
        await invoke("spotify_connect", { clientId });
        await get().fetchSpotifyStatus();
    } catch (e: any) {
        get().setErrorToast(`Spotify connect failed: ${e}`);
    }
},

spotifyDisconnect: async () => {
    try {
        await invoke("spotify_disconnect");
        set({ spotifyStatus: { connected: false, user_name: "" } });
    } catch (e: any) {
        get().setErrorToast(`Spotify disconnect failed: ${e}`);
    }
},
```

Add `fetchSpotifyStatus()` call to the `initializeApp` function (after `fetchTracks` call):
```typescript
get().fetchSpotifyStatus();
```

**Step 2: Add Spotify tab to Settings panel in App.tsx**

Find the settings tab buttons section and add a "Spotify" tab. Find the tab content section and add the Spotify settings panel with:
- Client ID input field
- Connect/Disconnect button
- Connection status display

**Step 3: Run TypeScript check**

Run: `cd ui && npx tsc --noEmit`
Expected: No errors.

**Step 4: Commit**

```
git add ui/src/store.ts ui/src/App.tsx
git commit -m "feat(ui): add Spotify settings tab with connect/disconnect"
```

---

### Task 7: Add Spotify playlist browser to frontend

**Files:**
- Create: `ui/src/components/SpotifyPanel.tsx`
- Modify: `ui/src/App.tsx`
- Modify: `ui/src/store.ts`

**Step 1: Create SpotifyPanel component**

Create `ui/src/components/SpotifyPanel.tsx` with:
- Playlist grid showing artwork, name, track count
- Clicking a playlist opens a track list view
- "Import Playlist" button that calls `spotify_import_playlist`
- Progress bar for import status (listens to `spotify-import-progress` event)
- "spotDL not found" warning with install instructions if `spotify_check_spotdl` fails

**Step 2: Add playlist store actions**

In `ui/src/store.ts`, add:
```typescript
spotifyPlaylists: SpotifyPlaylist[];
spotifyPlaylistTracks: SpotifyTrack[];
fetchSpotifyPlaylists: () => Promise<void>;
fetchSpotifyPlaylistTracks: (playlistId: string) => Promise<void>;
importSpotifyPlaylist: (playlistId: string) => Promise<void>;
```

**Step 3: Wire into App.tsx**

Add the SpotifyPanel as a new tab or panel in the Library section, visible when Spotify is connected.

**Step 4: Run TypeScript check**

Run: `cd ui && npx tsc --noEmit`
Expected: No errors.

**Step 5: Commit**

```
git add ui/src/components/SpotifyPanel.tsx ui/src/App.tsx ui/src/store.ts
git commit -m "feat(ui): add Spotify playlist browser with import progress"
```

---

### Task 8: Final integration test

**Step 1: Run full Rust test suite**

Run: `cargo test`
Expected: All tests pass.

**Step 2: Run TypeScript check**

Run: `cd ui && npx tsc --noEmit`
Expected: No errors.

**Step 3: Run clippy**

Run: `cargo clippy -- -D warnings 2>&1 | head -20`
Expected: No new warnings from our code.

**Step 4: Commit any fixes**

```
git commit -m "chore: fix clippy warnings and finalize Spotify integration"
```
