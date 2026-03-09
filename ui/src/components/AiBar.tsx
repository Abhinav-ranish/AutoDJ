import { useEffect, useRef, useState, useCallback } from "react";
import { useStore, DjMode, AiDecision, EngineState, AutodjStatus, AiDebugInfo } from "../store";

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

export function AiBar() {
    const { mode, setMode, requestNextTrack, aiDecision, isThinking, tracks, loadTrack, engineState, clearAiDecision, autodjStatus, autodjMessage, setAutodjStatus } = useStore();

    const suggestedTrack = aiDecision?.track_selection
        ? tracks.find(t => t.id === aiDecision.track_selection)
        : null;

    // ─── Autonomous AutoDJ Controller ────────────────────────────
    const autodjPhaseRef = useRef<"idle" | "monitoring" | "planning" | "loading" | "aligning" | "transitioning">("idle");
    const pendingTrackRef = useRef<string | null>(null);
    const transitionLockRef = useRef(false);
    const cachedProfileRef = useRef<any>(null);
    const cachedProfileTrackIdRef = useRef<string | null>(null);

    useEffect(() => {
        if (mode !== "autodj") {
            autodjPhaseRef.current = "idle";
            pendingTrackRef.current = null;
            transitionLockRef.current = false;
            cachedProfileRef.current = null;
            cachedProfileTrackIdRef.current = null;
            setAutodjStatus("idle");
            return;
        }
        setAutodjStatus("starting", "Initializing AutoDJ...");
        autodjPhaseRef.current = "idle";
    }, [mode]);

    // Main AutoDJ tick
    useEffect(() => {
        if (mode !== "autodj") return;

        const tick = async () => {
            const state = useStore.getState();
            const es = state.engineState;
            if (!es) return;

            const deckA = es.deck_a;
            const deckB = es.deck_b;
            const aPlaying = deckA.state === "playing";
            const bPlaying = deckB.state === "playing";
            const analyzedTracks = state.tracks.filter(t => t.import_status === "analyzed");

            if (!aPlaying && !bPlaying) {
                if (autodjPhaseRef.current === "transitioning") return;
                if (analyzedTracks.length === 0) {
                    setAutodjStatus("error", "No analyzed tracks in library");
                    return;
                }
                if (autodjPhaseRef.current === "idle" || autodjPhaseRef.current === "monitoring") {
                    autodjPhaseRef.current = "planning";
                    setAutodjStatus("picking_first_track", "Selecting first track...");
                    const queueFirst = state.advanceQueue();
                    const pick = queueFirst
                        ? analyzedTracks.find(t => t.id === queueFirst) || analyzedTracks[0]
                        : analyzedTracks[Math.floor(Math.random() * analyzedTracks.length)];
                    await state.loadTrack("a", pick.id);
                    autodjPhaseRef.current = "monitoring";
                    setAutodjStatus("monitoring", `Playing: ${pick.title || "Unknown"}`);
                    return;
                }
                return;
            }

            const playingDeck: "a" | "b" = aPlaying ? "a" : "b";
            const idleDeck: "a" | "b" = playingDeck === "a" ? "b" : "a";
            const playingSnap = playingDeck === "a" ? deckA : deckB;
            const idleSnap = playingDeck === "a" ? deckB : deckA;
            const positionFrac = playingSnap.position_fraction;
            const positionMs = playingSnap.position_ms;
            const emergencyFraction = 0.92;

            const playingTrackId = playingSnap.track_id;
            if (playingTrackId && playingTrackId !== cachedProfileTrackIdRef.current) {
                try {
                    const profile = await state.getTrackProfile(playingTrackId);
                    cachedProfileRef.current = profile;
                    cachedProfileTrackIdRef.current = playingTrackId;
                } catch {
                    cachedProfileRef.current = null;
                }
            }

            const profile = cachedProfileRef.current;

            // Silence detection
            if (es.silence_frames > 90 && !transitionLockRef.current) {
                autodjPhaseRef.current = "planning";
                setAutodjStatus("error", "Silence detected — loading next track...");
                const others = analyzedTracks.filter(t => t.id !== playingSnap.track_id);
                if (others.length > 0) {
                    const pick = others[Math.floor(Math.random() * others.length)];
                    await state.loadTrack(idleDeck, pick.id);
                    await new Promise(r => setTimeout(r, 500));
                    transitionLockRef.current = true;
                    autodjPhaseRef.current = "transitioning";
                    try {
                        await state.executeTransition(idleDeck.toUpperCase(), "filter_sweep", 4);
                    } catch (e) {
                        console.error("Silence recovery transition failed:", e);
                    }
                    setTimeout(() => {
                        transitionLockRef.current = false;
                        autodjPhaseRef.current = "monitoring";
                        cachedProfileRef.current = null;
                        cachedProfileTrackIdRef.current = null;
                        state.clearAiDecision();
                        setAutodjStatus("monitoring", "Recovered from silence");
                    }, 3000);
                }
                return;
            }

            if (autodjPhaseRef.current !== "monitoring") {
                if (positionFrac > emergencyFraction && !transitionLockRef.current) {
                    if (idleSnap.state !== "empty" && idleSnap.state !== "playing") {
                        transitionLockRef.current = true;
                        autodjPhaseRef.current = "transitioning";
                        setAutodjStatus("transitioning", "Emergency transition!");
                        try {
                            await state.executeTransition(idleDeck.toUpperCase(), "filter_sweep", 8);
                        } catch (e) {
                            console.error("Emergency transition failed:", e);
                        }
                        setTimeout(() => {
                            transitionLockRef.current = false;
                            autodjPhaseRef.current = "monitoring";
                            pendingTrackRef.current = null;
                            cachedProfileRef.current = null;
                            cachedProfileTrackIdRef.current = null;
                            state.clearAiDecision();
                            setAutodjStatus("monitoring", "Monitoring playback...");
                        }, 5000);
                    }
                }
                return;
            }

            // Section-aware transition trigger
            let shouldTrigger = false;
            let triggerReason = "";

            if (profile?.transition_windows?.length > 0) {
                const futureWindows = (profile.transition_windows as Array<{anchor: string, start_ms: number, end_ms: number, score: number}>)
                    .filter((w) => w.start_ms > positionMs && w.start_ms - positionMs < 30_000)
                    .sort((a, b) => b.score - a.score);

                if (futureWindows.length > 0) {
                    const best = futureWindows[0];
                    if (best.start_ms - positionMs < 2000) {
                        shouldTrigger = true;
                        triggerReason = `window:${best.anchor}@${best.start_ms}ms`;
                    }
                } else if (positionFrac >= 0.75) {
                    shouldTrigger = true;
                    triggerReason = "fallback:75%";
                }
            } else {
                if (positionFrac >= 0.75) {
                    shouldTrigger = true;
                    triggerReason = "classic:75%";
                }
            }

            if (shouldTrigger && !state.isThinking && !transitionLockRef.current) {
                autodjPhaseRef.current = "planning";

                const queueTrackId = state.advanceQueue();
                if (queueTrackId) {
                    setAutodjStatus("selecting_next", `Queue: next track`);
                    pendingTrackRef.current = queueTrackId;
                } else {
                    setAutodjStatus("selecting_next", `AI selecting...`);
                    const decision = await state.requestNextTrack();
                    if (!decision || !decision.track_selection) {
                        if (analyzedTracks.length > 1) {
                            const others = analyzedTracks.filter(t => t.id !== playingSnap.track_id);
                            pendingTrackRef.current = others[Math.floor(Math.random() * others.length)].id;
                        } else if (analyzedTracks.length === 1) {
                            pendingTrackRef.current = analyzedTracks[0].id;
                        } else {
                            autodjPhaseRef.current = "monitoring";
                            setAutodjStatus("error", "No tracks to transition to");
                            return;
                        }
                    } else {
                        pendingTrackRef.current = decision.track_selection;
                    }
                }

                const nextTrackId = pendingTrackRef.current!;
                const nextTrack = state.tracks.find(t => t.id === nextTrackId);
                setAutodjStatus("loading_next", `Loading: ${nextTrack?.title || "next track"}...`);
                autodjPhaseRef.current = "loading";

                await state.loadTrack(idleDeck, nextTrackId);
                await new Promise(r => setTimeout(r, 300));

                // Phrase alignment
                if (profile?.phrases?.boundaries_ms?.length > 0) {
                    autodjPhaseRef.current = "aligning";
                    setAutodjStatus("transitioning", "Aligning to phrase boundary...");
                    const boundaries = profile.phrases.boundaries_ms as number[];
                    const curPos = useStore.getState().engineState
                        ? (playingDeck === "a" ? useStore.getState().engineState!.deck_a.position_ms : useStore.getState().engineState!.deck_b.position_ms)
                        : positionMs;
                    const nextBoundary = boundaries.find((b: number) => b > curPos);
                    if (nextBoundary && nextBoundary - curPos < 15_000) {
                        await new Promise<void>(resolve => {
                            const check = setInterval(() => {
                                const snap = useStore.getState().engineState;
                                if (!snap) { clearInterval(check); resolve(); return; }
                                const nowPos = playingDeck === "a" ? snap.deck_a.position_ms : snap.deck_b.position_ms;
                                if (nowPos >= nextBoundary - 100) { clearInterval(check); resolve(); }
                            }, 100);
                            setTimeout(() => resolve(), 15_000);
                        });
                    }
                }

                transitionLockRef.current = true;
                autodjPhaseRef.current = "transitioning";

                const plan = state.aiDecision?.transition_plan;
                const templateName = plan?.template_name || "smooth_beatmix";
                const overrides = plan?.parameter_overrides || {};
                const durationBars = overrides.duration_bars || 16;

                setAutodjStatus("transitioning", `${templateName} → ${nextTrack?.title || "Unknown"}`);

                try {
                    await state.executeTransition(idleDeck.toUpperCase(), templateName, durationBars);
                } catch (e) {
                    console.error("Transition failed:", e);
                }

                const bpm = es.master_bpm || 128;
                const transitionMs = (durationBars / bpm) * 60 * 4 * 1000;
                setTimeout(() => {
                    transitionLockRef.current = false;
                    autodjPhaseRef.current = "monitoring";
                    pendingTrackRef.current = null;
                    cachedProfileRef.current = null;
                    cachedProfileTrackIdRef.current = null;
                    useStore.getState().clearAiDecision();
                    const curEs = useStore.getState().engineState;
                    const nowPlayingId = idleDeck === "a" ? curEs?.deck_a?.track_id : curEs?.deck_b?.track_id;
                    const nowTrack = useStore.getState().tracks.find(t => t.id === nowPlayingId);
                    setAutodjStatus("monitoring", `Playing: ${nowTrack?.title || "Unknown"}`);
                }, Math.min(transitionMs + 1000, 60000));
            }
        };

        const interval = setInterval(tick, 500);
        tick();
        return () => clearInterval(interval);
    }, [mode]);

    const getModeLabel = (m: DjMode) => {
        switch (m) {
            case "manual": return "Manual";
            case "assisted": return "Assisted";
            case "autodj": return "Auto-DJ";
        }
    };

    const activeDeck = engineState?.deck_a?.state === "playing" ? "A" : "B";
    const currentTrackId = activeDeck === "A" ? engineState?.deck_a?.track_id : engineState?.deck_b?.track_id;
    const currentTrack = tracks.find(t => t.id === currentTrackId);
    const currentTrackName = currentTrack?.title || "Nothing playing";
    const displayBpm = currentTrack?.bpm?.toFixed(1) || engineState?.master_bpm?.toFixed(1) || "128.0";

    return (
        <div className="ai-bar">
            {/* Mode Switcher */}
            <div className="ai-bar-modes">
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

            {/* Now Playing */}
            <div className="ai-bar-now">
                <span className="ai-bar-label">NOW</span>
                <span className="ai-bar-track">{currentTrackName}</span>
                <span className="ai-bar-bpm">{displayBpm}</span>
            </div>

            {/* Spectrum */}
            <div className="ai-bar-spectrum">
                <SpectrumAnalyzer spectrum={engineState?.spectrum ?? []} />
            </div>

            {/* Up Next / AI Suggestion */}
            <div className="ai-bar-next">
                {suggestedTrack ? (
                    <>
                        <span className="ai-bar-label ai-bar-label-next">NEXT</span>
                        <span className="ai-bar-track">{suggestedTrack.title || "Unknown"}</span>
                        <span className="ai-bar-bpm">{suggestedTrack.bpm?.toFixed(1)}</span>
                        <button className="ai-bar-queue-btn" onClick={() => loadTrack("b", suggestedTrack.id)}>
                            Queue →
                        </button>
                    </>
                ) : (
                    <span className="ai-bar-empty">
                        {isThinking ? "AI thinking..." : "No suggestion"}
                    </span>
                )}
            </div>

            {/* Action */}
            <div className="ai-bar-action">
                {mode === "autodj" ? (
                    <div className="ai-bar-status">
                        <div className={`status-dot ${autodjStatus === "transitioning" ? "transitioning" : autodjStatus === "error" ? "error" : "active"}`} />
                        <span>{autodjMessage || "AI is DJing"}</span>
                    </div>
                ) : (
                    <button
                        onClick={requestNextTrack}
                        disabled={isThinking}
                        className={`ai-bar-find-btn ${isThinking ? "thinking" : ""}`}
                    >
                        {isThinking ? "..." : "Find Next"}
                    </button>
                )}
            </div>
        </div>
    );
}

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

    return (
        <div className={`transitions-panel ${isTransitioning ? "transitions-panel--active" : ""}`}>
            <span className="transitions-panel-label">TRANSITION</span>
            <select
                value={template}
                onChange={e => setTemplate(e.target.value)}
                disabled={isTransitioning}
                className="transition-select"
            >
                <option value="smooth_beatmix">Smooth Beatmix</option>
                <option value="filter_sweep">Filter Sweep</option>
                <option value="echo_drop">Echo Drop</option>
                <option value="drop_swap">Drop Swap</option>
                <option value="loop_roll">Loop Roll</option>
                <option value="bass_kill_swap">Bass Kill Swap</option>
                <option value="break_blend">Break Blend</option>
            </select>
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
            {errorMsg && <div className="transition-error-toast">⚠ {errorMsg}</div>}
        </div>
    );
}
