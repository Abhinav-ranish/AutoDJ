import { useEffect, useCallback } from "react";
import { useStore } from "./store";
import { open } from "@tauri-apps/plugin-dialog";

function formatTime(ms: number): string {
    const totalSeconds = Math.floor(ms / 1000);
    const minutes = Math.floor(totalSeconds / 60);
    const seconds = totalSeconds % 60;
    return `${minutes}:${seconds.toString().padStart(2, "0")}`;
}

// ─── Library View ────────────────────────────────────────────────

function LibraryView() {
    const { tracks, importing, fetchTracks, importFiles, loadTrack } = useStore();

    useEffect(() => {
        fetchTracks();
    }, [fetchTracks]);

    const handleImport = useCallback(async () => {
        const selected = await open({
            multiple: true,
            filters: [
                {
                    name: "Audio",
                    extensions: ["mp3", "wav", "flac", "aac", "ogg", "m4a", "opus"],
                },
            ],
        });
        if (selected) {
            const paths = Array.isArray(selected) ? selected : [selected];
            await importFiles(paths);
        }
    }, [importFiles]);

    return (
        <div className="library">
            <div className="library-toolbar">
                <button
                    className="import-btn"
                    onClick={handleImport}
                    disabled={importing}
                >
                    {importing ? "⟳ Importing..." : "＋ Import Files"}
                </button>
                <span className="track-count">{tracks.length} tracks</span>
            </div>

            <div className="track-list">
                <div className="track-list-header">
                    <span>Title</span>
                    <span>Artist</span>
                    <span>BPM</span>
                    <span>Key</span>
                    <span>Duration</span>
                    <span>Status</span>
                </div>
                {tracks.length === 0 ? (
                    <div className="empty-state">
                        <div className="empty-state-icon">🎵</div>
                        <div>No tracks yet — import some audio files to get started</div>
                    </div>
                ) : (
                    tracks.map((track) => (
                        <div className="track-row" key={track.id}>
                            <span className="track-title">
                                {track.title || "Unknown Title"}
                            </span>
                            <span className="track-artist">
                                {track.artist || "Unknown Artist"}
                            </span>
                            <span className="track-duration">—</span>
                            <span className="track-duration">—</span>
                            <span className="track-duration">
                                {track.duration_ms ? formatTime(track.duration_ms) : "—"}
                            </span>
                            <span className="load-actions">
                                <span
                                    className={`status-badge status-${track.import_status}`}
                                >
                                    {track.import_status === "ready" ? "✓" : "⟳"}{" "}
                                    {track.import_status}
                                </span>
                                {track.import_status === "ready" && (
                                    <>
                                        <button
                                            className="deck-load-btn deck-a"
                                            onClick={(e) => {
                                                e.stopPropagation();
                                                loadTrack("A", track.id);
                                            }}
                                        >
                                            A
                                        </button>
                                        <button
                                            className="deck-load-btn deck-b"
                                            onClick={(e) => {
                                                e.stopPropagation();
                                                loadTrack("B", track.id);
                                            }}
                                        >
                                            B
                                        </button>
                                    </>
                                )}
                            </span>
                        </div>
                    ))
                )}
            </div>
        </div>
    );
}

// ─── EQ Knob ─────────────────────────────────────────────────────

function EqKnob({
    label,
    value,
    onChange,
    onKill,
}: {
    label: string;
    value: number;
    onChange: (db: number) => void;
    onKill: () => void;
}) {
    const isKilled = value <= -90;
    return (
        <div className="eq-knob">
            <label>{label}</label>
            <input
                type="range"
                className="eq-slider"
                min={-96}
                max={6}
                step={0.5}
                value={value}
                onChange={(e) => onChange(Number(e.target.value))}
            />
            <span className="eq-value">{isKilled ? "OFF" : `${value.toFixed(0)}`}</span>
            <button
                className={`eq-kill ${isKilled ? "killed" : ""}`}
                onClick={onKill}
            >
                ✕
            </button>
        </div>
    );
}

// ─── Deck Panel ──────────────────────────────────────────────────

