use serde::{Deserialize, Serialize};

/// The complete analysis profile of a track.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackProfile {
    pub version: u32,
    pub track_id: String,
    pub file_hash: String,
    pub analysis_timestamp: String,

    pub tempo: TempoInfo,
    pub perceived_tempo: Option<PerceivedTempoInfo>,
    pub beats: BeatInfo,
    pub key: KeyInfo,
    pub loudness: LoudnessInfo,
    pub energy: EnergyInfo,
    pub regions: RegionInfo,
    pub sections: Vec<Section>,
    pub waveform: WaveformInfo,

    // Phase 12: DSP enrichment fields
    pub spectral: Option<SpectralProfile>,
    pub vocal_presence: Option<f64>,
    pub rhythm: Option<RhythmProfile>,
    pub energy_trajectory: Option<String>,
    pub intro_length_ms: Option<u64>,
    pub outro_length_ms: Option<u64>,
    pub first_drop_ms: Option<u64>,

    // Phase 19: transition windows, phrase grid, auto-cues
    pub transition_windows: Vec<TransitionWindow>,
    pub phrases: Option<PhraseInfo>,
    pub auto_cues: Vec<AutoCue>,

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
pub struct PerceivedTempoInfo {
    pub bpm: f64,
    pub perceived_tempo: f64,
    pub groove_type: String,
    pub tempo_confidence: f64,
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
pub struct SpectralProfile {
    pub centroid: f64,
    pub flatness: f64,
    pub bass_energy: f64,
    pub mid_energy: f64,
    pub high_energy: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RhythmProfile {
    pub complexity: f64,
    pub danceability: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveformInfo {
    pub peaks_file: String,
    pub samples_per_pixel: u32,
    pub channels: u16,
}

// ─── Phase 19: Transition Windows & Phrase Grid ─────────────────

/// A candidate time window where a transition can safely start.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionWindow {
    /// Human-readable anchor type, e.g. "outro_start", "pre_drop_16", "break_start".
    pub anchor: String,
    /// Start of the window in milliseconds.
    pub start_ms: u64,
    /// End of the window in milliseconds.
    pub end_ms: u64,
    /// Confidence / quality score (0.0–1.0).
    pub score: f64,
}

/// Phrase grid derived from beat grid (4/8/16 bar phrase boundaries).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhraseInfo {
    /// Length of one phrase in bars (typically 8 or 16).
    pub phrase_bars: u32,
    /// Absolute ms positions of each phrase boundary.
    pub boundaries_ms: Vec<u64>,
}

/// A canonical cue marker auto-generated during analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoCue {
    /// Canonical type: "intro", "drop", "break", "outro", "first_beat".
    pub cue_type: String,
    /// Position in milliseconds.
    pub position_ms: u64,
    /// Confidence (0.0–1.0).
    pub confidence: f64,
}
