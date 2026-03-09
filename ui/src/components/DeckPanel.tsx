import { useEffect, useMemo } from "react";
import { useStore } from "../store";
import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import { formatTime, formatGroove } from "./utils";
import { WaveformDisplay } from "./WaveformDisplay";
import { TempoStrip } from "./TempoStrip";
import { ControlStrip } from "./ControlStrip";
import { IconPlay, IconPause } from "./Icons";
import { useState } from "react";

export function DeckPanel({
    deckId,
    className,
    masterBpm,
}: {
    deckId: "A" | "B";
    className: string;
    masterBpm: number;
}) {
    const store = useStore();
    const { engineState, tracks, mode, getTrackProfile } = store;

    const snapshot = deckId === "A" ? engineState?.deck_a : engineState?.deck_b;
    const isPlaying = snapshot?.state === "playing";
    const isEmpty = !snapshot || snapshot.state === "empty";
    const deckKey = deckId.toLowerCase() as "a" | "b";

    const aPlaying = engineState?.deck_a?.state === "playing";
    const bPlaying = engineState?.deck_b?.state === "playing";
    let activeDeckKey: "a" | "b" = engineState && engineState.crossfader > 0 ? "b" : "a";
    if (aPlaying && !bPlaying) activeDeckKey = "a";
    if (!aPlaying && bPlaying) activeDeckKey = "b";
    const isActiveDeck = !isEmpty && activeDeckKey === deckKey;

    // ── Album Art + Hot Cues ──
    const [albumArtUrl, setAlbumArtUrl] = useState<string | null>(null);
    const [hotCues, setHotCues] = useState<Array<{ index: number; position_ms: number; label: string | null }>>([]);

    // ── Waveform Profile ──
    const [trackProfile, setTrackProfile] = useState<any>(null);

    const matchedTrack = useMemo(() => {
        if (!snapshot || snapshot.state === "empty") return null;
        const snapDur = snapshot.duration_ms;
        if (snapDur <= 0) return null;
        let best: any = null;
        let bestDiff = Infinity;
        for (const t of tracks) {
            if (t.duration_ms != null) {
                const diff = Math.abs(t.duration_ms - snapDur);
                if (diff < bestDiff && diff < 500) {
                    bestDiff = diff;
                    best = t;
                }
            }
        }
        return best;
    }, [snapshot?.duration_ms, snapshot?.state, tracks]);

    useEffect(() => {
        if (matchedTrack?.id) {
            getTrackProfile(matchedTrack.id).then(setTrackProfile);
            invoke<string | null>("get_album_art", { trackId: matchedTrack.id }).then(path => {
                if (path) setAlbumArtUrl(convertFileSrc(path));
                else setAlbumArtUrl(null);
            }).catch(() => setAlbumArtUrl(null));
            invoke<Array<[number, number, string | null]>>("get_hot_cues", { trackId: matchedTrack.id }).then(cues => {
                setHotCues(cues.map(([index, position_ms, label]) => ({ index, position_ms, label })));
            }).catch(() => setHotCues([]));
        } else {
            setTrackProfile(null);
            setAlbumArtUrl(null);
            setHotCues([]);
        }
    }, [matchedTrack?.id, getTrackProfile]);

    // ── Derive track info ──
    const trackTitle = matchedTrack?.title || (isEmpty ? "No Track Loaded" : "Unknown Track");
    const trackArtist = matchedTrack?.artist || "";
    const trackKey = matchedTrack?.key || "";
    const trackGenre = matchedTrack?.genre || "";
    const trackGroove = matchedTrack?.groove_type;
    const trackBpm = matchedTrack?.bpm;
    const displayBpm = trackBpm?.toFixed(1) || masterBpm.toFixed(1);

    // ── Transition windows (marker-aware mixing) ──
    // This is the strongest real feature per reality_audit.md #10.
    // Show mix point markers on the progress bar when available.
    const transitionWindowsRaw = (trackProfile?.transition_windows ?? []) as Array<{
        anchor: string; start_ms: number; end_ms: number; score: number;
    }>;
    const transitionWindows = [...transitionWindowsRaw].sort((a, b) => a.start_ms - b.start_ms);
    const durationMs = snapshot?.duration_ms ?? 0;
    const positionMs = snapshot?.position_ms ?? 0;
    const activeWindow = transitionWindows.find((w) => positionMs >= w.start_ms && positionMs <= w.end_ms) || null;
    const nextWindow = transitionWindows.find((w) => w.start_ms > positionMs) || null;

    const windowStatus = activeWindow
        ? `MIX WINDOW LIVE · ${activeWindow.anchor.replace(/_/g, " ")}`
        : nextWindow
            ? `NEXT WINDOW ${nextWindow.anchor.replace(/_/g, " ")} IN ${formatTime(nextWindow.start_ms - positionMs)}`
            : null;

    return (
        <div className={`deck ${className} ${isActiveDeck ? "deck-active" : "deck-standby"} ${isEmpty ? "deck-empty" : ""}`}>
            {/* ── Deck Header ── */}
            <div className="deck-header">
                <div className="deck-header-left">
                    <span className="deck-label">DECK {deckId}</span>
                    {!isEmpty && (
                        <span className={`deck-role-chip ${isActiveDeck ? "active" : "standby"}`}>
                            {isActiveDeck ? "ON AIR" : "STANDBY"}
                        </span>
                    )}
                    {!isEmpty && (
                        <span className={`deck-state-badge ${isPlaying ? "playing" : "paused"}`}>
                            {isPlaying ? <IconPlay size={10} /> : <IconPause size={10} />}
                            {isPlaying ? "LIVE" : "PAUSED"}
                        </span>
                    )}
                </div>
                {!isEmpty && (
                    <span className="deck-bpm-readout">{displayBpm}</span>
                )}
            </div>

            {/* ── Track Info ── */}
            <div className="deck-track-info">
                <div className="deck-track-info-left">
                    {albumArtUrl ? (
                        <img
                            src={albumArtUrl}
                            alt=""
                            className="deck-album-art"
                            onError={() => setAlbumArtUrl(null)}
                        />
                    ) : !isEmpty ? (
                        <div className="deck-album-art deck-album-art-placeholder">
                            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                                <circle cx="12" cy="12" r="10" />
                                <circle cx="12" cy="12" r="3" />
                            </svg>
                        </div>
                    ) : null}
                    <div className="deck-track-text">
                        <span className="deck-track-title">{trackTitle}</span>
                        {trackArtist && <span className="deck-track-artist">{trackArtist}</span>}
                    </div>
                </div>
                {!isEmpty && (
                    <div className="deck-track-meta">
                        {trackKey && <span className="meta-tag">{trackKey}</span>}
                        {trackGenre && <span className="meta-tag meta-tag-genre">{trackGenre}</span>}
                        {trackGroove && trackGroove !== "standard" && (
                            <span className="meta-tag meta-tag-groove">{formatGroove(trackGroove)}</span>
                        )}
                    </div>
                )}
            </div>

            {/* ── Waveform ── */}
            <WaveformDisplay
                deckId={deckId}
                snapshot={snapshot}
                trackProfile={trackProfile}
                isEmpty={isEmpty}
            />

            {/* ── Time / Progress ── */}
            {!isEmpty && (
                <div className="deck-progress-section">
                    <div className="deck-progress-bar" onClick={(e) => {
                        if (!snapshot) return;
                        const rect = e.currentTarget.getBoundingClientRect();
                        const fraction = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
                        store.seek(deckId, fraction * snapshot.duration_ms);
                    }}>
                        <div className="deck-progress-fill" style={{ width: `${(snapshot?.position_fraction ?? 0) * 100}%` }} />
                        {/* Mix point markers — marker-aware mixing visualization */}
                        {durationMs > 0 && transitionWindows.map((w, i) => (
                            <div
                                key={i}
                                className="deck-mix-point"
                                style={{ left: `${(w.start_ms / durationMs) * 100}%` }}
                                title={`Mix point: ${w.anchor.replace(/_/g, " ")} (score: ${w.score.toFixed(2)})`}
                            />
                        ))}
                    </div>
                    <div className="deck-time-row">
                        <span className="deck-time-elapsed">{formatTime(snapshot?.position_ms ?? 0)}</span>
                        {transitionWindows.length > 0 && (
                            <span className="deck-mix-point-label" title="Marker-aware mix points detected">
                                {transitionWindows.length} mix {transitionWindows.length === 1 ? "point" : "points"}
                            </span>
                        )}
                        <span className="deck-time-remaining">-{formatTime((snapshot?.duration_ms ?? 0) - (snapshot?.position_ms ?? 0))}</span>
                    </div>
                    {windowStatus && (
                        <div className={`deck-window-status ${activeWindow ? "live" : "queued"}`}>
                            {windowStatus}
                        </div>
                    )}
                </div>
            )}

            {/* ── Tempo Strip ── */}
            {!isEmpty && mode !== "autodj" && (
                <TempoStrip
                    deckId={deckId}
                    snapshot={snapshot!}
                    engineState={engineState!}
                    masterBpm={masterBpm}
                    matchedTrack={matchedTrack}
                    trackProfile={trackProfile}
                />
            )}

            {/* ── Transport + Performance Controls ── */}
            {mode !== "autodj" && (
                <ControlStrip
                    deckId={deckId}
                    snapshot={snapshot}
                    isEmpty={isEmpty}
                    isPlaying={isPlaying}
                    hotCues={hotCues}
                    setHotCues={setHotCues}
                    matchedTrack={matchedTrack}
                    masterBpm={masterBpm}
                />
            )}
        </div>
    );
}
