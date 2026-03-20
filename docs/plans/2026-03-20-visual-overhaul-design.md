# Visual Overhaul Design — djay Pro Inspired

**Date:** 2026-03-20
**Goal:** Redesign the AutoDJ frontend to match the aesthetic of djay Pro — unified layout with vinyl turntables, album art card grid library, playlist sidebar, and overall visual polish.

---

## Layout Architecture

Replace the current 3-view tabbed layout (Library/Playlists/DJ) with a **unified single-screen** layout where decks and library are always visible:

```
┌──────────────────────────────────────────────────┐
│ Top Bar: Track A info  |  AutoDJ logo  |  Track B│
├────────────────┬──────────┬──────────────────────┤
│   Deck A       │  Mixer   │   Deck B             │
│  (vinyl +      │  (EQ     │  (vinyl +            │
│   waveform +   │  knobs + │   waveform +         │
│   controls)    │  xfader) │   controls)          │
├────────────────┴──────────┴──────────────────────┤
│ Transport: Set | ◁ | ◁◁ | 4 ▸ | ▸▸ | ▸ | Loop  │
├──────────┬───────────────────────────────────────┤
│ Playlist │  Album Art Card Grid                  │
│ Sidebar  │  (scrollable, filterable, searchable) │
│ (local + │                                       │
│  Spotify)│                                       │
└──────────┴───────────────────────────────────────┘
```

Key changes from current:
- No tab switching — decks and library on same screen
- Playlist sidebar on left (Spotify playlists + local playlists)
- Album art grid replaces the track table
- Transport controls in horizontal strip between decks and library
- Settings stays as modal overlay

## Component: Vinyl Turntable (per deck)

- CSS-only spinning vinyl disc (~160px diameter)
- Album art circular crop centered (~80px) inside the platter
- Radial gradient for groove lines on the vinyl surface
- `@keyframes spin { to { rotate(360deg) } }` at ~1.8s per revolution (33rpm)
- `animation-play-state: paused` when deck is not playing
- Track info overlaid near vinyl: title, artist, BPM, key, time
- Waveform canvas below vinyl (~150px tall), same engine but more vivid colors
- Tonearm graphic (optional SVG) at top-right of platter

## Component: Album Art Card Grid

Replace table-based library with responsive card grid:

- Each card: album art square (~140px), title bold below, artist secondary below
- BPM + key as small semi-transparent badges overlaid on artwork bottom
- CSS Grid: `repeat(auto-fill, minmax(150px, 1fr))`
- Hover: slight scale-up (`transform: scale(1.03)`), glow border
- Click: loads to next available deck
- Right-click: context menu (Load A, Load B, Add to playlist)
- Search/filter bar above the grid (keep existing filters)
- Scrollable area with overflow-y

## Component: Playlist Sidebar

Left sidebar (~200px) in the library area:

- "Your Library" header
- Local playlists listed with name + track count
- Spotify section (when connected): playlists with green Spotify dot
- Clicking a playlist filters the card grid to those tracks
- "All Tracks" at top to show full library
- Collapsible toggle button
- Search icon for quick playlist search

## Component: Mixer (center column)

Keep existing vertical mixer but add visual polish:

- Circular knob-style EQ controls (CSS radial with rotation transform) instead of sliders
- Each knob: HI, MID, LO with value readout
- Kill buttons below each knob
- Gain knob per deck at top
- Crossfader with gradient track (cyan-to-amber)
- VU meters beside each deck's knobs (vertical LED-style bars)

## Visual Polish

- Keep existing dark theme colors (`--bg-deepest: #06060a`, etc.)
- More prominent gradients and glow effects on active elements
- Album art as visual anchors throughout (deck info, library cards)
- Smoother transitions (CSS `transition: all 0.2s ease`)
- Subtle background blur/gradient behind decks area
- Font hierarchy stays: Syne for headings, Space Grotesk for body

## Files to modify

- `ui/src/App.tsx` — restructure to unified layout, remove view tabs
- `ui/src/index.css` — new layout grid, vinyl styles, card grid styles, knob styles
- `ui/src/components/DeckPanel.tsx` — add vinyl turntable, restructure layout
- `ui/src/components/MixerChannel.tsx` — knob-style EQ, VU meters
- Create: `ui/src/components/VinylDisplay.tsx` — spinning vinyl with album art
- Create: `ui/src/components/TrackCardGrid.tsx` — album art card grid
- Create: `ui/src/components/PlaylistSidebar.tsx` — playlist sidebar
- `ui/src/components/ControlStrip.tsx` — consolidate into transport bar
- `ui/src/components/WaveformDisplay.tsx` — taller, more vivid
