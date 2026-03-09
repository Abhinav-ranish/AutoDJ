import { useState, useEffect } from "react";
import { useStore, DeckSnapshot } from "../store";
import { invoke } from "@tauri-apps/api/core";
import { formatTime } from "./utils";
import { IconPlay, IconPause, IconX, IconBrake, IconSpinback, IconSlip, IconRepeat } from "./Icons";

export function ControlStrip({
    deckId,
    snapshot,
    isEmpty,
    isPlaying,
    hotCues,
    setHotCues,
    matchedTrack,
    masterBpm,
}: {
    deckId: "A" | "B";
    snapshot: DeckSnapshot | undefined;
    isEmpty: boolean;
    isPlaying: boolean;
    hotCues: Array<{ index: number; position_ms: number; label: string | null }>;
    setHotCues: React.Dispatch<React.SetStateAction<Array<{ index: number; position_ms: number; label: string | null }>>>;
    matchedTrack: any;
    masterBpm: number;
}) {
    const { play, pause, setLoop, clearLoop, beatRepeat, beatRepeatOff, setSlipMode, vinylBrake, spinback, seek } = useStore();
    const loopActive = snapshot?.loop_active ?? false;
    const [activeLoopBars, setActiveLoopBars] = useState<number | null>(null);

    useEffect(() => {
        if (!snapshot?.loop_active) {
            setActiveLoopBars(null);
        } else if (snapshot.loop_start_ms !== 0 && snapshot.loop_end_ms !== 0) {
            const derivedLoopBars = Math.round(
                (snapshot.loop_end_ms - snapshot.loop_start_ms) / ((60000 / masterBpm) * 4)
            );
            setActiveLoopBars(derivedLoopBars);
        }
    }, [snapshot?.loop_active, snapshot?.loop_start_ms, snapshot?.loop_end_ms, masterBpm]);

    const handleSetHotCue = async (index: number) => {
        if (!matchedTrack?.id || !snapshot) return;
        const posMs = snapshot.position_ms;
        await invoke("set_hot_cue", { trackId: matchedTrack.id, cueIndex: index, positionMs: posMs, label: null });
        setHotCues(prev => {
            const existing = prev.filter(c => c.index !== index);
            return [...existing, { index, position_ms: posMs, label: null }].sort((a, b) => a.index - b.index);
        });
    };

    const handleJumpHotCue = async (index: number) => {
        if (!matchedTrack?.id) return;
        await invoke("jump_to_hot_cue", { deck: deckId === "A" ? "a" : "b", trackId: matchedTrack.id, cueIndex: index });
    };

    const hotCueColors = ["#ff3b5c", "#00e676", "#448aff", "#ffea00"];

    return (
        <div className="control-strip">
            {/* ── Transport Row ── */}
            <div className="control-row transport-row">
                <button
                    className={`ctrl-btn ctrl-btn-lg play-btn ${isPlaying ? "active" : ""}`}
                    onClick={() => (isPlaying ? pause(deckId) : play(deckId))}
                    disabled={isEmpty}
                >
                    {isPlaying ? <IconPause size={18} /> : <IconPlay size={18} />}
                </button>

                <div className="control-divider" />

                {/* CUE buttons */}
                <div className="control-group">
                    <span className="control-group-label">CUE</span>
                    <div className="control-group-btns">
                        {[0, 1, 2, 3].map((idx) => {
                            const cue = hotCues.find(c => c.index === idx);
                            return (
                                <button
                                    key={idx}
                                    className={`ctrl-btn ctrl-btn-sm cue-btn ${cue ? "cue-set" : ""}`}
                                    style={cue ? { borderColor: hotCueColors[idx], color: hotCueColors[idx] } : {}}
                                    onClick={() => cue ? handleJumpHotCue(idx) : handleSetHotCue(idx)}
                                    disabled={isEmpty}
                                    title={cue ? `Jump to ${formatTime(cue.position_ms)}` : `Set cue ${idx + 1}`}
                                >
                                    {idx + 1}
                                </button>
                            );
                        })}
                    </div>
                </div>
            </div>

            {/* ── Loop Row ── */}
            <div className="control-row">
                <div className="control-group">
                    <span className="control-group-label">LOOP</span>
                    <div className="control-group-btns">
                        {[4, 8, 16].map((bars) => (
                            <button
                                key={bars}
                                className={`ctrl-btn ctrl-btn-sm loop-btn ${activeLoopBars === bars ? "loop-active" : ""}`}
                                onClick={() => { setActiveLoopBars(bars); setLoop(deckId, bars); }}
                                disabled={isEmpty}
                            >
                                {bars}
                            </button>
                        ))}
                        {loopActive && (
                            <button
                                className="ctrl-btn ctrl-btn-sm loop-off-btn"
                                onClick={() => { setActiveLoopBars(null); clearLoop(deckId); }}
                            >
                                <IconX size={10} />
                            </button>
                        )}
                    </div>
                </div>

                <div className="control-divider" />

                {/* Beat Repeat — real sample-accurate engine primitive */}
                <div className="control-group">
                    <span className="control-group-label">ROLL</span>
                    <div className="control-group-btns">
                        {[0.5, 1, 2, 4].map((beats) => (
                            <button
                                key={beats}
                                className="ctrl-btn ctrl-btn-sm fx-btn"
                                onClick={() => beatRepeat(deckId, beats)}
                                disabled={isEmpty}
                                title={`Repeat ${beats}b`}
                            >
                                <IconRepeat size={10} />
                                <span className="fx-btn-label">{beats}</span>
                            </button>
                        ))}
                        <button
                            className="ctrl-btn ctrl-btn-sm fx-off-btn"
                            onClick={() => beatRepeatOff(deckId)}
                            disabled={isEmpty}
                        >
                            <IconX size={10} />
                        </button>
                    </div>
                </div>
            </div>

            {/* ── Performance Row: Vinyl Brake, Spinback, Slip (all real engine primitives) ── */}
            <div className="control-row perf-row">
                <div className="control-group">
                    <span className="control-group-label">REAL</span>
                    <div className="control-group-btns">
                        <button className="ctrl-btn ctrl-btn-sm perf-btn" onClick={() => vinylBrake(deckId, 1000)} disabled={isEmpty} title="Vinyl Brake">
                            <IconBrake size={12} />
                        </button>
                        <button className="ctrl-btn ctrl-btn-sm perf-btn" onClick={() => spinback(deckId, 800)} disabled={isEmpty} title="Spinback">
                            <IconSpinback size={12} />
                        </button>
                        <button className="ctrl-btn ctrl-btn-sm perf-btn" onClick={() => setSlipMode(deckId, true)} disabled={isEmpty} title="Slip On">
                            <IconSlip size={12} />
                        </button>
                        <button className="ctrl-btn ctrl-btn-sm perf-btn" onClick={() => setSlipMode(deckId, false)} disabled={isEmpty} title="Slip Off">
                            <IconSlip size={12} />
                            <span className="perf-btn-off-mark" />
                        </button>
                    </div>
                </div>
            </div>

            <div className="control-truth-note">
                Real primitives: beat repeat, vinyl brake, spinback, slip.
            </div>
        </div>
    );
}
