import { useEffect, useState } from "react";
import { useStore } from "../store";
import { DeckPanel } from "./DeckPanel";
import { MixerControls } from "./MixerChannel";
import { AiBar, TransitionsPanel } from "./AiBar";
import { AutoDjPanel } from "./AutoDjPanel";
import { IconRecord } from "./Icons";

export function DJView() {
    const { engineState, setCrossfader, isRecording, recordingStartTime, startRecording, stopRecording, mode } = useStore();

    const masterBpm = engineState?.master_bpm ?? 128;
    const snapA = engineState?.deck_a;
    const snapB = engineState?.deck_b;
    const aPlaying = snapA?.state === "playing";
    const bPlaying = snapB?.state === "playing";
    let activeDeck: "A" | "B" = (engineState?.crossfader ?? 0) > 0 ? "B" : "A";
    if (aPlaying && !bPlaying) activeDeck = "A";
    if (!aPlaying && bPlaying) activeDeck = "B";
    const crossfaderPercent = Math.round((engineState?.crossfader ?? 0) * 100);

    const [recordingElapsed, setRecordingElapsed] = useState(0);
    useEffect(() => {
        if (!isRecording || !recordingStartTime) return;
        const interval = setInterval(() => {
            setRecordingElapsed(Math.floor((Date.now() - recordingStartTime) / 1000));
        }, 1000);
        return () => clearInterval(interval);
    }, [isRecording, recordingStartTime]);

    const recMinutes = Math.floor(recordingElapsed / 60);
    const recSeconds = recordingElapsed % 60;

    return (
        <div className="dj-view">
            <AiBar />

            <div className="decks-container">
                <DeckPanel deckId="A" className="deck-a-panel" masterBpm={masterBpm} />

                <div className={`center-mixer center-mixer-active-${activeDeck.toLowerCase()}`}>
                    <div className="mixer-spine-header">
                        <span className="mixer-spine-label">MIXER</span>
                    </div>
                    <div className={`center-mixer-eqs ${mode === "autodj" ? "center-mixer-eqs-readonly" : ""}`}>
                        {snapA && snapA.state !== "empty" && (
                            <div className="center-mixer-section center-mixer-a">
                                <span className="center-mixer-label">A</span>
                                <MixerControls deckId="A" snapshot={snapA} mode={mode} />
                            </div>
                        )}
                        {snapB && snapB.state !== "empty" && (
                            <div className="center-mixer-section center-mixer-b">
                                <span className="center-mixer-label">B</span>
                                <MixerControls deckId="B" snapshot={snapB} mode={mode} />
                            </div>
                        )}
                    </div>

                    <div className="center-mixer-xfader">
                        <div className="center-mixer-master-readout">
                            <span className={`center-mixer-master-chip master-${activeDeck.toLowerCase()}`}>
                                MASTER {activeDeck}
                            </span>
                            <span className="center-mixer-master-value">
                                {crossfaderPercent > 0 ? `+${crossfaderPercent}` : crossfaderPercent}
                            </span>
                        </div>
                        <div className="xfader-slider-row">
                            <span className="xfader-label xfader-a">A</span>
                            <input
                                type="range"
                                className="crossfader-slider"
                                min={-100}
                                max={100}
                                value={(engineState?.crossfader ?? 0) * 100}
                                onChange={(e) => setCrossfader(Number(e.target.value) / 100)}
                            />
                            <span className="xfader-label xfader-b">B</span>
                        </div>
                    </div>

                    <button
                        className={`rec-button ${isRecording ? "rec-active" : ""}`}
                        onClick={isRecording ? stopRecording : startRecording}
                        title={isRecording ? "Stop Recording" : "Start Recording"}
                    >
                        <IconRecord />
                        {isRecording ? ` ${recMinutes}:${String(recSeconds).padStart(2, "0")}` : " REC"}
                    </button>
                </div>

                <DeckPanel deckId="B" className="deck-b-panel" masterBpm={masterBpm} />
            </div>

            {mode === "manual" && <TransitionsPanel />}
            {mode === "autodj" && <AutoDjPanel />}
        </div>
    );
}
