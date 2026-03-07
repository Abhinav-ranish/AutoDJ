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

export interface AiDecision {
    decision: string;
    reason: string;
    track_selection?: string;
    transition_plan?: {
        template_name: string;
        parameter_overrides?: any;
    };
}

export type DjMode = "manual" | "assisted" | "autodj";

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
    pollEngineState: () => Promise<void>;
    setActiveView: (view: "dj" | "library") => void;
    analyzeTrack: (trackId: string) => Promise<void>;
    getTrackProfile: (trackId: string) => Promise<any>;
    executeTransition: (deck: string, template: string, durationBars: number) => Promise<void>;
    clearLibrary: () => Promise<void>;

    // AI Actions
    setMode: (mode: DjMode) => Promise<void>;
    requestNextTrack: () => Promise<void>;
    setAiConfig: (config: any) => Promise<void>;

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
    aiConfig: null,

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

            // AutoDJ Mode: Auto-execute transition if decision is ready
            if (get().mode === "autodj" && decision.decision === "transition_now" && decision.transition_plan) {
                const plan = decision.transition_plan;
                const overrides = plan.parameter_overrides || {};
                const durationBars = overrides.duration_bars || 16;
                const incomingDeck = overrides.incoming_deck ? overrides.incoming_deck.toLowerCase() : "b";

                // For auto-transition, we target the deck that is currently playing to execute the outgoing transition.
                // Assuming we get the currently active deck (naive approach here, since we don't know for sure).
                // Usually the transition is executed ON the playing deck TO the incoming deck.
                // For now use the 'incomingDeck' inverse, or just pass 'a' if incoming is 'b'.
                const targetDeck = incomingDeck === "a" ? "B" : "A";

                await get().executeTransition(targetDeck, plan.template_name, durationBars);
            }
        } catch (e) {
            console.error("AI Planning failed:", e);
        }
        set({ isThinking: false });
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
