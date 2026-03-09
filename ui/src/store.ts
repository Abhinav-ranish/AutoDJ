import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

export interface TrackRecord {
    id: string;
    file_hash: string;
    original_path: string;
    title: string | null;
    artist: string | null;
    duration_ms: number | null;
    import_status: string;
    error_message: string | null;
    bpm: number | null;
    perceived_tempo: number | null;
    groove_type: string | null;
    key: string | null;
    lufs: number | null;
    genre: string | null;
    mood: string | null;
    vibe: string | null;
}

export interface DeckSnapshot {
    state: string;
    track_id: string | null;
    position_ms: number;
    duration_ms: number;
    position_fraction: number;
    gain_db: number;
    eq_low: number;
    eq_mid: number;
    eq_high: number;
    eq_kill_low: boolean;
    eq_kill_mid: boolean;
    eq_kill_high: boolean;
    eq_isolator: boolean;
    filter_cutoff: number;
    filter_resonance: number;
    loop_active: boolean;
    loop_start_ms: number;
    loop_end_ms: number;
    speed: number;
    rms: number;
    peak: number;
}

export interface EngineState {
    deck_a: DeckSnapshot;
    deck_b: DeckSnapshot;
    crossfader: number;
    master_bpm: number;
    beat_number: number;
    bar_number: number;
    sample_pos: number;
    master_rms: number;
    silence_frames: number;
    underrun_count: number;
    spectrum: number[];
}

export interface MixAction {
    at_ms: number;
    kind: Record<string, any>;
}

export interface AiDecision {
    decision: string;
    reason: string;
    track_selection?: string;
    execute_at_ms?: number;
    transition_plan?: {
        template_name: string;
        parameter_overrides?: any;
    };
    actions?: MixAction[];
    reasoning_tags?: string[];
    confidence?: number;
    // Phase 34: Enhanced decision fields
    entry_marker?: string;
    exit_marker?: string;
    creative_intensity?: number;
    set_arc?: string;
    planner_mode?: string;
    template_scores?: [string, number][];
}

export interface PlaylistRecord {
    id: string;
    name: string;
    description: string | null;
    created_at: number;
    updated_at: number;
    track_count: number;
}

export interface SessionRecord {
    id: string;
    started_at: number;
    ended_at: number | null;
    track_count: number | null;
    total_duration_ms: number | null;
}

export type DjMode = "manual" | "assisted" | "autodj";

export type AutodjStatus =
    | "idle"
    | "starting"
    | "picking_first_track"
    | "monitoring"
    | "selecting_next"
    | "loading_next"
    | "transitioning"
    | "error";

export interface LibraryFilters {
    query: string;
    genre: string;
    mood: string;
    bpmMin: string;
    bpmMax: string;
    keyFilter: string;
    sortBy: string;
    sortOrder: "asc" | "desc";
}

export interface AiDebugInfo {
    provider: string;
    model: string;
    enabled: boolean;
    has_api_key: boolean;
    playing_deck: string;
    position_ms: number;
    duration_ms: number;
    position_fraction: number;
    master_bpm: number;
    beat_number: number;
    bar_number: number;
    silence_frames: number;
    crossfader: number;
}

interface DJStore {
    tracks: TrackRecord[];
    engineState: EngineState | null;
    importing: boolean;
    activeView: "dj" | "library" | "playlists";
    mode: DjMode;
    aiDecision: AiDecision | null;
    isThinking: boolean;
    errorToast: string | null;
    analyzingTracks: Set<string>;
    classifyingTracks: Set<string>;
    autodjStatus: AutodjStatus;
    autodjMessage: string;
    autodjTransitioning: boolean;
    libraryFilters: LibraryFilters;
    aiConfig: {
        enabled: boolean;
        provider: string;
        api_key: string;
        model: string;
        endpoint_url: string;
        timeout_seconds: number;
        prompt_template: string | null;
    } | null;

