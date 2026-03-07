use crate::profile::{BeatInfo, PerceivedTempoInfo, TempoInfo};

/// Analyze the perceived tempo and groove of an audio track.
///
/// Uses onset density analysis at the sub-beat level to classify groove:
/// - Counts onset events per beat within each bar
/// - Measures syncopation (off-beat vs on-beat energy ratio)
/// - Combines with BPM range heuristics for genre classification
pub fn analyze_perceived_tempo(
    samples: &[f32],
    sample_rate: u32,
    tempo_info: &TempoInfo,
    beat_info: &BeatInfo,
) -> PerceivedTempoInfo {
    let raw_bpm = tempo_info.bpm;

    // Fallback if no beats detected
    if beat_info.times_ms.len() < 8 || samples.is_empty() {
        return PerceivedTempoInfo {
            bpm: raw_bpm,
            perceived_tempo: raw_bpm,
            groove_type: "standard".to_string(),
            tempo_confidence: 0.0,
        };
    }

    // ─── Step 1: Compute per-beat onset density ───────────────────
    // For each beat window, calculate:
    //   - RMS energy (overall loudness)
    //   - Spectral flux (onset count proxy via positive energy deltas)
    //   - Sub-beat syncopation ratio

    let hop_samples = (sample_rate as f32 * 0.005) as usize * 2; // 5ms hops, stereo
    let mut beat_energies: Vec<f32> = Vec::new();
    let mut sub_beat_onset_counts: Vec<usize> = Vec::new();

    for i in 0..beat_info.times_ms.len().saturating_sub(1) {
        let start_ms = beat_info.times_ms[i];
        let end_ms = beat_info.times_ms[i + 1];

        let start_sample = ((start_ms * sample_rate as u64) / 1000) as usize * 2;
        let end_sample = ((end_ms * sample_rate as u64) / 1000) as usize * 2;

        if end_sample > samples.len() || start_sample >= end_sample {
            continue;
        }

        let chunk = &samples[start_sample..end_sample];

        // RMS of this beat
        let rms: f32 = (chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len() as f32).sqrt();
        beat_energies.push(rms);

        // Count onset transients within this beat using 5ms hop spectral flux
        let mut prev_rms = 0.0f32;
        let mut onsets = 0usize;
        for hop_chunk in chunk.chunks(hop_samples) {
            let hop_rms = (hop_chunk.iter().map(|s| s * s).sum::<f32>()
                / hop_chunk.len().max(1) as f32)
                .sqrt();
            let delta = hop_rms - prev_rms;
            // Positive flux above threshold = onset
            if delta > 0.02 {
                onsets += 1;
            }
            prev_rms = hop_rms;
        }
        sub_beat_onset_counts.push(onsets);
    }

    if beat_energies.is_empty() {
        return PerceivedTempoInfo {
            bpm: raw_bpm,
            perceived_tempo: raw_bpm,
            groove_type: "standard".to_string(),
            tempo_confidence: 0.0,
        };
    }

    // ─── Step 2: Aggregate metrics ───────────────────────────────
    let avg_energy: f32 = beat_energies.iter().sum::<f32>() / beat_energies.len() as f32;
    let energy_variance: f32 = beat_energies
        .iter()
        .map(|e| (e - avg_energy).powi(2))
        .sum::<f32>()
        / beat_energies.len() as f32;

    let avg_onsets: f32 =
        sub_beat_onset_counts.iter().sum::<usize>() as f32 / sub_beat_onset_counts.len() as f32;

    // Measure sparsity: how many beats have very low energy relative to avg
    let sparse_beats = beat_energies
        .iter()
        .filter(|&&e| e < avg_energy * 0.3)
        .count();
    let sparsity_ratio = sparse_beats as f32 / beat_energies.len() as f32;

    // Measure syncopation: variance in onset counts per beat
    let onset_variance: f32 = sub_beat_onset_counts
        .iter()
        .map(|&c| (c as f32 - avg_onsets).powi(2))
        .sum::<f32>()
        / sub_beat_onset_counts.len() as f32;

    // ─── Step 3: Groove classification ───────────────────────────
    let (groove_type, perceived_tempo, confidence) = classify_groove(
        raw_bpm,
        avg_onsets,
        energy_variance,
        sparsity_ratio,
        onset_variance,
    );

    PerceivedTempoInfo {
        bpm: raw_bpm,
        perceived_tempo,
        groove_type: groove_type.to_string(),
        tempo_confidence: confidence,
    }
}

