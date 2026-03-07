import { useEffect, useCallback, useState, useRef, useMemo } from "react";
import { useStore, EngineState, DjMode } from "./store";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import logo from "./assets/logo.png";

function formatTime(ms: number): string {
    const totalSeconds = Math.floor(ms / 1000);
    const minutes = Math.floor(totalSeconds / 60);
    const seconds = totalSeconds % 60;
    return `${minutes}:${seconds.toString().padStart(2, "0")}`;
}

// ─── Library View ────────────────────────────────────────────────

function LibraryView() {
    const { tracks, importing, fetchTracks, importFiles, loadTrack, clearLibrary, analyzingTracks } = useStore();

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

    const handleClear = useCallback(async () => {
        if (confirm("Are you sure you want to clear your entire library? This will delete all tracks from the database and clear the cache.")) {
            await clearLibrary();
        }
    }, [clearLibrary]);

    return (
        <div className="library">
            <div className="library-toolbar">
                <div style={{ display: "flex", gap: "10px" }}>
                    <button
                        className="import-btn"
                        onClick={handleImport}
                        disabled={importing}
                        style={{ backgroundColor: "#4CAF50", color: "white", border: "none" }}
                    >
                        {importing ? "⟳ Importing..." : "＋ Import Files"}
                    </button>
                    <button
                        onClick={handleClear}
                        disabled={importing || tracks.length === 0}
                        style={{
                            padding: "8px 16px",
                            backgroundColor: "#f44336",
                            color: "white",
                            border: "none",
                            borderRadius: "4px",
                            cursor: importing || tracks.length === 0 ? "not-allowed" : "pointer",
                            opacity: importing || tracks.length === 0 ? 0.5 : 1
                        }}
                    >
                        🗑 Clear Library
                    </button>
                </div>
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
                    tracks.map((track) => {
                        const isAnalyzing = analyzingTracks.has(track.id);
                        return (
                            <div className="track-row" key={track.id}>
                                <span className="track-title">
                                    {track.title || "Unknown Title"}
                                </span>
                                <span className="track-artist">
                                    {track.artist || "Unknown Artist"}
                                </span>
                                <span className="track-duration">
                                    {track.bpm ? track.bpm.toFixed(1) : "—"}
                                </span>
                                <span className="track-duration">
                                    {track.key || "—"}
                                </span>
                                <span className="track-duration">
                                    {track.duration_ms ? formatTime(track.duration_ms) : "—"}
                                </span>
                                <span className="load-actions">
                                    <span
                                        className={`status-badge status-${track.import_status}`}
                                    >
                                        {track.import_status === "analyzed" ? "✓ Analyzed"
                                            : track.import_status === "error" || track.import_status === "analysis_failed" ? "✕ Error"
                                                : track.import_status === "ready" ? "✓ Uploaded"
                                                    : "⟳ Loading..."}
                                    </span>
                                    {isAnalyzing && (
                                        <span className="status-badge" style={{ marginLeft: "4px", color: "#00d4ff", background: "#00d4ff20" }}>
                                            ⟳ Analyzing...
                                        </span>
                                    )}
                                    {(track.import_status === "ready" || track.import_status === "analyzed") && (
                                        <>
                                            <button
                                                className="deck-load-btn deck-a"
                                                onClick={(e) => {
                                                    e.stopPropagation();
                                                    loadTrack("a", track.id);
                                                }}
                                            >
                                                A
                                            </button>
                                            <button
                                                className="deck-load-btn deck-b"
                                                onClick={(e) => {
                                                    e.stopPropagation();
                                                    loadTrack("b", track.id);
                                                }}
                                            >
                                                B
                                            </button>
                                        </>
                                    )}
                                </span>
                            </div>
                        );
                    })
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
                max={12}
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

// ─── Waveform Drawing ─────────────────────────────────────────────

function drawWaveform(
    ctx: CanvasRenderingContext2D,
    width: number,
    height: number,
    profile: any,
    positionFraction: number,
    accentColor: string,
    bgColor: string
) {
    ctx.clearRect(0, 0, width, height);

    // Background
    ctx.fillStyle = bgColor;
    ctx.fillRect(0, 0, width, height);

    // Get waveform data
    let data: number[] = [];
    if (profile) {
        // Priority fields based on backend common names
        const keys = ["rms_db_per_beat", "energy_per_bar", "energy_per_beat", "rms"];
        for (const k of keys) {
            if (Array.isArray(profile[k]) && profile[k].length > 0) {
                data = profile[k];
                break;
            }
        }
        // Fallback: search for any array of numbers
        if (data.length === 0) {
            for (const val of Object.values(profile)) {
                if (Array.isArray(val) && val.length > 0 && typeof val[0] === 'number') {
                    data = val as number[];
                    break;
                }
            }
        }
    }

    if (data.length > 0) {
        const barGap = 1;
        const barWidth = Math.max(1, width / data.length);
        const maxVal = Math.max(...data.map(Math.abs).filter(v => !isNaN(v))) || 1;

        for (let i = 0; i < data.length; i++) {
            const val = Math.abs(data[i]);
            const h = (val / maxVal) * (height * 0.8); // 80% height usage
            const x = i * (width / data.length);
            const y = (height - h) / 2;

            ctx.fillStyle = accentColor;
            ctx.globalAlpha = (i / data.length) < positionFraction ? 0.4 : 1.0;
            ctx.fillRect(x, y, Math.max(1, barWidth - barGap), h);
        }
    } else {
        // Decorative noise pattern (fallback)
        ctx.strokeStyle = accentColor;
        ctx.lineWidth = 1;
        ctx.globalAlpha = 0.3;
        ctx.beginPath();
        const midY = height / 2;
        for (let x = 0; x < width; x += 4) {
            // Deterministic wave shape
            const h = 10 + Math.sin(x * 0.1) * 15 + Math.cos(x * 0.03) * 10;
            ctx.moveTo(x, midY - h / 2);
            ctx.lineTo(x, midY + h / 2);
        }
        ctx.stroke();
    }

    // Playhead
    ctx.globalAlpha = 1.0;
    const playheadX = positionFraction * width;
    ctx.strokeStyle = "#ffffff";
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.moveTo(playheadX, 0);
    ctx.lineTo(playheadX, height);
    ctx.stroke();
}

// ─── Deck Panel ──────────────────────────────────────────────────

function DeckPanel({
    deckId,
    className,
    masterBpm,
}: {
    deckId: "A" | "B";
    className: string;
    masterBpm: number;
}) {
    const { engineState, play, pause, setDeckGain, setEq, setFilter, setLoop, clearLoop, setDeckSpeed, tracks, mode, getTrackProfile, seek } = useStore();

    const snapshot =
        deckId === "A" ? engineState?.deck_a : engineState?.deck_b;
    const isPlaying = snapshot?.state === "playing";
    const isEmpty = !snapshot || snapshot.state === "empty";
    const loopActive = snapshot?.loop_active ?? false;

    // ── Waveform State ──────────────────────────────────────────
    const [trackProfile, setTrackProfile] = useState<any>(null);
    const canvasRef = useRef<HTMLCanvasElement>(null);

    // Match the loaded deck to a track in the library to get its ID (for profile fetch)
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

    // Fetch profile once when track ID changes
    useEffect(() => {
        if (matchedTrack?.id) {
            getTrackProfile(matchedTrack.id).then(setTrackProfile);
        } else {
            setTrackProfile(null);
        }
    }, [matchedTrack?.id, getTrackProfile]);

    // Handle canvas resizing
    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) return;

        const resize = () => {
            canvas.width = canvas.offsetWidth;
            canvas.height = canvas.offsetHeight;
        };

        const observer = new ResizeObserver(resize);
        observer.observe(canvas);
        resize();

        return () => observer.disconnect();
    }, [isEmpty]);

    // Draw waveform at 30Hz playhead rate (snapshot updates)
    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas || isEmpty) return;

        const ctx = canvas.getContext("2d");
        if (!ctx) return;

        const computedStyle = getComputedStyle(canvas);
        const bgColor = computedStyle.getPropertyValue("--bg-elevated").trim();
        const accentVar = deckId === "A" ? "--accent-a" : "--accent-b";
        const accentColor = computedStyle.getPropertyValue(accentVar).trim();

        drawWaveform(
            ctx,
            canvas.width,
            canvas.height,
            trackProfile,
            snapshot?.position_fraction ?? 0,
            accentColor,
            bgColor
        );
    }, [snapshot?.position_fraction, trackProfile, isEmpty, deckId]);

    const handleSeek = (e: React.MouseEvent<HTMLCanvasElement>) => {
        const canvas = canvasRef.current;
        if (!canvas || !snapshot) return;
        const rect = canvas.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const fraction = Math.max(0, Math.min(1, x / rect.width));
        seek(deckId, fraction * snapshot.duration_ms);
    };

    // Track which bar count is active for the loop buttons
    const [activeLoopBars, setActiveLoopBars] = useState<number | null>(null);

    // ── Locally-optimistic slider state ──────────────────────────
    // Mirrors the engine snapshot but updates immediately on drag so
    // the slider doesn't freeze while waiting for the 30 Hz broadcast.
    const [localGain, setLocalGain] = useState(snapshot?.gain_db ?? 0);
    const [localHi, setLocalHi] = useState(snapshot?.eq_high ?? 0);
    const [localMid, setLocalMid] = useState(snapshot?.eq_mid ?? 0);
    const [localLo, setLocalLo] = useState(snapshot?.eq_low ?? 0);
    const [localFilter, setLocalFilter] = useState(
        filterToSlider(snapshot?.filter_cutoff ?? 20000)
    );

    // ── Per-deck tempo state ──────────────────────────────────────
    // originalBpm: try to match the loaded track from the library by
    // duration (best proxy we have since DeckSnapshot has no track_id).
    // Falls back to master_bpm, then 128.

    // Sync activeLoopBars from engine snapshot when loop is cleared externally
    useEffect(() => {
        if (!snapshot?.loop_active) {
            setActiveLoopBars(null);
        } else if (snapshot.loop_start_ms !== 0 && snapshot.loop_end_ms !== 0) {
            // If loop is active and has valid start/end, derive bars from engine
            const derivedLoopBars = Math.round(
                (snapshot.loop_end_ms - snapshot.loop_start_ms) /
                ((60000 / masterBpm) * 4)
            );
            setActiveLoopBars(derivedLoopBars);
        }
    }, [snapshot?.loop_active, snapshot?.loop_start_ms, snapshot?.loop_end_ms, masterBpm]);

    const getOriginalBpm = (snap: any) => {
        if (!snap || snap.state === "empty" || !snap.track_id) return masterBpm;
        const track = tracks.find(t => t.id === snap.track_id);
        if (track && track.bpm) {
            return track.bpm;
        }
        return masterBpm;
    };

    const originalBpm = getOriginalBpm(snapshot);
    const otherSnapshot = deckId === "A" ? engineState?.deck_b : engineState?.deck_a;
    const otherOriginalBpm = getOriginalBpm(otherSnapshot);
    const otherCurrentBpm = otherOriginalBpm * (otherSnapshot?.speed ?? 1.0);

    const [localBpm, setLocalBpm] = useState(masterBpm);

    // Reset local values when the deck state changes (new track loaded, etc.)
    useEffect(() => {
        setLocalGain(snapshot?.gain_db ?? 0);
        setLocalHi(snapshot?.eq_high ?? 0);
        setLocalMid(snapshot?.eq_mid ?? 0);
        setLocalLo(snapshot?.eq_low ?? 0);
        setLocalFilter(filterToSlider(snapshot?.filter_cutoff ?? 20000));
    }, [snapshot?.state, snapshot?.gain_db, snapshot?.eq_high, snapshot?.eq_mid, snapshot?.eq_low, snapshot?.filter_cutoff]);

    // Sync localBpm when originBpm changes (new track loaded)
    useEffect(() => {
        setLocalBpm(originalBpm);
    }, [originalBpm]);

    return (
        <div className={`deck ${className}`}>
            <div className="deck-label">DECK {deckId}</div>

            {isEmpty ? (
                <div className="waveform-placeholder">No track loaded</div>
            ) : (
                <canvas
                    ref={canvasRef}
                    className="waveform-canvas"
                    onClick={handleSeek}
                    style={{ cursor: "pointer" }}
                />
            )}

            {!isEmpty && (
                <>
                    <div className="progress-bar" onClick={(e) => {
                        if (!snapshot) return;
                        const rect = e.currentTarget.getBoundingClientRect();
                        const x = e.clientX - rect.left;
                        const fraction = Math.max(0, Math.min(1, x / rect.width));
                        seek(deckId, fraction * snapshot.duration_ms);
                    }} style={{ cursor: "pointer" }}>
                        <div
                            className="progress-fill"
                            style={{
                                width: `${(snapshot?.position_fraction ?? 0) * 100}%`,
                            }}
                        />
                    </div>

                    <div className="deck-info">
                        <span className="deck-track-title">
                            {isEmpty ? "Empty" : isPlaying ? "▶ Playing" : "⏸ Paused"}
                        </span>
                        <span className="deck-track-title">
                            {snapshot?.track_id
                                ? tracks.find(t => t.id === snapshot.track_id)?.title ?? "Unknown Track"
                                : "No track loaded"}
                        </span>
                        <span className="deck-time">
                            {formatTime(snapshot?.position_ms ?? 0)} /{" "}
                            {formatTime(snapshot?.duration_ms ?? 0)}
                        </span>
                    </div>

                    {/* ─── AI Suggestion Badge (Assisted Mode) ─── */}
                    {mode === "assisted" && !isEmpty && (
                        <div className="status-badge" style={{
                            alignSelf: 'flex-start',
                            marginTop: '4px',
                            background: deckId === "A" ? 'var(--accent-a-glow)' : 'var(--accent-b-glow)',
                            color: deckId === "A" ? 'var(--accent-a)' : 'var(--accent-b)'
                        }}>
                            ✨ AI RECOMMENDS
                        </div>
                    )}

                    {/* ─── Mixer Controls ─── */}
                    {mode !== "autodj" && (
                        <div className="mixer-controls">
                            {/* Gain Slider */}
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
                                    {localGain <= -11.5
                                        ? "-∞"
                                        : `${localGain.toFixed(1)}`}
                                </span>
                            </div>

                            {/* EQ Knobs */}
                            <div className="eq-controls">
                                <EqKnob
                                    label="HI"
                                    value={localHi}
                                    onChange={(db) => {
                                        setLocalHi(db);
                                        setEq(deckId, "high", db);
                                    }}
                                    onKill={() => {
                                        const next = localHi <= -90 ? 0 : -96;
                                        setLocalHi(next);
                                        setEq(deckId, "high", next);
                                    }}
                                />
                                <EqKnob
                                    label="MID"
                                    value={localMid}
                                    onChange={(db) => {
                                        setLocalMid(db);
                                        setEq(deckId, "mid", db);
                                    }}
                                    onKill={() => {
                                        const next = localMid <= -90 ? 0 : -96;
                                        setLocalMid(next);
                                        setEq(deckId, "mid", next);
                                    }}
                                />
                                <EqKnob
                                    label="LO"
                                    value={localLo}
                                    onChange={(db) => {
                                        setLocalLo(db);
                                        setEq(deckId, "low", db);
                                    }}
                                    onKill={() => {
                                        const next = localLo <= -90 ? 0 : -96;
                                        setLocalLo(next);
                                        setEq(deckId, "low", next);
                                    }}
                                />
                            </div>

                            {/* Filter Knob - Only in Manual */}
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
                                        {sliderToFilter(localFilter) >= 19500
                                            ? "OFF"
                                            : `${Math.round(sliderToFilter(localFilter))}Hz`}
                                    </span>
                                </div>
                            )}
                        </div>
                    )}

                    {/* ─── Tempo Controls ─── */}
                    {(mode === "manual" || mode === "assisted") && (
                        <div className="tempo-controls">
                            <div className="tempo-header">
                                <span className="tempo-label">TEMPO</span>
                                <span className="tempo-display">{localBpm.toFixed(1)} BPM</span>
                            </div>
                            <div className="tempo-row">
                                {/* Fine nudge left */}
                                <button
                                    className="tempo-btn"
                                    title="Nudge −2 BPM"
                                    onClick={() => {
                                        const next = Math.max(originalBpm * 0.75, localBpm - 2);
                                        setLocalBpm(next);
                                        setDeckSpeed(deckId, next / originalBpm);
                                    }}
                                    disabled={isEmpty}
                                >
                                    «
                                </button>

                                {/* Tempo slider ±25% */}
                                <input
                                    type="range"
                                    className="tempo-slider"
                                    min={originalBpm * 0.75}
                                    max={originalBpm * 1.25}
                                    step={0.1}
                                    value={localBpm}
                                    disabled={isEmpty}
                                    onChange={(e) => {
                                        const v = Number(e.target.value);
                                        setLocalBpm(v);
                                        setDeckSpeed(deckId, v / originalBpm);
                                    }}
                                />

                                {/* Fine nudge right */}
                                <button
                                    className="tempo-btn"
                                    title="Nudge +2 BPM"
                                    onClick={() => {
                                        const next = Math.min(originalBpm * 1.25, localBpm + 2);
                                        setLocalBpm(next);
                                        setDeckSpeed(deckId, next / originalBpm);
                                    }}
                                    disabled={isEmpty}
                                >
                                    »
                                </button>
                            </div>
                            <div className="tempo-actions">
                                {/* SYNC to opposite deck */}
                                <button
                                    className={`sync-btn sync-btn-${deckId.toLowerCase()}`}
                                    onClick={() => {
                                        setLocalBpm(otherCurrentBpm);
                                        setDeckSpeed(deckId, otherCurrentBpm / originalBpm);
                                    }}
                                    disabled={isEmpty}
                                >
                                    SYNC
                                </button>
                                {/* Reset deck to defaults */}
                                <button
                                    className="tempo-btn reset-btn"
                                    title="Reset deck parameters"
                                    onClick={() => {
                                        setLocalBpm(originalBpm);
                                        setDeckSpeed(deckId, 1);

                                        setLocalGain(0);
                                        setDeckGain(deckId, 0);

                                        setLocalHi(0);
                                        setEq(deckId, "high", 0);
                                        setLocalMid(0);
                                        setEq(deckId, "mid", 0);
                                        setLocalLo(0);
                                        setEq(deckId, "low", 0);

                                        const defaultHz = sliderToFilter(50);
                                        setLocalFilter(50);
                                        setFilter(deckId, defaultHz, snapshot?.filter_resonance ?? 0.7);
                                    }}
                                    disabled={isEmpty}
                                >
                                    ↺
                                </button>
                            </div>
                        </div>
                    )}
                </>
            )}

            {mode !== "autodj" && (
                <div className="transport-controls">
                    <button
                        className={`transport-btn ${isPlaying ? "playing" : ""}`}
                        onClick={() => (isPlaying ? pause(deckId) : play(deckId))}
                        disabled={isEmpty}
                    >
                        {isPlaying ? "⏸" : "▶"}
                    </button>

                    {/* Loop controls */}
                    <div className="loop-controls-group">
                        <span className="loop-section-label">LOOP</span>
                        <div className="loop-buttons">
                            {[4, 8, 16].map((bars) => (
                                <button
                                    key={bars}
                                    className={`transport-btn loop-btn ${activeLoopBars === bars ? "active-loop" : ""}`}
                                    onClick={() => {
                                        setActiveLoopBars(bars);
                                        setLoop(deckId, bars);
                                    }}
                                    disabled={isEmpty}
                                >
                                    {bars}
                                </button>
                            ))}
                            {loopActive && (
                                <button
                                    className="transport-btn loop-off-btn"
                                    onClick={() => {
                                        setActiveLoopBars(null);
                                        clearLoop(deckId);
                                    }}
                                    title="Clear loop"
                                >
                                    ✕
                                </button>
                            )}
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
}

// ─── Filter ↔ Slider Helpers ─────────────────────────────────────

function filterToSlider(hz: number): number {
    if (hz >= 19500) return 50; // Center bypass

    // If it's in Highpass range (<= 1000Hz)
    if (hz <= 1000) {
        const frac = (hz - 20) / (1000 - 20);
        return 51 + frac * 49;
    }

    // If it's in Lowpass range (> 1000Hz)
    else {
        const frac = (hz - 1000) / (20000 - 1000);
        return frac * 49;
    }
}

/** Convert 0–100 slider value to cutoff Hz (log scale) */
function sliderToFilter(val: number): number {
    if (val === 50) return 20000; // bypass (LP at 20kHz)

    // Left side (0-49): Lowpass filtering, cutoff goes from 1000Hz (at 0) up to 20000Hz (at 50)
    if (val < 50) {
        const frac = val / 49; // 0 to 1
        return 1000 + frac * (20000 - 1000);
    }

    // Right side (51-100): Highpass filtering, cutoff goes from 20Hz (at 50) up to 1000Hz (at 100)
    else {
        const frac = (val - 51) / 49; // 0 to 1
        return 20 + frac * (1000 - 20);
    }
}

// ─── DJ View ─────────────────────────────────────────────────────

function DJView() {
    const { engineState, setCrossfader } = useStore();

    const masterBpm = engineState?.master_bpm ?? 128;

    return (
        <div className="dj-view">
            <div className="decks-container">
                <DeckPanel deckId="A" className="deck-a-panel" masterBpm={masterBpm} />
                <DeckPanel deckId="B" className="deck-b-panel" masterBpm={masterBpm} />
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

            <div style={{ display: "flex", gap: "20px" }}>
                {useStore.getState().mode === "manual" && <TransitionsPanel />}
                <AiPanel />
            </div>
        </div>
    );
}

// ─── AI Conductor Panel ──────────────────────────────────────────

function AiPanel() {
    const { mode, setMode, requestNextTrack, aiDecision, isThinking, tracks, loadTrack, engineState } = useStore();

    const suggestedTrack = aiDecision?.track_selection
        ? tracks.find(t => t.id === aiDecision.track_selection)
        : null;

    // Map raw AI action to human-readable status
    const getStatusText = () => {
        if (isThinking) return "Analyzing mix…";
        if (!aiDecision) return "Waiting for input";
        switch (aiDecision.decision) {
            case "transition_now": return "Ready to transition";
            case "wait": return "Holding — continue current track";
            default: return aiDecision.decision;
        }
    };

    const getModeLabel = (m: DjMode) => {
        switch (m) {
            case "manual": return "Manual";
            case "assisted": return "Assisted";
            case "autodj": return "Auto-DJ";
        }
    };

    const getModeDescription = () => {
        switch (mode) {
            case "manual": return "Full control. You mix.";
            case "assisted": return "AI suggests, you decide.";
            case "autodj": return "Sit back. AI DJs for you.";
        }
    };

    const activeDeck = engineState?.deck_a?.state === "playing" ? "A" : "B";
    const currentTrackId = activeDeck === "A" ? engineState?.deck_a?.track_id : engineState?.deck_b?.track_id;
    const currentTrack = tracks.find(t => t.id === currentTrackId);

    const currentTrackName = currentTrack?.title || "Nothing playing";
    const displayBpm = currentTrack?.bpm?.toFixed(1) || engineState?.master_bpm?.toFixed(1) || "128.0";

    return (
        <div className="ai-panel ai-panel-card" style={{ flex: 1 }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <h3 style={{ margin: 0, fontSize: "14px", textTransform: "uppercase", letterSpacing: "1px", color: "var(--text-secondary)", fontFamily: "var(--font-display)" }}>
                    🤖 AI Conductor
                </h3>
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

            <div className="mode-description">{getModeDescription()}</div>

            {/* AI Suggestion Area */}
            <div style={{ display: "flex", flexDirection: "column", gap: "10px" }}>
                <div className="ai-card now-playing">
                    <span className="ai-card-label">What's Playing</span>
                    <span className="ai-card-title">{currentTrackName}</span>
                    <div className="ai-card-meta">
                        <span>{displayBpm} BPM</span>
                    </div>
                </div>

                {suggestedTrack ? (
                    <div className="ai-card up-next">
                        <span className="ai-card-label">Up Next</span>
                        <span className="ai-card-title">{suggestedTrack.title || "Unknown"}</span>
                        <div className="ai-card-meta">
                            <span>{suggestedTrack.artist}</span>
                            <span>•</span>
                            <span>{suggestedTrack.bpm?.toFixed(1)} BPM</span>
                            <span>•</span>
                            <span>{suggestedTrack.key}</span>
                        </div>
                        <div className="ai-insight">
                            💡 "{aiDecision?.reason}"
                        </div>
                        <button
                            className="queue-btn"
                            onClick={() => loadTrack("b", suggestedTrack.id)}
                        >
                            Queue on Deck B →
                        </button>
                    </div>
                ) : (
                    <div className="ai-card" style={{ minHeight: '80px', display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--text-muted)', fontSize: '12px' }}>
                        {isThinking ? "Thinking..." : "No suggestion yet"}
                    </div>
                )}
            </div>

            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginTop: "auto" }}>
                <span className="ai-status-text">{getStatusText()}</span>
                {mode === "autodj" ? (
                    <div className="ai-listening-indicator">
                        <div className="listening-dot"></div>
                        <span>AI is listening</span>
                    </div>
                ) : (
                    <button
                        onClick={requestNextTrack}
                        disabled={isThinking}
                        className={`find-track-btn ${isThinking ? "thinking" : ""}`}
                        style={{ width: "auto", minWidth: "180px" }}
                    >
                        {mode === "manual" ? "✨ Find My Next Track" : "🔄 Get AI Suggestion"}
                    </button>
                )}
            </div>
        </div>
    );
}

// ─── Transitions Panel ───────────────────────────────────────────

function TransitionsPanel() {
    const { executeTransition, engineState } = useStore();
    const [template, setTemplate] = useState("smooth_beatmix");
    const [duration, setDuration] = useState(16);
    const [isTransitioning, setIsTransitioning] = useState(false);
    const [transitionDurationMs, setTransitionDurationMs] = useState(0);
    const [errorMsg, setErrorMsg] = useState<string | null>(null);

    const handleGo = async () => {
        if (!engineState) return;

        // Auto-detect direction based strictly on the Crossfader Position.
        // If crossfader is > 0 (leaning towards B), we are playing Deck B. Incoming deck should be A.
        // If crossfader is < 0 (leaning towards A), we are playing Deck A. Incoming deck should be B.
        // If crossfader is 0, we check which deck is actively playing audio.
        let incomingDeck = "B";

        if (engineState.crossfader > 0.1) {
            incomingDeck = "A";
        } else if (engineState.crossfader < -0.1) {
            incomingDeck = "B";
        } else {
            // Crossfader is centered. Transition towards whichever deck is PAUSED or EMPTY. 
            // If both are playing, default to B.
            const aPlaying = engineState.deck_a?.state === "playing";
            const bPlaying = engineState.deck_b?.state === "playing";

            if (aPlaying && !bPlaying) incomingDeck = "B";
            else if (!aPlaying && bPlaying) incomingDeck = "A";
            else incomingDeck = "B";
        }

        // Compute animation duration: duration beats × (60s / BPM) — each "beat" here is a bar (4 beats)
        const masterBpm = engineState.master_bpm ?? 128;
        const durationSeconds = (duration / masterBpm) * 60 * 4;
        const durationMs = durationSeconds * 1000;

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
        <div
            className={`transitions-panel${isTransitioning ? " transitions-panel--active" : ""}`}
            style={{ marginTop: "20px", padding: "15px", background: "rgba(255,255,255,0.05)", borderRadius: "8px", flex: 1 }}
        >
            <h3 style={{ margin: "0 0 10px 0", fontSize: "14px", textTransform: "uppercase", letterSpacing: "1px", color: "#aaa" }}>
                Auto Transition
            </h3>
            <div style={{ display: "flex", gap: "15px", alignItems: "flex-end" }}>
                <div style={{ display: "flex", flexDirection: "column", gap: "5px" }}>
                    <label style={{ fontSize: "12px", color: "#888" }}>STYLE</label>
                    <select
                        value={template}
                        onChange={e => setTemplate(e.target.value)}
                        disabled={isTransitioning}
                        style={{ padding: "8px", background: "#111", color: "#eee", border: "1px solid #333", borderRadius: "4px", opacity: isTransitioning ? 0.5 : 1 }}
                    >
                        <option value="smooth_beatmix">Smooth Beatmix</option>
                        <option value="filter_sweep">Filter Sweep</option>
                        <option value="echo_drop">Echo Drop</option>
                    </select>
                </div>

                <div style={{ display: "flex", flexDirection: "column", gap: "5px", flex: 1 }}>
                    <label style={{ fontSize: "12px", color: "#888" }}>DURATION ({duration} BTT)</label>
                    <input
                        type="range"
                        min={4} max={64} step={4}
                        value={duration}
                        disabled={isTransitioning}
                        onChange={e => setDuration(Number(e.target.value))}
                        style={{ width: "100%", opacity: isTransitioning ? 0.5 : 1 }}
                    />
                </div>

                <button
                    onClick={handleGo}
                    disabled={isTransitioning}
                    className={isTransitioning ? "transitioning-btn" : ""}
                    style={{
                        padding: "10px 20px",
                        background: isTransitioning ? "#2a6e2e" : "#4CAF50",
                        color: "white",
                        border: "none",
                        borderRadius: "4px",
                        cursor: isTransitioning ? "not-allowed" : "pointer",
                        fontWeight: "bold",
                        display: "flex",
                        alignItems: "center",
                        gap: "8px",
                        minWidth: "130px",
                        justifyContent: "center",
                        transition: "background 0.3s ease",
                    }}
                >
                    {isTransitioning ? (
                        <>
                            <span className="transition-spinner" />
                            Transitioning…
                        </>
                    ) : (
                        "GO ✨"
                    )}
                </button>
            </div>

            {/* Progress bar — only rendered while transitioning */}
            {isTransitioning && (
                <div className="transition-progress-track" style={{ marginTop: "12px" }}>
                    <div
                        className="transition-progress-bar"
                        style={{ animationDuration: `${transitionDurationMs}ms` }}
                    />
                </div>
            )}

            {/* Inline error toast */}
            {errorMsg && (
                <div className="transition-error-toast" style={{ marginTop: "10px" }}>
                    ⚠️ {errorMsg}
                </div>
            )}
        </div>
    );
}

// ─── AI Config Panel ─────────────────────────────────────────────

function AiConfigPanel() {
    const aiConfig = useStore((state) => state.aiConfig);
    const setAiConfig = useStore((state) => state.setAiConfig);
    const [apiKey, setApiKey] = useState("");
    const [isSaving, setIsSaving] = useState(false);



    const handleSave = async () => {
        setIsSaving(true);
        try {
            await setAiConfig({
                ...aiConfig,
                enabled: true,
                api_key: apiKey,
                model: "gemini-1.5-pro",
                endpoint_url: "https://generativelanguage.googleapis.com/v1beta/models"
            });
            setApiKey(""); // Clear so it shows the masked view
        } catch (err) {
            console.error("Failed to save AI config:", err);
        } finally {
            setIsSaving(false);
        }
    };

    return (
        <div className="ai-panel ai-panel-card" style={{ flex: 1, minWidth: "300px" }}>
            <h3 style={{ margin: "0 0 15px 0", fontSize: "14px", textTransform: "uppercase", letterSpacing: "1px", color: "var(--text-secondary)", fontFamily: "var(--font-display)" }}>
                ⚙️ Gemini AI Integration
            </h3>

            <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
                <div style={{ display: "flex", flexDirection: "column", gap: "5px" }}>
                    <label style={{ fontSize: "12px", color: "#888" }}>GEMINI API KEY</label>
                    {aiConfig?.api_key && apiKey === "" ? (
                        <div style={{ display: "flex", alignItems: "center", gap: "8px", background: "rgba(0,0,0,0.2)", padding: "10px", borderRadius: "6px", border: "1px solid var(--border)" }}>
                            <span style={{ fontFamily: "monospace", color: "var(--status-ready)", flex: 1, fontSize: "14px" }}>
                                ✓ {aiConfig.api_key.substring(0, 4)}....{aiConfig.api_key.slice(-4)}X
                            </span>
                            <button onClick={() => setApiKey(aiConfig.api_key)} style={{ background: "transparent", border: "1px solid #555", color: "#ccc", borderRadius: "4px", cursor: "pointer", padding: "4px 8px", fontSize: "10px", fontWeight: "bold", transition: "all 0.2s ease" }}>
                                EDIT
                            </button>
                        </div>
                    ) : (
                        <input
                            type="password"
                            placeholder="AIzeSy..."
                            value={apiKey}
                            onChange={(e) => setApiKey(e.target.value)}
                            style={{
                                padding: "10px",
                                background: "rgba(0,0,0,0.2)",
                                border: "1px solid var(--border)",
                                borderRadius: "6px",
                                color: "var(--text)",
                                fontFamily: "monospace"
                            }}
                        />
                    )}
                </div>

                <button
                    onClick={handleSave}
                    disabled={isSaving}
                    style={{
                        padding: "10px",
                        background: apiKey ? "var(--primary-glow)" : "var(--surface)",
                        color: apiKey ? "#fff" : "var(--text-muted)",
                        border: "none",
                        borderRadius: "6px",
                        fontWeight: "bold",
                        cursor: isSaving || !apiKey ? "not-allowed" : "pointer",
                        transition: "all 0.2s ease",
                        marginTop: "8px"
                    }}
                >
                    {isSaving ? "Saving..." : "Save AI Credentials"}
                </button>
            </div>
        </div>
    );
}

// ─── App ─────────────────────────────────────────────────────────

export default function App() {
    const { activeView, setActiveView, setEngineState } = useStore();
    const [isSettingsOpen, setIsSettingsOpen] = useState(false);

    useEffect(() => {
        // Load initial state elements
        useStore.getState().initStore();

        const unlisten = listen<EngineState>("engine-state", (event) => {
            setEngineState(event.payload);
        });
        return () => {
            unlisten.then(f => f());
        };
    }, [setEngineState]);

    return (
        <div className="app">
            <header className="header">
                <div className="logo">
                    <img src={logo} alt="AutoDJ Logo" style={{ height: "28px" }} />
                    AutoDJ
                </div>
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
                    <button className="settings-btn" onClick={() => setIsSettingsOpen(true)}>
                        ⚙️
                    </button>
                </div>
            </header>

            <main className="main-content">
                {activeView === "library" ? <LibraryView /> : <DJView />}
            </main>

            {isSettingsOpen && (
                <div className="settings-modal-overlay" onClick={() => setIsSettingsOpen(false)}>
                    <div className="settings-modal" onClick={e => e.stopPropagation()}>
                        <button className="settings-modal-close" onClick={() => setIsSettingsOpen(false)}>✕</button>
                        <AiConfigPanel />
                    </div>
                </div>
            )}
        </div>
    );
}