    // Actions
    initStore: () => Promise<void>;
    fetchTracks: () => Promise<void>;
    setLibraryFilters: (filters: Partial<LibraryFilters>) => void;
    clearLibraryFilters: () => void;
    importFiles: (paths: string[]) => Promise<void>;
    loadTrack: (deck: "a" | "b", trackId: string) => Promise<void>;
    play: (deck: string) => Promise<void>;
    pause: (deck: string) => Promise<void>;
    seek: (deck: string, positionMs: number) => Promise<void>;
    setCrossfader: (position: number) => Promise<void>;
    setDeckGain: (deck: string, gainDb: number) => Promise<void>;
    setEq: (deck: string, band: string, gainDb: number) => Promise<void>;
    setFilter: (deck: string, cutoffHz: number, resonance: number) => Promise<void>;
    setLoop: (deck: string, lengthBars: number) => Promise<void>;
    clearLoop: (deck: string) => Promise<void>;
    setDeckSpeed: (deck: string, speedMultiplier: number) => Promise<void>;
    setEqKill: (deck: string, band: string, enabled: boolean) => Promise<void>;
    setEqMode: (deck: string, mode: string) => Promise<void>;
    seekMs: (deck: string, positionMs: number) => Promise<void>;
    getAutoCues: (trackId: string) => Promise<Array<[string, number, number]>>;
    submitTransitionFeedback: (fromTrackId: string, toTrackId: string, templateUsed: string, rating: number, notes?: string) => Promise<void>;
    pollEngineState: () => Promise<void>;
    setActiveView: (view: "dj" | "library" | "playlists") => void;
    analyzeTrack: (trackId: string) => Promise<void>;
    classifyTrack: (trackId: string) => Promise<void>;
    getTrackProfile: (trackId: string) => Promise<any>;
    executeTransition: (deck: string, template: string, durationBars: number) => Promise<void>;
    clearLibrary: () => Promise<void>;

    // AI Actions
    setMode: (mode: DjMode) => Promise<void>;
    requestNextTrack: () => Promise<AiDecision | null>;
    clearAiDecision: () => void;
    setAiConfig: (config: any) => Promise<void>;
    setAutodjStatus: (status: AutodjStatus, message?: string) => void;

    // Recording
    isRecording: boolean;
    recordingStartTime: number | null;
    startRecording: () => Promise<void>;
    stopRecording: () => Promise<void>;

    // Preferences & Devices
    audioDevices: string[];
    getAudioDevices: () => Promise<void>;
    savePreference: (key: string, value: string) => Promise<void>;
    loadPreference: (key: string) => Promise<string | null>;

    // Batch operations
    batchClassifying: boolean;
    batchClassifyProgress: { done: number; total: number };
    classifyAll: () => Promise<void>;
    updateTrackClassification: (trackId: string, field: string, value: string) => Promise<void>;

    // Playlists
    playlists: PlaylistRecord[];
    selectedPlaylistId: string | null;
    playlistTracks: TrackRecord[];
    autodjQueue: string[];  // ordered track IDs for AutoDJ playlist mode
    autodjQueueIndex: number;
    autodjShuffle: boolean;

    // Phase 30: DJ Performance Controls
    beatRepeat: (deck: string, beats: number) => Promise<void>;
    beatRepeatOff: (deck: string) => Promise<void>;
    setSlipMode: (deck: string, enabled: boolean) => Promise<void>;
    vinylBrake: (deck: string, durationMs: number) => Promise<void>;
    spinback: (deck: string, durationMs: number) => Promise<void>;
    jumpToMarker: (deck: string, marker: string, positionMs: number) => Promise<void>;

    // Phase 33/39: AI Debug & Health
    testAiProvider: () => Promise<{ ok: boolean; message: string }>;
    aiDebugInfo: AiDebugInfo | null;
    getAiDebugInfo: () => Promise<void>;

    // Phase 35: DJ Personality
    djPersonality: string;
    setDjPersonality: (personality: string) => void;

    fetchPlaylists: () => Promise<void>;
    createPlaylist: (name: string, description?: string) => Promise<string | null>;
    renamePlaylist: (playlistId: string, name: string) => Promise<void>;
    deletePlaylist: (playlistId: string) => Promise<void>;
    selectPlaylist: (playlistId: string | null) => Promise<void>;
    addTrackToPlaylist: (playlistId: string, trackId: string) => Promise<void>;
    removeTrackFromPlaylist: (playlistId: string, trackId: string) => Promise<void>;
    reorderPlaylist: (playlistId: string, trackIds: string[]) => Promise<void>;
    loadPlaylistToQueue: (playlistId: string, shuffle: boolean) => Promise<void>;
    advanceQueue: () => string | null;
    getSessions: () => Promise<SessionRecord[]>;
    exportCueSheet: (sessionId: string) => Promise<string>;

