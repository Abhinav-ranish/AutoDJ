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
}

export interface DeckSnapshot {
    state: string;
    position_ms: number;
    duration_ms: number;
    position_fraction: number;
    gain_db: number;
    eq_low: number;
    eq_mid: number;
    eq_high: number;
    filter_cutoff: number;
    filter_resonance: number;
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

interface DJStore {
    tracks: TrackRecord[];
    engineState: EngineState | null;
    importing: boolean;
    activeView: "dj" | "library";

    // Actions
    fetchTracks: () => Promise<void>;
    importFiles: (paths: string[]) => Promise<void>;
    loadTrack: (deck: string, trackId: string) => Promise<void>;
    play: (deck: string) => Promise<void>;
    pause: (deck: string) => Promise<void>;
    setCrossfader: (position: number) => Promise<void>;
    setDeckGain: (deck: string, gainDb: number) => Promise<void>;
    setEq: (deck: string, band: string, gainDb: number) => Promise<void>;
    setFilter: (deck: string, cutoffHz: number, resonance: number) => Promise<void>;
    pollEngineState: () => Promise<void>;
    setActiveView: (view: "dj" | "library") => void;
}

export const useStore = create<DJStore>((set, get) => ({
    tracks: [],
    engineState: null,
    importing: false,
    activeView: "library",

    fetchTracks: async () => {
        try {
            const tracks = await invoke<TrackRecord[]>("get_tracks");
            set({ tracks });
        } catch (e) {
            console.error("Failed to fetch tracks:", e);
        }
    },

    importFiles: async (paths: string[]) => {
        set({ importing: true });
        try {
            await invoke("import_files", { paths });
            await get().fetchTracks();
        } catch (e) {
            console.error("Import failed:", e);
        }
        set({ importing: false });
    },

    loadTrack: async (deck: string, trackId: string) => {
        try {
            await invoke("load_track", { deck, trackId });
            set({ activeView: "dj" });
        } catch (e) {
            console.error("Load failed:", e);
        }
    },

    play: async (deck: string) => {
        try {
            await invoke("play", { deck });
        } catch (e) {
            console.error("Play failed:", e);
        }
    },

    pause: async (deck: string) => {
        try {
            await invoke("pause", { deck });
        } catch (e) {
            console.error("Pause failed:", e);
        }
    },

    setCrossfader: async (position: number) => {
        try {
            await invoke("set_crossfader", { position });
        } catch (e) {
            console.error("Crossfader failed:", e);
        }
    },

    setDeckGain: async (deck: string, gainDb: number) => {
        try {
            await invoke("set_deck_gain", { deck, gainDb });
        } catch (e) {
            console.error("Gain failed:", e);
        }
    },

    setEq: async (deck: string, band: string, gainDb: number) => {
        try {
            await invoke("set_eq", { deck, band, gainDb });
        } catch (e) {
            console.error("EQ failed:", e);
        }
    },

    setFilter: async (deck: string, cutoffHz: number, resonance: number) => {
        try {
            await invoke("set_filter", { deck, cutoffHz, resonance });
        } catch (e) {
            console.error("Filter failed:", e);
        }
    },

    pollEngineState: async () => {
        try {
            const state = await invoke<EngineState | null>("get_engine_state");
            if (state) {
                set({ engineState: state });
            }
        } catch (e) {
            // Silently ignore polling errors
        }
    },

    setActiveView: (view) => set({ activeView: view }),
}));
