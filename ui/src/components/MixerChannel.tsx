import { useState, useEffect, useCallback, useRef } from "react";
import { useStore, DeckSnapshot, DjMode } from "../store";
import { filterToSlider, sliderToFilter } from "./utils";

function RotaryKnob({
    label,
    value,
    min,
    max,
    onChange,
    onKill,
    isKilled,
}: {
    label: string;
    value: number;
    min: number;
    max: number;
    onChange: (v: number) => void;
    onKill?: () => void;
    isKilled?: boolean;
}) {
    const knobRef = useRef<HTMLDivElement>(null);
    const dragRef = useRef<{ startY: number; startValue: number } | null>(null);

    // Map value to rotation angle: -135deg (min) to +135deg (max)
    const range = max - min;
    const fraction = (value - min) / range;
    const angle = -135 + fraction * 270;

    const handleMouseDown = useCallback((e: React.MouseEvent) => {
        e.preventDefault();
        dragRef.current = { startY: e.clientY, startValue: value };

        const handleMouseMove = (ev: MouseEvent) => {
            if (!dragRef.current) return;
            const deltaY = dragRef.current.startY - ev.clientY;
            const sensitivity = range / 150; // 150px drag = full range
            const newValue = Math.max(min, Math.min(max, dragRef.current.startValue + deltaY * sensitivity));
            onChange(newValue);
        };

        const handleMouseUp = () => {
            dragRef.current = null;
            window.removeEventListener("mousemove", handleMouseMove);
            window.removeEventListener("mouseup", handleMouseUp);
        };

        window.addEventListener("mousemove", handleMouseMove);
        window.addEventListener("mouseup", handleMouseUp);
    }, [value, min, max, range, onChange]);

    const handleWheel = useCallback((e: React.WheelEvent) => {
        e.preventDefault();
        const step = range / 50;
        const newValue = Math.max(min, Math.min(max, value + (e.deltaY < 0 ? step : -step)));
        onChange(newValue);
    }, [value, min, max, range, onChange]);

    const displayValue = isKilled ? "OFF" : value.toFixed(0);

    return (
        <div className="rotary-knob-container">
            <label className="rotary-knob-label">{label}</label>
            <div
                ref={knobRef}
                className={`rotary-knob ${isKilled ? "rotary-knob-killed" : ""}`}
                onMouseDown={handleMouseDown}
                onWheel={handleWheel}
                title={`${label}: ${displayValue}`}
            >
                <div className="rotary-knob-body" style={{ transform: `rotate(${angle}deg)` }}>
                    <div className="rotary-knob-indicator" />
                </div>
            </div>
            <span className="rotary-knob-value">{displayValue}</span>
            {onKill && (
                <button className={`eq-kill ${isKilled ? "killed" : ""}`} onClick={onKill}>{"✕"}</button>
            )}
        </div>
    );
}

export function MixerControls({
    deckId,
    snapshot,
    mode,
}: {
    deckId: "A" | "B";
    snapshot: DeckSnapshot;
    mode: DjMode;
}) {
    const { setDeckGain, setEq, setFilter, setEqKill } = useStore();

    const [localGain, setLocalGain] = useState(snapshot?.gain_db ?? 0);
    const [localHi, setLocalHi] = useState(snapshot?.eq_high ?? 0);
    const [localMid, setLocalMid] = useState(snapshot?.eq_mid ?? 0);
    const [localLo, setLocalLo] = useState(snapshot?.eq_low ?? 0);
    const [localFilter, setLocalFilter] = useState(filterToSlider(snapshot?.filter_cutoff ?? 20000));

    useEffect(() => {
        setLocalGain(snapshot?.gain_db ?? 0);
        setLocalHi(snapshot?.eq_high ?? 0);
        setLocalMid(snapshot?.eq_mid ?? 0);
        setLocalLo(snapshot?.eq_low ?? 0);
        setLocalFilter(filterToSlider(snapshot?.filter_cutoff ?? 20000));
    }, [snapshot?.state, snapshot?.gain_db, snapshot?.eq_high, snapshot?.eq_mid, snapshot?.eq_low, snapshot?.filter_cutoff]);

    return (
        <div className="mixer-controls">
            <RotaryKnob
                label="GAIN"
                value={localGain}
                min={-12}
                max={12}
                onChange={(v) => {
                    setLocalGain(v);
                    setDeckGain(deckId, v);
                }}
            />

            <div className="eq-controls">
                <RotaryKnob
                    label="HI"
                    value={localHi}
                    min={-96}
                    max={12}
                    onChange={(db) => { setLocalHi(db); setEq(deckId, "high", db); }}
                    onKill={() => {
                        const isKilled = snapshot?.eq_kill_high ?? (localHi <= -90);
                        setEqKill(deckId, "high", !isKilled);
                        if (isKilled) { setLocalHi(0); setEq(deckId, "high", 0); }
                    }}
                    isKilled={snapshot?.eq_kill_high ?? false}
                />
                <RotaryKnob
                    label="MID"
                    value={localMid}
                    min={-96}
                    max={12}
                    onChange={(db) => { setLocalMid(db); setEq(deckId, "mid", db); }}
                    onKill={() => {
                        const isKilled = snapshot?.eq_kill_mid ?? (localMid <= -90);
                        setEqKill(deckId, "mid", !isKilled);
                        if (isKilled) { setLocalMid(0); setEq(deckId, "mid", 0); }
                    }}
                    isKilled={snapshot?.eq_kill_mid ?? false}
                />
                <RotaryKnob
                    label="LO"
                    value={localLo}
                    min={-96}
                    max={12}
                    onChange={(db) => { setLocalLo(db); setEq(deckId, "low", db); }}
                    onKill={() => {
                        const isKilled = snapshot?.eq_kill_low ?? (localLo <= -90);
                        setEqKill(deckId, "low", !isKilled);
                        if (isKilled) { setLocalLo(0); setEq(deckId, "low", 0); }
                    }}
                    isKilled={snapshot?.eq_kill_low ?? false}
                />
            </div>

            {mode === "manual" && (
                <RotaryKnob
                    label="FILTER"
                    value={localFilter}
                    min={0}
                    max={100}
                    onChange={(sv) => {
                        setLocalFilter(sv);
                        const hz = sliderToFilter(sv);
                        setFilter(deckId, hz, snapshot?.filter_resonance ?? 0.7);
                    }}
                />
            )}
        </div>
    );
}
