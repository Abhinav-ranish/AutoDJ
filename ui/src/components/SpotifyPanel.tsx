import { useEffect, useState } from "react";
import { useStore } from "../store";
import { invoke } from "@tauri-apps/api/core";

function formatDuration(ms: number): string {
    const totalSeconds = Math.floor(ms / 1000);
    const minutes = Math.floor(totalSeconds / 60);
    const seconds = totalSeconds % 60;
    return `${minutes}:${seconds.toString().padStart(2, "0")}`;
}

export function SpotifyPanel() {
    const {
        spotifyStatus,
        spotifyPlaylists,
        spotifyPlaylistTracks,
        spotifyImporting,
        spotifyImportProgress,
        fetchSpotifyPlaylists,
        fetchSpotifyPlaylistTracks,
        importSpotifyPlaylist,
    } = useStore();

    const [selectedPlaylistId, setSelectedPlaylistId] = useState<string | null>(null);
    const [spotdlAvailable, setSpotdlAvailable] = useState<boolean | null>(null);

    // Check spotDL on mount
    useEffect(() => {
        invoke("spotify_check_spotdl")
            .then(() => setSpotdlAvailable(true))
            .catch(() => setSpotdlAvailable(false));
    }, []);

    // Fetch playlists when connected
    useEffect(() => {
        if (spotifyStatus?.connected) {
            fetchSpotifyPlaylists();
        }
    }, [spotifyStatus?.connected, fetchSpotifyPlaylists]);

    if (!spotifyStatus?.connected) {
        return null;
    }

    const handlePlaylistClick = (playlistId: string) => {
        setSelectedPlaylistId(playlistId);
        fetchSpotifyPlaylistTracks(playlistId);
    };

    const handleImport = () => {
        if (selectedPlaylistId) {
            importSpotifyPlaylist(selectedPlaylistId);
        }
    };

    const selectedPlaylist = spotifyPlaylists.find(p => p.id === selectedPlaylistId);

    return (
        <div style={{ display: "flex", flexDirection: "column", gap: "12px", marginTop: "12px" }}>
            <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <span style={{
                    fontSize: "13px",
                    fontWeight: 700,
                    color: "var(--text-primary)",
                    fontFamily: "var(--font-display)",
                    textTransform: "uppercase",
                    letterSpacing: "0.5px",
                }}>
                    Spotify Playlists
                </span>
                <span style={{
                    fontSize: "10px",
                    color: "#1DB954",
                    fontWeight: 600,
                    padding: "2px 6px",
                    background: "rgba(29, 185, 84, 0.15)",
                    borderRadius: "4px",
                }}>
                    {spotifyStatus.user_name}
                </span>
            </div>

            {spotdlAvailable === false && (
                <div style={{
                    padding: "8px 12px",
                    background: "rgba(255, 152, 0, 0.15)",
                    border: "1px solid rgba(255, 152, 0, 0.3)",
                    borderRadius: "6px",
                    fontSize: "11px",
                    color: "#ffb74d",
                    lineHeight: 1.5,
                }}>
                    spotDL not found. Install it to import tracks:
                    <br />
                    <span style={{ fontFamily: "var(--font-mono)", color: "var(--text-secondary)" }}>
                        pip install spotdl
                    </span>
                </div>
            )}

            {spotifyImporting && spotifyImportProgress && (
                <div style={{
                    padding: "8px 12px",
                    background: "rgba(0,0,0,0.2)",
                    borderRadius: "6px",
                    border: "1px solid var(--border)",
                }}>
                    <div style={{ fontSize: "11px", color: "var(--text-muted)", marginBottom: "6px" }}>
                        Importing {spotifyImportProgress.current}/{spotifyImportProgress.total}: {spotifyImportProgress.track}
                    </div>
                    <div style={{
                        height: "4px",
                        background: "rgba(255,255,255,0.1)",
                        borderRadius: "2px",
                        overflow: "hidden",
                    }}>
                        <div style={{
                            height: "100%",
                            width: `${(spotifyImportProgress.current / spotifyImportProgress.total) * 100}%`,
                            background: "linear-gradient(90deg, #1DB954, #1aa34a)",
                            borderRadius: "2px",
                            transition: "width 0.3s ease",
                        }} />
                    </div>
                </div>
            )}

            {!selectedPlaylistId ? (
                // Playlist grid
                <div style={{
                    display: "grid",
                    gridTemplateColumns: "repeat(auto-fill, minmax(200px, 1fr))",
                    gap: "8px",
                    maxHeight: "300px",
                    overflowY: "auto",
                }}>
                    {spotifyPlaylists.length === 0 ? (
                        <div style={{ padding: "20px", color: "var(--text-muted)", fontSize: "12px", textAlign: "center", gridColumn: "1 / -1" }}>
                            No playlists found
                        </div>
                    ) : (
                        spotifyPlaylists.map(playlist => (
                            <div
                                key={playlist.id}
                                onClick={() => handlePlaylistClick(playlist.id)}
                                style={{
                                    padding: "10px 12px",
                                    background: "rgba(0,0,0,0.2)",
                                    border: "1px solid var(--border)",
                                    borderRadius: "6px",
                                    cursor: "pointer",
                                    transition: "background 0.15s, border-color 0.15s",
                                }}
                                onMouseEnter={e => {
                                    e.currentTarget.style.background = "rgba(29, 185, 84, 0.1)";
                                    e.currentTarget.style.borderColor = "rgba(29, 185, 84, 0.3)";
                                }}
                                onMouseLeave={e => {
                                    e.currentTarget.style.background = "rgba(0,0,0,0.2)";
                                    e.currentTarget.style.borderColor = "var(--border)";
                                }}
                            >
                                <div style={{
                                    fontSize: "12px",
                                    fontWeight: 600,
                                    color: "var(--text-primary)",
                                    marginBottom: "4px",
                                    overflow: "hidden",
                                    textOverflow: "ellipsis",
                                    whiteSpace: "nowrap",
                                }}>
                                    {playlist.name}
                                </div>
                                <div style={{
                                    fontSize: "10px",
                                    color: "var(--text-muted)",
                                }}>
                                    {playlist.track_count} tracks &middot; {playlist.owner}
                                </div>
                            </div>
                        ))
                    )}
                </div>
            ) : (
                // Track list for selected playlist
                <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
                    <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                        <button
                            onClick={() => { setSelectedPlaylistId(null); }}
                            style={{
                                background: "transparent",
                                border: "1px solid var(--border)",
                                color: "var(--text-secondary)",
                                borderRadius: "4px",
                                cursor: "pointer",
                                padding: "4px 8px",
                                fontSize: "11px",
                                fontWeight: 600,
                            }}
                        >
                            Back
                        </button>
                        <span style={{ fontSize: "12px", fontWeight: 600, color: "var(--text-primary)", flex: 1 }}>
                            {selectedPlaylist?.name} ({spotifyPlaylistTracks.length} tracks)
                        </span>
                        <button
                            onClick={handleImport}
                            disabled={spotifyImporting || spotdlAvailable === false}
                            style={{
                                padding: "6px 12px",
                                background: (spotifyImporting || spotdlAvailable === false) ? "var(--bg-elevated)" : "linear-gradient(135deg, #1DB954, #1aa34a)",
                                color: (spotifyImporting || spotdlAvailable === false) ? "var(--text-muted)" : "white",
                                border: "none",
                                borderRadius: "4px",
                                fontWeight: 700,
                                fontSize: "11px",
                                cursor: (spotifyImporting || spotdlAvailable === false) ? "not-allowed" : "pointer",
                            }}
                        >
                            {spotifyImporting ? "Importing..." : "Import All"}
                        </button>
                    </div>

                    <div style={{
                        maxHeight: "300px",
                        overflowY: "auto",
                        border: "1px solid var(--border)",
                        borderRadius: "6px",
                    }}>
                        <div style={{
                            display: "grid",
                            gridTemplateColumns: "1fr 1fr 1fr 60px",
                            padding: "6px 10px",
                            fontSize: "10px",
                            fontWeight: 700,
                            color: "var(--text-muted)",
                            textTransform: "uppercase",
                            letterSpacing: "0.5px",
                            borderBottom: "1px solid var(--border)",
                            background: "rgba(0,0,0,0.15)",
                        }}>
                            <span>Title</span>
                            <span>Artist</span>
                            <span>Album</span>
                            <span>Duration</span>
                        </div>
                        {spotifyPlaylistTracks.map(track => (
                            <div
                                key={track.id}
                                style={{
                                    display: "grid",
                                    gridTemplateColumns: "1fr 1fr 1fr 60px",
                                    padding: "6px 10px",
                                    fontSize: "11px",
                                    borderBottom: "1px solid rgba(255,255,255,0.03)",
                                    color: "var(--text-secondary)",
                                }}
                            >
                                <span style={{
                                    overflow: "hidden",
                                    textOverflow: "ellipsis",
                                    whiteSpace: "nowrap",
                                    color: "var(--text-primary)",
                                }}>
                                    {track.name}
                                </span>
                                <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                                    {track.artist}
                                </span>
                                <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                                    {track.album}
                                </span>
                                <span style={{ color: "var(--text-muted)" }}>
                                    {formatDuration(track.duration_ms)}
                                </span>
                            </div>
                        ))}
                    </div>
                </div>
            )}
        </div>
    );
}
