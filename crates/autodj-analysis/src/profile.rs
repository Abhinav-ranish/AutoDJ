use serde::{Deserialize, Serialize};

/// The complete analysis profile of a track.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackProfile {
    pub version: u32,
    pub track_id: String,
    pub file_hash: String,
    pub analysis_timestamp: String,

    pub tempo: TempoInfo,
    pub beats: BeatInfo,
    pub key: KeyInfo,
    pub loudness: LoudnessInfo,
    pub energy: EnergyInfo,
    pub regions: RegionInfo,
    pub sections: Vec<Section>,
    pub waveform: WaveformInfo,

    pub duration_ms: u64,
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TempoInfo {
    pub bpm: f64,
    pub confidence: f64,
    pub candidates: Vec<TempoCandidate>,
    pub variable_tempo: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TempoCandidate {
    pub bpm: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatInfo {
    pub count: u64,
    pub times_ms: Vec<u64>,
    pub downbeat_indices: Vec<usize>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyInfo {
    pub key: String,
    pub camelot: String,
    pub confidence: f64,
    pub candidates: Vec<KeyCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyCandidate {
    pub key: String,
    pub camelot: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoudnessInfo {
    pub integrated_lufs: f64,
    pub true_peak_db: f64,
    pub rms_db: f64,
    pub dynamic_range_db: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyInfo {
    pub window_seconds: f64,
    pub curve: Vec<f64>,
    pub average: f64,
    pub peak: f64,
    pub drop_candidates_ms: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionInfo {
    pub intro: Option<TimeRange>,
    pub outro: Option<TimeRange>,
    pub safe_loops: Vec<SafeLoop>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start_ms: u64,
    pub end_ms: u64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafeLoop {
    pub start_beat: u64,
    pub length_bars: u32,
    pub quality: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    pub label: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveformInfo {
    pub peaks_file: String,
    pub samples_per_pixel: u32,
    pub channels: u16,
}
