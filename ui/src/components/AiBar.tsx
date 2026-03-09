import { useState } from "react";
import { useStore, DjMode, EngineState } from "../store";
import { formatTime } from "./utils";

const SPECTRUM_LABELS = ["32", "64", "125", "250", "500", "1K", "2K", "4K"];

function SpectrumAnalyzer({ spectrum }: { spectrum: number[] }) {
    return (
        <div className="spectrum-analyzer">
            {spectrum.slice(0, 8).map((val, i) => (
                <div key={i} className="spectrum-col">
                    <div className="spectrum-bar-track">
                        <div className="spectrum-bar" style={{ height: `${Math.min(val * 100, 100)}%` }} />
                    </div>
                    <span className="spectrum-label">{SPECTRUM_LABELS[i]}</span>
                </div>
            ))}
        </div>
    );
}

function humanizeToken(token: string): string {
    return token.replace(/_/g, " ").trim();
}

function templateLabel(template: string | null | undefined): string | null {
    if (!template) return null;
    return humanizeToken(template)
        .split(" ")
        .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
        .join(" ");
}

function decisionSourceLabel(
    source: string | null | undefined
): { label: string; kind: "llm" | "heuristic" | "queue" | "none"; detail: string | null } {
    if (!source) return { label: "", kind: "none", detail: null };
    if (source.startsWith("llm_live:")) {
        return { label: "LLM advisory", kind: "llm", detail: source };
    }
    if (source.includes("queue")) {
        return { label: "Queue driven", kind: "queue", detail: source };
    }
    if (source === "heuristic_only") {
        return { label: "Heuristic scorer", kind: "heuristic", detail: source };
    }
    return {
        label: "Heuristic scorer",
        kind: "heuristic",
        detail: humanizeToken(source),
    };
}

function fallbackReasonLabel(reason: string | null | undefined): { label: string | null; isExpected: boolean } {
    if (!reason) return { label: null, isExpected: false };
    const map: Record<string, string> = {
        missing_api_key: "No API key -> heuristic",
        disabled: "LLM disabled -> heuristic",
        provider_error: "LLM unavailable -> heuristic",
        provider_offline: "Provider offline -> heuristic",
        timeout: "LLM timeout -> heuristic",
        parse_failure: "LLM parse failed -> heuristic",
        schema_validation_failure: "LLM schema mismatch -> heuristic",
        unsafe_action_rejected: "Unsafe action rejected",
        insufficient_confidence: "Low confidence -> fallback",
        planner_error: "Planner error -> runtime fallback",
        queue_override: "Queue override",
        queue_seed: "Queue seed",
        initial_random_seed: "Initial random seed",
        silence_recovery: "Silence recovery",
        emergency_guard: "Emergency guard",
        default_template: "Default template fallback",
    };
    return {
        label: map[reason] || "Fallback to heuristic scorer",
        isExpected: [
            "missing_api_key",
            "disabled",
            "provider_offline",
            "queue_override",
            "queue_seed",
            "initial_random_seed",
        ].includes(reason),
    };
}

function formatTriggerReason(
    reason: string | null | undefined
): { short: string; detail: string; isMarkerAware: boolean } | null {
    if (!reason) return null;
    if (reason.startsWith("window:")) {
        const match = reason.match(/^window:(.+?)@(\d+)ms$/);
        if (match) {
            const anchor = humanizeToken(match[1]);
            const ms = parseInt(match[2], 10);
            const sec = (ms / 1000).toFixed(1);
            return { short: anchor, detail: `Marker: ${anchor} at ${sec}s`, isMarkerAware: true };
        }
        return { short: "Marker", detail: reason, isMarkerAware: true };
    }
    if (reason.startsWith("fallback:") || reason.startsWith("classic:") || reason.startsWith("no_profile:")) {
        return { short: "Position", detail: "Position fallback trigger", isMarkerAware: false };
    }
    if (reason.startsWith("emergency") || reason === "silence_recovery") {
        return { short: "Emergency", detail: humanizeToken(reason), isMarkerAware: false };
    }
    return null;
}

