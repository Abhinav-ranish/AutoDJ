# Backend Follow-Up from Reality Audit

Date: 2026-03-09

## What is still real (backend/runtime)

1. Conductor ownership moved backend-side, but decision truth is still lossy.
- Files: `crates/autodj-app/src/conductor.rs`, `crates/autodj-app/src/commands.rs`
- Reality: `conductor-state` exists, but queue-driven picks and planner-failure fallbacks often lose structured source/fallback/template truth because snapshot fields are mostly derived from `last_decision` only.

2. Explainability is present but not structured enough for runtime auditing.
- Files: `crates/autodj-app/src/conductor.rs`, `crates/autodj-app/src/commands.rs`, `ui/src/store.ts`
- Reality: trigger timing is mostly encoded as string reasons (`window:...`, `fallback:...`) and requires parsing. We need explicit trigger metadata and explicit decision metadata.

3. Repetition controls are underfed by runtime ownership.
- Files: `crates/autodj-app/src/conductor.rs`, `crates/autodj-ai/src/planner.rs`
- Reality: planner supports `recent_templates`, replay caps, and FX cooldown memory, but conductor does not consistently record executed template/track/FX outcomes back into planner memory.

4. FX honesty is improved in docs/UI language, but runtime/template support still has a mismatch.
- Files: `crates/autodj-core/src/transition.rs`, `crates/autodj-ai/src/planner.rs`, `ui/src/components/AiBar.tsx`
- Reality: planner and UI reference `fx_echo_slam`, but core transition expander does not implement that template yet.

## File ownership map for this pass

- Conductor loop + runtime truth model: `crates/autodj-app/src/conductor.rs`
- Debug command surface: `crates/autodj-app/src/commands.rs`
- Planner memory hooks + anti-repetition runtime feeding: `crates/autodj-app/src/conductor.rs` and `crates/autodj-ai/src/planner.rs` (consumption side already exists)
- Transition template/runtime truth alignment: `crates/autodj-core/src/transition.rs`
- Frontend state shape for new backend metadata: `ui/src/store.ts` (consumer only)

## Changes in this pass

1. Add structured conductor metadata.
- Introduce explicit trigger metadata (kind/anchor/window/observed position).
- Introduce explicit decision metadata (source, fallback reason, selected track, template, entry marker, confidence, scores, simulated flag).
- Keep legacy snapshot fields for backward compatibility.

2. Make planner/runtime truth durable across non-LLM paths.
- Persist decision metadata even for queue-only and runtime fallback paths.
- Record executed template/track/artist/FX signals back into planner memory so anti-repetition logic has real runtime inputs.

3. Close template/runtime mismatch.
- Implement `fx_echo_slam` in transition expander (honestly as simulated/app-layer style behavior, not fake DSP claims).

4. Extend debug payloads.
- `get_ai_debug_info` will expose structured conductor trigger/decision objects so frontend/devtools can show exact runtime cause/effect chains.

## Explicitly not changing in this pass

- No new true delay/reverb/scratch DSP in audio thread.
- No sample-accurate master sync clock redesign.
- No rewrite of LLM-vs-heuristic architecture.
- No frontend orchestration logic reintroduction.
