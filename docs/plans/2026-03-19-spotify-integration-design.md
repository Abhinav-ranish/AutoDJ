# Spotify Integration Design

**Date:** 2026-03-19
**Goal:** Add Spotify OAuth integration for browsing playlists, downloading tracks via spotDL, and enriching library metadata with Spotify genre/mood/energy data.

---

## User Flow

1. **Settings > Connect Spotify** — OAuth PKCE login via system browser
2. **Library > Spotify panel** — browse user's playlists (name, artwork, track count)
3. **Click playlist > Import** — view track list, click "Import Playlist"
4. **Download pipeline** — spotDL downloads each track > decode to PCM > analyze > enrich with Spotify metadata
5. **Existing tracks** — auto-enriched with Spotify genre/mood on import (supplements, doesn't replace our BPM/key/energy analysis)

## Architecture

```
Frontend (React)
  Settings: Connect/Disconnect Spotify button + status
  Library: Spotify playlist browser panel
  Import progress: per-track status bar
        |
        | Tauri commands
        v
Backend (Rust - autodj-app/src/spotify.rs)
  OAuth PKCE (local redirect server on localhost:9876)
  Token management (persist in DB preferences)
  Spotify Web API client:
    GET /me/playlists
    GET /playlists/{id}/tracks
    GET /artists/{id} (genres)
    GET /audio-features/{id}
  spotDL subprocess orchestration
  Metadata mapper (Spotify -> classification)
        |
        | spawns
        v
spotDL (external Python tool)
  User installs: pip install spotdl
  AutoDJ calls: spotdl download <url> --output ~/.autodj/downloads/
  Output: MP3/FLAC files fed into existing import pipeline
```

## Components

### 1. Spotify OAuth Module (`crates/autodj-app/src/spotify.rs`)

- **PKCE flow**: generate code_verifier + code_challenge, open auth URL in system browser, spin up tiny HTTP server on `localhost:9876` for redirect callback, exchange code for access_token + refresh_token
- **Token persistence**: store in DB `preferences` table (spotify_access_token, spotify_refresh_token, spotify_token_expiry, spotify_user_name, spotify_client_id)
- **Auto-refresh**: check expiry before API calls, refresh transparently

### 2. Spotify API Client (same module)

- `get_user_playlists()` -> `Vec<SpotifyPlaylist>` (id, name, image_url, track_count)
- `get_playlist_tracks(playlist_id)` -> `Vec<SpotifyTrack>` (id, name, artist, album, duration_ms, spotify_url)
- `get_artist_genres(artist_id)` -> `Vec<String>`
- `get_audio_features(track_id)` -> energy, danceability, valence, tempo, key
- Uses reqwest, handles pagination automatically
- Rate limit: Spotify allows ~30 req/sec (not a concern for our use case)

### 3. spotDL Orchestration

- Check if `spotdl` binary is available via `which spotdl` / `spotdl --version`
- For each track: spawn `spotdl download <spotify_url> --output ~/.autodj/downloads/ --format mp3`
- Parse stdout for progress (spotDL prints download status)
- Feed downloaded files into existing `import_files()` pipeline
- Skip tracks already in library (dedup by artist+title fuzzy match)

### 4. Metadata Mapper

Maps Spotify audio features to our classification columns:
- **Genre**: from artist endpoint (Spotify genres are on artists, not tracks)
- **Mood**: derived from `valence` (>0.7 = euphoric, 0.4-0.7 = uplifting, 0.2-0.4 = melancholic, <0.2 = dark)
- **Vibe**: derived from `energy` + `danceability` (high both = anthem/peak_time, low energy = warm_up/closer)
- **mix_notes**: generated from audio features summary
- **ai_tags**: generated from genre + features combination
- Saves via existing `db.save_classification()` — same columns as LLM classifier

### 5. Tauri Commands

- `spotify_connect()` — starts OAuth flow, returns user display name
- `spotify_disconnect()` — clears tokens from DB
- `spotify_status()` -> `{ connected: bool, user_name: Option<String> }`
- `spotify_get_playlists()` -> `Vec<SpotifyPlaylist>`
- `spotify_get_playlist_tracks(playlist_id)` -> `Vec<SpotifyTrack>`
- `spotify_import_playlist(playlist_id)` — kicks off full download+import pipeline
- `spotify_import_track(spotify_url)` — single track download+import
- `spotify_check_spotdl()` -> `bool` — check if spotDL is installed
- `spotify_enrich_track(track_id)` — enrich existing library track with Spotify metadata

### 6. Frontend

- **Settings panel**: "Connect Spotify" button with green/red status indicator + username
- **Library view**: new "Spotify" tab/panel showing playlist grid (artwork + name + track count)
- **Playlist detail modal**: track list with checkboxes, "Import Selected" / "Import All" buttons
- **Import progress**: per-track status (downloading > decoding > analyzing > enriching > done)
- **spotDL warning**: if not installed, show message with install command (`pip install spotdl`)

## Import Pipeline (per playlist track)

```
1. Check if already in library (artist+title fuzzy match) -> skip if exists
2. spotDL download -> MP3 in ~/.autodj/downloads/
3. import_files([path]) -> hash, metadata extract, decode to PCM
4. analyze_track(track_id) -> BPM, key, energy, sections
5. Spotify enrich -> save_classification(genre, mood, vibe from Spotify data)
6. Emit progress event to frontend
```

## Data Priority

Spotify metadata **supplements** existing analysis:
- Our analyzer: BPM, key, energy, spectral features (more precise for DJ mixing)
- Spotify: genre, mood, vibe (richer metadata our analyzer can't detect)
- LLM classifier: fallback when Spotify match not found

## Token Persistence

DB `preferences` table keys:
- `spotify_access_token`
- `spotify_refresh_token`
- `spotify_token_expiry` (Unix timestamp)
- `spotify_user_name`
- `spotify_client_id` (user provides in settings)

## Error Handling

- spotDL not installed: clear error with install instructions
- spotDL fails for a track: skip, continue with rest, report failures at end
- Spotify token expired: auto-refresh, retry once
- Track already exists: skip silently (dedup)
- No Spotify match for library track: fall back to LLM classification
- Network errors: retry with backoff, surface error to UI

## Prerequisites

- User creates a Spotify Developer app at https://developer.spotify.com/dashboard
- User provides their Client ID in AutoDJ settings (no secret needed for PKCE)
- User installs spotDL: `pip install spotdl` (and FFmpeg, which is already required by AutoDJ)
