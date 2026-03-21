import { useEffect, useCallback, useState, useRef } from "react";
import { useStore } from "../store";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { formatTime } from "./utils";

// In-memory album art cache
const artCache = new Map<string, string | null>();

function TrackCard({
    track,
    onLoad,
    onContextMenu,
}: {
    track: any;
    onLoad: (trackId: string) => void;
    onContextMenu: (e: React.MouseEvent, trackId: string) => void;
}) {
    const [artUrl, setArtUrl] = useState<string | null>(artCache.get(track.id) ?? null);
    const [artLoaded, setArtLoaded] = useState(artCache.has(track.id));
    const cardRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        if (artLoaded) return;

        const observer = new IntersectionObserver(
            (entries) => {
                if (entries[0].isIntersecting) {
                    observer.disconnect();
                    invoke<string | null>("get_album_art", { trackId: track.id })
                        .then((path) => {
                            const url = path ? convertFileSrc(path) : null;
                            artCache.set(track.id, url);
                            setArtUrl(url);
                            setArtLoaded(true);
                        })
                        .catch(() => {
                            artCache.set(track.id, null);
                            setArtLoaded(true);
                        });
                }
            },
            { threshold: 0.1 }
        );

        if (cardRef.current) observer.observe(cardRef.current);
        return () => observer.disconnect();
    }, [track.id, artLoaded]);

    return (
        <div
            ref={cardRef}
            className="track-card"
            onClick={() => onLoad(track.id)}
            onContextMenu={(e) => onContextMenu(e, track.id)}
        >
            <div className="track-card-art-wrap">
                {artUrl ? (
                    <img src={artUrl} alt="" className="track-card-art" />
                ) : (
                    <div className="track-card-art track-card-art-placeholder">
                        <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1" opacity="0.3">
                            <circle cx="12" cy="12" r="10" />
                            <circle cx="12" cy="12" r="3" />
                        </svg>
                    </div>
                )}
                <div className="track-card-badges">
                    {track.bpm && <span className="track-card-badge">{track.bpm.toFixed(0)}</span>}
                    {track.key && <span className="track-card-badge">{track.key}</span>}
                </div>
            </div>
            <div className="track-card-info">
                <span className="track-card-title">{track.title || "Unknown"}</span>
                <span className="track-card-artist">{track.artist || "Unknown"}</span>
            </div>
        </div>
    );
}

