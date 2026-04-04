import { useEffect, useState } from "react";
import { useStore, EngineState } from "./store";
import { listen } from "@tauri-apps/api/event";
import logo from "./assets/logo.png";

import { useDeckState } from "./hooks/useDeckState";
import { DeckPanel } from "./components/DeckPanel";
import { MixerControls } from "./components/MixerChannel";
import { AiBar, TransitionsPanel } from "./components/AiBar";
import { AutoDjPanel } from "./components/AutoDjPanel";
import { TransportBar } from "./components/TransportBar";
import { TrackCardGrid } from "./components/TrackCardGrid";
import { PlaylistSidebar } from "./components/PlaylistSidebar";
import { SettingsPanel } from "./components/SettingsPanel";
import { SpotifyPanel } from "./components/SpotifyPanel";
import { IconSettings, IconRecord } from "./components/Icons";

// ─── Library Page ────────────────────────────────────────────────

function LibraryPage() {
    return (
        <div className="library-page">
            <section className="library-area">
                <PlaylistSidebar />
                <TrackCardGrid />
            </section>
        </div>
    );
}

// ─── Spotify Page ────────────────────────────────────────────────

function SpotifyPage() {
    const { spotifyStatus, spotifyConnect, spotifyDisconnect, fetchSpotifyStatus, savePreference, loadPreference } = useStore();
    const [clientId, setClientId] = useState("");
    const [clientSecret, setClientSecret] = useState("");
    const [connecting, setConnecting] = useState(false);

    useEffect(() => {
        fetchSpotifyStatus();
        loadPreference("spotify_client_id").then(v => { if (v) setClientId(v); });
        loadPreference("spotify_client_secret").then(v => { if (v) setClientSecret(v); });
    }, []);

    const handleConnect = async () => {
        if (!clientId.trim()) return;
        setConnecting(true);
        try {
            await savePreference("spotify_client_id", clientId.trim());
            if (clientSecret.trim()) {
                await savePreference("spotify_client_secret", clientSecret.trim());
            }
            await spotifyConnect(clientId.trim(), clientSecret.trim() || undefined);
        } catch (err) {
            console.error("Spotify connect error:", err);
        } finally {
            setConnecting(false);
        }
    };

    return (
        <div className="spotify-page">
            {spotifyStatus?.connected ? (
                <>
                    <div className="spotify-page-connected">
                        <span className="spotify-page-dot" />
                        <span className="spotify-page-user">Connected as {spotifyStatus.user_name}</span>
                        <button className="spotify-page-disconnect" onClick={() => spotifyDisconnect()}>
                            Disconnect
                        </button>
                    </div>
                    <SpotifyPanel />
                </>
            ) : (
                <div className="spotify-page-connect">
                    <div className="spotify-page-logo-area">
                        <svg width="48" height="48" viewBox="0 0 24 24" fill="#1DB954">
                            <path d="M12 0C5.4 0 0 5.4 0 12s5.4 12 12 12 12-5.4 12-12S18.66 0 12 0zm5.521 17.34c-.24.359-.66.48-1.021.24-2.82-1.74-6.36-2.101-10.561-1.141-.418.122-.779-.179-.899-.539-.12-.421.18-.78.54-.9 4.56-1.021 8.52-.6 11.64 1.32.42.18.479.659.301 1.02zm1.44-3.3c-.301.42-.841.6-1.262.3-3.239-1.98-8.159-2.58-11.939-1.38-.479.12-1.02-.12-1.14-.6-.12-.48.12-1.021.6-1.141C9.6 9.9 15 10.561 18.72 12.84c.361.181.54.78.241 1.2zm.12-3.36C15.24 8.4 8.82 8.16 5.16 9.301c-.6.179-1.2-.181-1.38-.721-.18-.601.18-1.2.72-1.381 4.26-1.26 11.28-1.02 15.721 1.621.539.3.719 1.02.419 1.56-.299.421-1.02.599-1.559.3z"/>
                        </svg>
                        <h3 className="spotify-page-title">Connect Spotify</h3>
                    </div>
                    <p className="spotify-page-desc">
                        Browse your playlists and import tracks directly into AutoDJ.
                    </p>
                    <div className="spotify-page-form">
                        <label className="spotify-page-label">Client ID</label>
                        <input
                            type="text"
                            placeholder="Enter your Spotify Client ID"
                            value={clientId}
                            onChange={(e) => setClientId(e.target.value)}
                            className="spotify-page-input"
                        />
                        <label className="spotify-page-label" style={{ marginTop: "8px" }}>Client Secret</label>
                        <input
                            type="password"
                            placeholder="Enter your Spotify Client Secret"
                            value={clientSecret}
                            onChange={(e) => setClientSecret(e.target.value)}
                            className="spotify-page-input"
                        />
                        <span className="spotify-page-hint">
                            Get both at developer.spotify.com &gt; Your App &gt; Settings
                        </span>
                        <button
                            onClick={handleConnect}
                            disabled={!clientId.trim() || connecting}
                            className="spotify-page-connect-btn"
                        >
                            {connecting ? "Connecting..." : "Connect"}
                        </button>
                    </div>
                </div>
            )}
        </div>
    );
}

