import { useStore } from "../store";

export function AutoDjPanel() {
    const { conductorState, tracks, engineState, aiDecision } = useStore();
    const cs = conductorState;

    const templateScores = cs?.template_scores ?? aiDecision?.template_scores ?? null;
    const reasoningTags = cs?.reasoning_tags ?? [];
    const pendingTitle = cs?.pending_track_title ?? "\u2014";
    const phase = cs?.phase ?? "idle";
    const message = cs?.message ?? "";
    const confidence = cs?.confidence ?? aiDecision?.confidence ?? null;
    const entryMarker = cs?.entry_marker ?? aiDecision?.entry_marker ?? null;

    // Current playing track info
    const activeDeckSnap = engineState?.deck_a?.state === "playing" ? engineState?.deck_a
        : engineState?.deck_b?.state === "playing" ? engineState?.deck_b : null;
    const currentTrack = activeDeckSnap?.track_id
        ? tracks.find((t) => t.id === activeDeckSnap.track_id) : null;
    const pendingTrack = cs?.pending_track_id
        ? tracks.find((t) => t.id === cs.pending_track_id) : null;

    const phaseLabel: Record<string, string> = {
        idle: "Idle",
        monitoring: "Monitoring",
        planning: "Planning Next",
        loading: "Loading Track",
        aligning: "Aligning BPM",
        transitioning: "Transitioning",
    };

    const phaseColor: Record<string, string> = {
        idle: "var(--text-muted)",
        monitoring: "var(--status-ready)",
        planning: "var(--status-pending)",
        loading: "var(--accent-a)",
        aligning: "var(--accent-b)",
        transitioning: "var(--status-active)",
    };

    return (
        <div className="autodj-panel">
            {/* Left: Session Status */}
            <div className="autodj-panel-section autodj-session-status">
                <span className="autodj-panel-header">SESSION</span>
                <div className="autodj-phase-row">
                    <span className="autodj-phase-dot" style={{ background: phaseColor[phase] || "var(--text-muted)" }} />
                    <span className="autodj-phase-label">{phaseLabel[phase] || phase}</span>
                </div>
                {message && <span className="autodj-phase-message">{message}</span>}
                {confidence != null && (
                    <div className="autodj-stat-row">
                        <span className="autodj-stat-label">Confidence</span>
                        <span className="autodj-stat-value">{Math.round(confidence * 100)}%</span>
                    </div>
                )}
                {entryMarker && (
                    <div className="autodj-stat-row">
                        <span className="autodj-stat-label">Entry Point</span>
                        <span className="autodj-stat-value autodj-stat-marker">{entryMarker}</span>
                    </div>
                )}
                {currentTrack && (
                    <div className="autodj-stat-row">
                        <span className="autodj-stat-label">Now</span>
                        <span className="autodj-stat-value" title={currentTrack.title || ""}>{currentTrack.title || "\u2014"}</span>
                    </div>
                )}
                {pendingTrack && (
                    <div className="autodj-stat-row">
                        <span className="autodj-stat-label">Next</span>
                        <span className="autodj-stat-value" title={pendingTrack.title || ""}>{pendingTrack.title || pendingTitle}</span>
                    </div>
                )}
            </div>

            {/* Center: Template Scores */}
            {templateScores && templateScores.length > 0 && (
                <div className="autodj-panel-section autodj-template-scores">
                    <span className="autodj-panel-header">TEMPLATE SCORES</span>
                    <div className="autodj-scores-grid">
                        {templateScores.slice(0, 8).map(([name, score]: [string, number]) => (
                            <div key={name} className="autodj-score-row">
                                <span className="autodj-score-name">
                                    {name.replace(/_/g, " ")}
                                </span>
                                <div className="autodj-score-bar-track">
                                    <div
                                        className="autodj-score-bar-fill"
                                        style={{ width: `${Math.min(score * 100, 100)}%` }}
                                    />
                                </div>
                                <span className="autodj-score-value">{(score * 100).toFixed(0)}</span>
                            </div>
                        ))}
                    </div>
                </div>
            )}

            {/* Right: Reasoning Tags */}
            {reasoningTags.length > 0 && (
                <div className="autodj-panel-section autodj-reasoning">
                    <span className="autodj-panel-header">AI REASONING</span>
                    <div className="autodj-tags-wrap">
                        {reasoningTags.map((tag: string, i: number) => (
                            <span key={i} className="autodj-reasoning-tag">
                                {tag.replace(/_/g, " ")}
                            </span>
                        ))}
                    </div>
                </div>
            )}
        </div>
    );
}
