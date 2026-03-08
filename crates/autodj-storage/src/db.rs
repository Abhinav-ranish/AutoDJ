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
    // Phase 13: AI classification
    pub genre: Option<String>,
    pub mood: Option<String>,
    pub vibe: Option<String>,
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
    // Phase 12: DSP enrichment fields
    pub spectral_centroid: Option<f64>,
    pub spectral_flatness: Option<f64>,
    pub bass_energy: Option<f64>,
    pub mid_energy: Option<f64>,
    pub high_energy: Option<f64>,
    pub vocal_presence: Option<f64>,
    pub rhythm_complexity: Option<f64>,
    pub danceability: Option<f64>,
    pub energy_trajectory: Option<&'a str>,
    pub intro_length_ms: Option<i64>,
    pub outro_length_ms: Option<i64>,
    pub first_drop_ms: Option<i64>,
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

            CREATE TABLE IF NOT EXISTS auto_cues (
                track_id    TEXT NOT NULL REFERENCES tracks(id),
                cue_type    TEXT NOT NULL,
                position_ms INTEGER NOT NULL,
                confidence  REAL NOT NULL DEFAULT 1.0,
                PRIMARY KEY (track_id, cue_type)
            );

            CREATE TABLE IF NOT EXISTS transition_feedback (
                id              TEXT PRIMARY KEY,
                session_id      TEXT NOT NULL,
                from_track_id   TEXT NOT NULL,
                to_track_id     TEXT NOT NULL,
                template_used   TEXT NOT NULL,
                rating          INTEGER NOT NULL,
                notes           TEXT,
                decision_json   TEXT,
                created_at      INTEGER NOT NULL
            );
            ",
        )?;

        // Migrations for existing databases: add columns that may not exist yet
        // SQLite silently ignores ALTER TABLE ADD COLUMN if it fails, so we catch errors
        let migrations = [
            "ALTER TABLE track_profiles ADD COLUMN perceived_tempo REAL",
            "ALTER TABLE track_profiles ADD COLUMN groove_type TEXT",
            // Phase 12: DSP enrichment columns
            "ALTER TABLE track_profiles ADD COLUMN spectral_centroid REAL",
            "ALTER TABLE track_profiles ADD COLUMN spectral_flatness REAL",
            "ALTER TABLE track_profiles ADD COLUMN bass_energy REAL",
            "ALTER TABLE track_profiles ADD COLUMN mid_energy REAL",
            "ALTER TABLE track_profiles ADD COLUMN high_energy REAL",
            "ALTER TABLE track_profiles ADD COLUMN vocal_presence REAL",
            "ALTER TABLE track_profiles ADD COLUMN rhythm_complexity REAL",
            "ALTER TABLE track_profiles ADD COLUMN danceability REAL",
            "ALTER TABLE track_profiles ADD COLUMN energy_trajectory TEXT",
            "ALTER TABLE track_profiles ADD COLUMN intro_length_ms INTEGER",
            "ALTER TABLE track_profiles ADD COLUMN outro_length_ms INTEGER",
            "ALTER TABLE track_profiles ADD COLUMN first_drop_ms INTEGER",
            // Phase 13: Gemini classification columns
            "ALTER TABLE track_profiles ADD COLUMN genre TEXT",
            "ALTER TABLE track_profiles ADD COLUMN subgenre TEXT",
            "ALTER TABLE track_profiles ADD COLUMN mood TEXT",
            "ALTER TABLE track_profiles ADD COLUMN vibe TEXT",
            "ALTER TABLE track_profiles ADD COLUMN mix_notes TEXT",
            "ALTER TABLE track_profiles ADD COLUMN ai_tags TEXT",
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
            "SELECT t.id, t.file_hash, t.original_path, t.title, t.artist, t.duration_ms, t.import_status, t.error_message, p.bpm, p.perceived_tempo, p.groove_type, p.key_name, p.lufs, p.genre, p.mood, p.vibe
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
                genre: row.get(13)?,
                mood: row.get(14)?,
                vibe: row.get(15)?,
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
            "INSERT INTO track_profiles (track_id, profile_json, bpm, bpm_confidence, perceived_tempo, groove_type, key_name, camelot, key_confidence, lufs, energy_avg, spectral_centroid, spectral_flatness, bass_energy, mid_energy, high_energy, vocal_presence, rhythm_complexity, danceability, energy_trajectory, intro_length_ms, outro_length_ms, first_drop_ms, analyzed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24)
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
                spectral_centroid=excluded.spectral_centroid,
                spectral_flatness=excluded.spectral_flatness,
                bass_energy=excluded.bass_energy,
                mid_energy=excluded.mid_energy,
                high_energy=excluded.high_energy,
                vocal_presence=excluded.vocal_presence,
                rhythm_complexity=excluded.rhythm_complexity,
                danceability=excluded.danceability,
                energy_trajectory=excluded.energy_trajectory,
                intro_length_ms=excluded.intro_length_ms,
                outro_length_ms=excluded.outro_length_ms,
                first_drop_ms=excluded.first_drop_ms,
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
                data.spectral_centroid,
                data.spectral_flatness,
                data.bass_energy,
                data.mid_energy,
                data.high_energy,
                data.vocal_presence,
                data.rhythm_complexity,
                data.danceability,
                data.energy_trajectory,
                data.intro_length_ms,
                data.outro_length_ms,
                data.first_drop_ms,
                now
            ],
        )?;
        Ok(())
    }

    /// Save AI classification results for a track.
    pub fn save_classification(
        &self,
        track_id: &str,
        genre: &str,
        subgenre: &str,
        mood: &str,
        vibe: &str,
        mix_notes: &str,
        ai_tags: &str,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE track_profiles SET genre=?2, subgenre=?3, mood=?4, vibe=?5, mix_notes=?6, ai_tags=?7 WHERE track_id=?1",
            params![track_id, genre, subgenre, mood, vibe, mix_notes, ai_tags],
        )?;
        Ok(())
    }

    /// Get classification data for all tracks that have been classified.
    pub fn get_all_classifications(&self) -> Result<Vec<(String, String, String, String, String, String)>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT track_id, genre, subgenre, mood, vibe, mix_notes FROM track_profiles WHERE genre IS NOT NULL",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// Create a new session. Returns the session ID.
    pub fn create_session(&self) -> Result<String, DbError> {
        let id = format!("ses_{:012x}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() & 0xFFFFFFFFFFFF);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        self.conn.execute(
            "INSERT INTO sessions (id, started_at) VALUES (?1, ?2)",
            params![id, now],
        )?;
        Ok(id)
    }

    /// End a session.
    pub fn end_session(&self, session_id: &str) -> Result<(), DbError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        self.conn.execute(
            "UPDATE sessions SET ended_at=?2 WHERE id=?1",
            params![session_id, now],
        )?;
        Ok(())
    }

    /// Record a track play in mix history.
    pub fn record_track_play(
        &self,
        session_id: &str,
        track_id: &str,
        transition_type: Option<&str>,
        ai_decision_json: Option<&str>,
    ) -> Result<(), DbError> {
        let id = format!("mix_{:012x}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() & 0xFFFFFFFFFFFF);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        self.conn.execute(
            "INSERT INTO mix_history (id, track_id, started_at, transition_type, ai_decision_json, session_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, track_id, now, transition_type, ai_decision_json, session_id],
        )?;
        // Update session track count
        self.conn.execute(
            "UPDATE sessions SET track_count = track_count + 1 WHERE id = ?1",
            params![session_id],
        )?;
        Ok(())
    }

    /// Get the most recently played track IDs.
    pub fn get_recent_track_ids(&self, limit: usize) -> Result<Vec<String>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT track_id FROM mix_history ORDER BY started_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| row.get(0))?;
        let mut ids = Vec::new();
        for row in rows {
            ids.push(row?);
        }
        Ok(ids)
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
            "SELECT t.id, t.file_hash, t.original_path, t.title, t.artist, t.duration_ms, t.import_status, t.error_message, p.bpm, p.perceived_tempo, p.groove_type, p.key_name, p.lufs, p.genre, p.mood, p.vibe
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
                genre: row.get(13)?,
                mood: row.get(14)?,
                vibe: row.get(15)?,
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
            DELETE FROM auto_cues;
            DELETE FROM transition_feedback;
            DELETE FROM hot_cues;
            DELETE FROM mix_history;
            DELETE FROM track_profiles;
            DELETE FROM tracks;
            "
        )?;
        Ok(())
    }

    /// Save auto-cues for a track (replaces any existing).
    pub fn save_auto_cues(
        &self,
        track_id: &str,
        cues: &[(String, u64, f64)], // (cue_type, position_ms, confidence)
    ) -> Result<(), DbError> {
        self.conn.execute(
            "DELETE FROM auto_cues WHERE track_id = ?1",
            params![track_id],
        )?;
        for (cue_type, position_ms, confidence) in cues {
            self.conn.execute(
                "INSERT INTO auto_cues (track_id, cue_type, position_ms, confidence)
                 VALUES (?1, ?2, ?3, ?4)",
                params![track_id, cue_type, *position_ms as i64, confidence],
            )?;
        }
        Ok(())
    }

    /// Get auto-cues for a track.
    pub fn get_auto_cues(&self, track_id: &str) -> Result<Vec<(String, u64, f64)>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT cue_type, position_ms, confidence FROM auto_cues WHERE track_id = ?1 ORDER BY position_ms",
        )?;
        let rows = stmt.query_map(params![track_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u64, row.get::<_, f64>(2)?))
        })?;
        let mut cues = Vec::new();
        for row in rows {
            cues.push(row?);
        }
        Ok(cues)
    }

    /// Save transition feedback for a mix.
    pub fn save_transition_feedback(
        &self,
        session_id: &str,
        from_track_id: &str,
        to_track_id: &str,
        template_used: &str,
        rating: i32,
        notes: Option<&str>,
        decision_json: Option<&str>,
    ) -> Result<(), DbError> {
        let id = format!("fb_{:012x}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() & 0xFFFFFFFFFFFF);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        self.conn.execute(
            "INSERT INTO transition_feedback (id, session_id, from_track_id, to_track_id, template_used, rating, notes, decision_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![id, session_id, from_track_id, to_track_id, template_used, rating, notes, decision_json, now],
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
