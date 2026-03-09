import { useEffect, useCallback, useState, useRef, useMemo } from "react";
import { useStore, EngineState, DjMode, AutodjStatus } from "./store";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import logo from "./assets/logo.png";

function formatTime(ms: number): string {
    const totalSeconds = Math.floor(ms / 1000);
    const minutes = Math.floor(totalSeconds / 60);
    const seconds = totalSeconds % 60;
    return `${minutes}:${seconds.toString().padStart(2, "0")}`;
}

function formatGroove(groove: string): string {
    const labels: Record<string, string> = {
        four_on_floor: "4/4",
        trap_half_time: "Trap ½×",
        dembow: "Dembow",
        breakbeat: "Breakbeat",
        boom_bap: "Boom Bap",
        drill: "Drill",
        jersey_club: "Jersey",
        drum_and_bass: "DnB",
        standard: "Std",
    };
    return labels[groove] || groove.replace(/_/g, " ");
}

// ─── Library View ────────────────────────────────────────────────

function LibraryView() {
    const { tracks, importing, fetchTracks, importFiles, loadTrack, clearLibrary, analyzingTracks, libraryFilters, setLibraryFilters, clearLibraryFilters, analyzeTrack, classifyTrack, classifyAll, batchClassifying, batchClassifyProgress, aiConfig } = useStore();
    const searchTimerRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
    const [contextMenu, setContextMenu] = useState<{ x: number; y: number; trackId: string } | null>(null);

    useEffect(() => {
        fetchTracks();
    }, [fetchTracks]);

    // Debounced search
    const handleSearchChange = useCallback((value: string) => {
        setLibraryFilters({ query: value });
        if (searchTimerRef.current) clearTimeout(searchTimerRef.current);
        searchTimerRef.current = setTimeout(() => {
            useStore.getState().fetchTracks();
        }, 300);
    }, [setLibraryFilters]);

    const handleFilterChange = useCallback((key: string, value: string) => {
        setLibraryFilters({ [key]: value } as any);
        // Fetch immediately for dropdown changes
        setTimeout(() => useStore.getState().fetchTracks(), 0);
    }, [setLibraryFilters]);

    const handleSort = useCallback((column: string) => {
        const f = useStore.getState().libraryFilters;
        if (f.sortBy === column) {
            setLibraryFilters({ sortOrder: f.sortOrder === "asc" ? "desc" : "asc" });
        } else {
            setLibraryFilters({ sortBy: column, sortOrder: "asc" });
        }
        setTimeout(() => useStore.getState().fetchTracks(), 0);
    }, [setLibraryFilters]);

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

    const sortIndicator = (col: string) => {
        if (libraryFilters.sortBy !== col) return "";
        return libraryFilters.sortOrder === "asc" ? " \u25B2" : " \u25BC";
    };

    const hasFilters = libraryFilters.query || libraryFilters.genre || libraryFilters.mood || libraryFilters.bpmMin || libraryFilters.bpmMax || libraryFilters.keyFilter;

    // Close context menu on outside click
    useEffect(() => {
        const handleClick = () => setContextMenu(null);
        if (contextMenu) {
            document.addEventListener("click", handleClick);
            return () => document.removeEventListener("click", handleClick);
        }
    }, [contextMenu]);

    return (
        <div className="library">
            <div className="library-toolbar">
                <div style={{ display: "flex", gap: "10px", alignItems: "center" }}>
                    <button
                        className="import-btn"
                        onClick={handleImport}
                        disabled={importing}
                        style={{ backgroundColor: "#4CAF50", color: "white", border: "none" }}
                    >
                        {importing ? "⟳ Importing..." : "+ Import Files"}
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
                        Clear
                    </button>
                    {aiConfig?.api_key && (
                        <button
                            onClick={classifyAll}
                            disabled={batchClassifying || tracks.length === 0}
                            style={{
                                padding: "8px 16px",
                                backgroundColor: batchClassifying ? "var(--bg-elevated)" : "rgba(178, 102, 255, 0.8)",
                                color: "white",
                                border: "none",
                                borderRadius: "4px",
                                cursor: batchClassifying || tracks.length === 0 ? "not-allowed" : "pointer",
                                opacity: batchClassifying || tracks.length === 0 ? 0.7 : 1,
                                fontSize: "13px",
                                fontWeight: 600,
                            }}
                        >
                            {batchClassifying
                                ? `Classifying ${batchClassifyProgress.done}/${batchClassifyProgress.total}`
                                : "Classify All"}
                        </button>
                    )}
                </div>
                <div className="library-search">
                    <input
                        type="text"
                        className="search-input"
                        placeholder="Search title, artist..."
                        value={libraryFilters.query}
                        onChange={(e) => handleSearchChange(e.target.value)}
                    />
                </div>
                <span className="track-count">{tracks.length} tracks</span>
            </div>

            <div className="library-filters">
                <input
                    type="text"
                    className="filter-input"
                    placeholder="Genre"
                    value={libraryFilters.genre}
                    onChange={(e) => handleFilterChange("genre", e.target.value)}
                />
                <input
                    type="text"
                    className="filter-input"
                    placeholder="Mood"
                    value={libraryFilters.mood}
                    onChange={(e) => handleFilterChange("mood", e.target.value)}
                />
                <input
                    type="number"
                    className="filter-input filter-bpm"
                    placeholder="BPM min"
                    value={libraryFilters.bpmMin}
                    onChange={(e) => handleFilterChange("bpmMin", e.target.value)}
                />
                <input
                    type="number"
                    className="filter-input filter-bpm"
                    placeholder="BPM max"
                    value={libraryFilters.bpmMax}
                    onChange={(e) => handleFilterChange("bpmMax", e.target.value)}
                />
                <input
                    type="text"
                    className="filter-input"
                    placeholder="Key"
                    value={libraryFilters.keyFilter}
                    onChange={(e) => handleFilterChange("keyFilter", e.target.value)}
                />
                {hasFilters && (
                    <button className="filter-clear-btn" onClick={clearLibraryFilters}>
                        Clear Filters
                    </button>
                )}
            </div>

            <div className="track-list">
                <div className="track-list-header">
                    <span className="sortable" onClick={() => handleSort("title")}>Title{sortIndicator("title")}</span>
                    <span className="sortable" onClick={() => handleSort("artist")}>Artist{sortIndicator("artist")}</span>
                    <span className="sortable" onClick={() => handleSort("bpm")}>BPM{sortIndicator("bpm")}</span>
                    <span>Groove</span>
                    <span className="sortable" onClick={() => handleSort("key")}>Key{sortIndicator("key")}</span>
                    <span className="sortable" onClick={() => handleSort("genre")}>Genre{sortIndicator("genre")}</span>
                    <span className="sortable" onClick={() => handleSort("mood")}>Mood{sortIndicator("mood")}</span>
                    <span className="sortable" onClick={() => handleSort("duration")}>Duration{sortIndicator("duration")}</span>
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
                            <div className="track-row" key={track.id} onContextMenu={(e) => {
                                            e.preventDefault();
                                            setContextMenu({ x: e.clientX, y: e.clientY, trackId: track.id });
                                        }}>
                                <span className="track-title">
                                    {track.title || "Unknown Title"}
                                </span>
                                <span className="track-artist">
                                    {track.artist || "Unknown Artist"}
                                </span>
                                <span className="track-duration">
                                    {track.bpm ? track.bpm.toFixed(1) : "—"}
                                    {track.perceived_tempo && track.perceived_tempo !== track.bpm ? (
                                        <span style={{ fontSize: "10px", color: "var(--accent)", marginLeft: "4px" }}>({track.perceived_tempo.toFixed(0)})</span>
                                    ) : null}
                                </span>
                                <span className="track-duration">
                                    {track.groove_type && track.groove_type !== "standard" ? (
                                        <span className="groove-badge">{formatGroove(track.groove_type)}</span>
                                    ) : "—"}
                                </span>
                                <span className="track-duration">
                                    {track.key || "—"}
                                </span>
                                <span className="track-duration">
                                    {track.genre ? <span className="genre-badge">{track.genre}</span> : "—"}
                                </span>
                                <span className="track-duration">
                                    {track.mood ? <span className="mood-badge">{track.mood}</span> : "—"}
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

            {/* Context Menu */}
            {contextMenu && (
                <div
                    className="context-menu"
                    style={{ top: contextMenu.y, left: contextMenu.x }}
                >
                    <button onClick={() => { loadTrack("a", contextMenu.trackId); setContextMenu(null); }}>
                        Load to Deck A
                    </button>
                    <button onClick={() => { loadTrack("b", contextMenu.trackId); setContextMenu(null); }}>
                        Load to Deck B
                    </button>
                    <div className="context-menu-divider" />
                    <button onClick={() => { analyzeTrack(contextMenu.trackId); setContextMenu(null); }}>
                        Re-analyze
                    </button>
                    <button onClick={() => { classifyTrack(contextMenu.trackId); setContextMenu(null); }}>
                        Classify with AI
                    </button>
                </div>
            )}
        </div>
    );
}

