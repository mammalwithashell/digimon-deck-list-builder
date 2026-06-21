## 1. Keyframes — the three phases

- [ ] 1.1 In `code/frontend/src/index.css`, add wireframe/grid-sweep keyframes layered over the card art.
- [ ] 1.2 Add orbit-ring spin-up keyframes (atom/ring), tinted by the card hue.
- [ ] 1.3 Add a flash-reveal keyframe for the new card + name, with an optional subtle Card-Slash beam accent.
- [ ] 1.4 Sequence the phases via `animation-delay` within the show window; keep total duration tasteful (~1.4–1.8s). Retire/repurpose the old `digivolveBanner` scaleX keyframes.

## 2. Component restyle (keep event effect intact)

- [ ] 2.1 In `DigivolveBanner.tsx`, keep the existing event-reading `useEffect` (event find, `lastSeqRef` dedupe, timer/dismiss) unchanged; swap only the rendered subtree to the phased cut-in.
- [ ] 2.2 Reuse `COLOR_HEX`/`COLOR_NAMES` for the card hue and combine with theme tokens for the tint.
- [ ] 2.3 Read the motion level (from `add-effects-and-motion-settings`) to choose the variant; keep `pointer-events: none` + click-to-dismiss + timer cleanup.

## 3. Motion degradation

- [ ] 3.1 `full`: render the complete wireframe → orbit → reveal sequence (+ beam accent).
- [ ] 3.2 `reduced`: render the simple banner + card drop that still identifies the digivolved card (consistent with the change ① functional-one-shot classification).
- [ ] 3.3 `off`: show the result instantly / minimally with no animated sequence.

## 4. Tests

- [ ] 4.1 Full sequence renders at motion `full`; simple banner at `reduced`; instant/minimal at `off`.
- [ ] 4.2 A single digivolve event yields exactly one cut-in (seq dedupe preserved) and auto-dismisses.
- [ ] 4.3 Overlay is non-interactive (`pointer-events: none`) and clears timers on unmount.

## 5. Verification

- [ ] 5.1 Manual pass in a real digivolve, both themes: sequence reads as the anime cut-in, tint matches the card color, pace feels good (also vs an agent at non-instant bot speed).
- [ ] 5.2 Confirm no double cut-ins and no change to event emission.
- [ ] 5.3 Typecheck + frontend tests green; `openspec validate upgrade-digivolve-cutin`.
