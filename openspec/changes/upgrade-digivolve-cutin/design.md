## Context

`DigivolveBanner` (`code/frontend/src/components/game/DigivolveBanner.tsx`) is a
`position: fixed inset-0` overlay that reads `useGameStore(s => s.events)`, finds the
latest `digivolve` event with `seq > lastSeqRef`, derives the card id/name and a color
index from the post-digivolve permanent, shows for 1400ms, and auto-dismisses (click
also dismisses). Its visuals use the `animate-digivolve-banner` and
`animate-digivolve-card-drop` keyframes in `index.css`, plus a `--glow-color` from
`COLOR_HEX`. The motion preference (`add-effects-and-motion-settings`) provides the
`full`/`reduced`/`off` level; the change ① classification treats a digivolve banner as
functional one-shot (survives `reduced`).

## Goals / Non-Goals

**Goals:**
- A recognizable anime-style cut-in: wireframe sweep → orbit ring → reveal.
- Reuse the existing trigger, dedupe, and lifecycle exactly (rule 15).
- Tint by card color + theme; read well in both themes.
- Graceful degradation by motion level.

**Non-Goals:**
- Changing how/when the `digivolve` event is emitted (engine/server untouched).
- Battle/security/phase VFX (separate, existing).
- Per-Digimon bespoke animations / 3D; this is a stylized 2D cut-in over the card art.
- Audio (deferred).

## Decisions

### Decision: Restyle the existing component, keep the event effect intact
Only the rendered visual + a small phase timeline change; the `useEffect` that reads
events, sets `lastSeqRef`, and arms the dismiss timer is preserved. This keeps the
rule-15 dedupe/lifecycle contract and avoids any double-trigger risk. Alternative
considered: a new component subscribing separately — rejected (duplicate event wiring,
risk of double cut-ins).

### Decision: Phase the cut-in with a short internal timeline
Drive three phases (wireframe ~0–35%, orbit ring ~25–70%, reveal ~60–100%) within the
existing show window (extend modestly if needed, e.g. ~1.4–1.8s). Implement as CSS
keyframes per phase layered over the `Card` art, sequenced by `animation-delay` (no
heavy JS timeline). Alternative considered: a JS animation library — rejected
(unnecessary dependency; CSS keyframes are enough and match the existing approach).

### Decision: Tint from card color + theme, reuse existing lookup
Keep the `COLOR_HEX`/`COLOR_NAMES` derivation for the per-card hue; combine with theme
tokens so the wireframe/ring/flash read correctly on both dark and light. The orbit
ring and wireframe use the card hue; the flash uses a theme-aware white/screen.

### Decision: Degrade by motion level
- `full`: the complete wireframe → orbit → reveal sequence (+ optional Card-Slash beam
  accent on reveal).
- `reduced`: fall back to today's simple banner + card drop (still communicates *what*
  digivolved — functional one-shot, consistent with change ①).
- `off`: instant/minimal — show the result briefly with no animation (or skip the
  cinematic entirely), never a long sequence.
Gating reads `data-motion` (CSS) and/or the motion accessor (to pick the variant).

### Decision: Non-interactive, self-cleaning overlay
Keep `pointer-events: none` on the container (click-to-dismiss handled as today),
clear timers on unmount, and ensure the overlay never persists or blocks the board.

## Risks / Trade-offs

- [A longer/elaborate cut-in slows game pace, especially vs an agent] → keep the total
  duration tasteful (~1.4–1.8s), reuse the auto-dismiss, and ensure it overlays
  (non-blocking) so play continues; consider the bot-speed context during tuning.
- [Wireframe/ring over arbitrary card art looks wrong for some cards] → keep the
  wireframe a stylized overlay (grid + scan) rather than a true 3D mesh, so it works
  uniformly over any 2D card image.
- [Motion fallback drift from change ①’s classification] → explicitly map the three
  levels here and reference change ①; `reduced` keeps the functional banner, not the
  cinematic.
- [Double cut-ins / missed dedupe if the effect is refactored] → do not touch the
  event-reading effect; only swap the rendered subtree and keyframes.

## Open Questions

- Total duration: keep ~1.4s or extend to ~1.8s for the three phases to breathe?
  Resolve during implementation against game-pace feel (and bot speed).
- Include the Card-Slash beam accent now or leave a hook for later? Lean: include a
  subtle beam on reveal at `full` only.