// ─── EQ Knob ─────────────────────────────────────────────────────

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
    const { engineState, play, pause, setDeckGain, setEq, setFilter, setLoop, clearLoop, setDeckSpeed, setEqKill, tracks, mode, getTrackProfile, seek } = useStore();

    const snapshot =
        deckId === "A" ? engineState?.deck_a : engineState?.deck_b;
    const isPlaying = snapshot?.state === "playing";
    const isEmpty = !snapshot || snapshot.state === "empty";
    const loopActive = snapshot?.loop_active ?? false;

    // ── Album Art + Hot Cues ─────────────────────────────────────
    const [albumArtUrl, setAlbumArtUrl] = useState<string | null>(null);
    const [hotCues, setHotCues] = useState<Array<{ index: number; position_ms: number; label: string | null }>>([]);

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

    // Fetch profile, album art, and hot cues when track changes
    useEffect(() => {
        if (matchedTrack?.id) {
            getTrackProfile(matchedTrack.id).then(setTrackProfile);
            // Fetch album art
            invoke<string | null>("get_album_art", { trackId: matchedTrack.id }).then(path => {
                if (path) {
                    setAlbumArtUrl(convertFileSrc(path));
                } else {
                    setAlbumArtUrl(null);
                }
            }).catch(() => setAlbumArtUrl(null));
            // Fetch hot cues
            invoke<Array<[number, number, string | null]>>("get_hot_cues", { trackId: matchedTrack.id }).then(cues => {
                setHotCues(cues.map(([index, position_ms, label]) => ({ index, position_ms, label })));
            }).catch(() => setHotCues([]));
        } else {
            setTrackProfile(null);
            setAlbumArtUrl(null);
            setHotCues([]);
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

    const [pitchRange, setPitchRange] = useState(0.08);

    // Sync localBpm when originBpm changes (new track loaded)
    useEffect(() => {
        setLocalBpm(originalBpm);
    }, [originalBpm]);

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

    const hotCueColors = ["#ff3333", "#33ff33", "#3333ff", "#ffff33"];

    return (
        <div className={`deck ${className}`}>
            <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
                <div className="deck-label">DECK {deckId}</div>
                {albumArtUrl && (
                    <img
                        src={albumArtUrl}
                        alt="Album Art"
                        className="deck-album-art"
                    />
                )}
                {!isEmpty && matchedTrack && (
                    <span style={{ fontSize: "12px", color: "var(--text-secondary)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", flex: 1 }}>
                        {matchedTrack.artist || ""}
                    </span>
                )}
            </div>

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
                                        const isKilled = snapshot?.eq_kill_high ?? (localHi <= -90);
                                        setEqKill(deckId, "high", !isKilled);
                                        if (isKilled) {
                                            setLocalHi(0);
                                            setEq(deckId, "high", 0);
                                        }
                                    }}
                                    isKilled={snapshot?.eq_kill_high ?? false}
                                />
                                <EqKnob
                                    label="MID"
                                    value={localMid}
                                    onChange={(db) => {
                                        setLocalMid(db);
                                        setEq(deckId, "mid", db);
                                    }}
                                    onKill={() => {
                                        const isKilled = snapshot?.eq_kill_mid ?? (localMid <= -90);
                                        setEqKill(deckId, "mid", !isKilled);
                                        if (isKilled) {
                                            setLocalMid(0);
                                            setEq(deckId, "mid", 0);
                                        }
                                    }}
                                    isKilled={snapshot?.eq_kill_mid ?? false}
                                />
                                <EqKnob
                                    label="LO"
                                    value={localLo}
                                    onChange={(db) => {
                                        setLocalLo(db);
                                        setEq(deckId, "low", db);
                                    }}
                                    onKill={() => {
                                        const isKilled = snapshot?.eq_kill_low ?? (localLo <= -90);
                                        setEqKill(deckId, "low", !isKilled);
                                        if (isKilled) {
                                            setLocalLo(0);
                                            setEq(deckId, "low", 0);
                                        }
                                    }}
                                    isKilled={snapshot?.eq_kill_low ?? false}
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
                                <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
                                    <select
                                        value={pitchRange}
                                        onChange={(e) => setPitchRange(Number(e.target.value))}
                                        className="pitch-range-select"
                                        disabled={isEmpty}
                                        style={{
                                            background: "rgba(0,0,0,0.3)",
                                            color: "var(--text-muted)",
                                            border: "1px solid var(--border)",
                                            borderRadius: "4px",
                                            fontSize: "10px",
                                            padding: "2px 4px",
                                            cursor: "pointer"
                                        }}
                                    >
                                        <option value={0.06}>±6%</option>
                                        <option value={0.08}>±8%</option>
                                        <option value={0.10}>±10%</option>
                                        <option value={0.16}>±16%</option>
                                    </select>
                                    <span className="tempo-display">{localBpm.toFixed(1)} BPM</span>
                                    {(() => {
                                        const track = tracks.find(t => t.id === snapshot?.track_id);
                                        if (track?.groove_type && track.groove_type !== "standard") {
                                            return <span className="groove-badge" style={{ marginLeft: "6px", fontSize: "10px" }}>{formatGroove(track.groove_type)}</span>;
                                        }
                                        return null;
                                    })()}
                                </div>
                            </div>
                            <div className="tempo-row">
                                {/* Fine nudge left */}
                                <button
                                    className="tempo-btn"
                                    title="Nudge −2 BPM"
                                    onClick={() => {
                                        const next = Math.max(originalBpm * (1 - pitchRange), localBpm - 2);
                                        setLocalBpm(next);
                                        setDeckSpeed(deckId, next / originalBpm);
                                    }}
                                    disabled={isEmpty}
                                >
                                    «
                                </button>

                                {/* Tempo slider ±X% */}
                                <input
                                    type="range"
                                    className="tempo-slider"
                                    min={originalBpm * (1 - pitchRange)}
                                    max={originalBpm * (1 + pitchRange)}
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
                                        const next = Math.min(originalBpm * (1 + pitchRange), localBpm + 2);
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

                    {/* Hot Cue buttons */}
                    <div className="loop-controls-group">
                        <span className="loop-section-label">CUE</span>
                        <div className="loop-buttons">
                            {[0, 1, 2, 3].map((idx) => {
                                const cue = hotCues.find(c => c.index === idx);
                                return (
                                    <button
                                        key={idx}
                                        className={`transport-btn hot-cue-btn ${cue ? "hot-cue-set" : ""}`}
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
    const { engineState, setCrossfader, isRecording, recordingStartTime, startRecording, stopRecording } = useStore();

    const masterBpm = engineState?.master_bpm ?? 128;

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
                <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
                    <input
                        type="range"
                        className="crossfader-slider"
                        min={-100}
                        max={100}
                        value={(engineState?.crossfader ?? 0) * 100}
                        onChange={(e) => setCrossfader(Number(e.target.value) / 100)}
                        style={{ flex: 1 }}
                    />
                    <button
                        className={`rec-button ${isRecording ? "rec-active" : ""}`}
                        onClick={isRecording ? stopRecording : startRecording}
                        title={isRecording ? "Stop Recording" : "Start Recording"}
                    >
                        <span className="rec-dot" />
                        {isRecording ? `${recMinutes}:${String(recSeconds).padStart(2, "0")}` : "REC"}
                    </button>
                </div>
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
    const { mode, setMode, requestNextTrack, aiDecision, isThinking, tracks, loadTrack, engineState, clearAiDecision, autodjStatus, autodjMessage, setAutodjStatus } = useStore();

    const suggestedTrack = aiDecision?.track_selection
        ? tracks.find(t => t.id === aiDecision.track_selection)
        : null;

    // ─── Autonomous AutoDJ Controller ────────────────────────────
    // Phase 19/20: Section-aware state machine with phrase-aligned triggers.
    // States: idle → monitoring → planning → loading → aligning → transitioning → monitoring
    const autodjPhaseRef = useRef<"idle" | "monitoring" | "planning" | "loading" | "aligning" | "transitioning">("idle");
    const pendingTrackRef = useRef<string | null>(null);
    const transitionLockRef = useRef(false);
    const cachedProfileRef = useRef<any>(null);
    const cachedProfileTrackIdRef = useRef<string | null>(null);

    // Reset state when leaving autodj mode
    useEffect(() => {
        if (mode !== "autodj") {
            autodjPhaseRef.current = "idle";
            pendingTrackRef.current = null;
            transitionLockRef.current = false;
            cachedProfileRef.current = null;
            cachedProfileTrackIdRef.current = null;
            setAutodjStatus("idle");
            return;
        }

        setAutodjStatus("starting", "Initializing AutoDJ...");
        autodjPhaseRef.current = "idle";
    }, [mode]);

    // Main AutoDJ tick — runs at ~2Hz (every 500ms) for responsive position tracking
    useEffect(() => {
        if (mode !== "autodj") return;

        const tick = async () => {
            const state = useStore.getState();
            const es = state.engineState;
            if (!es) return;

            const deckA = es.deck_a;
            const deckB = es.deck_b;
            const aPlaying = deckA.state === "playing";
            const bPlaying = deckB.state === "playing";
            const analyzedTracks = state.tracks.filter(t => t.import_status === "analyzed");

            // ── Phase: No track playing — pick and load a first track ──
            if (!aPlaying && !bPlaying) {
                if (autodjPhaseRef.current === "transitioning") return;

                if (analyzedTracks.length === 0) {
                    setAutodjStatus("error", "No analyzed tracks in library");
                    return;
                }

                if (autodjPhaseRef.current === "idle" || autodjPhaseRef.current === "monitoring") {
                    autodjPhaseRef.current = "planning";
                    setAutodjStatus("picking_first_track", "Selecting first track...");

                    const pick = analyzedTracks[Math.floor(Math.random() * analyzedTracks.length)];
                    await state.loadTrack("a", pick.id);
                    autodjPhaseRef.current = "monitoring";
                    setAutodjStatus("monitoring", `Playing: ${pick.title || "Unknown"}`);
                    return;
                }
                return;
            }

            // ── Phase: A track is playing — monitor position ──
            const playingDeck: "a" | "b" = aPlaying ? "a" : "b";
            const idleDeck: "a" | "b" = playingDeck === "a" ? "b" : "a";
            const playingSnap = playingDeck === "a" ? deckA : deckB;
            const idleSnap = playingDeck === "a" ? deckB : deckA;
            const positionFrac = playingSnap.position_fraction;
            const durationMs = playingSnap.duration_ms;
            const positionMs = playingSnap.position_ms;

            // ── Load track profile for section-aware triggers ──
            const playingTrackId = playingSnap.track_id;
            if (playingTrackId && playingTrackId !== cachedProfileTrackIdRef.current) {
                try {
                    const profile = await state.getTrackProfile(playingTrackId);
                    cachedProfileRef.current = profile;
                    cachedProfileTrackIdRef.current = playingTrackId;
                } catch {
                    cachedProfileRef.current = null;
                }
            }

            const profile = cachedProfileRef.current;
            const emergencyFraction = 0.92;

            // ── Silence detection: if >3 seconds of silence, emergency load ──
            if (es.silence_frames > 90 && !transitionLockRef.current) {
                // 90 snapshots at 30Hz = ~3 seconds of silence
                console.warn("AutoDJ: silence detected for >3s, triggering emergency load");
                autodjPhaseRef.current = "planning";
                setAutodjStatus("error", "Silence detected — loading next track...");
                const others = analyzedTracks.filter(t => t.id !== playingSnap.track_id);
                if (others.length > 0) {
                    const pick = others[Math.floor(Math.random() * others.length)];
                    await state.loadTrack(idleDeck, pick.id);
                    await new Promise(r => setTimeout(r, 500));
                    transitionLockRef.current = true;
                    autodjPhaseRef.current = "transitioning";
                    try {
                        await state.executeTransition(idleDeck.toUpperCase(), "filter_sweep", 4);
                    } catch (e) {
                        console.error("Silence recovery transition failed:", e);
                    }
                    setTimeout(() => {
                        transitionLockRef.current = false;
                        autodjPhaseRef.current = "monitoring";
                        cachedProfileRef.current = null;
                        cachedProfileTrackIdRef.current = null;
                        state.clearAiDecision();
                        setAutodjStatus("monitoring", "Recovered from silence");
                    }, 3000);
                }
                return;
            }

            if (autodjPhaseRef.current !== "monitoring") {
                // Emergency safety: track almost done and no transition started
                if (positionFrac > emergencyFraction && !transitionLockRef.current) {
                    if (idleSnap.state !== "empty" && idleSnap.state !== "playing") {
                        transitionLockRef.current = true;
                        autodjPhaseRef.current = "transitioning";
                        setAutodjStatus("transitioning", "Emergency transition!");
                        try {
                            await state.executeTransition(idleDeck.toUpperCase(), "filter_sweep", 8);
                        } catch (e) {
                            console.error("Emergency transition failed:", e);
                        }
                        setTimeout(() => {
                            transitionLockRef.current = false;
                            autodjPhaseRef.current = "monitoring";
                            pendingTrackRef.current = null;
                            cachedProfileRef.current = null;
                            cachedProfileTrackIdRef.current = null;
                            state.clearAiDecision();
                            setAutodjStatus("monitoring", "Monitoring playback...");
                        }, 5000);
                    }
                }
                return;
            }

            // ── Section-aware transition trigger ──
            // Use transition windows from track profile if available, else fallback to 75%
            let shouldTrigger = false;
            let triggerReason = "";

            if (profile?.transition_windows?.length > 0) {
                // Find the best upcoming window whose start is within 30 seconds
                const futureWindows = (profile.transition_windows as Array<{anchor: string, start_ms: number, end_ms: number, score: number}>)
                    .filter((w) => w.start_ms > positionMs && w.start_ms - positionMs < 30_000)
                    .sort((a, b) => b.score - a.score);

                if (futureWindows.length > 0) {
                    const best = futureWindows[0];
                    // Trigger if we're within 2 seconds of window start
                    if (best.start_ms - positionMs < 2000) {
                        shouldTrigger = true;
                        triggerReason = `window:${best.anchor}@${best.start_ms}ms(score=${best.score.toFixed(2)})`;
                    }
                } else if (positionFrac >= 0.75) {
                    // No upcoming windows within 30s but past 75% — fallback
                    shouldTrigger = true;
                    triggerReason = "fallback:75%";
                }
            } else {
                // No profile data — use simple fraction trigger
                if (positionFrac >= 0.75) {
                    shouldTrigger = true;
                    triggerReason = "classic:75%";
                }
            }

            if (shouldTrigger && !state.isThinking && !transitionLockRef.current) {
                autodjPhaseRef.current = "planning";
                setAutodjStatus("selecting_next", `AI selecting... (${triggerReason})`);

                const decision = await state.requestNextTrack();

                if (!decision || !decision.track_selection) {
                    if (analyzedTracks.length > 1) {
                        const currentTrackId = playingSnap.track_id;
                        const others = analyzedTracks.filter(t => t.id !== currentTrackId);
                        const pick = others[Math.floor(Math.random() * others.length)];
                        pendingTrackRef.current = pick.id;
                    } else if (analyzedTracks.length === 1) {
                        pendingTrackRef.current = analyzedTracks[0].id;
                    } else {
                        autodjPhaseRef.current = "monitoring";
                        setAutodjStatus("error", "No tracks to transition to");
                        return;
                    }
                } else {
                    pendingTrackRef.current = decision.track_selection;
                }

                // Load the next track onto the idle deck
                const nextTrackId = pendingTrackRef.current!;
                const nextTrack = state.tracks.find(t => t.id === nextTrackId);
                setAutodjStatus("loading_next", `Loading: ${nextTrack?.title || "next track"}...`);
                autodjPhaseRef.current = "loading";

                await state.loadTrack(idleDeck, nextTrackId);
                await new Promise(r => setTimeout(r, 300));

                // ── Phrase alignment: if we have phrase data, wait for next phrase boundary ──
                if (profile?.phrases?.boundaries_ms?.length > 0) {
                    autodjPhaseRef.current = "aligning";
                    setAutodjStatus("transitioning", "Aligning to phrase boundary...");

                    const boundaries = profile.phrases.boundaries_ms as number[];
                    const curPos = useStore.getState().engineState
                        ? (playingDeck === "a"
                            ? useStore.getState().engineState!.deck_a.position_ms
                            : useStore.getState().engineState!.deck_b.position_ms)
                        : positionMs;

                    const nextBoundary = boundaries.find((b: number) => b > curPos);
                    if (nextBoundary && nextBoundary - curPos < 15_000) {
                        // Wait until we reach the phrase boundary (check every 100ms)
                        const waitForBoundary = () => new Promise<void>(resolve => {
                            const check = setInterval(() => {
                                const snap = useStore.getState().engineState;
                                if (!snap) { clearInterval(check); resolve(); return; }
                                const nowPos = playingDeck === "a" ? snap.deck_a.position_ms : snap.deck_b.position_ms;
                                if (nowPos >= nextBoundary - 100) {
                                    clearInterval(check);
                                    resolve();
                                }
                            }, 100);
                            // Safety timeout: don't wait more than 15 seconds
                            setTimeout(() => { resolve(); }, 15_000);
                        });
                        await waitForBoundary();
                    }
                }

                // Execute the transition
                transitionLockRef.current = true;
                autodjPhaseRef.current = "transitioning";

                const plan = state.aiDecision?.transition_plan;
                const templateName = plan?.template_name || "smooth_beatmix";
                const overrides = plan?.parameter_overrides || {};
                const durationBars = overrides.duration_bars || 16;

                setAutodjStatus("transitioning", `${templateName} → ${nextTrack?.title || "Unknown"}`);

                try {
                    await state.executeTransition(idleDeck.toUpperCase(), templateName, durationBars);
                } catch (e) {
                    console.error("Transition failed:", e);
                }

                // Wait for the transition to play out, then reset for next cycle
                const bpm = es.master_bpm || 128;
                const transitionMs = (durationBars / bpm) * 60 * 4 * 1000;
                setTimeout(() => {
                    transitionLockRef.current = false;
                    autodjPhaseRef.current = "monitoring";
                    pendingTrackRef.current = null;
                    cachedProfileRef.current = null;
                    cachedProfileTrackIdRef.current = null;
                    useStore.getState().clearAiDecision();
                    const curEs = useStore.getState().engineState;
                    const nowPlayingId = idleDeck === "a" ? curEs?.deck_a?.track_id : curEs?.deck_b?.track_id;
                    const nowTrack = useStore.getState().tracks.find(t => t.id === nowPlayingId);
                    setAutodjStatus("monitoring", `Playing: ${nowTrack?.title || "Unknown"}`);
                }, Math.min(transitionMs + 1000, 60000));
            }
        };

        const interval = setInterval(tick, 500);
        tick();

        return () => clearInterval(interval);
    }, [mode]);

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
                        {/* Phase 20: Reasoning tags + confidence */}
                        {aiDecision?.reasoning_tags && aiDecision.reasoning_tags.length > 0 && (
                            <div style={{ display: "flex", gap: "4px", flexWrap: "wrap", marginTop: "4px" }}>
                                {aiDecision.reasoning_tags.map((tag, i) => (
                                    <span key={i} style={{
                                        fontSize: "9px",
                                        padding: "1px 6px",
                                        borderRadius: "8px",
                                        background: "rgba(var(--accent-rgb, 100, 100, 255), 0.2)",
                                        color: "var(--text-muted)",
                                        border: "1px solid rgba(255,255,255,0.1)",
                                    }}>{tag}</span>
                                ))}
                                {aiDecision.confidence != null && (
                                    <span style={{
                                        fontSize: "9px",
                                        padding: "1px 6px",
                                        borderRadius: "8px",
                                        background: aiDecision.confidence > 0.7 ? "rgba(76,175,80,0.2)" : "rgba(255,152,0,0.2)",
                                        color: "var(--text-muted)",
                                    }}>conf: {(aiDecision.confidence * 100).toFixed(0)}%</span>
                                )}
                            </div>
                        )}
                        {/* Phase 20: Action timeline preview */}
                        {aiDecision?.actions && aiDecision.actions.length > 0 && (
                            <div style={{
                                marginTop: "6px",
                                padding: "6px 8px",
                                background: "rgba(0,0,0,0.2)",
                                borderRadius: "6px",
                                fontSize: "10px",
                                maxHeight: "80px",
                                overflowY: "auto",
                            }}>
                                <div style={{ color: "var(--text-muted)", marginBottom: "3px", textTransform: "uppercase", letterSpacing: "0.5px", fontSize: "9px" }}>
                                    Action Timeline ({aiDecision.actions.length} steps)
                                </div>
                                {aiDecision.actions.slice(0, 6).map((action, i) => (
                                    <div key={i} style={{ color: "var(--text-secondary)", display: "flex", gap: "6px" }}>
                                        <span style={{ color: "var(--text-muted)", minWidth: "40px" }}>{(action.at_ms / 1000).toFixed(1)}s</span>
                                        <span>{action.kind?.type ?? "?"}</span>
                                    </div>
                                ))}
                                {aiDecision.actions.length > 6 && (
                                    <div style={{ color: "var(--text-muted)" }}>+{aiDecision.actions.length - 6} more…</div>
                                )}
                            </div>
                        )}
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
                    <div className="autodj-status-area">
                        <div className="ai-listening-indicator">
                            <div className={`listening-dot ${autodjStatus === "transitioning" ? "transitioning-dot" : autodjStatus === "error" ? "error-dot" : ""}`}></div>
                            <span>{autodjMessage || "AI is DJing"}</span>
                        </div>
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
                        <option value="drop_swap">Drop Swap</option>
                        <option value="loop_roll">Loop Roll</option>
                        <option value="bass_kill_swap">Bass Kill Swap</option>
                        <option value="break_blend">Break Blend</option>
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

// ─── Settings Panel ─────────────────────────────────────────────

function SettingsPanel() {
    const { aiConfig, setAiConfig, audioDevices, getAudioDevices, savePreference, loadPreference } = useStore();
    const [apiKey, setApiKey] = useState("");
    const [isSaving, setIsSaving] = useState(false);
    const [defaultTemplate, setDefaultTemplate] = useState("smooth_beatmix");
    const [defaultDuration, setDefaultDuration] = useState(16);
    const [settingsTab, setSettingsTab] = useState<"ai" | "audio" | "autodj">("ai");

    // Load audio devices and saved preferences
    useEffect(() => {
        getAudioDevices();
        loadPreference("default_template").then(v => { if (v) setDefaultTemplate(v); });
        loadPreference("default_duration").then(v => { if (v) setDefaultDuration(Number(v)); });
    }, []);

    const handleSaveAi = async () => {
        setIsSaving(true);
        try {
            await setAiConfig({
                ...aiConfig,
                enabled: true,
                api_key: apiKey,
                model: "gemini-1.5-pro",
                endpoint_url: "https://generativelanguage.googleapis.com/v1beta/models"
            });
            setApiKey("");
        } catch (err) {
            console.error("Failed to save AI config:", err);
        } finally {
            setIsSaving(false);
        }
    };

    const handleSaveDefaults = async () => {
        await savePreference("default_template", defaultTemplate);
        await savePreference("default_duration", String(defaultDuration));
    };

    const tabStyle = (tab: string) => ({
        padding: "6px 14px",
        borderRadius: "6px",
        border: "none",
        background: settingsTab === tab ? "var(--bg-elevated)" : "transparent",
        color: settingsTab === tab ? "var(--accent-a)" : "var(--text-muted)",
        fontSize: "11px",
        fontWeight: 700 as const,
        cursor: "pointer" as const,
        textTransform: "uppercase" as const,
        letterSpacing: "0.5px",
    });

    const inputStyle = {
        padding: "10px",
        background: "rgba(0,0,0,0.2)",
        border: "1px solid var(--border)",
        borderRadius: "6px",
        color: "var(--text-primary)",
        fontFamily: "var(--font-mono)",
        fontSize: "13px",
        width: "100%",
    };

    const labelStyle = {
        fontSize: "11px",
        color: "var(--text-muted)",
        fontWeight: 600 as const,
        textTransform: "uppercase" as const,
        letterSpacing: "0.5px",
    };

    return (
        <div style={{ display: "flex", flexDirection: "column", gap: "16px" }}>
            <h3 style={{ margin: 0, fontSize: "16px", fontFamily: "var(--font-display)", color: "var(--text-primary)" }}>
                Settings
            </h3>

            <div style={{ display: "flex", gap: "4px", background: "var(--bg-deepest)", padding: "4px", borderRadius: "8px" }}>
                <button style={tabStyle("ai")} onClick={() => setSettingsTab("ai")}>AI</button>
                <button style={tabStyle("audio")} onClick={() => setSettingsTab("audio")}>Audio</button>
                <button style={tabStyle("autodj")} onClick={() => setSettingsTab("autodj")}>AutoDJ</button>
            </div>

            {settingsTab === "ai" && (
                <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
                    <div style={{ display: "flex", flexDirection: "column", gap: "5px" }}>
                        <label style={labelStyle}>Gemini API Key</label>
                        {aiConfig?.api_key && apiKey === "" ? (
                            <div style={{ display: "flex", alignItems: "center", gap: "8px", background: "rgba(0,0,0,0.2)", padding: "10px", borderRadius: "6px", border: "1px solid var(--border)" }}>
                                <span style={{ fontFamily: "monospace", color: "var(--status-ready)", flex: 1, fontSize: "14px" }}>
                                    ✓ {aiConfig.api_key.substring(0, 4)}....{aiConfig.api_key.slice(-4)}
                                </span>
                                <button onClick={() => setApiKey(aiConfig.api_key)} style={{ background: "transparent", border: "1px solid var(--border)", color: "var(--text-secondary)", borderRadius: "4px", cursor: "pointer", padding: "4px 8px", fontSize: "10px", fontWeight: "bold" }}>
                                    EDIT
                                </button>
                            </div>
                        ) : (
                            <input
                                type="password"
                                placeholder="AIzeSy..."
                                value={apiKey}
                                onChange={(e) => setApiKey(e.target.value)}
                                style={inputStyle}
                            />
                        )}
                    </div>
                    <button
                        onClick={handleSaveAi}
                        disabled={isSaving || !apiKey}
                        style={{
                            padding: "10px",
                            background: apiKey ? "linear-gradient(135deg, var(--accent-a), #0099cc)" : "var(--bg-elevated)",
                            color: apiKey ? "var(--bg-deepest)" : "var(--text-muted)",
                            border: "none",
                            borderRadius: "6px",
                            fontWeight: "bold",
                            cursor: isSaving || !apiKey ? "not-allowed" : "pointer",
                        }}
                    >
                        {isSaving ? "Saving..." : "Save AI Credentials"}
                    </button>
                </div>
            )}

            {settingsTab === "audio" && (
                <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
                    <div style={{ display: "flex", flexDirection: "column", gap: "5px" }}>
                        <label style={labelStyle}>Output Device</label>
                        {audioDevices.length > 0 ? (
                            <select style={{ ...inputStyle, cursor: "pointer" }}>
                                {audioDevices.map((d, i) => (
                                    <option key={i} value={d}>{d}</option>
                                ))}
                            </select>
                        ) : (
                            <span style={{ fontSize: "12px", color: "var(--text-muted)", padding: "10px", background: "rgba(0,0,0,0.2)", borderRadius: "6px" }}>
                                No output devices detected
                            </span>
                        )}
                    </div>
                    <div style={{ fontSize: "11px", color: "var(--text-muted)", fontStyle: "italic" }}>
                        Audio device hot-swap requires app restart.
                    </div>
                </div>
            )}

            {settingsTab === "autodj" && (
                <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
                    <div style={{ display: "flex", flexDirection: "column", gap: "5px" }}>
                        <label style={labelStyle}>Default Transition Style</label>
                        <select
                            value={defaultTemplate}
                            onChange={(e) => setDefaultTemplate(e.target.value)}
                            style={{ ...inputStyle, cursor: "pointer" }}
                        >
                            <option value="smooth_beatmix">Smooth Beatmix</option>
                            <option value="filter_sweep">Filter Sweep</option>
                            <option value="echo_drop">Echo Drop</option>
                            <option value="drop_swap">Drop Swap</option>
                            <option value="loop_roll">Loop Roll</option>
                            <option value="bass_kill_swap">Bass Kill Swap</option>
                            <option value="break_blend">Break Blend</option>
                        </select>
                    </div>
                    <div style={{ display: "flex", flexDirection: "column", gap: "5px" }}>
                        <label style={labelStyle}>Default Transition Duration ({defaultDuration} bars)</label>
                        <input
                            type="range"
                            min={4} max={64} step={4}
                            value={defaultDuration}
                            onChange={(e) => setDefaultDuration(Number(e.target.value))}
                            style={{ width: "100%", cursor: "pointer" }}
                        />
                    </div>
                    <button
                        onClick={handleSaveDefaults}
                        style={{
                            padding: "10px",
                            background: "linear-gradient(135deg, var(--accent-a), #0099cc)",
                            color: "var(--bg-deepest)",
                            border: "none",
                            borderRadius: "6px",
                            fontWeight: "bold",
                            cursor: "pointer",
                        }}
                    >
                        Save Defaults
                    </button>
                </div>
            )}
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

        // ── Drag & Drop Listener ──
        const unlistenDrop = listen<any>("tauri://drop", (event) => {
            const paths = event.payload?.paths || (Array.isArray(event.payload) ? event.payload : undefined);
            if (paths && paths.length > 0) {
                useStore.getState().importFiles(paths);
            }
        });

        // ── Keyboard Shortcuts Listener ──
        const handleKeyDown = (e: KeyboardEvent) => {
            // Ignore if user is typing in an input
            if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
                return;
            }

            const state = useStore.getState();
            if (!state.engineState) return;

            const isDeckAPlaying = state.engineState.deck_a.state === "playing";
            const isDeckBPlaying = state.engineState.deck_b.state === "playing";

            if (e.code === "Space") {
                e.preventDefault();
                // Play/pause Deck A logic:
                if (isDeckAPlaying) {
                    state.pause("a");
                } else {
                    state.play("a");
                }
            }
            else if (e.code === "KeyZ") {
                e.preventDefault();
                if (state.engineState.deck_a.loop_active) {
                    state.clearLoop("a");
                } else {
                    state.setLoop("a", 4);
                }
            }
            else if (e.code === "KeyX") {
                e.preventDefault();
                if (state.engineState.deck_b.loop_active) {
                    state.clearLoop("b");
                } else {
                    state.setLoop("b", 4);
                }
            }
            else if (e.code === "KeyQ") {
                e.preventDefault();
                const currentLowA = state.engineState.deck_a.eq_low;
                state.setEq("a", "low", currentLowA <= -90 ? 0 : -96);
            }
            else if (e.code === "KeyW") {
                e.preventDefault();
                const currentLowB = state.engineState.deck_b.eq_low;
                state.setEq("b", "low", currentLowB <= -90 ? 0 : -96);
            }
        };

        window.addEventListener("keydown", handleKeyDown);

        return () => {
            unlisten.then(f => f());
            unlistenDrop.then(f => f());
            window.removeEventListener("keydown", handleKeyDown);
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
                        <SettingsPanel />
                    </div>
                </div>
            )}
        </div>
    );
}
