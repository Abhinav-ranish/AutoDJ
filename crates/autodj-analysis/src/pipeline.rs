//! Analysis Pipeline Orchestrator

use crate::beats::track_beats;
use crate::bpm::detect_bpm;
use crate::energy::analyze_energy;
use crate::key::detect_key;
use crate::loudness::analyze_loudness;
use crate::perceived_tempo::analyze_perceived_tempo;
use crate::profile::{EnergyInfo, RhythmProfile, SpectralProfile, TrackProfile};
use crate::rhythm::analyze_rhythm;
use crate::sections::analyze_sections;
use crate::spectral::analyze_spectral;
use crate::vocal_detection::detect_vocal_presence;
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

    // Parallelize independent analysis steps using rayon
    let ((tempo, beat_info), (key, loudness, energy_info, waveform)) = rayon::join(
        || {
            let t = detect_bpm(samples, sample_rate);
            let b = track_beats(samples, sample_rate, t.bpm);
            (t, b)
        },
        || {
            let (key, (loudness, (energy, waveform))) = rayon::join(
                || detect_key(samples, sample_rate),
                || rayon::join(
                    || analyze_loudness(samples, sample_rate),
                    || rayon::join(
                        || analyze_energy(samples, sample_rate, 0.5),
                        || analyze_waveform(samples, &file_hash, peaks_dir)
                    )
                )
            );
            (key, loudness, energy, waveform)
        }
    );

    let waveform = waveform.map_err(|e| format!("Failed to generate waveform: {e}"))?;

    // Sections depend on energy curve, so compute after energy analysis
    let sections = analyze_sections(duration_ms, &energy_info.curve);

    // Phase 12: Run DSP enrichment analyzers in parallel
    let (spectral_info, (vocal, rhythm_info)) = rayon::join(
        || analyze_spectral(samples, sample_rate),
        || {
            rayon::join(
                || detect_vocal_presence(samples, sample_rate),
                || analyze_rhythm(samples, sample_rate, tempo.bpm),
            )
        },
    );

    let spectral_profile = SpectralProfile {
        centroid: spectral_info.centroid,
        flatness: spectral_info.flatness,
        bass_energy: spectral_info.bass_energy,
        mid_energy: spectral_info.mid_energy,
        high_energy: spectral_info.high_energy,
    };

    let rhythm_profile = RhythmProfile {
        complexity: rhythm_info.complexity,
        danceability: rhythm_info.danceability,
    };

    // Compute energy trajectory and structural markers
    let energy_trajectory = compute_energy_trajectory(&energy_info.curve);
    let (intro_length, outro_length) = detect_intro_outro(&energy_info, duration_ms);
    let first_drop = energy_info.drop_candidates_ms.first().copied();

    // Detect Safe Loops and Regions
    let mut safe_loops = Vec::new();
    let times_ms = &beat_info.times_ms;
    let energy_curve = &energy_info.curve;
    let window_ms = (energy_info.window_seconds * 1000.0) as u64;

    if times_ms.len() > 32 {
        // Scan for 16-beat (4 bar) safe loops
        for i in (0..times_ms.len().saturating_sub(16)).step_by(16) {
            let start_ms = times_ms[i];
            let end_ms = times_ms[i + 16];
            
            // Calculate average energy and variance in this window
            let start_idx = (start_ms / window_ms) as usize;
            let end_idx = (end_ms / window_ms) as usize;
            
            if end_idx < energy_curve.len() && start_idx < end_idx {
                let segment = &energy_curve[start_idx..end_idx];
                let avg: f64 = segment.iter().sum::<f64>() / segment.len() as f64;
                let variance: f64 = segment.iter().map(|&v| (v - avg).powi(2)).sum::<f64>() / segment.len() as f64;
                
                // If energy is stable (low variance) and not silence
                if variance < 0.02 && avg > 0.1 {
                    safe_loops.push(crate::profile::SafeLoop {
                        start_beat: i as u64,
                        length_bars: 4,
                        quality: (1.0 - variance * 5.0).max(0.0f64),
                    });
                }
            }
        }
    }

    let intro = if !times_ms.is_empty() {
        Some(crate::profile::TimeRange {
            start_ms: times_ms[0],
            end_ms: times_ms.get(32).cloned().unwrap_or(*times_ms.last().unwrap()),
            confidence: 0.8,
        })
    } else { None };

    let outro = if times_ms.len() > 32 {
        Some(crate::profile::TimeRange {
            start_ms: times_ms[times_ms.len() - 32],
            end_ms: *times_ms.last().unwrap(),
            confidence: 0.8,
        })
    } else { None };

    let regions = crate::profile::RegionInfo {
        intro,
        outro,
        safe_loops,
    };

    let perceived_tempo = analyze_perceived_tempo(samples, sample_rate, &tempo, &beat_info);

    Ok(TrackProfile {
        version: 1,
        track_id,
        file_hash,
        analysis_timestamp: now,
        tempo,
        perceived_tempo: Some(perceived_tempo),
        beats: beat_info,
        key,
        loudness,
        energy: energy_info,
        regions,
        sections,
        waveform,
        spectral: Some(spectral_profile),
        vocal_presence: Some(vocal),
        rhythm: Some(rhythm_profile),
        energy_trajectory: Some(energy_trajectory),
        intro_length_ms: intro_length,
        outro_length_ms: outro_length,
        first_drop_ms: first_drop,
        duration_ms,
        sample_rate,
        channels,
    })
}

/// Classify the overall energy trajectory of a track.
fn compute_energy_trajectory(curve: &[f64]) -> String {
    if curve.len() < 4 {
        return "steady".to_string();
    }
    let q_len = curve.len() / 4;
    if q_len == 0 {
        return "steady".to_string();
    }
    let first_quarter: f64 = curve[..q_len].iter().sum::<f64>() / q_len as f64;
    let last_quarter: f64 = curve[curve.len() - q_len..].iter().sum::<f64>() / q_len as f64;
    let mid_len = curve.len() / 2;
    let mid_half: f64 = if mid_len > 0 {
        curve[q_len..q_len + mid_len].iter().sum::<f64>() / mid_len as f64
    } else {
        first_quarter
    };

    if last_quarter > first_quarter * 1.3 {
        "building".to_string()
    } else if first_quarter > last_quarter * 1.3 {
        "declining".to_string()
    } else if mid_half > first_quarter * 1.3 && mid_half > last_quarter * 1.3 {
        "dynamic".to_string()
    } else {
        "steady".to_string()
    }
}

/// Detect intro and outro lengths based on energy curve.
fn detect_intro_outro(energy: &EnergyInfo, duration_ms: u64) -> (Option<u64>, Option<u64>) {
    let curve = &energy.curve;
    if curve.len() < 4 {
        return (None, None);
    }

    let avg = energy.average;
    let window_ms = (energy.window_seconds * 1000.0) as u64;
    if window_ms == 0 {
        return (None, None);
    }

    // Intro: first stretch of below-average energy
    let intro_end = curve.iter().position(|&e| e > avg * 0.5).unwrap_or(0);
    let intro_ms = if intro_end > 0 {
        Some(intro_end as u64 * window_ms)
    } else {
        None
    };

    // Outro: last stretch of below-average energy
    let outro_start = curve
        .iter()
        .rposition(|&e| e > avg * 0.5)
        .unwrap_or(curve.len());
    let outro_ms = if outro_start < curve.len() {
        Some(duration_ms.saturating_sub(outro_start as u64 * window_ms))
    } else {
        None
    };

    (intro_ms, outro_ms)
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
