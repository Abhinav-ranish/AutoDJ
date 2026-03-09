# AutoDJ Frontend — Performance Surface Audit
**Date:** 2026-03-08
**Scope:** UI/UX audit ahead of DJ cockpit redesign pass

---

## Current Strengths

- Strong dual-deck identity (cyan/amber accent system is coherent and consistent)
- Dark palette is appropriate for the domain — not overly styled
- Waveform canvas is already the largest single element
- 42px mono BPM readout in deck header provides good data prominence
- Deck color propagation via parent class selectors is clean and effective
- `has(:deck-state-badge.playing)` approach for active deck state is correct direction
- Mode pill segmented control is clean and functional
- Custom font stack (Syne / Space Grotesk / JetBrains Mono) is well-matched to the aesthetic
- Keyboard shortcuts (Space, Z, X, Q, W) present
- AutoDJ controller loop is well-implemented (phrase alignment, transition windows, silence detection)

---

## Current Weaknesses

### 1. Visual Hierarchy — Too Flat
Every major surface uses an identical treatment:
```
gradient bg + border-radius-lg + 1px border + shadow + glow
```
Deck panels, center mixer, AI bar, transitions panel — all read as equal-weight cards. There is no clear visual hierarchy between primary performance surfaces (decks, waveform, crossfader) and secondary utility surfaces (controls, metadata).

### 2. AI Bar Reads as a Status Strip
The AI bar is horizontal and compact. It presents NOW/NEXT information but:
- Everything has the same weight (mode pills, NOW, spectrum, NEXT, action)
- The spectrum occupies 180px of horizontal space — proportionally large relative to its informational value in live context
- AutoDJ status is a tiny dot + 10px mono text tucked in the corner — the AI's active state is the least visible thing on the bar
- No transition intelligence is surfaced (template, planner mode, confidence, timing reason)
- In manual/assisted mode, "Find Next" button is the only visible AI action — modest and unassuming when it should feel like a smart feature

### 3. Active Deck Glow is Too Subtle
The `:has(.deck-state-badge.playing)` glow uses `rgba(0, 212, 255, 0.06)` — effectively invisible at most monitor calibrations. The deck top-border uses `accent-a-dim` (not the full accent). The result: the playing deck barely reads as more active than the idle deck.

### 4. Center Mixer Has No Authority
The 220px center column is another card sitting between the decks. It lacks:
- A readable title or identity as "the mixer spine"
- Visual distinctiveness from the deck panel cards
- Crossfader that feels large/physical
- Clear A/B spatial orientation

### 5. Deck Panel Feels Stacked, Not Composed
The deck reads as a vertical stack of sections:
- Header (label + badge + BPM)
- Track info (art + title/artist + meta tags)
- Waveform
- Progress
- Tempo strip
- Control strip

Each section has a visual separator (border-top). This creates a "section list" feel rather than a coherent instrument. The waveform is visually comparable in weight to the control strip rows.

### 6. Meta-label Noise
The deck track info area has multiple small elements at similar size:
- KEY tag (mono 10px)
- GENRE tag (mono 10px, accent-a colored)
- GROOVE tag (mono 10px, accent-b colored)
- Each in a box with border

Three small tagged badges in a horizontal row creates visual noise that earns no space relative to what a DJ needs at a glance (the track name reads as only slightly larger at 16px).

### 7. Control Group Labeling
Control strip has group labels — `CUE`, `LOOP`, `FX`, `PERF` — at 9px mono. These labels are technically helpful but add visual noise in a dense strip. The implementation-language feel (especially `FX` with beat-repeat values `0.5 / 1 / 2 / 4`) reads as engineer-facing.

### 8. Spectrum Takes Too Much Space in AI Bar
The spectrum display takes 180px of horizontal space in the AI bar. It's a real-time display that adds authenticity but its position as a `flex-shrink: 0` item in the center of the bar pushes the more informative NOW/NEXT sections to smaller flex widths.

---

## What Still Feels App-Like vs DJ-Like

| Element | App-like | DJ-like | Gap |
|---------|----------|---------|-----|
| AI bar | status strip with equal-weight items | live conductor rail with hierarchy | layout/weight |
| Deck panels | stacked sections | coherent instrument surface | composition |
| Active deck | barely distinguishable from idle | commanding visual presence | glow/border |
| Center mixer | narrow utility card | the spine of the mixing surface | visual authority |
| BPM readout | good (42px) | — | already decent |
| Controls | button soup with group labels | tactile, purposeful, performable | density/grouping |
| AI status | tiny dot in corner | dominant status during autonomous mode | size/position |
| Waveform | 120px canvas | should dominate the deck | height/proportion |

