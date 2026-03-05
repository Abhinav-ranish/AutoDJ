//! autodj-analysis: Track analysis (stub for Milestone D).
//!
//! This crate will contain BPM detection, key detection, beat tracking,
//! energy curves, section segmentation, and waveform peak generation.

/// Placeholder for the track profile.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TrackProfile {
    pub track_id: String,
    pub bpm: Option<f64>,
    pub key: Option<String>,
    pub camelot: Option<String>,
    pub energy_avg: Option<f64>,
    pub duration_ms: u64,
}
