// Lightweight SVG icons — no emoji, no dependencies
export function IconPlay({ size = 16 }: { size?: number }) {
    return (
        <svg width={size} height={size} viewBox="0 0 16 16" fill="currentColor">
            <path d="M4 2.5v11l9-5.5z" />
        </svg>
    );
}

export function IconPause({ size = 16 }: { size?: number }) {
    return (
        <svg width={size} height={size} viewBox="0 0 16 16" fill="currentColor">
            <rect x="3" y="2" width="3.5" height="12" rx="0.5" />
            <rect x="9.5" y="2" width="3.5" height="12" rx="0.5" />
        </svg>
    );
}

export function IconStop({ size = 14 }: { size?: number }) {
    return (
        <svg width={size} height={size} viewBox="0 0 14 14" fill="currentColor">
            <rect x="2" y="2" width="10" height="10" rx="1" />
        </svg>
    );
}

export function IconLoop({ size = 14 }: { size?: number }) {
    return (
        <svg width={size} height={size} viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
            <path d="M10.5 3.5H5a2.5 2.5 0 000 5h4a2.5 2.5 0 010 5H3.5" />
            <polyline points="8.5,1.5 10.5,3.5 8.5,5.5" />
            <polyline points="5.5,8.5 3.5,10.5 5.5,12.5" />
        </svg>
    );
}

export function IconSync({ size = 14 }: { size?: number }) {
    return (
        <svg width={size} height={size} viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
            <path d="M1.5 7a5.5 5.5 0 019.13-3M12.5 7a5.5 5.5 0 01-9.13 3" />
            <polyline points="10.63,1.5 10.63,4 8.13,4" />
            <polyline points="3.37,12.5 3.37,10 5.87,10" />
        </svg>
    );
}

export function IconReset({ size = 14 }: { size?: number }) {
    return (
        <svg width={size} height={size} viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
            <path d="M2 7a5 5 0 119 3" />
            <polyline points="2,4 2,7 5,7" />
        </svg>
    );
}

export function IconBrake({ size = 14 }: { size?: number }) {
    return (
        <svg width={size} height={size} viewBox="0 0 14 14" fill="currentColor">
            <circle cx="7" cy="7" r="5.5" fill="none" stroke="currentColor" strokeWidth="1.5" />
            <rect x="5" y="4" width="4" height="6" rx="0.5" />
        </svg>
    );
}

export function IconSpinback({ size = 14 }: { size?: number }) {
    return (
        <svg width={size} height={size} viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
            <path d="M12 7A5 5 0 112 7a5 5 0 0110 0z" />
            <polyline points="2,4 2,7 5,7" />
        </svg>
    );
}

export function IconSlip({ size = 14 }: { size?: number }) {
    return (
        <svg width={size} height={size} viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
            <line x1="7" y1="2" x2="7" y2="12" />
            <circle cx="7" cy="7" r="2" fill="currentColor" />
        </svg>
    );
}

export function IconRepeat({ size = 14 }: { size?: number }) {
    return (
        <svg width={size} height={size} viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
            <rect x="2" y="3" width="10" height="8" rx="2" />
            <path d="M5 7h4M7 5v4" />
        </svg>
    );
}

export function IconX({ size = 12 }: { size?: number }) {
    return (
        <svg width={size} height={size} viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
            <line x1="2" y1="2" x2="10" y2="10" />
            <line x1="10" y1="2" x2="2" y2="10" />
        </svg>
    );
}

export function IconSettings({ size = 18 }: { size?: number }) {
    return (
        <svg width={size} height={size} viewBox="0 0 18 18" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
            <circle cx="9" cy="9" r="2.5" />
            <path d="M9 1.5v2M9 14.5v2M1.5 9h2M14.5 9h2M3.1 3.1l1.4 1.4M13.5 13.5l1.4 1.4M3.1 14.9l1.4-1.4M13.5 4.5l1.4-1.4" />
        </svg>
    );
}

export function IconRecord({ size = 10 }: { size?: number }) {
    return (
        <svg width={size} height={size} viewBox="0 0 10 10">
            <circle cx="5" cy="5" r="5" fill="currentColor" />
        </svg>
    );
}
