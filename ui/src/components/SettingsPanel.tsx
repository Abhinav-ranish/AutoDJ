import { useState, useEffect } from "react";
import { useStore } from "../store";

export function SettingsPanel() {
    const { aiConfig, setAiConfig, audioDevices, getAudioDevices, savePreference, loadPreference, spotifyStatus, spotifyConnect, spotifyDisconnect, fetchSpotifyStatus } = useStore();
    const [apiKey, setApiKey] = useState("");
    const [provider, setProvider] = useState<"gemini" | "groq" | "ollama">(
        aiConfig?.provider === "groq" ? "groq" : aiConfig?.provider === "ollama" ? "ollama" : "gemini"
    );
    const [ollamaModel, setOllamaModel] = useState(aiConfig?.provider === "ollama" ? (aiConfig?.model || "qwen2.5:7b") : "qwen2.5:7b");
    const [ollamaEndpoint, setOllamaEndpoint] = useState("http://localhost:11434/v1");
    const [isSaving, setIsSaving] = useState(false);
    const [defaultTemplate, setDefaultTemplate] = useState("smooth_beatmix");
    const [defaultDuration, setDefaultDuration] = useState(16);
    const [settingsTab, setSettingsTab] = useState<"ai" | "audio" | "autodj" | "spotify">("ai");
    const [spotifyClientId, setSpotifyClientId] = useState("");

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
        loadPreference("spotify_client_id").then(v => { if (v) setSpotifyClientId(v); });
        fetchSpotifyStatus();
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
                <button style={tabStyle("spotify")} onClick={() => setSettingsTab("spotify")}>Spotify</button>
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
                                            \u2713 {aiConfig.api_key.substring(0, 4)}....{aiConfig.api_key.slice(-4)}
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

            {settingsTab === "spotify" && (
                <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
                    {spotifyStatus?.connected ? (
                        <>
                            <div style={{ display: "flex", alignItems: "center", gap: "8px", background: "rgba(0,0,0,0.2)", padding: "10px", borderRadius: "6px", border: "1px solid var(--border)" }}>
                                <span style={{ fontFamily: "monospace", color: "var(--status-ready)", flex: 1, fontSize: "14px" }}>
                                    Connected as {spotifyStatus.user_name}
                                </span>
                                <button
                                    onClick={async () => { await spotifyDisconnect(); }}
                                    style={{ background: "transparent", border: "1px solid var(--border)", color: "var(--text-secondary)", borderRadius: "4px", cursor: "pointer", padding: "4px 8px", fontSize: "10px", fontWeight: "bold" }}
                                >
                                    DISCONNECT
                                </button>
                            </div>
                        </>
                    ) : (
                        <>
                            <div style={{ display: "flex", flexDirection: "column", gap: "5px" }}>
                                <label style={labelStyle}>Spotify Client ID</label>
                                <input
                                    type="text"
                                    placeholder="Enter your Spotify Client ID"
                                    value={spotifyClientId}
                                    onChange={(e) => setSpotifyClientId(e.target.value)}
                                    style={inputStyle}
                                />
                            </div>
                            <div style={{ fontSize: "11px", color: "var(--text-muted)", padding: "6px 8px", background: "rgba(0,0,0,0.15)", borderRadius: "4px", lineHeight: 1.5 }}>
                                Create an app at <span style={{ color: "var(--text-secondary)", fontFamily: "var(--font-mono)" }}>developer.spotify.com</span> to get a Client ID.
                            </div>
                            <button
                                onClick={async () => {
                                    if (!spotifyClientId.trim()) return;
                                    await savePreference("spotify_client_id", spotifyClientId.trim());
                                    await spotifyConnect(spotifyClientId.trim());
                                }}
                                disabled={!spotifyClientId.trim()}
                                style={{
                                    padding: "10px",
                                    background: spotifyClientId.trim() ? "linear-gradient(135deg, #1DB954, #1aa34a)" : "var(--bg-elevated)",
                                    color: spotifyClientId.trim() ? "white" : "var(--text-muted)",
                                    border: "none",
                                    borderRadius: "6px",
                                    fontWeight: "bold",
                                    cursor: spotifyClientId.trim() ? "pointer" : "not-allowed",
                                }}
                            >
                                Connect Spotify
                            </button>
                        </>
                    )}
                </div>
            )}
        </div>
    );
}