function deriveActiveDeckId(engineState: EngineState | null): "a" | "b" {
    if (!engineState) return "a";
    const aPlaying = engineState.deck_a.state === "playing";
    const bPlaying = engineState.deck_b.state === "playing";

    if (aPlaying && !bPlaying) return "a";
    if (bPlaying && !aPlaying) return "b";
    return engineState.crossfader <= 0 ? "a" : "b";
}

function formatEta(ms: number): string {
    if (ms <= 0) return "now";
    return `in ${formatTime(ms)}`;
}

export function AiBar() {
    const { mode, setMode, requestNextTrack, aiDecision, isThinking, tracks, loadTrack, engineState, conductorState } = useStore();

    const cs = conductorState;
    const suggestedTrackId = mode === "autodj"
        ? cs?.pending_track_id
        : (cs?.pending_track_id || aiDecision?.track_selection);

    const suggestedTrack = suggestedTrackId
        ? tracks.find((t) => t.id === suggestedTrackId)
        : null;

    const getModeLabel = (m: DjMode) => {
        switch (m) {
            case "manual": return "Manual";
            case "assisted": return "Assisted";
            case "autodj": return "Auto-DJ";
        }
    };

    const activeDeckId = deriveActiveDeckId(engineState);
    const activeDeckSnap = activeDeckId === "a" ? engineState?.deck_a : engineState?.deck_b;
    const currentTrackId = activeDeckSnap?.track_id;
    const currentTrack = tracks.find((t) => t.id === currentTrackId);
    const isAnythingPlaying = engineState?.deck_a?.state === "playing" || engineState?.deck_b?.state === "playing";
    const currentTrackName = currentTrack?.title || (isAnythingPlaying ? "Loading..." : "Nothing playing");
    const currentArtist = currentTrack?.artist || "";
    const liveBpm = (activeDeckSnap?.speed && currentTrack?.bpm)
        ? currentTrack.bpm * activeDeckSnap.speed
        : currentTrack?.bpm ?? engineState?.master_bpm;
    const displayBpm = liveBpm?.toFixed(1) || "--";

    // Backend conductor state is the primary source for AutoDJ runtime state.
    const templateChosen = templateLabel(cs?.template_chosen ?? aiDecision?.transition_plan?.template_name);
    const templateIsSimulated = cs?.decision_meta?.template_is_simulated ?? null;
    const confidence = cs?.confidence ?? aiDecision?.confidence ?? null;
    const decisionSource = decisionSourceLabel(
        mode === "autodj"
            ? cs?.decision_source
            : (cs?.decision_source ?? aiDecision?.planner_mode)
    );
    const fallback = fallbackReasonLabel(
        mode === "autodj"
            ? cs?.fallback_reason
            : (cs?.fallback_reason ?? aiDecision?.fallback_reason)
    );
    const entryMarker = cs?.entry_marker ?? aiDecision?.entry_marker ?? null;
    const triggerInfo = formatTriggerReason(cs?.trigger_meta?.reason ?? cs?.trigger_reason);
    const conductorMessage = cs?.message || "";
    const conductorPhase = cs?.phase || "idle";
    const conductorActive = cs?.active ?? false;
    const suggestedTitle = suggestedTrack?.title || cs?.pending_track_title || "--";

    const triggerAnchor = cs?.trigger_meta?.anchor ? humanizeToken(cs.trigger_meta.anchor) : null;
    const triggerWindowStartMs = cs?.trigger_meta?.window_start_ms ?? null;
    const observedPositionMs = cs?.trigger_meta?.observed_position_ms ?? activeDeckSnap?.position_ms ?? null;
    const windowEtaMs = triggerWindowStartMs != null && observedPositionMs != null
        ? triggerWindowStartMs - observedPositionMs
        : null;
    const triggerKind = cs?.trigger_meta?.kind ?? null;

    const transitionIntent = triggerKind === "transition_window"
        ? `${triggerAnchor ? `${triggerAnchor} window` : "Marker window"} ${windowEtaMs != null ? formatEta(windowEtaMs) : ""}`.trim()
        : triggerInfo?.detail ?? null;

    const autodjPhase: "active" | "transitioning" | "error" =
        conductorPhase === "transitioning" ? "transitioning" :
        cs?.error ? "error" : "active";

    const statusMessage = mode === "autodj"
        ? (conductorActive
            ? (transitionIntent || conductorMessage || "Monitoring marker windows")
            : (suggestedTrackId ? `Prepared: ${suggestedTitle}` : "Idle"))
        : (aiDecision ? `Suggested: ${suggestedTitle}` : "");

    return (
        <div className="conductor-rail">
            <div className="conductor-mode">
                <div className="mode-pill-container">
                    {(["manual", "assisted", "autodj"] as DjMode[]).map((m) => (
                        <button
                            key={m}
                            className={`mode-pill ${mode === m ? "active" : ""} mode-${m}`}
                            onClick={() => setMode(m)}
                        >
                            {getModeLabel(m)}
                        </button>
                    ))}
                </div>
            </div>

            <div className={`conductor-now ${activeDeckId === "a" ? "deck-a-active" : "deck-b-active"}`}>
                <div className="conductor-section-header">
                    <span className="conductor-section-label">NOW</span>
                    <span className={`conductor-active-deck conductor-active-deck-${activeDeckId}`}>
                        DECK {activeDeckId.toUpperCase()}
                    </span>
                    {isAnythingPlaying && (
                        <span className="conductor-live-chip">ON AIR</span>
                    )}
                </div>
                <div className="conductor-track-primary">
                    <span className={`conductor-deck-pip conductor-deck-pip-${activeDeckId}`} />
                    <span className="conductor-track-name">{currentTrackName}</span>
                </div>
                <div className="conductor-track-sub">
                    {currentArtist && <span className="conductor-track-artist">{currentArtist}</span>}
                    {displayBpm !== "--" && <span className="conductor-bpm">{displayBpm}</span>}
                </div>
            </div>

            <div className="conductor-bridge">
                <div className={`conductor-bridge-inner ${conductorPhase === "transitioning" ? "active" : ""}`}>
                    {templateChosen && mode !== "manual" ? (
                        <div className="conductor-bridge-detail">
                            <span className="conductor-template-chip">{templateChosen}</span>
                            {templateIsSimulated === true && (
                                <span
                                    className="conductor-template-sim-chip"
                                    title="Template is simulated with gain/filter automation (not dedicated DSP)"
                                >
                                    SIM
                                </span>
                            )}
                            {triggerInfo && (
                                <span
                                    className={`conductor-trigger-chip ${triggerInfo.isMarkerAware ? "conductor-trigger-marker" : "conductor-trigger-fallback"}`}
                                    title={triggerInfo.detail}
                                >
                                    {triggerInfo.short}
                                </span>
                            )}
                        </div>
                    ) : triggerInfo ? (
                        <div className="conductor-bridge-detail">
                            <span
                                className={`conductor-trigger-chip ${triggerInfo.isMarkerAware ? "conductor-trigger-marker" : "conductor-trigger-fallback"}`}
                                title={triggerInfo.detail}
                            >
                                {triggerInfo.short}
                            </span>
                        </div>
                    ) : (
                        <span className="conductor-arrow-glyph">&rarr;</span>
                    )}
                    {entryMarker && (
                        <div className="conductor-marker-row">
                            <span className="conductor-marker-chip entry" title="Entry point on incoming track">{entryMarker}</span>
                        </div>
                    )}
                </div>
                {transitionIntent && (
                    <div className="conductor-bridge-intent" title={transitionIntent}>
                        {transitionIntent}
                    </div>
                )}
            </div>

            <div className="conductor-next">
                <span className="conductor-section-label conductor-section-label-b">NEXT</span>
                <div className="conductor-track-primary">
                    {suggestedTrackId ? (
                        <span className="conductor-track-name conductor-track-name-next">
                            {suggestedTitle}
                        </span>
                    ) : (
                        <span className="conductor-empty">
                            {isThinking || conductorPhase === "planning" ? "Scoring tracks..." : "--"}
                        </span>
                    )}
                </div>
                {suggestedTrackId && (
                    <div className="conductor-track-sub">
                        {suggestedTrack?.bpm && <span className="conductor-bpm conductor-bpm-b">{suggestedTrack.bpm.toFixed(1)}</span>}
                        {suggestedTrack?.key && <span className="conductor-key">{suggestedTrack.key}</span>}
                        {triggerAnchor && <span className="conductor-next-intent">{triggerAnchor}</span>}
                        {mode !== "autodj" && (
                            <button
                                className="conductor-queue-btn"
                                onClick={() => suggestedTrack && loadTrack("b", suggestedTrack.id)}
                                disabled={!suggestedTrack}
                            >
                                LOAD B
                            </button>
                        )}
                    </div>
                )}
            </div>

            <div className="conductor-spectrum">
                <SpectrumAnalyzer spectrum={engineState?.spectrum ?? []} />
            </div>

            <div className="conductor-action">
                {mode === "autodj" ? (
                    <div className={`conductor-autodj-status conductor-autodj-${autodjPhase}`}>
                        <div className={`conductor-status-dot conductor-dot-${autodjPhase}`} />
                        <div className="conductor-status-body">
                            <div className="conductor-status-top">
                                <span className="conductor-status-mode">AUTO-DJ</span>
                                {decisionSource.kind !== "none" && (
                                    <span
                                        className={`conductor-source-chip conductor-source-${decisionSource.kind}`}
                                        title={decisionSource.detail ?? undefined}
                                    >
                                        {decisionSource.label}
                                    </span>
                                )}
                            </div>
                            <span className="conductor-status-message">{statusMessage}</span>
                            {fallback.label && (
                                <span
                                    className={`conductor-fallback-chip ${fallback.isExpected ? "expected" : "warn"}`}
                                    title={cs?.fallback_reason ?? aiDecision?.fallback_reason ?? undefined}
                                >
                                    {fallback.label}
                                </span>
                            )}
                        </div>
                        {confidence != null && (
                            <span
                                className="conductor-confidence"
                                title="Planner confidence score (coarse; no full score breakdown exposed yet)"
                            >
                                {Math.round(confidence * 100)}%
                            </span>
                        )}
                    </div>
                ) : (
                    <div className="conductor-manual-action">
                        <button
                            onClick={requestNextTrack}
                            disabled={isThinking}
                            className={`conductor-find-btn ${isThinking ? "thinking" : ""}`}
                        >
                            {isThinking ? (
                                <><span className="transition-spinner" /> Scoring</>
                            ) : "Find Next"}
                        </button>
                        {decisionSource.kind !== "none" && (
                            <span
                                className={`conductor-source-chip conductor-source-${decisionSource.kind}`}
                                title={decisionSource.detail ?? undefined}
                            >
                                {decisionSource.label}
                            </span>
                        )}
                        {fallback.label && (
                            <span
                                className={`conductor-fallback-chip ${fallback.isExpected ? "expected" : "warn"}`}
                                title={aiDecision?.fallback_reason ?? undefined}
                            >
                                {fallback.label}
                            </span>
                        )}
                    </div>
                )}
            </div>
        </div>
    );
}

