## Why

We want the app's two themes — dark "Digi-OS" and light "Adventure '99" — to feel
alive (live backgrounds, cursor-follow lighting, richer in-game VFX). Before adding
any of that, we need one place that decides *how much motion the user wants* and a
single gate every animation respects. Today that gate does not exist: only the app
shell's CRT scan line (`ds-crt-scan` in `components.css`) honors
`prefers-reduced-motion`, while the entire game animation library in `index.css`
(digivolve / battle / security / phase) ignores it. Shipping live atmosphere on top
of an ungated animation layer would be an accessibility regression and a performance
risk on the fixed 1920×1080 Tauri canvas. This change is the foundation the rest of
the "make it alive" roadmap reads from.

## What Changes

- Add a persisted **Motion** preference with three levels: `full` (all motion),
  `reduced` (essential/functional motion only — no ambient/looping effects), `off`
  (no non-essential animation). Default is derived from the OS
  `prefers-reduced-motion` setting on first run.
- Add a persisted **Live background** on/off preference (a coarse switch the later
  live-atmosphere change consumes; defined now so the control exists when atmosphere
  ships, and so it can default off under reduced/`off` motion).
- Apply the resolved motion level as a `data-motion` attribute on `<html>`, set
  pre-paint in `index.html` (mirroring the existing `data-theme` bootstrap) so there
  is no flash of motion before React hydrates.
- **Retrofit the existing animation library** (`index.css` keyframe/`animate-*`
  classes and the `components.css` CRT scan) to respect `data-motion`: ambient and
  looping effects stop at `reduced`/`off`; functional one-shot feedback (e.g. card
  enter, security reveal) is preserved at `reduced` and removed only at `off`.
- Surface both controls in the desktop **Graphics Settings** page, alongside the
  existing Theme and Fullscreen rows.
- Expose a small selector/helper so later changes (cursor lighting, live atmosphere,
  digivolve cut-in) read the effective motion level instead of re-deriving it.

No engine, gameplay, or network behavior changes. Frontend-only, desktop-targeted.

## Capabilities

### New Capabilities
- `ui-effects-preferences`: a persisted, user-facing Motion level and Live-background
  toggle that default from the OS reduced-motion setting, are exposed in Graphics
  Settings, and are applied as a global `data-motion` gate that the app's animation
  library (and future live-effect changes) honor for accessibility and performance.

### Modified Capabilities
<!-- None: no existing spec's requirements change. The Graphics Settings page is not
     currently covered by a capability spec, so adding controls to it introduces new
     behavior rather than modifying specified behavior. -->

## Impact

- **State**: `code/frontend/src/stores/uiStore.ts` — new `motion` and `liveBackground`
  fields with load/persist helpers + storage keys, following the existing pattern
  (`botSpeed`, `railCollapsed`).
- **Bootstrap**: `code/frontend/index.html` — pre-paint script resolves and sets
  `data-motion` (parallel to the existing `data-theme` bootstrap).
- **Styles**: `code/frontend/src/index.css` and
  `code/frontend/src/design/components/components.css` — animations gated behind
  `data-motion` / `prefers-reduced-motion`.
- **UI**: `code/frontend/src/pages/GraphicsSettingsPage.tsx` — new Motion select and
  Live-background toggle rows.
- **Tests**: `uiStore` persistence/default tests; a guard that the bootstrap storage
  key matches the store's literal (mirrors the existing theme bootstrap parity test).
- **Downstream**: unblocks `add-cursor-follow-lighting`, `add-live-theme-atmosphere`,
  `animate-board-atmosphere`, and `upgrade-digivolve-cutin`, which all read
  `data-motion` / the effective-motion helper.
