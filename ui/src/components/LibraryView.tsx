import { useEffect, useCallback, useState, useRef } from "react";
import { useStore } from "../store";
import { open } from "@tauri-apps/plugin-dialog";
import { formatTime, formatGroove } from "./utils";
import { SpotifyPanel } from "./SpotifyPanel";

export function LibraryView() {
    const { tracks, importing, fetchTracks, importFiles, loadTrack, clearLibrary, analyzingTracks, libraryFilters, setLibraryFilters, clearLibraryFilters, analyzeTrack, classifyTrack, classifyAll, batchClassifying, batchClassifyProgress, aiConfig, playlists, fetchPlaylists, addTrackToPlaylist, spotifyStatus } = useStore();
    const searchTimerRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
    const [contextMenu, setContextMenu] = useState<{ x: number; y: number; trackId: string } | null>(null);
    const [playlistSubmenu, setPlaylistSubmenu] = useState(false);
    const [showSpotify, setShowSpotify] = useState(false);

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
                        {importing ? "\u27F3 Importing..." : "+ Import Files"}
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
                    {aiConfig?.enabled && (
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
                    {spotifyStatus?.connected && (
                        <button
                            onClick={() => setShowSpotify(s => !s)}
                            style={{
                                padding: "8px 16px",
                                backgroundColor: showSpotify ? "#1DB954" : "rgba(29, 185, 84, 0.3)",
                                color: "white",
                                border: "none",
                                borderRadius: "4px",
                                cursor: "pointer",
                                fontSize: "13px",
                                fontWeight: 600,
                            }}
                        >
                            Spotify
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
                                        {track.bpm ? track.bpm.toFixed(1) : "\u2014"}
                                        {track.perceived_tempo && track.perceived_tempo !== track.bpm ? (
                                            <span style={{ fontSize: "10px", color: "var(--accent)", marginLeft: "4px" }}>({track.perceived_tempo.toFixed(0)})</span>
                                        ) : null}
                                    </span>
                                    <span className="track-duration">
                                        {track.groove_type && track.groove_type !== "standard" ? (
                                            <span className="groove-badge">{formatGroove(track.groove_type)}</span>
                                        ) : "\u2014"}
                                    </span>
                                    <span className="track-duration">
                                        {track.key || "\u2014"}
                                    </span>
                                    <span className="track-duration">
                                        {track.genre ? <span className="genre-badge">{track.genre}</span> : "\u2014"}
                                    </span>
                                    <span className="track-duration">
                                        {track.mood ? <span className="mood-badge">{track.mood}</span> : "\u2014"}
                                    </span>
                                    <span className="track-duration">
                                        {track.duration_ms ? formatTime(track.duration_ms) : "\u2014"}
                                    </span>
                                    <span className="load-actions">
                                        <span
                                            className={`status-badge status-${track.import_status}`}
                                        >
                                            {track.import_status === "analyzed" ? "\u2713 Analyzed"
                                                : track.import_status === "error" || track.import_status === "analysis_failed" ? "\u2715 Error"
                                                    : track.import_status === "ready" ? "\u2713 Uploaded"
                                                        : "\u27F3 Loading..."}
                                        </span>
                                        {isAnalyzing && (
                                            <span className="status-badge" style={{ marginLeft: "4px", color: "#00d4ff", background: "#00d4ff20" }}>
                                                {"⟳ Analyzing..."}
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
                        {"Add to Playlist ▸"}
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

            {showSpotify && <SpotifyPanel />}
        </div>
    );
}