// Honest template descriptions — per reality_audit.md #6
const TEMPLATE_INFO: Record<string, { label: string; desc: string; simulated?: boolean }> = {
    smooth_beatmix:  { label: "Smooth Beatmix",      desc: "Crossfade with EQ blending" },
    filter_sweep:    { label: "Filter Sweep",        desc: "Low-pass filter transition" },
    echo_drop:       { label: "Echo Drop (Sim)",     desc: "Simulated echo via gain ramp (no DSP delay)", simulated: true },
    drop_swap:       { label: "Drop Swap",           desc: "Cut at the drop point" },
    loop_roll:       { label: "Loop Roll",           desc: "Loop tightening into transition" },
    bass_kill_swap:  { label: "Bass Kill Swap",      desc: "EQ kill bass swap" },
    break_blend:     { label: "Break Blend",         desc: "Blend during breakdown section" },
    backspin_cut:    { label: "Backspin Cut (Sim)",  desc: "Simulated backspin via gain ramp (use spinback for real reverse)", simulated: true },
    drop_ride:       { label: "Drop Ride",           desc: "Dual-drop ride with bass swap" },
    fx_echo_slam:    { label: "FX Echo Slam (Sim)",  desc: "Simulated echo out + filter slam (app-layer, no DSP)", simulated: true },
};