---

## Hierarchy Problems

**Should dominate:**
1. Currently playing waveform (biggest spatial element per deck)
2. Active/playing deck panel overall
3. Center mixer crossfader ribbon
4. AI conductor rail — especially in AutoDJ mode

**Should be secondary (currently equals primary):**
- Paused/idle deck
- Meta tags, time readouts
- Group labels in control strip
- Spectrum bars

**Should be tertiary:**
- Numeric time readouts
- LOOP/FX/PERF labels
- BPM in AI bar (relative to track title)

---

## Top 5 Frontend Fixes (Rationale)

### Fix 1 — Redesign AI bar as a conductor rail
The vertical information hierarchy needs to be inverted. In AutoDJ mode, the AI's state should be the most prominent element. NOW/NEXT should be large and readable. The spectrum can be a decorative secondary element. Transition template and confidence should appear as chips.

**Rationale:** This is the single change that most changes the feeling from "status panel" to "live control rail." It requires both component restructuring and new CSS classes.

### Fix 2 — Strengthen playing deck emphasis
The active deck should visually "come alive" — full-strength accent border-top, meaningful glow (not 0.06 alpha), and a brighter waveform treatment. The idle deck should comparatively feel quieter.

**Rationale:** The cognitive load of "which deck is playing" is the highest-frequency performance question. Making the answer obvious at a glance is the most DJ-native ergonomic improvement possible.

### Fix 3 — Give the center mixer identity and authority
Add a MIXER spine label. Make the crossfader visually larger (thicker track, bigger thumb). Give the EQ sections clear color-coded headers for A and B. Make the center column feel like the physical spine it is, not a utility card.

**Rationale:** The crossfader is the single most important physical metaphor in DJing. It should feel expensive, tactile, and central.

### Fix 4 — Increase waveform visual dominance
Increase waveform height from 120px to 150px. The waveform is the primary real-time signal the DJ reads. It should dominate the deck panel more than any control section.

**Rationale:** In every major DJ software (Rekordbox, Serato, Traktor), the waveform is the largest element. AutoDJ's waveform is one of several equal-height sections — inverting that relationship makes the deck feel more like an instrument.

### Fix 5 — Reduce meta-label noise, improve track title presence
Increase deck track title to 18px (from 16px). Make meta-tags (KEY, GENRE, GROOVE) quieter — softer colors, no box borders. The track identity should be instantly readable.

**Rationale:** At live mixing speed, the DJ needs to read the track name at a glance. 16px bold in a panel with many competing 10px elements produces visual competition that requires more cognitive scanning than necessary.

---

## Implementation Plan

### Files to change
| File | Change | Purpose |
|------|--------|---------|
| `AiBar.tsx` | Structural redesign | Conductor rail layout, AI data exposure |
| `App.tsx` (DJView) | Add mixer spine label | Center mixer identity |
| `index.css` | Large hierarchical pass | All visual improvements |

### What should be visually dominant after redesign
1. Playing deck panel (active border, strong glow)
2. Waveform canvases (150px, dominate the deck)
3. Conductor rail NOW/NEXT sections (large track titles)
4. Center crossfader (thicker, larger thumb)
5. AutoDJ status in conductor rail (bold, not a dot)

### What should be visually quieter
1. Idle deck panel (muted border, no glow)
2. Meta tags in deck header (smaller, softer)
3. Control group labels (CUE, LOOP, FX, PERF — reduce or remove where obvious)
4. Spectrum bars (decorative, not commanding)
5. Time readouts (present but secondary)

### Risk areas
- AiBar is tightly coupled to AutoDJ controller loop (do not touch the useEffect logic)
- Responsive breakpoints need to be updated when renaming AI bar classes
- Waveform height change may affect overall DJ view scroll behavior on small screens
- Center mixer crossfader is in App.tsx DJView (not in a separate component) — be careful about inline edits

### Validation notes
- All three modes (manual / assisted / autodj) must remain functional
- TransitionsPanel (manual only) must still layout correctly below decks
- Library and Playlist views must be unaffected
- Keyboard shortcuts (Space, Z, X) are bound at App level — do not remove