export function TrackCardGrid() {
    const {
        tracks,
        importing,
        fetchTracks,
        importFiles,
        loadTrack,
        clearLibrary,
        analyzingTracks,
        libraryFilters,
        setLibraryFilters,
        clearLibraryFilters,
        analyzeTrack,
        classifyTrack,
        classifyAll,
        batchClassifying,
        batchClassifyProgress,
        aiConfig,
        playlists,
        fetchPlaylists,
        addTrackToPlaylist,
        selectedPlaylistId,
        playlistTracks,
        spotifyStatus,
        engineState,
    } = useStore();

    const searchTimerRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
    const [contextMenu, setContextMenu] = useState<{ x: number; y: number; trackId: string } | null>(null);
    const [playlistSubmenu, setPlaylistSubmenu] = useState(false);

    useEffect(() => {
        fetchTracks();
    }, [fetchTracks]);

    // Determine which tracks to display
    const displayTracks = selectedPlaylistId ? playlistTracks : tracks;

    const handleSearchChange = useCallback((value: string) => {
        setLibraryFilters({ query: value });
        if (searchTimerRef.current) clearTimeout(searchTimerRef.current);
        searchTimerRef.current = setTimeout(() => {
            useStore.getState().fetchTracks();
        }, 300);
    }, [setLibraryFilters]);

    const handleFilterChange = useCallback((key: string, value: string) => {
        setLibraryFilters({ [key]: value } as any);
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
        if (confirm("Are you sure you want to clear your entire library?")) {
            await clearLibrary();
        }
    }, [clearLibrary]);

    const handleLoad = useCallback((trackId: string) => {
        // Load to next available deck
        const deckA = engineState?.deck_a;
        const deckB = engineState?.deck_b;
        if (!deckA || deckA.state === "empty") {
            loadTrack("a", trackId);
        } else if (!deckB || deckB.state === "empty") {
            loadTrack("b", trackId);
        } else {
            // Both loaded — load to standby deck
            const aPlaying = deckA.state === "playing";
            loadTrack(aPlaying ? "b" : "a", trackId);
        }
    }, [engineState, loadTrack]);

    const handleContextMenu = useCallback((e: React.MouseEvent, trackId: string) => {
        e.preventDefault();
        setContextMenu({ x: e.clientX, y: e.clientY, trackId });
    }, []);

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
        <div className="track-card-grid-container">
            {/* Toolbar */}
            <div className="track-card-grid-toolbar">
                <div className="track-card-grid-toolbar-left">
                    <button className="import-btn" onClick={handleImport} disabled={importing}
                        style={{ backgroundColor: "#4CAF50", color: "white", border: "none" }}>
                        {importing ? "\u27F3 Importing..." : "+ Import"}
                    </button>
                    <button onClick={handleClear} disabled={importing || tracks.length === 0}
                        style={{ padding: "6px 12px", backgroundColor: "#f44336", color: "white", border: "none", borderRadius: "4px", cursor: importing || tracks.length === 0 ? "not-allowed" : "pointer", opacity: importing || tracks.length === 0 ? 0.5 : 1, fontSize: "12px" }}>
                        Clear
                    </button>
                    {aiConfig?.enabled && (
                        <button onClick={classifyAll} disabled={batchClassifying || tracks.length === 0}
                            style={{ padding: "6px 12px", backgroundColor: batchClassifying ? "var(--bg-elevated)" : "rgba(178, 102, 255, 0.8)", color: "white", border: "none", borderRadius: "4px", cursor: batchClassifying || tracks.length === 0 ? "not-allowed" : "pointer", fontSize: "12px", fontWeight: 600 }}>
                            {batchClassifying ? `${batchClassifyProgress.done}/${batchClassifyProgress.total}` : "Classify"}
                        </button>
                    )}
                </div>
                <div className="track-card-grid-toolbar-center">
                    <input
                        type="text"
                        className="search-input"
                        placeholder="Search..."
                        value={libraryFilters.query}
                        onChange={(e) => handleSearchChange(e.target.value)}
                    />
                    <input type="text" className="filter-input" placeholder="Genre" value={libraryFilters.genre} onChange={(e) => handleFilterChange("genre", e.target.value)} />
                    <input type="number" className="filter-input filter-bpm" placeholder="BPM min" value={libraryFilters.bpmMin} onChange={(e) => handleFilterChange("bpmMin", e.target.value)} />
                    <input type="number" className="filter-input filter-bpm" placeholder="BPM max" value={libraryFilters.bpmMax} onChange={(e) => handleFilterChange("bpmMax", e.target.value)} />
                    <input type="text" className="filter-input" placeholder="Key" value={libraryFilters.keyFilter} onChange={(e) => handleFilterChange("keyFilter", e.target.value)} />
                    {hasFilters && (
                        <button className="filter-clear-btn" onClick={clearLibraryFilters}>Clear</button>
                    )}
                </div>
                <span className="track-count">{displayTracks.length} tracks</span>
            </div>

            {/* Card Grid */}
            <div className="track-card-grid">
                {displayTracks.length === 0 ? (
                    <div className="empty-state" style={{ gridColumn: "1 / -1" }}>
                        <div className="empty-state-icon">🎵</div>
                        <div>{selectedPlaylistId ? "No tracks in this playlist" : "No tracks yet — import some audio files"}</div>
                    </div>
                ) : (
                    displayTracks.map((track) => (
                        <TrackCard
                            key={track.id}
                            track={track}
                            onLoad={handleLoad}
                            onContextMenu={handleContextMenu}
                        />
                    ))
                )}
            </div>

            {/* Context Menu */}
            {contextMenu && (
                <div className="context-menu" style={{ top: contextMenu.y, left: contextMenu.x }}>
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
        </div>
    );
}