export function TransitionsPanel() {
    const { executeTransition, engineState } = useStore();
    const [template, setTemplate] = useState("smooth_beatmix");
    const [duration, setDuration] = useState(16);
    const [isTransitioning, setIsTransitioning] = useState(false);
    const [transitionDurationMs, setTransitionDurationMs] = useState(0);
    const [errorMsg, setErrorMsg] = useState<string | null>(null);

    const handleGo = async () => {
        if (!engineState) return;
        let incomingDeck = "B";
        if (engineState.crossfader > 0.1) incomingDeck = "A";
        else if (engineState.crossfader < -0.1) incomingDeck = "B";
        else {
            const aPlaying = engineState.deck_a?.state === "playing";
            const bPlaying = engineState.deck_b?.state === "playing";
            if (aPlaying && !bPlaying) incomingDeck = "B";
            else if (!aPlaying && bPlaying) incomingDeck = "A";
            else incomingDeck = "B";
        }

        const masterBpm = engineState.master_bpm ?? 128;
        const durationMs = (duration / masterBpm) * 60 * 4 * 1000;

        setIsTransitioning(true);
        setTransitionDurationMs(durationMs);
        setErrorMsg(null);

        try {
            await executeTransition(incomingDeck, template, duration);
        } catch (err: unknown) {
            const msg = err instanceof Error ? err.message : String(err);
            setErrorMsg(msg || "Transition failed");
        } finally {
            setIsTransitioning(false);
        }
    };

    const templateDesc = TEMPLATE_INFO[template]?.desc || "";
    const selectedTemplateIsSimulated = TEMPLATE_INFO[template]?.simulated ?? false;

    return (
        <div className={`transitions-panel ${isTransitioning ? "transitions-panel--active" : ""}`}>
            <span className="transitions-panel-label">TRANSITION</span>
            <select
                value={template}
                onChange={e => setTemplate(e.target.value)}
                disabled={isTransitioning}
                className="transition-select"
            >
                {Object.entries(TEMPLATE_INFO).map(([key, info]) => (
                    <option key={key} value={key}>{info.label}</option>
                ))}
            </select>
            {templateDesc && (
                <span className="transition-desc">{templateDesc}</span>
            )}
            {selectedTemplateIsSimulated && (
                <span className="transition-sim-chip" title="This transition is simulated, not dedicated DSP">
                    SIMULATED
                </span>
            )}
            <div className="transition-duration-control">
                <span className="transition-duration-label">{duration}b</span>
                <input
                    type="range"
                    min={4} max={64} step={4}
                    value={duration}
                    disabled={isTransitioning}
                    onChange={e => setDuration(Number(e.target.value))}
                    className="transition-duration-slider"
                />
            </div>
            <button
                onClick={handleGo}
                disabled={isTransitioning}
                className={`transition-go-btn ${isTransitioning ? "active" : ""}`}
            >
                {isTransitioning ? (
                    <><span className="transition-spinner" /> MIX</>
                ) : "GO"}
            </button>
            {isTransitioning && (
                <div className="transition-progress-track">
                    <div className="transition-progress-bar" style={{ animationDuration: `${transitionDurationMs}ms` }} />
                </div>
            )}
            {errorMsg && <div className="transition-error-toast">{errorMsg}</div>}
        </div>
    );
}
