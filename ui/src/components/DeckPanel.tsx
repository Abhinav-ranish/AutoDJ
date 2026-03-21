import { useDeckState } from "../hooks/useDeckState";
import { formatTime, formatGroove } from "./utils";
import { WaveformDisplay } from "./WaveformDisplay";
import { TempoStrip } from "./TempoStrip";
import { ControlStrip } from "./ControlStrip";
import { VinylDisplay } from "./VinylDisplay";
import { IconPlay, IconPause } from "./Icons";

export function DeckPanel({
    deckId,
    className,
    masterBpm,
}: {
    deckId: "A" | "B";
    className: string;
    masterBpm: number;
}) {
    const {
        snapshot,
        matchedTrack,
        albumArtUrl,
        setAlbumArtUrl,
        hotCues,
        setHotCues,
        trackProfile,
        isPlaying,
        isEmpty,
        isActiveDeck,
        deckKey,
        trackTitle,
        trackArtist,
        trackBpm,
        trackKey,
        trackGenre,
        trackGroove,
        displayBpm,
        transitionWindows,
        mode,
        engineState,
        store,
    } = useDeckState(deckId);

    const durationMs = snapshot?.duration_ms ?? 0;
    const positionMs = snapshot?.position_ms ?? 0;
    const activeWindow = transitionWindows.find((w) => positionMs >= w.start_ms && positionMs <= w.end_ms) || null;
    const nextWindow = transitionWindows.find((w) => w.start_ms > positionMs) || null;

    const windowStatus = activeWindow
        ? `MIX WINDOW LIVE \u00B7 ${activeWindow.anchor.replace(/_/g, " ")}`
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

            {/* ── Vinyl + Track Info Row ── */}
            <div className="deck-vinyl-row">
                <VinylDisplay
                    albumArtUrl={albumArtUrl}
                    isPlaying={isPlaying}
                    deckId={deckId}
                />
                <div className="deck-track-info deck-track-info-beside-vinyl">
                    <div className="deck-track-text">
                        <span className="deck-track-title">{trackTitle}</span>
                        {trackArtist && <span className="deck-track-artist">{trackArtist}</span>}
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
                        {/* Mix point markers */}
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

            {/* ── Performance Controls (beat repeat, vinyl brake, spinback, slip, echo) ── */}
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