    setErrorToast: (msg: string | null) => void;
    setEngineState: (state: EngineState) => void;
}

export const useStore = create<DJStore>((set, get) => ({
    tracks: [],
    engineState: null,
    importing: false,
    activeView: "library",
    mode: "manual",
    aiDecision: null,
    isThinking: false,
    errorToast: null,
    analyzingTracks: new Set(),
    classifyingTracks: new Set(),
    autodjStatus: "idle" as AutodjStatus,
    autodjMessage: "",
    autodjTransitioning: false,
    aiConfig: null,
    isRecording: false,
    recordingStartTime: null,
    audioDevices: [],
    batchClassifying: false,
    batchClassifyProgress: { done: 0, total: 0 },
    libraryFilters: {
        query: "",
        genre: "",
        mood: "",
        bpmMin: "",
        bpmMax: "",
        keyFilter: "",
        sortBy: "created_at",
        sortOrder: "desc" as const,
    },

    playlists: [],
    selectedPlaylistId: null,
    playlistTracks: [],
    autodjQueue: [],
    autodjQueueIndex: 0,
    autodjShuffle: false,
    aiDebugInfo: null,
    djPersonality: "smooth",

    setErrorToast: (msg: string | null) => {
        set({ errorToast: msg });
        if (msg) setTimeout(() => set({ errorToast: null }), 3000);
    },

    initStore: async () => {
        try {
            // Try DB preferences first, fallback to localStorage for migration
            const dbConfig = await invoke<string | null>("get_preference", { key: "ai_config" });
            if (dbConfig) {
                const config = JSON.parse(dbConfig);
                set({ aiConfig: config });
                await invoke("set_ai_config", { config });
            } else {
                // Migrate from localStorage if present
                const saved = localStorage.getItem("autodj_ai_config");
                if (saved) {
                    const config = JSON.parse(saved);
                    set({ aiConfig: config });
                    await invoke("set_ai_config", { config });
                    // Persist to DB and clean up localStorage
                    await invoke("set_preference", { key: "ai_config", value: saved });
                    localStorage.removeItem("autodj_ai_config");
                }
            }
        } catch (e) {
            console.error("Failed to restore AI config", e);
        }
    },

    fetchTracks: async () => {
        try {
            const f = get().libraryFilters;
            const params: Record<string, any> = {};
            if (f.query) params.query = f.query;
            if (f.genre) params.genre = f.genre;
            if (f.mood) params.mood = f.mood;
            if (f.bpmMin) params.bpmMin = parseFloat(f.bpmMin);
            if (f.bpmMax) params.bpmMax = parseFloat(f.bpmMax);
            if (f.keyFilter) params.keyFilter = f.keyFilter;
            if (f.sortBy !== "created_at") params.sortBy = f.sortBy;
            if (f.sortOrder !== "desc") params.sortOrder = f.sortOrder;
            const tracks = await invoke<TrackRecord[]>("get_tracks", params);
            set({ tracks });
        } catch (e: any) {
            get().setErrorToast(`Failed to fetch tracks: ${e}`);
            console.error(e);
        }
    },

    setLibraryFilters: (filters: Partial<LibraryFilters>) => {
        set((state) => ({
            libraryFilters: { ...state.libraryFilters, ...filters },
        }));
        // Debounced fetch is handled by the UI component
    },

    clearLibraryFilters: () => {
        set({
            libraryFilters: {
                query: "",
                genre: "",
                mood: "",
                bpmMin: "",
                bpmMax: "",
                keyFilter: "",
                sortBy: "created_at",
                sortOrder: "desc",
            },
        });
        get().fetchTracks();
    },

    importFiles: async (paths: string[]) => {
        set({ importing: true });
        try {
            await invoke("import_files", { paths });
            await get().fetchTracks();

            // Automatically analyze newly imported tracks
            const tracks = get().tracks;
            for (const track of tracks) {
                if (track.import_status === "ready") {
                    // Fire and forget so we don't block the UI
                    get().analyzeTrack(track.id).catch(console.error);
                }
            }
        } catch (e: any) {
            get().setErrorToast(`Import failed: ${e}`);
            console.error("Import failed:", e);
        }
        set({ importing: false });
    },

    loadTrack: async (deck: "a" | "b", trackId: string) => {
        try {
            await invoke("load_track", { deck: deck.toLowerCase(), trackId });
            set({ activeView: "dj" });
        } catch (e: any) {
            get().setErrorToast(`Load failed: ${e}`);
            console.error("Load failed:", e);
        }
    },

    play: async (deck: string) => {
        try {
            await invoke("play", { deck: deck.toLowerCase() });
        } catch (e: any) {
            get().setErrorToast(`Play failed: ${e}`);
            console.error("Play failed:", e);
        }
    },

    pause: async (deck: string) => {
        try {
            await invoke("pause", { deck: deck.toLowerCase() });
        } catch (e: any) {
            get().setErrorToast(`Pause failed: ${e}`);
            console.error("Pause failed:", e);
        }
    },

    seek: async (deck: string, positionMs: number) => {
        try {
            await invoke("seek", { deck: deck.toLowerCase(), positionMs: Math.round(positionMs) });
        } catch (e: any) {
            get().setErrorToast(`Seek failed: ${e}`);
            console.error("Seek failed:", e);
        }
    },

    setCrossfader: async (position: number) => {
        try {
            await invoke("set_crossfader", { position });
        } catch (e: any) {
            get().setErrorToast(`Crossfader failed: ${e}`);
            console.error("Crossfader failed:", e);
        }
    },

    setDeckGain: async (deck: string, gainDb: number) => {
        try {
            await invoke("set_deck_gain", { deck: deck.toLowerCase(), gainDb });
        } catch (e: any) {
            get().setErrorToast(`Gain failed: ${e}`);
            console.error("Gain failed:", e);
        }
    },

    setEq: async (deck: string, band: string, gainDb: number) => {
        try {
            await invoke("set_eq", { deck: deck.toLowerCase(), band, gainDb });
        } catch (e: any) {
            get().setErrorToast(`EQ failed: ${e}`);
            console.error("EQ failed:", e);
        }
    },

    setFilter: async (deck: string, cutoffHz: number, resonance: number) => {
        try {
            await invoke("set_filter", { deck: deck.toLowerCase(), cutoffHz, resonance });
        } catch (e: any) {
            get().setErrorToast(`Filter failed: ${e}`);
            console.error("Filter failed:", e);
        }
    },

    setLoop: async (deck: string, lengthBars: number) => {
        try {
            await invoke("set_loop", { deck: deck.toLowerCase(), lengthBars });
        } catch (e: any) {
            get().setErrorToast(`Loop failed: ${e}`);
            console.error("Loop failed:", e);
        }
    },

    clearLoop: async (deck: string) => {
        try {
            await invoke("clear_loop", { deck: deck.toLowerCase() });
        } catch (e: any) {
            get().setErrorToast(`Clear loop failed: ${e}`);
            console.error("Clear loop failed:", e);
        }
    },

    setDeckSpeed: async (deck: string, speedMultiplier: number) => {
        try {
            await invoke("set_speed", { deck: deck.toLowerCase(), speed: speedMultiplier });
        } catch (e: any) {
            get().setErrorToast(`Set tempo failed: ${e}`);
            console.error("Set tempo failed:", e);
        }
    },

    setEqKill: async (deck: string, band: string, enabled: boolean) => {
        try {
            await invoke("set_eq_kill", { deck: deck.toLowerCase(), band, enabled });
        } catch (e: any) {
            get().setErrorToast(`EQ Kill failed: ${e}`);
            console.error("EQ Kill failed:", e);
        }
    },

    setEqMode: async (deck: string, mode: string) => {
        try {
            await invoke("set_eq_mode", { deck: deck.toLowerCase(), mode });
        } catch (e: any) {
            get().setErrorToast(`EQ Mode failed: ${e}`);
            console.error("EQ Mode failed:", e);
        }
    },

    seekMs: async (deck: string, positionMs: number) => {
        try {
            await invoke("seek_ms", { deck: deck.toLowerCase(), positionMs: Math.round(positionMs) });
        } catch (e: any) {
            get().setErrorToast(`Seek failed: ${e}`);
            console.error("Seek failed:", e);
        }
    },

    getAutoCues: async (trackId: string) => {
        try {
            return await invoke<Array<[string, number, number]>>("get_auto_cues", { trackId });
        } catch (e: any) {
            console.error("Get auto cues failed:", e);
            return [];
        }
    },

    submitTransitionFeedback: async (fromTrackId: string, toTrackId: string, templateUsed: string, rating: number, notes?: string) => {
        try {
            await invoke("submit_transition_feedback", {
                fromTrackId,
                toTrackId,
                templateUsed,
                rating,
                notes: notes ?? null,
                decisionJson: null,
            });
        } catch (e: any) {
            console.error("Submit feedback failed:", e);
        }
    },

    pollEngineState: async () => {
        // Now handled via Tauri events
    },

    setActiveView: (view) => set({ activeView: view }),

    analyzeTrack: async (trackId: string) => {
        set((state) => ({ analyzingTracks: new Set(state.analyzingTracks).add(trackId) }));
        try {
            await invoke("analyze_track", { trackId });
            // Re-fetch tracks to update status immediately
            await get().fetchTracks();
            // Auto-classify if API key is configured
            const config = get().aiConfig;
            if (config?.api_key && !get().classifyingTracks.has(trackId)) {
                get().classifyTrack(trackId).catch(console.error);
            }
        } catch (e: any) {
            get().setErrorToast(`Analyze failed: ${e}`);
            console.error("Analyze failed:", e);
        } finally {
            set((state) => {
                const next = new Set(state.analyzingTracks);
                next.delete(trackId);
                return { analyzingTracks: next };
            });
        }
    },

    classifyTrack: async (trackId: string) => {
        set((state) => ({ classifyingTracks: new Set(state.classifyingTracks).add(trackId) }));
        try {
            await invoke("classify_track", { trackId });
            await get().fetchTracks();
        } catch (e: any) {
            console.warn(`Classify failed for ${trackId}:`, e);
        } finally {
            set((state) => {
                const next = new Set(state.classifyingTracks);
                next.delete(trackId);
                return { classifyingTracks: next };
            });
        }
    },

    getTrackProfile: async (trackId: string) => {
        try {
            const json = await invoke<string | null>("get_track_profile", { trackId });
            if (json) return JSON.parse(json);
        } catch (e: any) {
            get().setErrorToast(`Get profile failed: ${e}`);
            console.error("Get profile failed:", e);
        }
        return null;
    },

    executeTransition: async (deck: string, template: string, durationBars: number) => {
        try {
            await invoke("execute_transition", {
                template,
                params: { incoming_deck: deck.toUpperCase(), duration_bars: durationBars }
            });
        } catch (e: any) {
            get().setErrorToast(`Transition failed: ${e}`);
            console.error("Transition failed:", e);
            throw e;
        }
    },

    clearLibrary: async () => {
        try {
            await invoke("clear_library");
            set({ tracks: [] });
        } catch (e: any) {
            get().setErrorToast(`Clear library failed: ${e}`);
            console.error("Clear library failed:", e);
        }
    },

    startRecording: async () => {
        try {
            const now = new Date();
            const filename = `AutoDJ_Mix_${now.getFullYear()}-${String(now.getMonth()+1).padStart(2,"0")}-${String(now.getDate()).padStart(2,"0")}_${String(now.getHours()).padStart(2,"0")}${String(now.getMinutes()).padStart(2,"0")}.wav`;
            await invoke("start_recording", { filename });
            set({ isRecording: true, recordingStartTime: Date.now() });
        } catch (e: any) {
            get().setErrorToast(`Recording failed: ${e}`);
        }
    },

    stopRecording: async () => {
        try {
            await invoke("stop_recording");
            set({ isRecording: false, recordingStartTime: null });
        } catch (e: any) {
            get().setErrorToast(`Stop recording failed: ${e}`);
        }
    },

    getAudioDevices: async () => {
        try {
            const devices = await invoke<string[]>("get_audio_devices");
            set({ audioDevices: devices });
        } catch (e: any) {
            console.error("Failed to get audio devices:", e);
        }
    },

    savePreference: async (key: string, value: string) => {
        try {
            await invoke("set_preference", { key, value });
        } catch (e: any) {
            console.error(`Failed to save preference ${key}:`, e);
        }
    },

    loadPreference: async (key: string) => {
        try {
            return await invoke<string | null>("get_preference", { key });
        } catch (e: any) {
            console.error(`Failed to load preference ${key}:`, e);
            return null;
        }
    },

    classifyAll: async () => {
        const state = get();
        if (state.batchClassifying) return;
        const unclassified = state.tracks.filter(
            t => t.import_status === "analyzed" && !t.genre && !state.classifyingTracks.has(t.id)
        );
        if (unclassified.length === 0) {
            get().setErrorToast("All analyzed tracks already classified");
            return;
        }
        set({ batchClassifying: true, batchClassifyProgress: { done: 0, total: unclassified.length } });
        for (let i = 0; i < unclassified.length; i++) {
            try {
                await get().classifyTrack(unclassified[i].id);
            } catch (e) {
                console.warn(`Batch classify failed for ${unclassified[i].id}:`, e);
            }
            set({ batchClassifyProgress: { done: i + 1, total: unclassified.length } });
        }
        set({ batchClassifying: false });
        await get().fetchTracks();
    },

    updateTrackClassification: async (trackId: string, field: string, value: string) => {
        try {
            // Get existing classification first, update the field
            const db_key = `classification_${trackId}_${field}`;
            await invoke("set_preference", { key: db_key, value });
            // Update local state
            set((state) => ({
                tracks: state.tracks.map(t => {
                    if (t.id === trackId) {
                        return { ...t, [field]: value };
                    }
                    return t;
                })
            }));
        } catch (e: any) {
            console.error(`Failed to update classification:`, e);
        }
    },

    setMode: async (mode: DjMode) => {
        try {
            await invoke("set_mode", { mode });
            set({ mode });
        } catch (e: any) {
            get().setErrorToast(`Set mode failed: ${e}`);
            console.error("Set mode failed:", e);
        }
    },

    requestNextTrack: async () => {
        set({ isThinking: true, aiDecision: null });
        try {
            const decision = await invoke<AiDecision>("request_next_track");
            set({ aiDecision: decision });
            return decision;
        } catch (e) {
            console.error("AI Planning failed:", e);
            return null;
        } finally {
            set({ isThinking: false });
        }
    },

    clearAiDecision: () => set({ aiDecision: null }),

    setAutodjStatus: (status: AutodjStatus, message?: string) => {
        set({ autodjStatus: status, autodjMessage: message ?? "" });
    },

    setAiConfig: async (config: any) => {
        try {
            await invoke("set_ai_config", { config });
            await invoke("set_preference", { key: "ai_config", value: JSON.stringify(config) });
            set({ aiConfig: config });
        } catch (e: any) {
            get().setErrorToast(`Set AI config failed: ${e}`);
            console.error("Set AI config failed:", e);
        }
    },

    setEngineState: (state: EngineState) => set({ engineState: state }),

    // ─── Playlists ──────────────────────────────────────────────

    fetchPlaylists: async () => {
        try {
            const playlists = await invoke<PlaylistRecord[]>("get_playlists");
            set({ playlists });
        } catch (e: any) {
            get().setErrorToast(`Failed to fetch playlists: ${e}`);
        }
    },

    createPlaylist: async (name: string, description?: string) => {
        try {
            const id = await invoke<string>("create_playlist", { name, description: description ?? null });
            await get().fetchPlaylists();
            return id;
        } catch (e: any) {
            get().setErrorToast(`Failed to create playlist: ${e}`);
            return null;
        }
    },

    renamePlaylist: async (playlistId: string, name: string) => {
        try {
            await invoke("rename_playlist", { playlistId, name });
            await get().fetchPlaylists();
        } catch (e: any) {
            get().setErrorToast(`Failed to rename playlist: ${e}`);
        }
    },

    deletePlaylist: async (playlistId: string) => {
        try {
            await invoke("delete_playlist", { playlistId });
            if (get().selectedPlaylistId === playlistId) {
                set({ selectedPlaylistId: null, playlistTracks: [] });
            }
            await get().fetchPlaylists();
        } catch (e: any) {
            get().setErrorToast(`Failed to delete playlist: ${e}`);
        }
    },

    selectPlaylist: async (playlistId: string | null) => {
        set({ selectedPlaylistId: playlistId });
        if (playlistId) {
            try {
                const tracks = await invoke<TrackRecord[]>("get_playlist_tracks", { playlistId });
                set({ playlistTracks: tracks });
            } catch (e: any) {
                get().setErrorToast(`Failed to load playlist: ${e}`);
                set({ playlistTracks: [] });
            }
        } else {
            set({ playlistTracks: [] });
        }
    },

    addTrackToPlaylist: async (playlistId: string, trackId: string) => {
        try {
            await invoke("add_track_to_playlist", { playlistId, trackId });
            await get().fetchPlaylists();
            if (get().selectedPlaylistId === playlistId) {
                await get().selectPlaylist(playlistId);
            }
        } catch (e: any) {
            get().setErrorToast(`Failed to add track: ${e}`);
        }
    },

    removeTrackFromPlaylist: async (playlistId: string, trackId: string) => {
        try {
            await invoke("remove_track_from_playlist", { playlistId, trackId });
            await get().fetchPlaylists();
            if (get().selectedPlaylistId === playlistId) {
                await get().selectPlaylist(playlistId);
            }
        } catch (e: any) {
            get().setErrorToast(`Failed to remove track: ${e}`);
        }
    },

    reorderPlaylist: async (playlistId: string, trackIds: string[]) => {
        try {
            await invoke("reorder_playlist", { playlistId, trackIds });
            if (get().selectedPlaylistId === playlistId) {
                await get().selectPlaylist(playlistId);
            }
        } catch (e: any) {
            get().setErrorToast(`Failed to reorder playlist: ${e}`);
        }
    },

    loadPlaylistToQueue: async (playlistId: string, shuffle: boolean) => {
        try {
            const tracks = await invoke<TrackRecord[]>("get_playlist_tracks", { playlistId });
            let ids = tracks.map(t => t.id);
            if (shuffle) {
                for (let i = ids.length - 1; i > 0; i--) {
                    const j = Math.floor(Math.random() * (i + 1));
                    [ids[i], ids[j]] = [ids[j], ids[i]];
                }
            }
            set({ autodjQueue: ids, autodjQueueIndex: 0, autodjShuffle: shuffle });
        } catch (e: any) {
            get().setErrorToast(`Failed to load playlist queue: ${e}`);
        }
    },

    advanceQueue: () => {
        const { autodjQueue, autodjQueueIndex } = get();
        if (autodjQueueIndex >= autodjQueue.length) return null;
        const trackId = autodjQueue[autodjQueueIndex];
        set({ autodjQueueIndex: autodjQueueIndex + 1 });
        return trackId;
    },

    getSessions: async () => {
        try {
            return await invoke<SessionRecord[]>("get_sessions");
        } catch (e: any) {
            get().setErrorToast(`Failed to get sessions: ${e}`);
            return [];
        }
    },

    exportCueSheet: async (sessionId: string) => {
        try {
            return await invoke<string>("export_cue_sheet", { sessionId });
        } catch (e: any) {
            get().setErrorToast(`Failed to export cue sheet: ${e}`);
            return "";
        }
    },

    // Phase 30: DJ Performance Controls
    beatRepeat: async (deck: string, beats: number) => {
        try { await invoke("beat_repeat", { deck, beats }); } catch (e: any) { get().setErrorToast(`Beat repeat failed: ${e}`); }
    },
    beatRepeatOff: async (deck: string) => {
        try { await invoke("beat_repeat_off", { deck }); } catch (e: any) { get().setErrorToast(`Beat repeat off failed: ${e}`); }
    },
    setSlipMode: async (deck: string, enabled: boolean) => {
        try { await invoke("set_slip_mode", { deck, enabled }); } catch (e: any) { get().setErrorToast(`Slip mode failed: ${e}`); }
    },
    vinylBrake: async (deck: string, durationMs: number) => {
        try { await invoke("vinyl_brake", { deck, durationMs }); } catch (e: any) { get().setErrorToast(`Vinyl brake failed: ${e}`); }
    },
    spinback: async (deck: string, durationMs: number) => {
        try { await invoke("spinback", { deck, durationMs }); } catch (e: any) { get().setErrorToast(`Spinback failed: ${e}`); }
    },
    jumpToMarker: async (deck: string, marker: string, positionMs: number) => {
        try { await invoke("jump_to_marker", { deck, marker, positionMs }); } catch (e: any) { get().setErrorToast(`Jump to marker failed: ${e}`); }
    },

    // Phase 33/39: AI Debug & Health
    testAiProvider: async () => {
        try {
            const result = await invoke<{ ok: boolean; message: string }>("test_ai_provider");
            return result;
        } catch (e: any) {
            return { ok: false, message: `${e}` };
        }
    },
    getAiDebugInfo: async () => {
        try {
            const info = await invoke<AiDebugInfo>("get_ai_debug_info");
            set({ aiDebugInfo: info });
        } catch (e: any) {
            console.error("Failed to get AI debug info", e);
        }
    },

    // Phase 35: DJ Personality
    setDjPersonality: (personality: string) => {
        set({ djPersonality: personality });
    },
}));
