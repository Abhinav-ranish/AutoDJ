# Frontend Follow-up from Reality Audit

Date: 2026-03-09
Source of truth: `docs/reality_audit.md`

## UI issues from the audit that are real

1. Transition naming and presentation can over-imply DSP depth where runtime behavior is simulated.
2. Conductor status can look more explainable than the backend actually exposes.
3. Active performance state (who is on-air, what is next, why now) is still not clear enough at a glance.
4. Marker-aware mixing is implemented strongly but still undersold in the deck/conductor UI.
5. Surface weighting is too even; deck authority and center mixer authority are not yet performance-first.

## What the current UI still overstates

1. Simulated transition templates can read as if they are hardware-style FX.
2. Confidence/source chips can look like deep transparency rather than coarse planner metadata.
3. Tempo controls can be interpreted as deeper sync control than they are (current nudge is BPM shift, not phase nudge).

## What the current UI still undersells

1. Marker window selection and anchor timing as the primary “why now” transition reason.
2. Clear transition intent path: now track -> incoming track -> transition trigger.
3. On-air deck emphasis in mixed states where both decks may be running.

## Changes in this pass

1. Conductor rail
- Make decision source labels explicit and honest (LLM advisory vs heuristic vs queue-driven).
- Surface trigger intent and marker/position reason in plain language.
- Show simulated-template context directly in transition presentation when backend metadata provides it.
- Keep autodj decision display backend-state-first (`conductorState`) and use `aiDecision` as fallback only where needed.

2. Deck panels
- Add stronger active/standby deck state treatment.
- Add marker-window status line (in-window / next-window countdown) tied to transition windows from analysis profile.
- Keep progress markers but improve readability and purpose (“why this transition timing”).

3. Mixer and control ergonomics
- Strengthen center mixer authority and add clear master/active-side readout.
- Reframe control labels and helper copy to avoid implying a deep FX rack where none exists.
- Clarify tempo nudge behavior as BPM nudge only.

4. Transition template honesty
- Label simulated templates as simulated in selector copy and preserve concise descriptions of real behavior.

## Backend state still needed after this pass

1. Heuristic score breakdown by dimension for true explainability.
2. Decision-level vocal-risk and phrase-window diagnostics for stronger “why now” UI.
3. Stable backend-scheduled transition timing metadata beyond UI tick timing.