function DeckPanel({
    deckId,
    className,
}: {
    deckId: "A" | "B";
    className: string;
}) {
    const { engineState, play, pause, setDeckGain, setEq, setFilter, setLoop, clearLoop } = useStore();

    const snapshot =
        deckId === "A" ? engineState?.deck_a : engineState?.deck_b;
    const isPlaying = snapshot?.state === "playing";
    const isEmpty = !snapshot || snapshot.state === "empty";
    const loopActive = snapshot?.loop_active ?? false;

    return (
        <div className={`deck ${className}`}>
            <div className="deck-label">DECK {deckId}</div>

            <div className="waveform-placeholder">
                {isEmpty ? "No track loaded" : "Waveform"}
            </div>

            {!isEmpty && (
                <>
                    <div className="progress-bar">
                        <div
                            className="progress-fill"
                            style={{
                                width: `${(snapshot?.position_fraction ?? 0) * 100}%`,
                            }}
                        />
                    </div>

                    <div className="deck-info">
                        <span className="deck-track-title">
                            {isPlaying ? "▶ Playing" : "⏸ Paused"}
                        </span>
                        <span className="deck-time">
                            {formatTime(snapshot?.position_ms ?? 0)} /{" "}
                            {formatTime(snapshot?.duration_ms ?? 0)}
                        </span>
                    </div>

                    {/* ─── Mixer Controls ─── */}
                    <div className="mixer-controls">
                        {/* Gain Slider */}
                        <div className="gain-control">
                            <label>GAIN</label>
                            <input
                                type="range"
                                className="gain-slider"
                                min={-96}
                                max={12}
                                step={0.5}
                                value={snapshot?.gain_db ?? 0}
                                onChange={(e) =>
                                    setDeckGain(deckId, Number(e.target.value))
                                }
                            />
                            <span className="gain-value">
                                {(snapshot?.gain_db ?? 0) <= -90
                                    ? "-∞"
                                    : `${(snapshot?.gain_db ?? 0).toFixed(0)}`}
                            </span>
                        </div>

                        {/* EQ Knobs */}
                        <div className="eq-controls">
                            <EqKnob
                                label="HI"
                                value={snapshot?.eq_high ?? 0}
                                onChange={(db) => setEq(deckId, "high", db)}
                                onKill={() =>
                                    setEq(
                                        deckId,
                                        "high",
                                        (snapshot?.eq_high ?? 0) <= -90 ? 0 : -96
                                    )
                                }
                            />
                            <EqKnob
                                label="MID"
                                value={snapshot?.eq_mid ?? 0}
                                onChange={(db) => setEq(deckId, "mid", db)}
                                onKill={() =>
                                    setEq(
                                        deckId,
                                        "mid",
                                        (snapshot?.eq_mid ?? 0) <= -90 ? 0 : -96
                                    )
                                }
                            />
                            <EqKnob
                                label="LO"
                                value={snapshot?.eq_low ?? 0}
                                onChange={(db) => setEq(deckId, "low", db)}
                                onKill={() =>
                                    setEq(
                                        deckId,
                                        "low",
                                        (snapshot?.eq_low ?? 0) <= -90 ? 0 : -96
                                    )
                                }
                            />
                        </div>

                        {/* Filter Knob */}
                        <div className="filter-control">
                            <label>FILTER</label>
                            <input
                                type="range"
                                className="filter-slider"
                                min={0}
                                max={100}
                                step={1}
                                value={filterToSlider(snapshot?.filter_cutoff ?? 20000)}
                                onChange={(e) => {
                                    const hz = sliderToFilter(Number(e.target.value));
                                    setFilter(deckId, hz, snapshot?.filter_resonance ?? 0.7);
                                }}
                            />
                            <span className="filter-value">
                                {(snapshot?.filter_cutoff ?? 20000) >= 19500
                                    ? "OFF"
                                    : `${Math.round(snapshot?.filter_cutoff ?? 0)}Hz`}
                            </span>
                        </div>
                    </div>
                </>
            )}

            <div className="transport-controls">
                <button
                    className={`transport-btn ${isPlaying ? "playing" : ""}`}
                    onClick={() => (isPlaying ? pause(deckId) : play(deckId))}
                    disabled={isEmpty}
                >
                    {isPlaying ? "⏸" : "▶"}
                </button>

                {/* Loop controls */}
                {[4, 8, 16].map((bars) => (
                    <button
                        key={bars}
                        className={`transport-btn loop-btn ${loopActive ? "active-loop" : ""}`}
                        onClick={() =>
                            loopActive ? clearLoop(deckId) : setLoop(deckId, bars)
                        }
                        disabled={isEmpty}
                    >
                        {loopActive ? "✕" : `${bars}`}
                    </button>
                ))}
            </div>
        </div>
    );
}

// ─── Filter ↔ Slider Helpers ─────────────────────────────────────

/** Convert cutoff Hz (20–20000) to a 0–100 slider value (log scale) */
function filterToSlider(hz: number): number {
    const logMin = Math.log(20);
    const logMax = Math.log(20000);
    return ((Math.log(Math.max(20, hz)) - logMin) / (logMax - logMin)) * 100;
}

/** Convert 0–100 slider value to cutoff Hz (log scale) */
function sliderToFilter(val: number): number {
    const logMin = Math.log(20);
    const logMax = Math.log(20000);
    return Math.exp(logMin + (val / 100) * (logMax - logMin));
}

// ─── DJ View ─────────────────────────────────────────────────────

function DJView() {
    const { engineState, setCrossfader, pollEngineState } = useStore();

    // Poll engine state at ~30fps
    useEffect(() => {
        const interval = setInterval(pollEngineState, 33);
        return () => clearInterval(interval);
    }, [pollEngineState]);

    return (
        <div className="dj-view">
            <div className="decks-container">
                <DeckPanel deckId="A" className="deck-a-panel" />
                <DeckPanel deckId="B" className="deck-b-panel" />
            </div>

            <div className="crossfader-container">
                <div className="crossfader-labels">
                    <span>A</span>
                    <span>CROSSFADER</span>
                    <span>B</span>
                </div>
                <input
                    type="range"
                    className="crossfader-slider"
                    min={-100}
                    max={100}
                    value={(engineState?.crossfader ?? 0) * 100}
                    onChange={(e) => setCrossfader(Number(e.target.value) / 100)}
                />
            </div>
        </div>
    );
}

// ─── App ─────────────────────────────────────────────────────────

export default function App() {
    const { activeView, setActiveView, engineState } = useStore();

    return (
        <div className="app">
            <header className="header">
                <div className="logo">AutoDJ</div>
                <nav className="nav-tabs">
                    <button
                        className={`nav-tab ${activeView === "library" ? "active" : ""}`}
                        onClick={() => setActiveView("library")}
                    >
                        Library
                    </button>
                    <button
                        className={`nav-tab ${activeView === "dj" ? "active" : ""}`}
                        onClick={() => setActiveView("dj")}
                    >
                        DJ
                    </button>
                </nav>
                <div className="header-info">
                    <span className="bpm-display">
                        {engineState?.master_bpm?.toFixed(1) ?? "—"} BPM
                    </span>
                </div>
            </header>

            <main className="main-content">
                {activeView === "library" ? <LibraryView /> : <DJView />}
            </main>
        </div>
    );
}
