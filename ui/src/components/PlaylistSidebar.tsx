import { useEffect, useState } from "react";
import { useStore } from "../store";
import { PlaylistView } from "./PlaylistView";

export function PlaylistSidebar() {
    const {
        playlists,
        fetchPlaylists,
        createPlaylist,
        renamePlaylist,
        deletePlaylist,
        selectedPlaylistId,
        selectPlaylist,
        loadPlaylistToQueue,
        spotifyStatus,
    } = useStore();

    const [collapsed, setCollapsed] = useState(false);
    const [newName, setNewName] = useState("");
    const [contextMenu, setContextMenu] = useState<{ x: number; y: number; playlistId: string } | null>(null);
    const [editingId, setEditingId] = useState<string | null>(null);
    const [editName, setEditName] = useState("");
    const [showManageModal, setShowManageModal] = useState(false);

    useEffect(() => {
        fetchPlaylists();
    }, [fetchPlaylists]);

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

    // Close context menu on outside click
    useEffect(() => {
        const handleClick = () => setContextMenu(null);
        if (contextMenu) {
            document.addEventListener("click", handleClick);
            return () => document.removeEventListener("click", handleClick);
        }
    }, [contextMenu]);

    if (collapsed) {
        return (
            <div className="playlist-sidebar-v2 playlist-sidebar-v2-collapsed">
                <button className="playlist-sidebar-v2-toggle" onClick={() => setCollapsed(false)} title="Expand sidebar">
                    {"▶"}
                </button>
            </div>
        );
    }

    return (
        <>
            <div className="playlist-sidebar-v2">
                <div className="playlist-sidebar-v2-header">
                    <span className="playlist-sidebar-v2-title">Your Library</span>
                    <button className="playlist-sidebar-v2-toggle" onClick={() => setCollapsed(true)} title="Collapse sidebar">
                        {"◀"}
                    </button>
                </div>

                {/* Create playlist */}
                <div className="playlist-sidebar-v2-create">
                    <input
                        type="text"
                        placeholder="New playlist..."
                        value={newName}
                        onChange={e => setNewName(e.target.value)}
                        onKeyDown={e => e.key === "Enter" && handleCreate()}
                        className="playlist-sidebar-v2-input"
                    />
                    <button onClick={handleCreate} disabled={!newName.trim()} className="playlist-sidebar-v2-add-btn">+</button>
                </div>

                {/* All Tracks */}
                <div className="playlist-sidebar-v2-list">
                    <div
                        className={`playlist-sidebar-v2-item ${!selectedPlaylistId ? "selected" : ""}`}
                        onClick={() => selectPlaylist(null as any)}
                    >
                        <span className="playlist-sidebar-v2-item-name">All Tracks</span>
                    </div>

                    {/* Local Playlists */}
                    {playlists.length > 0 && (
                        <div className="playlist-sidebar-v2-section-label">Playlists</div>
                    )}
                    {playlists.map(pl => (
                        <div
                            key={pl.id}
                            className={`playlist-sidebar-v2-item ${selectedPlaylistId === pl.id ? "selected" : ""}`}
                            onClick={() => selectPlaylist(pl.id)}
                            onContextMenu={(e) => {
                                e.preventDefault();
                                setContextMenu({ x: e.clientX, y: e.clientY, playlistId: pl.id });
                            }}
                        >
                            {editingId === pl.id ? (
                                <input
                                    type="text"
                                    value={editName}
                                    onChange={e => setEditName(e.target.value)}
                                    onKeyDown={e => { if (e.key === "Enter") handleRename(pl.id); if (e.key === "Escape") setEditingId(null); }}
                                    onBlur={() => handleRename(pl.id)}
                                    autoFocus
                                    className="playlist-sidebar-v2-rename-input"
                                    onClick={e => e.stopPropagation()}
                                />
                            ) : (
                                <>
                                    <span className="playlist-sidebar-v2-item-name">{pl.name}</span>
                                    <span className="playlist-sidebar-v2-item-count">{pl.track_count}</span>
                                </>
                            )}
                        </div>
                    ))}

                    {/* Spotify Section */}
                    {spotifyStatus?.connected && (
                        <>
                            <div className="playlist-sidebar-v2-section-label">
                                <span className="playlist-sidebar-v2-spotify-dot" />
                                Spotify
                            </div>
                        </>
                    )}
                </div>

                {/* Manage button */}
                <button className="playlist-sidebar-v2-manage-btn" onClick={() => setShowManageModal(true)}>
                    Manage Playlists
                </button>
            </div>

            {/* Context Menu */}
            {contextMenu && (
                <div className="context-menu" style={{ top: contextMenu.y, left: contextMenu.x }}>
                    <button onClick={() => { setEditingId(contextMenu.playlistId); setEditName(playlists.find(p => p.id === contextMenu.playlistId)?.name || ""); setContextMenu(null); }}>
                        Rename
                    </button>
                    <button onClick={() => { handleDelete(contextMenu.playlistId); setContextMenu(null); }}>
                        Delete
                    </button>
                    <div className="context-menu-divider" />
                    <button onClick={() => { loadPlaylistToQueue(contextMenu.playlistId, false); setContextMenu(null); }}>
                        Load to Queue
                    </button>
                    <button onClick={() => { loadPlaylistToQueue(contextMenu.playlistId, true); setContextMenu(null); }}>
                        Shuffle Queue
                    </button>
                </div>
            )}

            {/* Manage Playlists Modal */}
            {showManageModal && (
                <div className="settings-modal-overlay" onClick={() => setShowManageModal(false)}>
                    <div className="settings-modal" onClick={e => e.stopPropagation()} style={{ maxWidth: "900px", width: "90vw" }}>
                        <button className="settings-modal-close" onClick={() => setShowManageModal(false)}>{"✕"}</button>
                        <PlaylistView />
                    </div>
                </div>
            )}
        </>
    );
}
