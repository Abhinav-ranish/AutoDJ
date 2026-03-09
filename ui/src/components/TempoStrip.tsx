import { useState, useEffect } from "react";
import { useStore, DeckSnapshot, EngineState, TrackRecord } from "../store";
import { formatGroove } from "./utils";
import { IconSync, IconReset } from "./Icons";

export function TempoStrip({
    deckId,
    snapshot,
    engineState,
    masterBpm,
    matchedTrack,
    trackProfile,
}: {
    deckId: "A" | "B";
    snapshot: DeckSnapshot;
    engineState: EngineState;
    masterBpm: number;
    matchedTrack: TrackRecord | null;
    trackProfile: any;
}) {
    const { setDeckSpeed, tracks } = useStore();

    const [pitchRange, setPitchRange] = useState(0.08);

    const getOriginalBpm = (snap: any) => {
        if (!snap || snap.state === "empty" || !snap.track_id) return masterBpm;
        const track = tracks.find(t => t.id === snap.track_id);
        if (track && track.bpm) return track.bpm;
        return masterBpm;
    };

    const originalBpm = getOriginalBpm(snapshot);
    const otherSnapshot = deckId === "A" ? engineState.deck_b : engineState.deck_a;
    const otherOriginalBpm = getOriginalBpm(otherSnapshot);
    const otherCurrentBpm = otherOriginalBpm * (otherSnapshot?.speed ?? 1.0);

    const [localBpm, setLocalBpm] = useState(masterBpm);

    useEffect(() => {
        setLocalBpm(originalBpm);
    }, [originalBpm]);

    const percentChange = originalBpm > 0 ? (((localBpm - originalBpm) / originalBpm) * 100) : 0;

    return (
        <div className="tempo-strip">
            <div className="tempo-strip-header">
                <span className="tempo-strip-label">TEMPO</span>
                <select
                    value={pitchRange}
                    onChange={(e) => setPitchRange(Number(e.target.value))}
                    className="tempo-range-select"
                >
                    <option value={0.06}>±6%</option>
                    <option value={0.08}>±8%</option>
                    <option value={0.10}>±10%</option>
                    <option value={0.16}>±16%</option>
                </select>
            </div>

            <div className="tempo-slider-row">
                <button
                    className="tempo-nudge-btn"
                    onClick={() => {
                        const next = Math.max(originalBpm * (1 - pitchRange), localBpm - 2);
                        setLocalBpm(next);
                        setDeckSpeed(deckId, next / originalBpm);
                    }}
                    title="Nudge −2 BPM"
                >−</button>

                <input
                    type="range"
                    className="tempo-slider"
                    min={originalBpm * (1 - pitchRange)}
                    max={originalBpm * (1 + pitchRange)}
                    step={0.1}
                    value={localBpm}
                    onChange={(e) => {
                        const v = Number(e.target.value);
                        setLocalBpm(v);
                        setDeckSpeed(deckId, v / originalBpm);
                    }}
                />

                <button
                    className="tempo-nudge-btn"
                    onClick={() => {
                        const next = Math.min(originalBpm * (1 + pitchRange), localBpm + 2);
                        setLocalBpm(next);
                        setDeckSpeed(deckId, next / originalBpm);
                    }}
                    title="Nudge +2 BPM"
                >+</button>
            </div>

            <div className="tempo-info-row">
                <span className={`tempo-percent ${percentChange !== 0 ? "active" : ""}`}>
                    {percentChange > 0 ? "+" : ""}{percentChange.toFixed(1)}%
                </span>
                <div className="tempo-actions">
                    <button
                        className={`sync-btn sync-btn-${deckId.toLowerCase()}`}
                        onClick={() => {
                            setLocalBpm(otherCurrentBpm);
                            setDeckSpeed(deckId, otherCurrentBpm / originalBpm);
                        }}
                    >
                        <IconSync size={12} /> SYNC
                    </button>
                    <button
                        className="tempo-reset-btn"
                        onClick={() => {
                            setLocalBpm(originalBpm);
                            setDeckSpeed(deckId, 1);
                        }}
                        title="Reset tempo"
                    >
                        <IconReset size={12} />
                    </button>
                </div>
            </div>
        </div>
    );
}
