# AutoDJ UI Redesign — Audit & Implementation Plan

**Date**: 2026-03-08
**Goal**: Transform the UI from a developer control surface into polished, premium DJ software.

---

## 1. UI Audit — Current State

### What works
- Design system foundation is solid: CSS variables, color tokens, typography scale
- Cyan/amber deck accent pairing is distinctive and readable
- Waveform rendering + VU meters are functional
- AI conductor panel concept is strong
- Library view is usable

### Critical problems

#### A. Flat deck hierarchy — no visual priority
The deck panel reads as a flat stack of elements: label, waveform, progress bar, deck-info, mixer controls, tempo controls, transport controls. Nothing signals "look here first." The waveform — the most important element during performance — gets the same visual weight as the EQ sliders.

#### B. Button soup in transport row
Transport, loop (4/8/16), hot cues (1-4), FX (beat repeat 0.5/1/2/4/off), and PERF (brake/spinback/slip on/off) are all crammed into a single flex row of identically-sized 28-40px buttons. 16+ buttons with no grouping borders, no spacing hierarchy, and emoji icons (🔁, 🛑, 🔄, 📌) that lack professional feel.

#### C. Mixer controls are horizontally cramped
Gain, HI/MID/LO EQ, and filter are all vertical sliders packed into a horizontal flex row. On narrow decks they collapse or feel Squeezed. The gain/EQ/filter distinction isn't spatially clear.

#### D. Tempo section feels disconnected
Tempo is a separate `<div>` below the mixer with its own border-top. The BPM display, pitch range selector, nudge buttons, tempo slider, sync button, and reset button create another cluster of small elements with weak grouping.

#### E. Bottom panels are unstructured
Below the decks, TransitionsPanel + AiPanel + AiDebugPanel sit in a `flex row` with inline styles. The transitions panel uses raw inline style-objects for everything. The AI Debug panel is developer-facing and shouldn't occupy equal visual real estate.

#### F. Information density without hierarchy
Deck info shows play state, track title, and time in one row — but the play state ("▶ Playing") and title compete for the same visual slot. Metadata (BPM, key, groove) is scattered across multiple locations.

#### G. Inconsistent styling approach
Some components use CSS classes (good), others use verbose inline `style={{}}` objects (transitions panel, AI debug). This creates maintenance debt and visual inconsistency.

#### H. Emoji as icons
🔁, 🛑, 🔄, 📌, ✨, 🤖, ⚙️ — these render differently per OS and don't match a professional DJ tool aesthetic.

---

## 2. Reference Image Analysis (Noisehead Wavesound XR 360 Pro)

The reference shows a hardware-inspired all-in-one DJ controller with:

### Visual language
- **Dark matte surfaces** with subtle depth/shadow, not flat backgrounds
- **Circular jog wheels** as the dominant visual anchor — large, glowing, with album art center
- **Physical knob metaphor**: round knobs labeled HIGH/GAIN/SEAT/HALT/COLOR arranged in a grid
- **BPM readout** is large (125, 128) and positioned prominently at top-left/right of each platter
- **Waveform display** is centered between platters in a dedicated screen area
- **Transport buttons** (CUE, ▷) are in a clear bottom strip per deck
- **Minimal labeling** — clear spatial organization makes labels unnecessary
- **No button clutter** — each control has breathing room

### Key design principles to adopt
1. **Large focal elements**: Waveform/deck area should be the largest, most prominent thing
2. **Spatial grouping via panels**: Knobs/controls grouped in defined physical zones
3. **BPM as a prominent readout**, not a small label
4. **Transport at the bottom**, clearly separated
5. **EQ/mixer controls centered** between decks (hardware-style channel strip)
6. **Minimal text**, maximum visual clarity
7. **Rounded, physical feel** for interactive elements

---

## 3. Proposed Redesign System

### Layout architecture
```
┌────────────────────────────────────────────────────────┐
│ HEADER: Logo │ Nav │ Master BPM │ Settings             │
├────────────────────────────────────────────────────────┤
│                    DJ VIEW                              │
│                                                         │
│  ┌─ DECK A ──────────┐  ┌─ MIXER ─┐  ┌─ DECK B ─────┐│
│  │ Track Title/Artist │  │  HI     │  │ Track/Artist  ││
│  │ BPM ● Key ● Status│  │  MID    │  │ BPM ● Key     ││
│  │ ╔══════════════╗   │  │  LO     │  │ ╔════════════╗││
│  │ ║  WAVEFORM    ║   │  │  GAIN   │  │ ║ WAVEFORM   ║││
│  │ ╚══════════════╝   │  │  FILTER │  │ ╚════════════╝││
│  │ Time / Progress    │  │         │  │ Time/Progress  ││
│  │ Tempo [===●===]    │  │ ═══●═══ │  │ Tempo Slider   ││
│  │ SYNC │ ↺ │ ±BPM   │  │  X-FAD  │  │ SYNC │ ↺       ││
│  ├────────────────────┤  └─────────┘  ├────────────────┤│
│  │ ▶/⏸  │ CUE 1234   │              │ ▶/⏸  │ CUE 1234││
│  │ LOOP 4 8 16 ✕      │              │ LOOP 4 8 16 ✕   ││
│  │ FX ½ 1 2 4 │ PERF  │              │ FX ½ 1 2 4      ││
│  └────────────────────┘              └──────────────────┘│
│                                                          │
│  ┌── AI CONDUCTOR ─────────────────────────────────────┐│
│  │  Mode │ Now Playing │ Up Next │ Status │ Action     ││
│  └──────────────────────────────────────────────────────┘│
└──────────────────────────────────────────────────────────┘
```

