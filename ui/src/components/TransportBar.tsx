import { useState, useEffect } from "react";
import { useStore } from "../store";
import { invoke } from "@tauri-apps/api/core";
import { formatTime } from "./utils";
import { IconPlay, IconPause, IconX, IconRepeat } from "./Icons";

export function TransportBar() {
    const {
        engineState,
        play,
        pause,
        setLoop,
        clearLoop,
        seek,
        mode,
    } = useStore();

    const snapA = engineState?.deck_a;
    const snapB = engineState?.deck_b;
    const aPlaying = snapA?.state === "playing";
    const bPlaying = snapB?.state === "playing";
    const aEmpty = !snapA || snapA.state === "empty";
    const bEmpty = !snapB || snapB.state === "empty";
    const aLoopActive = snapA?.loop_active ?? false;
    const bLoopActive = snapB?.loop_active ?? false;

    return (
        <div className="transport-bar">
            {/* Deck A transport */}
            <div className="transport-bar-deck">
                <span className="transport-bar-deck-label transport-bar-label-a">A</span>
                <button
                    className={`ctrl-btn ctrl-btn-sm play-btn ${aPlaying ? "active" : ""}`}
                    onClick={() => (aPlaying ? pause("A") : play("A"))}
                    disabled={aEmpty}
                >
                    {aPlaying ? <IconPause size={14} /> : <IconPlay size={14} />}
                </button>
                <div className="transport-bar-loops">
                    {[4, 8, 16].map((bars) => (
                        <button
                            key={bars}
                            className={`ctrl-btn ctrl-btn-xs loop-btn ${aLoopActive && snapA && Math.round((snapA.loop_end_ms - snapA.loop_start_ms) / ((60000 / (engineState?.master_bpm ?? 128)) * 4)) === bars ? "loop-active" : ""}`}
                            onClick={() => setLoop("A", bars)}
                            disabled={aEmpty}
                        >
                            {bars}
                        </button>
                    ))}
                    {aLoopActive && (
                        <button className="ctrl-btn ctrl-btn-xs loop-off-btn" onClick={() => clearLoop("A")}>
                            <IconX size={8} />
                        </button>
                    )}
                </div>
            </div>

            {/* Center — seek / general controls */}
            <div className="transport-bar-center">
                <button
                    className="ctrl-btn ctrl-btn-sm"
                    onClick={() => { if (snapA && !aEmpty) seek("A", Math.max(0, snapA.position_ms - 5000)); }}
                    disabled={aEmpty}
                    title="Seek A -5s"
                >
                    {"◁◁"}
                </button>
                <button
                    className="ctrl-btn ctrl-btn-sm"
                    onClick={() => { if (snapA && !aEmpty) seek("A", snapA.position_ms + 5000); }}
                    disabled={aEmpty}
                    title="Seek A +5s"
                >
                    {"▷▷"}
                </button>
                <span className="transport-bar-divider" />
                <button
                    className="ctrl-btn ctrl-btn-sm"
                    onClick={() => { if (snapB && !bEmpty) seek("B", Math.max(0, snapB.position_ms - 5000)); }}
                    disabled={bEmpty}
                    title="Seek B -5s"
                >
                    {"◁◁"}
                </button>
                <button
                    className="ctrl-btn ctrl-btn-sm"
                    onClick={() => { if (snapB && !bEmpty) seek("B", snapB.position_ms + 5000); }}
                    disabled={bEmpty}
                    title="Seek B +5s"
                >
                    {"▷▷"}
                </button>
            </div>

            {/* Deck B transport */}
            <div className="transport-bar-deck">
                <div className="transport-bar-loops">
                    {[4, 8, 16].map((bars) => (
                        <button
                            key={bars}
                            className={`ctrl-btn ctrl-btn-xs loop-btn ${bLoopActive && snapB && Math.round((snapB.loop_end_ms - snapB.loop_start_ms) / ((60000 / (engineState?.master_bpm ?? 128)) * 4)) === bars ? "loop-active" : ""}`}
                            onClick={() => setLoop("B", bars)}
                            disabled={bEmpty}
                        >
                            {bars}
                        </button>
                    ))}
                    {bLoopActive && (
                        <button className="ctrl-btn ctrl-btn-xs loop-off-btn" onClick={() => clearLoop("B")}>
                            <IconX size={8} />
                        </button>
                    )}
                </div>
                <button
                    className={`ctrl-btn ctrl-btn-sm play-btn ${bPlaying ? "active" : ""}`}
                    onClick={() => (bPlaying ? pause("B") : play("B"))}
                    disabled={bEmpty}
                >
                    {bPlaying ? <IconPause size={14} /> : <IconPlay size={14} />}
                </button>
                <span className="transport-bar-deck-label transport-bar-label-b">B</span>
            </div>
        </div>
    );
}
