import { useEffect, useMemo, useState } from "react";
import { useStore } from "../store";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { formatTime, formatGroove } from "../components/utils";

export function useDeckState(deckId: "A" | "B") {
    const store = useStore();
    const { engineState, tracks, mode, getTrackProfile } = store;

    const snapshot = deckId === "A" ? engineState?.deck_a : engineState?.deck_b;
    const isPlaying = snapshot?.state === "playing";
    const isEmpty = !snapshot || snapshot.state === "empty";
    const deckKey = deckId.toLowerCase() as "a" | "b";

    const aPlaying = engineState?.deck_a?.state === "playing";
    const bPlaying = engineState?.deck_b?.state === "playing";
    let activeDeckKey: "a" | "b" = engineState && engineState.crossfader > 0 ? "b" : "a";
    if (aPlaying && !bPlaying) activeDeckKey = "a";
    if (!aPlaying && bPlaying) activeDeckKey = "b";
    const isActiveDeck = !isEmpty && activeDeckKey === deckKey;

    // ── Album Art + Hot Cues ──
    const [albumArtUrl, setAlbumArtUrl] = useState<string | null>(null);
    const [hotCues, setHotCues] = useState<Array<{ index: number; position_ms: number; label: string | null }>>([]);

    // ── Waveform Profile ──
    const [trackProfile, setTrackProfile] = useState<any>(null);

    const matchedTrack = useMemo(() => {
        if (!snapshot || snapshot.state === "empty") return null;
        const snapDur = snapshot.duration_ms;
        if (snapDur <= 0) return null;
        let best: any = null;
        let bestDiff = Infinity;
        for (const t of tracks) {
            if (t.duration_ms != null) {
                const diff = Math.abs(t.duration_ms - snapDur);
                if (diff < bestDiff && diff < 500) {
                    bestDiff = diff;
                    best = t;
                }
            }
        }
        return best;
    }, [snapshot?.duration_ms, snapshot?.state, tracks]);

    useEffect(() => {
        if (matchedTrack?.id) {
            getTrackProfile(matchedTrack.id).then(setTrackProfile);
            invoke<string | null>("get_album_art", { trackId: matchedTrack.id }).then(path => {
                if (path) setAlbumArtUrl(convertFileSrc(path));
                else setAlbumArtUrl(null);
            }).catch(() => setAlbumArtUrl(null));
            invoke<Array<[number, number, string | null]>>("get_hot_cues", { trackId: matchedTrack.id }).then(cues => {
                setHotCues(cues.map(([index, position_ms, label]) => ({ index, position_ms, label })));
            }).catch(() => setHotCues([]));
        } else {
            setTrackProfile(null);
            setAlbumArtUrl(null);
            setHotCues([]);
        }
    }, [matchedTrack?.id, getTrackProfile]);

    // ── Derive track info ──
    const trackTitle = matchedTrack?.title || (isEmpty ? "No Track Loaded" : "Unknown Track");
    const trackArtist = matchedTrack?.artist || "";
    const trackKey = matchedTrack?.key || "";
    const trackGenre = matchedTrack?.genre || "";
    const trackGroove = matchedTrack?.groove_type;
    const trackBpm = matchedTrack?.bpm;
    const displayBpm = trackBpm?.toFixed(1) || (engineState?.master_bpm ?? 128).toFixed(1);

    // ── Transition windows ──
    const transitionWindowsRaw = (trackProfile?.transition_windows ?? []) as Array<{
        anchor: string; start_ms: number; end_ms: number; score: number;
    }>;
    const transitionWindows = [...transitionWindowsRaw].sort((a, b) => a.start_ms - b.start_ms);

    return {
        snapshot,
        matchedTrack,
        albumArtUrl,
        setAlbumArtUrl,
        hotCues,
        setHotCues,
        trackProfile,
        isPlaying,
        isEmpty,
        isActiveDeck,
        deckKey,
        trackTitle,
        trackArtist,
        trackBpm,
        trackKey,
        trackGenre,
        trackGroove,
        displayBpm,
        transitionWindows,
        mode,
        engineState,
        store,
    };
}
