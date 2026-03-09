import { useEffect, useCallback, useState, useRef } from "react";
import { useStore, EngineState, DjMode, SessionRecord } from "./store";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import logo from "./assets/logo.png";

import { formatTime, formatGroove } from "./components/utils";
import { DeckPanel } from "./components/DeckPanel";
import { MixerControls } from "./components/MixerChannel";
import { AiBar, TransitionsPanel } from "./components/AiBar";
import { IconSettings, IconRecord } from "./components/Icons";

// ─── Library View ────────────────────────────────────────────────

function LibraryView() {
    const { tracks, importing, fetchTracks, importFiles, loadTrack, clearLibrary, analyzingTracks, libraryFilters, setLibraryFilters, clearLibraryFilters, analyzeTrack, classifyTrack, classifyAll, batchClassifying, batchClassifyProgress, aiConfig, playlists, fetchPlaylists, addTrackToPlaylist } = useStore();
    const searchTimerRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
    const [contextMenu, setContextMenu] = useState<{ x: number; y: number; trackId: string } | null>(null);
    const [playlistSubmenu, setPlaylistSubmenu] = useState(false);

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
                    <div className="context-menu-divider" />
                    <button
                        onMouseEnter={() => { if (!playlistSubmenu) { fetchPlaylists(); setPlaylistSubmenu(true); } }}
                        onMouseLeave={() => setPlaylistSubmenu(false)}
                        style={{ position: "relative" }}
                    >
                        Add to Playlist ▸
                        {playlistSubmenu && (
                            <div className="context-submenu">
                                {playlists.length === 0 ? (
                                    <span style={{ padding: "4px 8px", color: "var(--text-muted)", fontSize: "12px" }}>No playlists</span>
                                ) : playlists.map(pl => (
                                    <button key={pl.id} onClick={(e) => {
                                        e.stopPropagation();
                                        addTrackToPlaylist(pl.id, contextMenu.trackId);
                                        setContextMenu(null);
                                        setPlaylistSubmenu(false);
                                    }}>
                                        {pl.name}
                                    </button>
                                ))}
                            </div>
                        )}
                    </button>
                </div>
            )}
        </div>
    );
}

// ─── Playlist View ───────────────────────────────────────────────

