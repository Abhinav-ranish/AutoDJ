//! Phase 40: DJ Feel Tuning & Regression Suite
//!
//! Provides a quality scoring harness that evaluates a sequence of AI decisions
//! for diversity, flow, timing, musicality, and detects known bad behaviors.

use crate::planner::AiDecision;

/// A recorded session for quality analysis.
pub struct SessionLog {
    pub decisions: Vec<AiDecision>,
    pub track_ids: Vec<String>,
    pub templates_used: Vec<String>,
    pub artists: Vec<String>,
    /// Phase 48: Whether replay was allowed during this session.
    pub allow_replay: bool,
}

/// Quality scores for a DJ session (each 0.0–1.0).
#[derive(Debug, Clone)]
pub struct QualityReport {
    pub diversity_score: f64,
    pub flow_score: f64,
    pub timing_score: f64,
    pub musicality_score: f64,
    pub overall_score: f64,
    pub issues: Vec<QualityIssue>,
}

/// Detected quality issues in a session.
#[derive(Debug, Clone)]
pub struct QualityIssue {
    pub kind: IssueKind,
    pub description: String,
    pub severity: f64,
    pub at_decision_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IssueKind {
    TooEarlyMix,
    TooLateMix,
    RepeatedTrack,
    RepeatedArtist,
    OverusedTemplate,
    WrongEntryPoint,
    LowDiversity,
    EnergyFlatline,
    NoHypeMoments,
    /// Phase 47: Consecutive tracks have incompatible vibes.
    VibeMismatch,
    /// Phase 48: A track has been played more than 3 times in a session.
    ExcessiveReplay,
}

/// Evaluate a session log and produce a quality report.
pub fn evaluate_session(log: &SessionLog) -> QualityReport {
    let mut issues = Vec::new();

    // ── Diversity: how varied are the templates? ──────────────────
    let unique_templates: std::collections::HashSet<&str> =
        log.templates_used.iter().map(|s| s.as_str()).collect();
    let diversity_score = if log.templates_used.is_empty() {
        0.0
    } else {
        let ratio = unique_templates.len() as f64 / log.templates_used.len() as f64;
        ratio.min(1.0)
    };

    // Check for overused templates (>40% of all transitions)
    for tmpl in &unique_templates {
        let count = log
            .templates_used
            .iter()
            .filter(|t| t.as_str() == *tmpl)
            .count();
        let frac = count as f64 / log.templates_used.len().max(1) as f64;
        if frac > 0.4 && log.templates_used.len() > 3 {
            issues.push(QualityIssue {
                kind: IssueKind::OverusedTemplate,
                description: format!(
                    "Template '{}' used {}% of the time",
                    tmpl,
                    (frac * 100.0) as u32
                ),
                severity: frac,
                at_decision_index: None,
            });
        }
    }

    // ── Flow: check for repeated tracks and artists ──────────────
    let mut flow_penalty = 0.0;
    let mut track_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for (i, tid) in log.track_ids.iter().enumerate() {
        let count = track_counts.entry(tid.as_str()).or_insert(0);
        *count += 1;
        if *count > 1 {
            // Phase 48: Severity depends on whether replay is allowed
            let severity = if log.allow_replay { 0.4 } else { 0.8 };
            let penalty = if log.allow_replay { 0.1 } else { 0.2 };
            issues.push(QualityIssue {
                kind: IssueKind::RepeatedTrack,
                description: format!("Track '{}' played {} times", tid, count),
                severity,
                at_decision_index: Some(i),
            });
            flow_penalty += penalty;
        }
        // Phase 48: Excessive replay (>3 plays) is always an issue
        if *count > 3 {
            issues.push(QualityIssue {
                kind: IssueKind::ExcessiveReplay,
                description: format!(
                    "Track '{}' played {} times (exceeds hard cap of 3)",
                    tid, count
                ),
                severity: 1.0,
                at_decision_index: Some(i),
            });
            flow_penalty += 0.3;
        }
    }

    // Artist repetition within 5 tracks
    for (i, artist) in log.artists.iter().enumerate() {
        if artist.is_empty() {
            continue;
        }
        let recent = &log.artists[i.saturating_sub(5)..i];
        if recent.iter().any(|a| a.eq_ignore_ascii_case(artist)) {
            issues.push(QualityIssue {
                kind: IssueKind::RepeatedArtist,
                description: format!("Artist '{}' repeated within 5 tracks", artist),
                severity: 0.5,
                at_decision_index: Some(i),
            });
            flow_penalty += 0.1;
        }
    }
    let flow_score = (1.0_f64 - flow_penalty).max(0.0);

    // ── Timing: check for too-early or too-late mixes ────────────
    let mut timing_penalty = 0.0;
    for (i, d) in log.decisions.iter().enumerate() {
        if d.reasoning_tags
            .as_ref()
            .map(|t| t.iter().any(|s| s == "too_early_guard"))
            .unwrap_or(false)
        {
            issues.push(QualityIssue {
                kind: IssueKind::TooEarlyMix,
                description: format!("Decision {} triggered too-early guard", i),
                severity: 0.4,
                at_decision_index: Some(i),
            });
            timing_penalty += 0.05;
        }
    }
    let timing_score = (1.0_f64 - timing_penalty).max(0.0);

    // ── Musicality: verify entry points are reasonable ────────────
    let mut musicality_penalty = 0.0;
    for (i, d) in log.decisions.iter().enumerate() {
        // Penalize if creative_intensity is always the same (no dynamic range)
        if let Some(ci) = d.creative_intensity {
            if ci < 0.1 || ci > 0.95 {
                musicality_penalty += 0.02;
            }
        }
        // Check for missing entry markers on high-energy transitions
        if d.entry_marker.is_none() && d.confidence.unwrap_or(0.0) > 0.8 {
            issues.push(QualityIssue {
                kind: IssueKind::WrongEntryPoint,
                description: format!("Decision {} has high confidence but no entry marker", i),
                severity: 0.3,
                at_decision_index: Some(i),
            });
            musicality_penalty += 0.03;
        }
    }
    let musicality_score = (1.0_f64 - musicality_penalty).max(0.0);

    // Check for energy flatline (all decisions within narrow energy band)
    let confidence_values: Vec<f64> = log.decisions.iter().filter_map(|d| d.confidence).collect();
    if confidence_values.len() > 5 {
        let variance = statistical_variance(&confidence_values);
        if variance < 0.01 {
            issues.push(QualityIssue {
                kind: IssueKind::EnergyFlatline,
                description: "Decision confidence shows no variation — possible energy flatline"
                    .to_string(),
                severity: 0.4,
                at_decision_index: None,
            });
        }
    }

    if diversity_score < 0.3 && log.templates_used.len() > 3 {
        issues.push(QualityIssue {
            kind: IssueKind::LowDiversity,
            description: format!(
                "Only {} unique templates in {} transitions",
                unique_templates.len(),
                log.templates_used.len()
            ),
            severity: 0.6,
            at_decision_index: None,
        });
    }

    let overall_score =
        diversity_score * 0.25 + flow_score * 0.30 + timing_score * 0.20 + musicality_score * 0.25;

    QualityReport {
        diversity_score,
        flow_score,
        timing_score,
        musicality_score,
        overall_score,
        issues,
    }
}

fn statistical_variance(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64
}

/// Regression test fixture: defines a scenario that should produce a known outcome.
pub struct RegressionFixture {
    pub name: String,
    pub description: String,
    pub expected_issue: Option<IssueKind>,
    pub log: SessionLog,
}

/// Build built-in regression test fixtures.
pub fn built_in_fixtures() -> Vec<RegressionFixture> {
    vec![
        RegressionFixture {
            name: "overused_smooth_beatmix".to_string(),
            description: "10 transitions all using smooth_beatmix — should flag OverusedTemplate"
                .to_string(),
            expected_issue: Some(IssueKind::OverusedTemplate),
            log: SessionLog {
                decisions: (0..10)
                    .map(|_| AiDecision {
                        decision: "prepare_next".to_string(),
                        reason: "test".to_string(),
                        track_selection: None,
                        execute_at_ms: None,
                        transition_plan: None,
                        actions: None,
                        reasoning_tags: None,
                        confidence: Some(0.7),
                        entry_marker: Some("intro".to_string()),
                        exit_marker: None,
                        creative_intensity: Some(0.4),
                        set_arc: None,
                        planner_mode: Some("heuristic_only".to_string()),
                        template_scores: None,
                        fallback_reason: None,
                    })
                    .collect(),
                track_ids: (0..10).map(|i| format!("track_{}", i)).collect(),
                templates_used: vec!["smooth_beatmix".to_string(); 10],
                artists: (0..10).map(|i| format!("artist_{}", i)).collect(),
                allow_replay: false,
            },
        },
        RegressionFixture {
            name: "repeated_track".to_string(),
            description: "Same track played twice — should flag RepeatedTrack".to_string(),
            expected_issue: Some(IssueKind::RepeatedTrack),
            log: SessionLog {
                decisions: vec![
                    make_test_decision("prepare_next"),
                    make_test_decision("prepare_next"),
                    make_test_decision("prepare_next"),
                ],
                track_ids: vec![
                    "track_a".to_string(),
                    "track_b".to_string(),
                    "track_a".to_string(),
                ],
                templates_used: vec![
                    "smooth_beatmix".to_string(),
                    "filter_sweep".to_string(),
                    "drop_swap".to_string(),
                ],
                artists: vec![
                    "artist_1".to_string(),
                    "artist_2".to_string(),
                    "artist_1".to_string(),
                ],
                allow_replay: false,
            },
        },
        RegressionFixture {
            name: "repeated_artist".to_string(),
            description: "Same artist within 5 tracks — should flag RepeatedArtist".to_string(),
            expected_issue: Some(IssueKind::RepeatedArtist),
            log: SessionLog {
                decisions: (0..5).map(|_| make_test_decision("prepare_next")).collect(),
                track_ids: (0..5).map(|i| format!("track_{}", i)).collect(),
                templates_used: vec![
                    "smooth_beatmix".to_string(),
                    "filter_sweep".to_string(),
                    "drop_swap".to_string(),
                    "echo_drop".to_string(),
                    "loop_roll".to_string(),
                ],
                artists: vec![
                    "same_artist".to_string(),
                    "other".to_string(),
                    "another".to_string(),
                    "same_artist".to_string(),
                    "final".to_string(),
                ],
                allow_replay: false,
            },
        },
    ]
}

fn make_test_decision(decision: &str) -> AiDecision {
    AiDecision {
        decision: decision.to_string(),
        reason: "test fixture".to_string(),
        track_selection: None,
        execute_at_ms: None,
        transition_plan: None,
        actions: None,
        reasoning_tags: None,
        confidence: Some(0.7),
        entry_marker: Some("intro".to_string()),
        exit_marker: None,
        creative_intensity: Some(0.5),
        set_arc: None,
        planner_mode: Some("heuristic_only".to_string()),
        template_scores: None,
        fallback_reason: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overused_template_detected() {
        let fixtures = built_in_fixtures();
        let fixture = fixtures
            .iter()
            .find(|f| f.name == "overused_smooth_beatmix")
            .unwrap();
        let report = evaluate_session(&fixture.log);
        assert!(report
            .issues
            .iter()
            .any(|i| i.kind == IssueKind::OverusedTemplate));
        assert!(report.diversity_score < 0.3);
    }

    #[test]
    fn test_repeated_track_detected() {
        let fixtures = built_in_fixtures();
        let fixture = fixtures
            .iter()
            .find(|f| f.name == "repeated_track")
            .unwrap();
        let report = evaluate_session(&fixture.log);
        assert!(report
            .issues
            .iter()
            .any(|i| i.kind == IssueKind::RepeatedTrack));
    }

    #[test]
    fn test_repeated_artist_detected() {
        let fixtures = built_in_fixtures();
        let fixture = fixtures
            .iter()
            .find(|f| f.name == "repeated_artist")
            .unwrap();
        let report = evaluate_session(&fixture.log);
        assert!(report
            .issues
            .iter()
            .any(|i| i.kind == IssueKind::RepeatedArtist));
    }

    #[test]
    fn test_good_session_scores_high() {
        let log = SessionLog {
            decisions: (0..8)
                .map(|i| AiDecision {
                    decision: "prepare_next".to_string(),
                    reason: "test".to_string(),
                    track_selection: None,
                    execute_at_ms: None,
                    transition_plan: None,
                    actions: None,
                    reasoning_tags: None,
                    confidence: Some(0.5 + 0.05 * i as f64),
                    entry_marker: Some("intro".to_string()),
                    exit_marker: None,
                    creative_intensity: Some(0.3 + 0.08 * i as f64),
                    set_arc: None,
                    planner_mode: Some("heuristic_only".to_string()),
                    template_scores: None,
                    fallback_reason: None,
                })
                .collect(),
            track_ids: (0..8).map(|i| format!("track_{}", i)).collect(),
            templates_used: vec![
                "smooth_beatmix".to_string(),
                "filter_sweep".to_string(),
                "drop_swap".to_string(),
                "echo_drop".to_string(),
                "bass_kill_swap".to_string(),
                "break_blend".to_string(),
                "backspin_cut".to_string(),
                "loop_roll".to_string(),
            ],
            artists: (0..8).map(|i| format!("artist_{}", i)).collect(),
            allow_replay: false,
        };
        let report = evaluate_session(&log);
        assert!(
            report.overall_score > 0.7,
            "Good session should score >0.7, got {}",
            report.overall_score
        );
        assert!(
            report.issues.is_empty(),
            "Good session should have no issues: {:?}",
            report.issues
        );
    }

    #[test]
    fn test_replay_allowed_reduces_severity() {
        let log = SessionLog {
            decisions: vec![make_test_decision("prepare_next"); 3],
            track_ids: vec![
                "track_a".to_string(),
                "track_b".to_string(),
                "track_a".to_string(),
            ],
            templates_used: vec![
                "smooth_beatmix".to_string(),
                "filter_sweep".to_string(),
                "drop_swap".to_string(),
            ],
            artists: vec!["a1".to_string(), "a2".to_string(), "a3".to_string()],
            allow_replay: true,
        };
        let report = evaluate_session(&log);
        let repeat_issue = report
            .issues
            .iter()
            .find(|i| i.kind == IssueKind::RepeatedTrack)
            .unwrap();
        assert!(
            (repeat_issue.severity - 0.4).abs() < 0.01,
            "Replay-allowed severity should be 0.4, got {}",
            repeat_issue.severity
        );
    }

    #[test]
    fn test_replay_disallowed_high_severity() {
        let log = SessionLog {
            decisions: vec![make_test_decision("prepare_next"); 3],
            track_ids: vec![
                "track_a".to_string(),
                "track_b".to_string(),
                "track_a".to_string(),
            ],
            templates_used: vec![
                "smooth_beatmix".to_string(),
                "filter_sweep".to_string(),
                "drop_swap".to_string(),
            ],
            artists: vec!["a1".to_string(), "a2".to_string(), "a3".to_string()],
            allow_replay: false,
        };
        let report = evaluate_session(&log);
        let repeat_issue = report
            .issues
            .iter()
            .find(|i| i.kind == IssueKind::RepeatedTrack)
            .unwrap();
        assert!(
            (repeat_issue.severity - 0.8).abs() < 0.01,
            "Replay-disallowed severity should be 0.8, got {}",
            repeat_issue.severity
        );
    }

    #[test]
    fn test_excessive_replay_always_flagged() {
        let log = SessionLog {
            decisions: vec![make_test_decision("prepare_next"); 5],
            track_ids: vec![
                "track_a".to_string(),
                "track_a".to_string(),
                "track_a".to_string(),
                "track_a".to_string(),
                "track_b".to_string(),
            ],
            templates_used: vec![
                "smooth_beatmix".to_string(),
                "filter_sweep".to_string(),
                "drop_swap".to_string(),
                "echo_drop".to_string(),
                "loop_roll".to_string(),
            ],
            artists: (0..5).map(|i| format!("artist_{}", i)).collect(),
            allow_replay: true,
        };
        let report = evaluate_session(&log);
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.kind == IssueKind::ExcessiveReplay),
            "Should flag ExcessiveReplay when track played >3 times"
        );
    }
}
