## Why

Digivolution is the signature moment of Digimon, and the anime sells it with a
recognizable sequence: a wireframe grid sweeps over the character, an orbiting
atom/ring spins up, then the evolved form is revealed. Our current cut-in
(`DigivolveBanner`) is a single scaleX bar with a card drop — functional but flat. A
proper cut-in is a high-impact, self-contained upgrade that reuses the digivolve event
we already emit, and it slots cleanly onto the motion foundation so it degrades
gracefully for users who want less motion.

## What Changes

- Upgrade `DigivolveBanner` from the scaleX bar to a multi-phase cut-in at full motion:
  1. a wireframe/grid sweep over the evolving card art,
  2. an orbiting ring ("matrix"/atom) spin-up,
  3. a flash reveal of the new card + name, with an optional Card-Slash beam accent.
- Tint the sequence by the digivolving card's color (reuse the existing color lookup)
  and the active theme, so it reads on-theme in both Digi-OS and Adventure '99.
- Keep the **existing trigger and lifecycle unchanged**: it still listens to the
  `digivolve` event on `store.events`, dedupes by `seq` (rule 15, `lastSeqRef`), shows
  once, and auto-dismisses on a timer. No event/plumbing changes.
- Gate richness by `add-effects-and-motion-settings`: full sequence at motion `full`;
  degrade to the current simple banner at `reduced` (still shows what digivolved); at
  `off` the reveal is instant / minimal.
- Remain a non-interactive, dismissable overlay that cleans up and never blocks input.

No engine, gameplay, or network changes. Frontend-only, desktop-targeted.

## Capabilities

### New Capabilities
- `digivolve-cut-in`: a multi-phase, color/theme-tinted digivolution cut-in (wireframe
  → orbit ring → reveal) that reuses the existing digivolve event and lifecycle and
  degrades by motion level.

### Modified Capabilities
<!-- None at the requirement level. The digivolve event emission and game logic are
     unchanged; this restyles the existing client-side cut-in. -->

## Impact

- **Depends on**: `add-effects-and-motion-settings` (motion-level fallback). Optionally
  shares VFX primitives with `add-live-theme-atmosphere` but does not require it.
- **UI**: `code/frontend/src/components/game/DigivolveBanner.tsx` — replace the visual
  with the phased sequence (same event-reading effect, same timer/dismiss).
- **Styles**: `code/frontend/src/index.css` — replace/extend the
  `digivolveBanner`/`digivolveGlow`/`digivolveCardDrop` keyframes with the wireframe /
  orbit-ring / reveal phases; gate the elaborate phases by `data-motion`.
- **Reuse**: existing color lookup (`COLOR_HEX`/`COLOR_NAMES`) and the `Card`
  component for the revealed art.
- **Tests**: full sequence renders at motion `full`; simple banner at `reduced`;
  instant/minimal at `off`; still dedupes by `seq` and auto-dismisses; overlay is
  non-interactive.