// ─── DJ Page ─────────────────────────────────────────────────────

function DJPage({
    masterBpm,
    activeDeck,
    crossfaderPercent,
    recMinutes,
    recSeconds,
}: {
    masterBpm: number;
    activeDeck: "A" | "B";
    crossfaderPercent: number;
    recMinutes: number;
    recSeconds: number;
}) {
    const { engineState, setCrossfader, isRecording, startRecording, stopRecording, mode } = useStore();
    const snapA = engineState?.deck_a;
    const snapB = engineState?.deck_b;

    return (
        <div className="dj-page">
            {/* Deck Area */}
            <section className="deck-area">
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
                        <div className="xfader-slider-row">
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
            </section>

            {/* AI Bar */}
            <AiBar />
            {mode === "manual" && <TransitionsPanel />}
            {mode === "autodj" && <AutoDjPanel />}

            {/* Transport Bar */}
            <TransportBar />
        </div>
    );
}

// ─── App ─────────────────────────────────────────────────────────

export default function App() {
    const { activeView, setActiveView, setEngineState, setConductorState, engineState, isRecording, recordingStartTime, mode } = useStore();
    const [isSettingsOpen, setIsSettingsOpen] = useState(false);

    // Deck state for top bar
    const deckA = useDeckState("A");
    const deckB = useDeckState("B");

    const masterBpm = engineState?.master_bpm ?? 128;
    const snapA = engineState?.deck_a;
    const snapB = engineState?.deck_b;
    const aPlaying = snapA?.state === "playing";
    const bPlaying = snapB?.state === "playing";
    let activeDeck: "A" | "B" = (engineState?.crossfader ?? 0) > 0 ? "B" : "A";
    if (aPlaying && !bPlaying) activeDeck = "A";
    if (!aPlaying && bPlaying) activeDeck = "B";
    const crossfaderPercent = Math.round((engineState?.crossfader ?? 0) * 100);

    // Recording timer
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

    useEffect(() => {
        useStore.getState().initStore();

        const unlisten = listen<EngineState>("engine-state", (event) => {
            setEngineState(event.payload);
        });

        const unlistenConductor = listen<any>("conductor-state", (event) => {
            setConductorState(event.payload);
        });

        const unlistenDrop = listen<any>("tauri://drop", (event) => {
            const paths = event.payload?.paths || (Array.isArray(event.payload) ? event.payload : undefined);
            if (paths && paths.length > 0) {
                useStore.getState().importFiles(paths);
            }
        });

        const handleKeyDown = (e: KeyboardEvent) => {
            if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;

            const state = useStore.getState();
            if (!state.engineState) return;

            if (e.code === "Space") {
                e.preventDefault();
                if (state.engineState.deck_a.state === "playing") state.pause("a");
                else state.play("a");
            }
            else if (e.code === "KeyZ") {
                e.preventDefault();
                if (state.engineState.deck_a.loop_active) state.clearLoop("a");
                else state.setLoop("a", 4);
            }
            else if (e.code === "KeyX") {
                e.preventDefault();
                if (state.engineState.deck_b.loop_active) state.clearLoop("b");
                else state.setLoop("b", 4);
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

    const viewMap: Record<string, string> = { library: "library", dj: "dj", playlists: "library", spotify: "spotify" };
    const currentView = viewMap[activeView] || "dj";

    return (
        <div className="app">
            {/* ── Top Bar ── */}
            <header className="top-bar">
                <div className="top-bar-track top-bar-track-a">
                    {!deckA.isEmpty ? (
                        <>
                            <span className="top-bar-track-title">{deckA.trackTitle}</span>
                            <span className="top-bar-track-artist">{deckA.trackArtist}</span>
                            <span className="top-bar-track-meta">{deckA.displayBpm} BPM {deckA.trackKey && `\u00B7 ${deckA.trackKey}`}</span>
                        </>
                    ) : (
                        <span className="top-bar-track-empty">Deck A</span>
                    )}
                </div>

                <div className="top-bar-center">
                    <div className="top-bar-logo">
                        <img src={logo} alt="AutoDJ Logo" style={{ height: "24px" }} />
                        <span>AutoDJ</span>
                    </div>
                    <nav className="top-bar-tabs">
                        <button
                            className={`top-bar-tab ${currentView === "dj" ? "active" : ""}`}
                            onClick={() => setActiveView("dj")}
                        >
                            DJ
                        </button>
                        <button
                            className={`top-bar-tab ${currentView === "library" ? "active" : ""}`}
                            onClick={() => setActiveView("library")}
                        >
                            Library
                        </button>
                        <button
                            className={`top-bar-tab top-bar-tab-spotify ${currentView === "spotify" ? "active" : ""}`}
                            onClick={() => setActiveView("spotify" as any)}
                        >
                            Spotify
                        </button>
                    </nav>
                </div>

                <div className="top-bar-track top-bar-track-b">
                    {!deckB.isEmpty ? (
                        <>
                            <span className="top-bar-track-title">{deckB.trackTitle}</span>
                            <span className="top-bar-track-artist">{deckB.trackArtist}</span>
                            <span className="top-bar-track-meta">{deckB.displayBpm} BPM {deckB.trackKey && `\u00B7 ${deckB.trackKey}`}</span>
                        </>
                    ) : (
                        <span className="top-bar-track-empty">Deck B</span>
                    )}
                </div>
                <button className="settings-btn" onClick={() => setIsSettingsOpen(true)}>
                    <IconSettings />
                </button>
            </header>

            {/* ── Main Content (tabbed) ── */}
            <main className="main-content">
                {currentView === "dj" ? (
                    <DJPage
                        masterBpm={masterBpm}
                        activeDeck={activeDeck}
                        crossfaderPercent={crossfaderPercent}
                        recMinutes={recMinutes}
                        recSeconds={recSeconds}
                    />
                ) : currentView === "spotify" ? (
                    <SpotifyPage />
                ) : (
                    <LibraryPage />
                )}
            </main>

            {/* Settings Modal */}
            {isSettingsOpen && (
                <div className="settings-modal-overlay" onClick={() => setIsSettingsOpen(false)}>
                    <div className="settings-modal" onClick={e => e.stopPropagation()}>
                        <button className="settings-modal-close" onClick={() => setIsSettingsOpen(false)}>{"✕"}</button>
                        <SettingsPanel />
                    </div>
                </div>
            )}
        </div>
    );
}
