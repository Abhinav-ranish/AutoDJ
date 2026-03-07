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

/// Status of a track in the library.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ImportStatus {
    Pending,
    Decoding,
    Ready,
    Analyzed,
    AnalysisFailed,
    Error,
}

impl ImportStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Decoding => "decoding",
            Self::Ready => "ready",
            Self::Analyzed => "analyzed",
            Self::AnalysisFailed => "analysis_failed",
            Self::Error => "error",
        }
    }
}

impl rusqlite::ToSql for ImportStatus {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::from(self.as_str()))
    }
}

impl rusqlite::types::FromSql for ImportStatus {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value.as_str()? {
            "pending" => Ok(Self::Pending),
            "decoding" => Ok(Self::Decoding),
            "ready" => Ok(Self::Ready),
            "analyzed" => Ok(Self::Analyzed),
            "analysis_failed" => Ok(Self::AnalysisFailed),
            "error" => Ok(Self::Error),
            _ => Err(rusqlite::types::FromSqlError::InvalidType),
        }
    }
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
    pub import_status: ImportStatus,
    pub error_message: Option<String>,
    pub bpm: Option<f64>,
    pub perceived_tempo: Option<f64>,
    pub groove_type: Option<String>,
    pub key: Option<String>,
    pub lufs: Option<f64>,
}

/// Data needed to save a track profile to the database.
pub struct ProfileData<'a> {
    pub track_id: &'a str,
    pub profile_json: &'a str,
    pub bpm: f64,
    pub bpm_confidence: f64,
    pub perceived_tempo: f64,
    pub groove_type: &'a str,
    pub key_name: &'a str,
    pub camelot: &'a str,
    pub key_confidence: f64,
    pub lufs: f64,
    pub energy_avg: f64,
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
                perceived_tempo REAL,
                groove_type    TEXT,
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

            CREATE TABLE IF NOT EXISTS hot_cues (
                track_id   TEXT NOT NULL REFERENCES tracks(id),
                cue_index  INTEGER NOT NULL,
                position_ms INTEGER NOT NULL,
                label       TEXT,
                PRIMARY KEY (track_id, cue_index)
            );
            ",
        )?;

        // Migrations for existing databases: add columns that may not exist yet
        // SQLite silently ignores ALTER TABLE ADD COLUMN if it fails, so we catch errors
        let migrations = [
            "ALTER TABLE track_profiles ADD COLUMN perceived_tempo REAL",
            "ALTER TABLE track_profiles ADD COLUMN groove_type TEXT",
        ];
        for sql in &migrations {
            // Ignore "duplicate column" errors from re-running migrations
            let _ = self.conn.execute(sql, []);
        }

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

    pub fn update_status(
        &self,
        track_id: &str,
        status: ImportStatus,
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
            "SELECT t.id, t.file_hash, t.original_path, t.title, t.artist, t.duration_ms, t.import_status, t.error_message, p.bpm, p.perceived_tempo, p.groove_type, p.key_name, p.lufs
             FROM tracks t
             LEFT JOIN track_profiles p ON t.id = p.track_id
             ORDER BY t.created_at DESC",
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
                bpm: row.get(8)?,
                perceived_tempo: row.get(9)?,
                groove_type: row.get(10)?,
                key: row.get(11)?,
                lufs: row.get(12)?,
            })
        })?;

        let mut tracks = Vec::new();
        for row in rows {
            tracks.push(row?);
        }
        Ok(tracks)
    }

    /// Save a track profile to the database.
    pub fn save_track_profile(&self, data: &ProfileData) -> Result<(), DbError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        self.conn.execute(
            "INSERT INTO track_profiles (track_id, profile_json, bpm, bpm_confidence, perceived_tempo, groove_type, key_name, camelot, key_confidence, lufs, energy_avg, analyzed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(track_id) DO UPDATE SET
                profile_json=excluded.profile_json,
                bpm=excluded.bpm,
                bpm_confidence=excluded.bpm_confidence,
                perceived_tempo=excluded.perceived_tempo,
                groove_type=excluded.groove_type,
                key_name=excluded.key_name,
                camelot=excluded.camelot,
                key_confidence=excluded.key_confidence,
                lufs=excluded.lufs,
                energy_avg=excluded.energy_avg,
                analyzed_at=excluded.analyzed_at",
            params![
                data.track_id,
                data.profile_json,
                data.bpm,
                data.bpm_confidence,
                data.perceived_tempo,
                data.groove_type,
                data.key_name,
                data.camelot,
                data.key_confidence,
                data.lufs,
                data.energy_avg,
                now
            ],
        )?;
        Ok(())
    }

    /// Get a track profile as JSON string.
    pub fn get_track_profile_json(&self, track_id: &str) -> Result<Option<String>, DbError> {
        let mut stmt = self.conn.prepare("SELECT profile_json FROM track_profiles WHERE track_id = ?1")?;
        let mut rows = stmt.query_map(params![track_id], |row| row.get(0))?;
        
        match rows.next() {
            Some(Ok(json)) => Ok(Some(json)),
            Some(Err(e)) => Err(DbError::Sqlite(e)),
            None => Ok(None),
        }
    }

    /// Get a single track by ID.
    pub fn get_track(&self, track_id: &str) -> Result<Option<TrackRecord>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.file_hash, t.original_path, t.title, t.artist, t.duration_ms, t.import_status, t.error_message, p.bpm, p.perceived_tempo, p.groove_type, p.key_name, p.lufs
             FROM tracks t
             LEFT JOIN track_profiles p ON t.id = p.track_id
             WHERE t.id = ?1",
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
                bpm: row.get(8)?,
                perceived_tempo: row.get(9)?,
                groove_type: row.get(10)?,
                key: row.get(11)?,
                lufs: row.get(12)?,
            })
        })?;

        match rows.next() {
            Some(Ok(track)) => Ok(Some(track)),
            Some(Err(e)) => Err(DbError::Sqlite(e)),
            None => Ok(None),
        }
    }

    /// Set a hot cue for a track.
    pub fn save_hot_cue(&self, track_id: &str, cue_index: u32, position_ms: u64, label: Option<&str>) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO hot_cues (track_id, cue_index, position_ms, label)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(track_id, cue_index) DO UPDATE SET
                position_ms=excluded.position_ms,
                label=excluded.label",
            params![track_id, cue_index, position_ms as i64, label],
        )?;
        Ok(())
    }

    /// Get all hot cues for a track.
    pub fn get_hot_cues(&self, track_id: &str) -> Result<Vec<(u32, u64, Option<String>)>, DbError> {
        let mut stmt = self.conn.prepare("SELECT cue_index, position_ms, label FROM hot_cues WHERE track_id = ?1 ORDER BY cue_index")?;
        let rows = stmt.query_map(params![track_id], |row| {
            Ok((row.get(0)?, row.get::<_, i64>(1)? as u64, row.get(2)?))
        })?;

        let mut cues = Vec::new();
        for row in rows {
            cues.push(row?);
        }
        Ok(cues)
    }

    /// Clear all tracks and related data from the database.
    pub fn clear_all_tracks(&self) -> Result<(), DbError> {
        self.conn.execute_batch(
            "
            DELETE FROM hot_cues;
            DELETE FROM mix_history;
            DELETE FROM track_profiles;
            DELETE FROM tracks;
            "
        )?;
        Ok(())
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
        assert_eq!(track.import_status, ImportStatus::Pending);
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

        db.update_status(&id, ImportStatus::Decoding, None).unwrap();
        let track = db.get_track(&id).unwrap().unwrap();
        assert_eq!(track.import_status, ImportStatus::Decoding);
    }
}
