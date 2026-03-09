// Shared helpers for the DJ UI
export function formatTime(ms: number): string {
    const totalSeconds = Math.floor(ms / 1000);
    const minutes = Math.floor(totalSeconds / 60);
    const seconds = totalSeconds % 60;
    return `${minutes}:${seconds.toString().padStart(2, "0")}`;
}

export function formatGroove(groove: string): string {
    const labels: Record<string, string> = {
        four_on_floor: "4/4",
        trap_half_time: "Trap ½×",
        dembow: "Dembow",
        breakbeat: "Breakbeat",
        boom_bap: "Boom Bap",
        drill: "Drill",
        jersey_club: "Jersey",
        drum_and_bass: "DnB",
        standard: "Std",
    };
    return labels[groove] || groove.replace(/_/g, " ");
}

/** Convert cutoff Hz → 0–100 slider position */
export function filterToSlider(hz: number): number {
    if (hz >= 19500) return 50;
    if (hz <= 1000) {
        const frac = (hz - 20) / (1000 - 20);
        return 51 + frac * 49;
    } else {
        const frac = (hz - 1000) / (20000 - 1000);
        return frac * 49;
    }
}

/** Convert 0–100 slider value to cutoff Hz */
export function sliderToFilter(val: number): number {
    if (val === 50) return 20000;
    if (val < 50) {
        const frac = val / 49;
        return 1000 + frac * (20000 - 1000);
    } else {
        const frac = (val - 51) / 49;
        return 20 + frac * (1000 - 20);
    }
}
