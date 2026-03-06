//! Analysis Pipeline Orchestrator

use crate::beats::track_beats;
use crate::bpm::detect_bpm;
use crate::energy::analyze_energy;
use crate::key::detect_key;
use crate::loudness::analyze_loudness;
use crate::profile::{RegionInfo, TrackProfile};
use crate::sections::analyze_sections;
use crate::waveform::analyze_waveform;
use std::path::Path;

/// Run the full analysis pipeline on a track's PCM data.
pub fn analyze_track(
    track_id: String,
    file_hash: String,
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
    peaks_dir: &Path,
) -> Result<TrackProfile, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let duration_ms = (samples.len() as u64 * 1000) / (sample_rate as u64 * channels as u64);

    // In a real app we'd use `rayon` to run these in parallel, but they are fast enough
    // for this milestone to just run sequentially.
    let tempo = detect_bpm(samples, sample_rate);
    let beats = track_beats(samples, sample_rate, tempo.bpm);
    let key = detect_key(samples, sample_rate);
    let loudness = analyze_loudness(samples, sample_rate);
    let energy = analyze_energy(samples, sample_rate, 0.5);
    let sections = analyze_sections(duration_ms);
    let waveform = analyze_waveform(samples, &file_hash, peaks_dir)
        .map_err(|e| format!("Failed to generate waveform: {e}"))?;

    // Regions placeholder
    let regions = RegionInfo {
        intro: None,
        outro: None,
        safe_loops: vec![],
    };

    Ok(TrackProfile {
        version: 1,
        track_id,
        file_hash,
        analysis_timestamp: now,
        tempo,
        beats,
        key,
        loudness,
        energy,
        regions,
        sections,
        waveform,
        duration_ms,
        sample_rate,
        channels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_pipeline_integration() {
        let dir = tempdir().unwrap();
        let sr = 48000;
        let samples = vec![0.0f32; 48000 * 2 * 2]; // 2 seconds of silence

        let profile = analyze_track(
            "trk_test".to_string(),
            "hash123".to_string(),
            &samples,
            sr,
            2,
            dir.path(),
        )
        .expect("Pipeline should succeed");

        assert_eq!(profile.track_id, "trk_test");
        assert_eq!(profile.duration_ms, 2000);
        assert_eq!(profile.waveform.peaks_file, "hash123_peaks.bin");
        assert!(profile.tempo.bpm > 0.0);
    }
}
