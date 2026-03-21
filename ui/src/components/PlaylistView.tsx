import { useEffect, useState } from "react";
import { useStore, SessionRecord } from "../store";
import { formatTime } from "./utils";

export function PlaylistView() {
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
                                    <span className="pl-col-bpm">{track.bpm?.toFixed(1) ?? "\u2014"}</span>
                                    <span className="pl-col-key">{track.key || "\u2014"}</span>
                                    <span className="pl-col-dur">{track.duration_ms ? formatTime(track.duration_ms) : "\u2014"}</span>
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
                                    {"← Back"}
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