### Key structural changes
1. **Center mixer strip**: EQ + Gain + Filter + Crossfader in a vertical channel between decks
2. **Deck panels restructured top-to-bottom**: metadata → waveform → tempo → transport
3. **Transport/performance in distinct rows** at bottom of deck, not a single row
4. **AI panel as a slim horizontal bar** below, not a wide card
5. **AI Debug panel removed from main view** — moved to settings or a collapse toggle

### Component decomposition
```
App.tsx (slim shell)
├── Header
├── DJView
│   ├── DeckPanel (×2)
│   │   ├── DeckHeader (title, artist, status badge)
│   │   ├── DeckMetaBar (BPM, key, groove, genre)
│   │   ├── WaveformDisplay (canvas + VU)
│   │   ├── ProgressBar
│   │   ├── TempoStrip (slider + sync + nudge + reset)
│   │   └── ControlStrip
│   │       ├── TransportRow (play/pause, cue button)
│   │       ├── LoopRow (4/8/16/off)
│   │       ├── HotCueRow (1-4)
│   │       └── FxRow (beat repeat + perf)
│   ├── MixerChannel
│   │   ├── EqStrip (hi/mid/lo vertical)
│   │   ├── GainSlider
│   │   ├── FilterKnob
│   │   └── CrossfaderStrip
│   └── AiBar
│       ├── ModeSwitcher
│       ├── NowPlaying
│       ├── UpNext
│       └── StatusAction
├── LibraryView
└── PlaylistView
```

### Design tokens additions
```css
/* Panel system */
--panel-bg: #0c0c16;
--panel-border: #1a1a2e;
--panel-radius: 12px;
--panel-padding: 16px;

/* Control sizing */
--btn-sm: 28px;
--btn-md: 36px;
--btn-lg: 44px;

/* Section spacing */
--section-gap: 12px;
--control-gap: 6px;
```

---

## 4. Phased Implementation Plan

### Phase 1: Structural Cleanup & Component Extraction
**Scope**: Break the 2500-line monolith App.tsx into proper components. Establish the new layout grid. No visual redesign yet — just restructure.

Files to create:
- `ui/src/components/DeckPanel.tsx`
- `ui/src/components/DeckHeader.tsx`
- `ui/src/components/WaveformDisplay.tsx`
- `ui/src/components/MixerChannel.tsx`
- `ui/src/components/ControlStrip.tsx`
- `ui/src/components/TempoStrip.tsx`
- `ui/src/components/AiBar.tsx`
- `ui/src/components/TransitionsPanel.tsx`

Layout changes:
- DJ view shifts to 3-column grid: Deck A | Mixer | Deck B
- Each deck restructured into vertical sections
- Transport controls grouped into ControlStrip with labeled rows

### Phase 2: Visual Polish & Deck Redesign
**Scope**: Restyle deck panels with proper hierarchy. Larger waveforms, prominent BPM, better spacing, physical-feel controls.

- Waveform height increased to 100-120px
- BPM shown as a large monospace readout (like reference: "125")
- Track title/artist get more space
- Playing state as a subtle glow/badge, not text
- EQ/mixer controls get the center channel treatment
- Crossfader redesigned with better visual weight
- Remove all emoji icons, replace with SVG/CSS icons

### Phase 3: Control Grouping & Interaction Polish
**Scope**: Proper button primitives, grouped control sections, hover/active states, keyboard shortcuts visualization.

- DJButton primitive with consistent sizing, icon support, accent variants
- Loop/Cue/FX rows get subtle panel backgrounds and labels
- Active states: loop active glow, playing deck glow, armed cue highlight
- Transitions panel redesigned as a proper panel component
- AI Debug panel collapsed by default, expandable

### Phase 4: AI Bar & Final Integration
**Scope**: Compact AI conductor bar, final visual harmonization, animation polish.

- AI panel becomes a horizontal strip below decks
- Mode pill switcher refined
- Now Playing / Up Next in a compact horizontal layout
- Recording indicator in header
- Final spacing/padding pass across all views
- Library and Playlist views get matching style updates