function PlaylistView() {
    const { playlists, fetchPlaylists, createPlaylist, renamePlaylist, deletePlaylist, selectedPlaylistId, selectPlaylist, playlistTracks, removeTrackFromPlaylist, reorderPlaylist, loadTrack, loadPlaylistToQueue, autodjQueue, autodjQueueIndex, getSessions, exportCueSheet } = useStore();
    const [newName, setNewName] = useState("");
    const [editingId, setEditingId] = useState<string | null>(null);
    const [editName, setEditName] = useState("");
    const [dragIdx, setDragIdx] = useState<number | null>(null);
    const [showExport, setShowExport] = useState(false);
    const [sessions, setSessions] = useState<SessionRecord[]>([]);
    const [cueSheet, setCueSheet] = useState<string | null>(null);

    useEffect(() => { fetchPlaylists(); }, [fetchPlaylists]);

    const handleCreate = async () => {
        const name = newName.trim();
        if (!name) return;
        await createPlaylist(name);
        setNewName("");
    };

    const handleRename = async (id: string) => {
        const name = editName.trim();
        if (!name) return;
        await renamePlaylist(id, name);
        setEditingId(null);
        setEditName("");
    };

    const handleDelete = async (id: string) => {
        if (!confirm("Delete this playlist?")) return;
        await deletePlaylist(id);
    };

    // Drag-and-drop reorder
    const handleDragStart = (idx: number) => setDragIdx(idx);
    const handleDragOver = (e: React.DragEvent, _idx: number) => e.preventDefault();
    const handleDrop = async (targetIdx: number) => {
        if (dragIdx === null || dragIdx === targetIdx || !selectedPlaylistId) return;
        const ids = playlistTracks.map(t => t.id);
        const [moved] = ids.splice(dragIdx, 1);
        ids.splice(targetIdx, 0, moved);
        await reorderPlaylist(selectedPlaylistId, ids);
        setDragIdx(null);
    };

    const handleLoadQueue = (shuffle: boolean) => {
        if (!selectedPlaylistId) return;
        loadPlaylistToQueue(selectedPlaylistId, shuffle);
    };

    const handleShowExport = async () => {
        const s = await getSessions();
        setSessions(s);
        setShowExport(true);
        setCueSheet(null);
    };

    const handleExport = async (sessionId: string) => {
        const text = await exportCueSheet(sessionId);
        setCueSheet(text);
    };

    const selectedPlaylist = playlists.find(p => p.id === selectedPlaylistId);
    const queueRemaining = autodjQueue.length - autodjQueueIndex;

    return (
        <div className="playlist-view">
            <div className="playlist-sidebar">
                <div className="playlist-sidebar-header">
                    <h3>Playlists</h3>
                    <button className="playlist-export-btn" onClick={handleShowExport} title="Export cue sheet">📋</button>
                </div>
                <div className="playlist-create">
                    <input
                        type="text"
                        placeholder="New playlist..."
                        value={newName}
                        onChange={e => setNewName(e.target.value)}
                        onKeyDown={e => e.key === "Enter" && handleCreate()}
                        className="playlist-name-input"
                    />
                    <button onClick={handleCreate} disabled={!newName.trim()} className="playlist-create-btn">+</button>
                </div>
                <div className="playlist-list">
                    {playlists.map(pl => (
                        <div
                            key={pl.id}
                            className={`playlist-item ${selectedPlaylistId === pl.id ? "selected" : ""}`}
                            onClick={() => selectPlaylist(pl.id)}
                        >
                            {editingId === pl.id ? (
                                <input
                                    type="text"
                                    value={editName}
                                    onChange={e => setEditName(e.target.value)}
                                    onKeyDown={e => { if (e.key === "Enter") handleRename(pl.id); if (e.key === "Escape") setEditingId(null); }}
                                    onBlur={() => handleRename(pl.id)}
                                    autoFocus
                                    className="playlist-rename-input"
                                    onClick={e => e.stopPropagation()}
                                />
                            ) : (
                                <>
                                    <span className="playlist-item-name">{pl.name}</span>
                                    <span className="playlist-item-count">{pl.track_count}</span>
                                    <button className="playlist-item-edit" onClick={e => { e.stopPropagation(); setEditingId(pl.id); setEditName(pl.name); }} title="Rename">✏️</button>
                                    <button className="playlist-item-delete" onClick={e => { e.stopPropagation(); handleDelete(pl.id); }} title="Delete">🗑</button>
                                </>
                            )}
                        </div>
                    ))}
                    {playlists.length === 0 && (
                        <div className="playlist-empty">No playlists yet</div>
                    )}
                </div>
            </div>

            <div className="playlist-content">
                {selectedPlaylist ? (
                    <>
                        <div className="playlist-content-header">
                            <h3>{selectedPlaylist.name}</h3>
                            <span className="playlist-track-count">{playlistTracks.length} tracks</span>
                            <div style={{ marginLeft: "auto", display: "flex", gap: "8px" }}>
                                <button className="playlist-queue-btn" onClick={() => handleLoadQueue(false)} title="Load in order">
                                    ▶ Queue
                                </button>
                                <button className="playlist-queue-btn shuffle" onClick={() => handleLoadQueue(true)} title="Load shuffled">
                                    🔀 Shuffle
                                </button>
                            </div>
                        </div>
                        {queueRemaining > 0 && (
                            <div className="playlist-queue-status">
                                AutoDJ queue: {queueRemaining} tracks remaining
                            </div>
                        )}
                        <div className="playlist-tracks">
                            <div className="playlist-tracks-header">
                                <span className="pl-col-num">#</span>
                                <span className="pl-col-title">Title</span>
                                <span className="pl-col-artist">Artist</span>
                                <span className="pl-col-bpm">BPM</span>
                                <span className="pl-col-key">Key</span>
                                <span className="pl-col-dur">Duration</span>
                                <span className="pl-col-actions"></span>
                            </div>
                            {playlistTracks.map((track, idx) => (
                                <div
                                    key={track.id}
                                    className={`playlist-track-row ${dragIdx === idx ? "dragging" : ""}`}
                                    draggable
                                    onDragStart={() => handleDragStart(idx)}
                                    onDragOver={(e) => handleDragOver(e, idx)}
                                    onDrop={() => handleDrop(idx)}
                                >
                                    <span className="pl-col-num">{idx + 1}</span>
                                    <span className="pl-col-title">{track.title || "Unknown"}</span>
                                    <span className="pl-col-artist">{track.artist || "Unknown"}</span>
                                    <span className="pl-col-bpm">{track.bpm?.toFixed(1) ?? "—"}</span>
                                    <span className="pl-col-key">{track.key || "—"}</span>
                                    <span className="pl-col-dur">{track.duration_ms ? formatTime(track.duration_ms) : "—"}</span>
                                    <span className="pl-col-actions">
                                        <button className="deck-load-btn deck-a" onClick={() => loadTrack("a", track.id)}>A</button>
                                        <button className="deck-load-btn deck-b" onClick={() => loadTrack("b", track.id)}>B</button>
                                        <button className="playlist-remove-btn" onClick={() => removeTrackFromPlaylist(selectedPlaylistId!, track.id)} title="Remove">✕</button>
                                    </span>
                                </div>
                            ))}
                            {playlistTracks.length === 0 && (
                                <div className="playlist-empty-content">
                                    No tracks in this playlist. Right-click tracks in the Library to add them.
                                </div>
                            )}
                        </div>
                    </>
                ) : (
                    <div className="playlist-empty-content">
                        {showExport ? (
                            <div className="cue-sheet-panel">
                                <h3>Export Cue Sheet</h3>
                                <p style={{ color: "var(--text-secondary)", marginBottom: "12px" }}>Select a session to export its tracklist:</p>
                                {sessions.length === 0 ? (
                                    <p style={{ color: "var(--text-muted)" }}>No sessions recorded yet.</p>
                                ) : (
                                    <div className="session-list">
                                        {sessions.map(s => {
                                            const date = new Date(s.started_at * 1000);
                                            return (
                                                <button key={s.id} className="session-item" onClick={() => handleExport(s.id)}>
                                                    <span>{date.toLocaleDateString()} {date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}</span>
                                                    <span className="session-info">{s.track_count ?? 0} tracks</span>
                                                </button>
                                            );
                                        })}
                                    </div>
                                )}
                                {cueSheet && (
                                    <div className="cue-sheet-output">
                                        <pre>{cueSheet}</pre>
                                        <button className="playlist-queue-btn" onClick={() => navigator.clipboard.writeText(cueSheet)}>
                                            📋 Copy to Clipboard
                                        </button>
                                    </div>
                                )}
                                <button className="filter-clear-btn" onClick={() => { setShowExport(false); setCueSheet(null); }} style={{ marginTop: "12px" }}>
                                    ← Back
                                </button>
                            </div>
                        ) : (
                            <>
                                <div className="empty-state-icon">🎧</div>
                                <div>Select a playlist or create a new one</div>
                            </>
                        )}
                    </div>
                )}
            </div>
        </div>
    );
}

