import { useState, useEffect } from "react";
import { useStore, DeckSnapshot, DjMode } from "../store";
import { filterToSlider, sliderToFilter } from "./utils";

function EqKnob({
    label,
    value,
    onChange,
    onKill,
    isKilled: isKilledProp,
}: {
    label: string;
    value: number;
    onChange: (db: number) => void;
    onKill: () => void;
    isKilled?: boolean;
}) {
    const isKilled = isKilledProp ?? value <= -90;
    return (
        <div className="eq-knob">
            <label>{label}</label>
            <input
                type="range"
                className="eq-slider"
                min={-96}
                max={12}
                step={0.5}
                value={value}
                onChange={(e) => onChange(Number(e.target.value))}
            />
            <span className="eq-value">{isKilled ? "OFF" : `${value.toFixed(0)}`}</span>
            <button className={`eq-kill ${isKilled ? "killed" : ""}`} onClick={onKill}>✕</button>
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
            <div className="gain-control">
                <label>GAIN</label>
                <input
                    type="range"
                    className="gain-slider"
                    min={-12}
                    max={12}
                    step={0.5}
                    value={localGain}
                    onChange={(e) => {
                        const v = Number(e.target.value);
                        setLocalGain(v);
                        setDeckGain(deckId, v);
                    }}
                />
                <span className="gain-value">
                    {localGain <= -11.5 ? "-∞" : `${localGain.toFixed(1)}`}
                </span>
            </div>

            <div className="eq-controls">
                <EqKnob
                    label="HI"
                    value={localHi}
                    onChange={(db) => { setLocalHi(db); setEq(deckId, "high", db); }}
                    onKill={() => {
                        const isKilled = snapshot?.eq_kill_high ?? (localHi <= -90);
                        setEqKill(deckId, "high", !isKilled);
                        if (isKilled) { setLocalHi(0); setEq(deckId, "high", 0); }
                    }}
                    isKilled={snapshot?.eq_kill_high ?? false}
                />
                <EqKnob
                    label="MID"
                    value={localMid}
                    onChange={(db) => { setLocalMid(db); setEq(deckId, "mid", db); }}
                    onKill={() => {
                        const isKilled = snapshot?.eq_kill_mid ?? (localMid <= -90);
                        setEqKill(deckId, "mid", !isKilled);
                        if (isKilled) { setLocalMid(0); setEq(deckId, "mid", 0); }
                    }}
                    isKilled={snapshot?.eq_kill_mid ?? false}
                />
                <EqKnob
                    label="LO"
                    value={localLo}
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
                <div className="filter-control">
                    <label>FILTER</label>
                    <input
                        type="range"
                        className="filter-slider"
                        min={0}
                        max={100}
                        step={1}
                        value={localFilter}
                        onChange={(e) => {
                            const sv = Number(e.target.value);
                            setLocalFilter(sv);
                            const hz = sliderToFilter(sv);
                            setFilter(deckId, hz, snapshot?.filter_resonance ?? 0.7);
                        }}
                    />
                    <span className="filter-value">
                        {sliderToFilter(localFilter) >= 19500 ? "OFF" : `${Math.round(sliderToFilter(localFilter))}Hz`}
                    </span>
                </div>
            )}
        </div>
    );
}
