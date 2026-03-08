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
}

export interface EngineState {
    deck_a: DeckSnapshot;
    deck_b: DeckSnapshot;
    crossfader: number;
    master_bpm: number;
    beat_number: number;
    bar_number: number;
    sample_pos: number;
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

interface DJStore {
    tracks: TrackRecord[];
    engineState: EngineState | null;
    importing: boolean;
    activeView: "dj" | "library";
    mode: DjMode;
    aiDecision: AiDecision | null;
    isThinking: boolean;
    errorToast: string | null;
    analyzingTracks: Set<string>;
    classifyingTracks: Set<string>;
    autodjStatus: AutodjStatus;
    autodjMessage: string;
    autodjTransitioning: boolean;
    aiConfig: {
        enabled: boolean;
        api_key: string;
        model: string;
        endpoint_url: string;
        timeout_seconds: number;
        prompt_template: string | null;
    } | null;

    // Actions
    initStore: () => Promise<void>;
    fetchTracks: () => Promise<void>;
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
    setActiveView: (view: "dj" | "library") => void;
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

    setErrorToast: (msg: string | null) => {
        set({ errorToast: msg });
        if (msg) setTimeout(() => set({ errorToast: null }), 3000);
    },

    initStore: async () => {
        try {
            const saved = localStorage.getItem("autodj_ai_config");
            if (saved) {
                const config = JSON.parse(saved);
                set({ aiConfig: config });
                await invoke("set_ai_config", { config });
            }
        } catch (e) {
            console.error("Failed to restore AI config from localStorage", e);
        }
    },

    fetchTracks: async () => {
        try {
            const tracks = await invoke<TrackRecord[]>("get_tracks");
            set({ tracks });
        } catch (e: any) {
            get().setErrorToast(`Failed to fetch tracks: ${e}`);
            console.error(e);
        }
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
            localStorage.setItem("autodj_ai_config", JSON.stringify(config));
            set({ aiConfig: config });
        } catch (e: any) {
            get().setErrorToast(`Set AI config failed: ${e}`);
            console.error("Set AI config failed:", e);
        }
    },

    setEngineState: (state: EngineState) => set({ engineState: state }),
}));