/// Classify groove type based on BPM + rhythmic features.
fn classify_groove(
    bpm: f64,
    avg_onsets: f32,
    energy_variance: f32,
    sparsity_ratio: f32,
    onset_variance: f32,
) -> (&'static str, f64, f64) {
    // ── Reggaeton / Dembow ──────────────────────────────────────
    // Range: 80–115 BPM. Dembow has a distinctive dense, syncopated pattern.
    // High onset density + moderate variance = dembow rhythm.
    if bpm >= 80.0 && bpm <= 115.0 {
        if avg_onsets > 1.5 || onset_variance > 0.5 {
            return ("dembow", bpm * 1.15, 0.85);
        }
        // Could still be reggaeton with less pronounced pattern
        if energy_variance > 0.001 {
            return ("dembow", bpm * 1.10, 0.70);
        }
    }

    // ── Trap / Half-Time ────────────────────────────────────────
    // Range: 120–170 BPM with sparse beats (half-time feel).
    // High sparsity = lots of silent beats = half-time groove.
    if bpm >= 120.0 && bpm <= 170.0 && sparsity_ratio > 0.15 {
        return ("trap_half_time", bpm / 2.0, 0.82);
    }

    // ── Hip-Hop / Boom Bap ──────────────────────────────────────
    // Range: 80–100 BPM. Moderate onsets, moderate variance.
    if bpm >= 80.0 && bpm < 100.0 && avg_onsets < 2.0 && sparsity_ratio < 0.15 {
        return ("boom_bap", bpm, 0.70);
    }

    // ── Drill ───────────────────────────────────────────────────
    // Range: 135–150 BPM. Dense hi-hats (high onset count) + syncopation.
    if bpm >= 135.0 && bpm <= 155.0 && avg_onsets > 3.0 {
        return ("drill", bpm, 0.75);
    }

    // ── Jersey Club ─────────────────────────────────────────────
    // Range: 130–145 BPM. Very high onset density with bouncy feel.
    if bpm >= 130.0 && bpm <= 145.0 && avg_onsets > 2.5 && onset_variance > 1.0 {
        return ("jersey_club", bpm, 0.72);
    }

    // ── Four-on-the-Floor (House / Techno) ──────────────────────
    // Range: 118–135 BPM. Low energy variance = very steady kick pattern.
    if bpm >= 118.0 && bpm <= 135.0 && energy_variance < 0.002 && sparsity_ratio < 0.05 {
        return ("four_on_floor", bpm, 0.88);
    }

    // ── Techno ──────────────────────────────────────────────────
    // Range: 130–155 BPM. Very steady, very low variance.
    if bpm >= 130.0 && bpm <= 155.0 && energy_variance < 0.003 && sparsity_ratio < 0.08 {
        return ("four_on_floor", bpm, 0.80);
    }

    // ── Breakbeat ───────────────────────────────────────────────
    // High onset variance (irregular pattern) outside other categories.
    if onset_variance > 2.0 && avg_onsets > 2.0 {
        return ("breakbeat", bpm, 0.65);
    }

    // ── DnB ─────────────────────────────────────────────────────
    // Very fast: 160–185 BPM.
    if bpm >= 160.0 && bpm <= 185.0 {
        return ("drum_and_bass", bpm, 0.75);
    }

    // ── Default ─────────────────────────────────────────────────
    ("standard", bpm, 0.50)
}
