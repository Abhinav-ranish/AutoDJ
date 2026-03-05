//! SQLite database layer for track metadata and preferences.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

/// A track record in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackRecord {
    pub id: String,
    pub file_hash: String,
    pub original_path: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub duration_ms: Option<i64>,
    pub import_status: String,
    pub error_message: Option<String>,
}

/// The database handle.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create the database at the given path.
    pub fn open(path: &Path) -> Result<Self, DbError> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Open an in-memory database (for testing).
    pub fn open_memory() -> Result<Self, DbError> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<(), DbError> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS tracks (
                id            TEXT PRIMARY KEY,
                file_hash     TEXT NOT NULL,
                original_path TEXT NOT NULL,
                title         TEXT,
                artist        TEXT,
                album         TEXT,
                genre         TEXT,
                duration_ms   INTEGER,
                file_size     INTEGER,
                mime_type     TEXT,
                import_status TEXT NOT NULL DEFAULT 'pending',
                error_message TEXT,
                created_at    INTEGER NOT NULL,
                updated_at    INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_tracks_hash ON tracks(file_hash);
            CREATE INDEX IF NOT EXISTS idx_tracks_status ON tracks(import_status);

            CREATE TABLE IF NOT EXISTS track_profiles (
                track_id       TEXT PRIMARY KEY REFERENCES tracks(id),
                profile_json   TEXT NOT NULL,
                bpm            REAL,
                bpm_confidence REAL,
                key_name       TEXT,
                camelot        TEXT,
                key_confidence REAL,
                lufs           REAL,
                energy_avg     REAL,
                analyzed_at    INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS preferences (
                key        TEXT PRIMARY KEY,
                value      TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS sessions (
                id         TEXT PRIMARY KEY,
                started_at INTEGER NOT NULL,
                ended_at   INTEGER,
                track_count INTEGER DEFAULT 0,
                total_duration_ms INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS mix_history (
                id              TEXT PRIMARY KEY,
                track_id        TEXT REFERENCES tracks(id),
                started_at      INTEGER NOT NULL,
                ended_at        INTEGER,
                transition_type TEXT,
                transition_json TEXT,
                ai_decision_json TEXT,
                session_id      TEXT NOT NULL
            );
            ",
        )?;
        Ok(())
    }

    /// Generate a unique track ID.
    fn gen_track_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        format!("trk_{:012x}", ts & 0xFFFFFFFFFFFF)
    }

    /// Insert a new track. Returns the track ID.
    pub fn insert_track(
        &self,
        file_hash: &str,
        original_path: &str,
        title: Option<&str>,
        artist: Option<&str>,
    ) -> Result<String, DbError> {
        let id = Self::gen_track_id();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        self.conn.execute(
            "INSERT INTO tracks (id, file_hash, original_path, title, artist, import_status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'pending', ?6, ?6)",
            params![id, file_hash, original_path, title, artist, now],
        )?;

        Ok(id)
    }

    /// Check if a file hash already exists.
    pub fn hash_exists(&self, file_hash: &str) -> Result<bool, DbError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM tracks WHERE file_hash = ?1",
            params![file_hash],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Update track import status.
    pub fn update_status(
        &self,
        track_id: &str,
        status: &str,
        error: Option<&str>,
    ) -> Result<(), DbError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        self.conn.execute(
            "UPDATE tracks SET import_status = ?1, error_message = ?2, updated_at = ?3 WHERE id = ?4",
            params![status, error, now, track_id],
        )?;
        Ok(())
    }

    /// Update track duration after decoding.
    pub fn update_duration(&self, track_id: &str, duration_ms: i64) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE tracks SET duration_ms = ?1 WHERE id = ?2",
            params![duration_ms, track_id],
        )?;
        Ok(())
    }

    /// Get all tracks.
    pub fn get_all_tracks(&self) -> Result<Vec<TrackRecord>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, file_hash, original_path, title, artist, duration_ms, import_status, error_message
             FROM tracks ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(TrackRecord {
                id: row.get(0)?,
                file_hash: row.get(1)?,
                original_path: row.get(2)?,
                title: row.get(3)?,
                artist: row.get(4)?,
                duration_ms: row.get(5)?,
                import_status: row.get(6)?,
                error_message: row.get(7)?,
            })
        })?;

        let mut tracks = Vec::new();
        for row in rows {
            tracks.push(row?);
        }
        Ok(tracks)
    }

    /// Get a single track by ID.
    pub fn get_track(&self, track_id: &str) -> Result<Option<TrackRecord>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, file_hash, original_path, title, artist, duration_ms, import_status, error_message
             FROM tracks WHERE id = ?1",
        )?;

        let mut rows = stmt.query_map(params![track_id], |row| {
            Ok(TrackRecord {
                id: row.get(0)?,
                file_hash: row.get(1)?,
                original_path: row.get(2)?,
                title: row.get(3)?,
                artist: row.get(4)?,
                duration_ms: row.get(5)?,
                import_status: row.get(6)?,
                error_message: row.get(7)?,
            })
        })?;

        match rows.next() {
            Some(Ok(track)) => Ok(Some(track)),
            Some(Err(e)) => Err(DbError::Sqlite(e)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_query_track() {
        let db = Database::open_memory().unwrap();
        let id = db
            .insert_track("hash123", "/path/to/file.mp3", Some("Test Track"), Some("Test Artist"))
            .unwrap();

        let track = db.get_track(&id).unwrap().unwrap();
        assert_eq!(track.title.as_deref(), Some("Test Track"));
        assert_eq!(track.import_status, "pending");
    }

    #[test]
    fn test_hash_exists() {
        let db = Database::open_memory().unwrap();
        db.insert_track("unique_hash", "/path.mp3", None, None).unwrap();

        assert!(db.hash_exists("unique_hash").unwrap());
        assert!(!db.hash_exists("other_hash").unwrap());
    }

    #[test]
    fn test_update_status() {
        let db = Database::open_memory().unwrap();
        let id = db.insert_track("h", "/p.mp3", None, None).unwrap();

        db.update_status(&id, "decoded", None).unwrap();
        let track = db.get_track(&id).unwrap().unwrap();
        assert_eq!(track.import_status, "decoded");
    }
}
