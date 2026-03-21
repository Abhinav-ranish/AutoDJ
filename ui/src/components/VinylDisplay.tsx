export function VinylDisplay({
    albumArtUrl,
    isPlaying,
    deckId,
}: {
    albumArtUrl: string | null;
    isPlaying: boolean;
    deckId: "A" | "B";
}) {
    const accentClass = deckId === "A" ? "vinyl-deck-a" : "vinyl-deck-b";

    return (
        <div className={`vinyl-display ${accentClass}`}>
            <div
                className="vinyl-platter"
                style={{ animationPlayState: isPlaying ? "running" : "paused" }}
            >
                <div className="vinyl-grooves" />
                <div className="vinyl-label">
                    {albumArtUrl ? (
                        <img src={albumArtUrl} alt="" className="vinyl-label-art" />
                    ) : (
                        <div className="vinyl-label-placeholder">
                            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                                <circle cx="12" cy="12" r="10" />
                                <circle cx="12" cy="12" r="3" />
                            </svg>
                        </div>
                    )}
                </div>
                <div className="vinyl-spindle" />
            </div>
        </div>
    );
}
