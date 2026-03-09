import { useEffect, useRef } from "react";
import type { DeckSnapshot } from "../store";
import { useStore } from "../store";

function drawWaveform(
    ctx: CanvasRenderingContext2D,
    width: number,
    height: number,
    profile: any,
    positionFraction: number,
    accentColor: string,
    bgColor: string
) {
    ctx.clearRect(0, 0, width, height);
    ctx.fillStyle = bgColor;
    ctx.fillRect(0, 0, width, height);

    let data: number[] = [];
    if (profile) {
        const keys = ["rms_db_per_beat", "energy_per_bar", "energy_per_beat", "rms"];
        for (const k of keys) {
            if (Array.isArray(profile[k]) && profile[k].length > 0) {
                data = profile[k];
                break;
            }
        }
        if (data.length === 0) {
            for (const val of Object.values(profile)) {
                if (Array.isArray(val) && val.length > 0 && typeof val[0] === 'number') {
                    data = val as number[];
                    break;
                }
            }
        }
    }

    if (data.length > 0) {
        const barGap = 1;
        const barWidth = Math.max(1, width / data.length);
        const maxVal = Math.max(...data.map(Math.abs).filter(v => !isNaN(v))) || 1;

        for (let i = 0; i < data.length; i++) {
            const val = Math.abs(data[i]);
            const h = (val / maxVal) * (height * 0.85);
            const x = i * (width / data.length);
            const y = (height - h) / 2;
            const isPast = (i / data.length) < positionFraction;

            ctx.fillStyle = accentColor;
            ctx.globalAlpha = isPast ? 0.3 : 0.9;
            ctx.fillRect(x, y, Math.max(1, barWidth - barGap), h);
        }
    } else {
        ctx.strokeStyle = accentColor;
        ctx.lineWidth = 1;
        ctx.globalAlpha = 0.2;
        ctx.beginPath();
        const midY = height / 2;
        for (let x = 0; x < width; x += 3) {
            const h = 8 + Math.sin(x * 0.08) * 12 + Math.cos(x * 0.025) * 8;
            ctx.moveTo(x, midY - h / 2);
            ctx.lineTo(x, midY + h / 2);
        }
        ctx.stroke();
    }

    // Playhead
    ctx.globalAlpha = 1.0;
    const playheadX = positionFraction * width;

    // Playhead glow
    const grad = ctx.createLinearGradient(playheadX - 8, 0, playheadX + 8, 0);
    grad.addColorStop(0, "transparent");
    grad.addColorStop(0.4, accentColor + "40");
    grad.addColorStop(0.5, accentColor + "80");
    grad.addColorStop(0.6, accentColor + "40");
    grad.addColorStop(1, "transparent");
    ctx.fillStyle = grad;
    ctx.fillRect(playheadX - 8, 0, 16, height);

    // Playhead line
    ctx.strokeStyle = "#ffffff";
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.moveTo(playheadX, 0);
    ctx.lineTo(playheadX, height);
    ctx.stroke();
}

export function VuMeter({ rms, peak, accent }: { rms: number; peak: number; accent: string }) {
    const toDb = (v: number) => (v > 0.0001 ? 20 * Math.log10(v) : -80);
    const dbToPercent = (db: number) => Math.max(0, Math.min(100, ((db + 60) / 60) * 100));

    const rmsPercent = dbToPercent(toDb(rms));
    const peakPercent = dbToPercent(toDb(peak));
    const rmsDb = toDb(rms);
    const barColor =
        rmsDb > -3 ? "var(--status-error)" : rmsDb > -12 ? "var(--status-pending)" : accent;

    return (
        <div className="vu-meter">
            <div className="vu-meter-track">
                <div className="vu-meter-fill" style={{ height: `${rmsPercent}%`, background: barColor }} />
                {peak > 0.0001 && (
                    <div className="vu-meter-peak" style={{ bottom: `${peakPercent}%`, background: accent }} />
                )}
            </div>
        </div>
    );
}

export function WaveformDisplay({
    deckId,
    snapshot,
    trackProfile,
    isEmpty,
}: {
    deckId: "A" | "B";
    snapshot: any;
    trackProfile: any;
    isEmpty: boolean;
}) {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const { seek } = useStore();

    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) return;
        const resize = () => {
            canvas.width = canvas.offsetWidth * window.devicePixelRatio;
            canvas.height = canvas.offsetHeight * window.devicePixelRatio;
            const ctx = canvas.getContext("2d");
            if (ctx) ctx.scale(window.devicePixelRatio, window.devicePixelRatio);
        };
        const observer = new ResizeObserver(resize);
        observer.observe(canvas);
        resize();
        return () => observer.disconnect();
    }, [isEmpty]);

    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas || isEmpty) return;
        const ctx = canvas.getContext("2d");
        if (!ctx) return;

        const w = canvas.offsetWidth;
        const h = canvas.offsetHeight;
        ctx.setTransform(window.devicePixelRatio, 0, 0, window.devicePixelRatio, 0, 0);

        const computedStyle = getComputedStyle(canvas);
        const accentVar = deckId === "A" ? "--accent-a" : "--accent-b";
        const accentColor = computedStyle.getPropertyValue(accentVar).trim() || (deckId === "A" ? "#00d4ff" : "#ff9500");
        const bgColor = "#0e0e1a";

        drawWaveform(ctx, w, h, trackProfile, snapshot?.position_fraction ?? 0, accentColor, bgColor);
    }, [snapshot?.position_fraction, trackProfile, isEmpty, deckId]);

    const handleSeek = (e: React.MouseEvent<HTMLCanvasElement>) => {
        const canvas = canvasRef.current;
        if (!canvas || !snapshot) return;
        const rect = canvas.getBoundingClientRect();
        const fraction = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
        seek(deckId, fraction * snapshot.duration_ms);
    };

    if (isEmpty) {
        return (
            <div className="waveform-empty">
                <span>No track loaded</span>
            </div>
        );
    }

    return (
        <div className="waveform-container">
            <canvas
                ref={canvasRef}
                className="waveform-canvas"
                onClick={handleSeek}
            />
            <VuMeter
                rms={snapshot?.rms ?? 0}
                peak={snapshot?.peak ?? 0}
                accent={deckId === "A" ? "var(--accent-a)" : "var(--accent-b)"}
            />
        </div>
    );
}