// ─── DJ View ─────────────────────────────────────────────────────

function DJView() {
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

// ─── AutoDJ Activity Panel ──────────────────────────────────────

function AutoDjPanel() {
    const { conductorState, tracks, engineState, aiDecision } = useStore();
    const cs = conductorState;

    const templateScores = cs?.template_scores ?? aiDecision?.template_scores ?? null;
    const reasoningTags = cs?.reasoning_tags ?? [];
    const pendingTitle = cs?.pending_track_title ?? "—";
    const phase = cs?.phase ?? "idle";
    const message = cs?.message ?? "";
    const confidence = cs?.confidence ?? aiDecision?.confidence ?? null;
    const entryMarker = cs?.entry_marker ?? aiDecision?.entry_marker ?? null;

    // Current playing track info
    const activeDeckSnap = engineState?.deck_a?.state === "playing" ? engineState?.deck_a
        : engineState?.deck_b?.state === "playing" ? engineState?.deck_b : null;
    const currentTrack = activeDeckSnap?.track_id
        ? tracks.find((t) => t.id === activeDeckSnap.track_id) : null;
    const pendingTrack = cs?.pending_track_id
        ? tracks.find((t) => t.id === cs.pending_track_id) : null;

    const phaseLabel: Record<string, string> = {
        idle: "Idle",
        monitoring: "Monitoring",
        planning: "Planning Next",
        loading: "Loading Track",
        aligning: "Aligning BPM",
        transitioning: "Transitioning",
    };

    const phaseColor: Record<string, string> = {
        idle: "var(--text-muted)",
        monitoring: "var(--status-ready)",
        planning: "var(--status-pending)",
        loading: "var(--accent-a)",
        aligning: "var(--accent-b)",
        transitioning: "var(--status-active)",
    };

    return (
        <div className="autodj-panel">
            {/* Left: Session Status */}
            <div className="autodj-panel-section autodj-session-status">
                <span className="autodj-panel-header">SESSION</span>
                <div className="autodj-phase-row">
                    <span className="autodj-phase-dot" style={{ background: phaseColor[phase] || "var(--text-muted)" }} />
                    <span className="autodj-phase-label">{phaseLabel[phase] || phase}</span>
                </div>
                {message && <span className="autodj-phase-message">{message}</span>}
                {confidence != null && (
                    <div className="autodj-stat-row">
                        <span className="autodj-stat-label">Confidence</span>
                        <span className="autodj-stat-value">{Math.round(confidence * 100)}%</span>
                    </div>
                )}
                {entryMarker && (
                    <div className="autodj-stat-row">
                        <span className="autodj-stat-label">Entry Point</span>
                        <span className="autodj-stat-value autodj-stat-marker">{entryMarker}</span>
                    </div>
                )}
                {currentTrack && (
                    <div className="autodj-stat-row">
                        <span className="autodj-stat-label">Now</span>
                        <span className="autodj-stat-value" title={currentTrack.title || ""}>{currentTrack.title || "—"}</span>
                    </div>
                )}
                {pendingTrack && (
                    <div className="autodj-stat-row">
                        <span className="autodj-stat-label">Next</span>
                        <span className="autodj-stat-value" title={pendingTrack.title || ""}>{pendingTrack.title || pendingTitle}</span>
                    </div>
                )}
            </div>

            {/* Center: Template Scores */}
            {templateScores && templateScores.length > 0 && (
                <div className="autodj-panel-section autodj-template-scores">
                    <span className="autodj-panel-header">TEMPLATE SCORES</span>
                    <div className="autodj-scores-grid">
                        {templateScores.slice(0, 8).map(([name, score]) => (
                            <div key={name} className="autodj-score-row">
                                <span className="autodj-score-name">
                                    {name.replace(/_/g, " ")}
                                </span>
                                <div className="autodj-score-bar-track">
                                    <div
                                        className="autodj-score-bar-fill"
                                        style={{ width: `${Math.min(score * 100, 100)}%` }}
                                    />
                                </div>
                                <span className="autodj-score-value">{(score * 100).toFixed(0)}</span>
                            </div>
                        ))}
                    </div>
                </div>
            )}

            {/* Right: Reasoning Tags */}
            {reasoningTags.length > 0 && (
                <div className="autodj-panel-section autodj-reasoning">
                    <span className="autodj-panel-header">AI REASONING</span>
                    <div className="autodj-tags-wrap">
                        {reasoningTags.map((tag, i) => (
                            <span key={i} className="autodj-reasoning-tag">
                                {tag.replace(/_/g, " ")}
                            </span>
                        ))}
                    </div>
                </div>
            )}
        </div>
    );
}

// ─── Settings Panel ─────────────────────────────────────────────

function SettingsPanel() {
    const { aiConfig, setAiConfig, audioDevices, getAudioDevices, savePreference, loadPreference } = useStore();
    const [apiKey, setApiKey] = useState("");
    const [provider, setProvider] = useState<"gemini" | "groq" | "ollama">(
        aiConfig?.provider === "groq" ? "groq" : aiConfig?.provider === "ollama" ? "ollama" : "gemini"
    );
    const [ollamaModel, setOllamaModel] = useState(aiConfig?.provider === "ollama" ? (aiConfig?.model || "qwen2.5:7b") : "qwen2.5:7b");
    const [ollamaEndpoint, setOllamaEndpoint] = useState("http://localhost:11434/v1");
    const [isSaving, setIsSaving] = useState(false);
    const [defaultTemplate, setDefaultTemplate] = useState("smooth_beatmix");
    const [defaultDuration, setDefaultDuration] = useState(16);
    const [settingsTab, setSettingsTab] = useState<"ai" | "audio" | "autodj">("ai");

    // Sync provider from config when it changes externally
    useEffect(() => {
        if (aiConfig?.provider) {
            setProvider(aiConfig.provider === "groq" ? "groq" : aiConfig.provider === "ollama" ? "ollama" : "gemini");
            if (aiConfig.provider === "ollama" && aiConfig.model) setOllamaModel(aiConfig.model);
            if (aiConfig.provider === "ollama" && aiConfig.endpoint_url) setOllamaEndpoint(aiConfig.endpoint_url);
        }
    }, [aiConfig?.provider]);

    const providerDefaults = {
        gemini: { model: "gemini-2.5-flash", endpoint: "https://generativelanguage.googleapis.com/v1beta/models", placeholder: "AIzaSy...", needsKey: true },
        groq: { model: "llama-3.3-70b-versatile", endpoint: "https://api.groq.com/openai/v1", placeholder: "gsk_...", needsKey: true },
        ollama: { model: "qwen2.5:7b", endpoint: "http://localhost:11434/v1", placeholder: "", needsKey: false },
    };

    // Load audio devices and saved preferences
    useEffect(() => {
        getAudioDevices();
        loadPreference("default_template").then(v => { if (v) setDefaultTemplate(v); });
        loadPreference("default_duration").then(v => { if (v) setDefaultDuration(Number(v)); });
    }, []);

    const handleSaveAi = async () => {
        setIsSaving(true);
        try {
            const defaults = providerDefaults[provider];
            if (provider === "ollama") {
                await setAiConfig({
                    ...aiConfig,
                    enabled: true,
                    provider: "ollama",
                    api_key: null,
                    model: ollamaModel || defaults.model,
                    endpoint_url: ollamaEndpoint || defaults.endpoint,
                });
            } else {
                await setAiConfig({
                    ...aiConfig,
                    enabled: true,
                    provider,
                    api_key: apiKey,
                    model: defaults.model,
                    endpoint_url: defaults.endpoint,
                });
                setApiKey("");
            }
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
                        <label style={labelStyle}>AI Provider</label>
                        <div style={{ display: "flex", gap: "4px", background: "rgba(0,0,0,0.2)", padding: "3px", borderRadius: "6px", border: "1px solid var(--border)" }}>
                            {(["gemini", "groq", "ollama"] as const).map(p => (
                                <button
                                    key={p}
                                    onClick={() => { setProvider(p); setApiKey(""); }}
                                    style={{
                                        flex: 1,
                                        padding: "8px",
                                        border: "none",
                                        borderRadius: "4px",
                                        background: provider === p ? (p === "ollama" ? "var(--status-ready)" : "var(--accent-a)") : "transparent",
                                        color: provider === p ? "var(--bg-deepest)" : "var(--text-muted)",
                                        fontWeight: 700,
                                        fontSize: "12px",
                                        cursor: "pointer",
                                        textTransform: "uppercase" as const,
                                        letterSpacing: "0.5px",
                                    }}
                                >
                                    {p === "gemini" ? "Gemini" : p === "groq" ? "Groq" : "Ollama"}
                                </button>
                            ))}
                        </div>
                    </div>
                    {provider === "ollama" ? (
                        <>
                            <div style={{ display: "flex", flexDirection: "column", gap: "5px" }}>
                                <label style={labelStyle}>Ollama Model</label>
                                <input
                                    type="text"
                                    placeholder="qwen2.5:7b"
                                    value={ollamaModel}
                                    onChange={(e) => setOllamaModel(e.target.value)}
                                    style={inputStyle}
                                />
                            </div>
                            <div style={{ display: "flex", flexDirection: "column", gap: "5px" }}>
                                <label style={labelStyle}>Ollama Endpoint</label>
                                <input
                                    type="text"
                                    placeholder="http://localhost:11434/v1"
                                    value={ollamaEndpoint}
                                    onChange={(e) => setOllamaEndpoint(e.target.value)}
                                    style={inputStyle}
                                />
                            </div>
                            <div style={{ fontSize: "11px", color: "var(--text-muted)", padding: "6px 8px", background: "rgba(0,0,0,0.15)", borderRadius: "4px", lineHeight: 1.5 }}>
                                Local inference &mdash; no API key needed. Make sure Ollama is running.
                                <br />
                                <span style={{ color: "var(--text-secondary)", fontFamily: "var(--font-mono)" }}>ollama pull {ollamaModel}</span> to install the model.
                            </div>
                        </>
                    ) : (
                        <>
                            <div style={{ display: "flex", flexDirection: "column", gap: "5px" }}>
                                <label style={labelStyle}>{provider === "gemini" ? "Gemini" : "Groq"} API Key</label>
                                {aiConfig?.api_key && aiConfig?.provider === provider && apiKey === "" ? (
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
                                        placeholder={providerDefaults[provider].placeholder}
                                        value={apiKey}
                                        onChange={(e) => setApiKey(e.target.value)}
                                        style={inputStyle}
                                    />
                                )}
                            </div>
                            <div style={{ fontSize: "11px", color: "var(--text-muted)", padding: "6px 8px", background: "rgba(0,0,0,0.15)", borderRadius: "4px", lineHeight: 1.5 }}>
                                {provider === "gemini"
                                    ? <>Model: <span style={{ color: "var(--text-secondary)", fontFamily: "var(--font-mono)" }}>{providerDefaults.gemini.model}</span></>
                                    : <>Model: <span style={{ color: "var(--text-secondary)", fontFamily: "var(--font-mono)" }}>{providerDefaults.groq.model}</span> &mdash; ultra-fast inference via Groq</>
                                }
                            </div>
                        </>
                    )}
                    <button
                        onClick={handleSaveAi}
                        disabled={isSaving || (provider !== "ollama" && !apiKey)}
                        style={{
                            padding: "10px",
                            background: (provider === "ollama" || apiKey) ? "linear-gradient(135deg, var(--accent-a), #0099cc)" : "var(--bg-elevated)",
                            color: (provider === "ollama" || apiKey) ? "var(--bg-deepest)" : "var(--text-muted)",
                            border: "none",
                            borderRadius: "6px",
                            fontWeight: "bold",
                            cursor: isSaving || (provider !== "ollama" && !apiKey) ? "not-allowed" : "pointer",
                        }}
                    >
                        {isSaving ? "Saving..." : provider === "ollama" ? "Save Ollama Config" : "Save AI Credentials"}
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
                            <option value="echo_drop">Echo Drop (Sim)</option>
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
    const { activeView, setActiveView, setEngineState, setConductorState } = useStore();
    const [isSettingsOpen, setIsSettingsOpen] = useState(false);

    useEffect(() => {
        // Load initial state elements
        useStore.getState().initStore();

        const unlisten = listen<EngineState>("engine-state", (event) => {
            setEngineState(event.payload);
        });

        // Listen for conductor state from the backend AutoDJ loop
        const unlistenConductor = listen<any>("conductor-state", (event) => {
            setConductorState(event.payload);
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
            unlistenConductor.then(f => f());
            unlistenDrop.then(f => f());
            window.removeEventListener("keydown", handleKeyDown);
        };
    }, [setEngineState, setConductorState]);

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
                        className={`nav-tab ${activeView === "playlists" ? "active" : ""}`}
                        onClick={() => setActiveView("playlists")}
                    >
                        Playlists
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
                        <IconSettings />
                    </button>
                </div>
            </header>

            <main className="main-content">
                {activeView === "library" ? <LibraryView /> : activeView === "playlists" ? <PlaylistView /> : <DJView />}
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
