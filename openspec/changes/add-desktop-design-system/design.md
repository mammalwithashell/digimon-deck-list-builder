## Context

The desktop frontend (`code/frontend/src/`, React 18 + TS + Vite, Tailwind 4 +
plain CSS, Zustand state) currently carries three unrelated styling systems with
no shared tokens and no theme switching:

- the game board's OKLCH `.ib-*` palette in `index.css`,
- the deck-builder / launcher hex chrome (`--bld-*` in `DeckBuilderPage.css`,
  `--bg-*` in `launcher.css`, inline navy hex across lobby/mode-select), and
- the landing page's standalone phosphor-CRT look in `code/landing/index.html`.

The launcher and board already aim at a dark "Digi-OS" terminal direction, but
the rest of the client wears a legacy navy-blue rounded web-app look. The
desktop build wraps the game UI in a fixed 1920×1080 `CanvasScaler`
(`transform: scale(...)`), and tree-shakes desktop vs web via
`VITE_BUILD_TARGET`. There is no `ThemeProvider`, no `data-theme`, no
`prefers-color-scheme` handling, and `uiStore.ts` tracks only graphics presets
and bot speed.

This change introduces one design system (token layer + primitive library +
asset layer) and two flippable themes, applied to the launcher and board and to
the legacy pages' chrome, with the remaining page internals deferred to phase 2.

## Goals / Non-Goals

**Goals:**

- One role-based token layer as the single source of visual truth, replacing the
  scattered `--ib-*` / `--bld-*` / `--bg-*` palettes.
- Two named themes — dark "Digi-OS" and light "Adventure '99" — switchable via a
  single `data-theme` attribute, persisted, default dark, applied before first
  paint.
- A theme-agnostic React primitive library whose per-theme chrome lives in
  `[data-theme]` CSS, so call sites never branch on theme.
- A vendored asset layer (sleeves + pixel sprites) behind a typed manifest and
  slot components, theme-stable, with attribution.
- An in-app `/style-guide` route as living documentation and visual QA.
- Apply the system to the launcher + game board, and replace the legacy navy
  page chrome/headers.

**Non-Goals:**

- Deep restyle of dense page internals (deck-builder card grid, lobby flows,
  settings/models/admin) beyond outer chrome — that is phase 2.
- Any engine, RL, DB, or hosted-API change.
- A Tauri-shell runtime change for theming in v1 (theme persists in the webview's
  `localStorage`; a native store is a possible later refinement).
- Re-skinning the standalone landing page's own aesthetic (only its footer text
  changes here).

## Decisions

### D1: Attribute-driven theming over Tailwind variants or swapped stylesheets

Theme = a `data-theme` attribute on `<html>` plus a role-token layer in CSS:
`:root, [data-theme="dark"] { … }` and `[data-theme="light"] { … }`. Switching
flips the attribute; no restyle re-render, no component re-mount.

- *Alternative — Tailwind v4 `@theme` + a `theme-light:` custom variant on every
  component:* scatters per-theme overrides across the entire component tree and
  fights Tailwind utilities on the bespoke chrome (clip-path bevels, Win95
  box-shadow bevels, scanlines). Rejected.
- *Alternative — load `dark.css` / `light.css` and swap at runtime:* duplicates
  structure and lets themes drift out of parity. Rejected.

Tailwind remains for layout utilities; the retro chrome is plain CSS over the
token vars.

### D2: "One component, two chromes" — chrome in `[data-theme]` CSS

Primitives share structure/props; the structural delta between themes (dark =
thin glowing borders + `clip-path` corner cuts + scanlines; light = chunky
`box-shadow` bevels + square corners + beige) is CSS keyed on the root attribute.
Call sites stay theme-agnostic (`<Window title="Deck Builder">` renders correctly
in both).

- *Alternative — per-theme component variants or a render-prop that switches
  chrome in JS:* pushes theme awareness into every call site and doubles the
  component surface. Rejected.

### D3: Zustand `themeStore` + pre-paint inline bootstrap

A small persisted `themeStore` (or an extension of `uiStore`) holds
`theme: 'dark' | 'light'` with `setTheme`/`toggle`. A `ThemeProvider` syncs the
store to `document.documentElement.dataset.theme` and exposes `useTheme()`. To
avoid a flash, an inline bootstrap (in `index.html` or at the top of the entry
module, before React renders) reads the persisted value and sets `data-theme`
before first paint.

- *Alternative — React context only:* the first painted frame uses the default
  before the effect runs → flash. Rejected.
- *Alternative — `prefers-color-scheme`:* cannot express two *named* bespoke
  themes and ignores the explicit user choice. Rejected (we may still seed the
  first-run default from it later).

### D4: Token taxonomy — semantic roles; game colors + assets theme-stable

Tokens are role-based (`--surface-bg`, `--surface`, `--screen-bg`, `--ink-0..3`,
`--accent`, `--signal`, `--good/--warn/--danger`, `--player`/`--opp`,
`--frame-cut`, `--bevel-*`, `--border-w`, font roles, type scale). Player
(orange family) / opponent (blue family) and the vendored art are theme-stable —
the same hue identity in both themes, tuned only for contrast — so the board
stays legible and sprites never recolor.

### D5: Vendor assets as a committed pack + manifest + slot components

Sleeves (WE-Kaito) and pixel sprites (Project Drasil) are committed under
`design/assets/`, indexed by a typed manifest (id → asset + name + source), and
rendered through `CardSleeve` / `DigimonSprite` with a procedural fallback and
`image-rendering: pixelated`. The owner has accepted the IP/redistribution risk;
organizing behind a manifest keeps assets swappable and attributable even though
they are bundled.

- *Alternative — loose files imported ad hoc:* unattributable, unswappable,
  hard to audit. Rejected.

### D6: Fonts tuned for legibility

Dark: VT323 for large display moments, IBM Plex Mono for dense UI/body. Light:
Silkscreen for titles/labels/buttons, a readable sans for dense body copy (deck
lists, card text). Pixel/display faces are never used for dense text.

### D7: Scope = foundation + hero surfaces; phase 2 for deep internals

This change lands the system + switcher + primitives + assets + style guide, and
applies it to the launcher, the board, and the legacy pages' outer chrome.
Deep both-theme migration of dense page internals is deferred so the change stays
shippable.

### D8: Desktop-gate the switcher and style guide

The token layer is shared, but the `ThemeSwitch` placement and the `/style-guide`
route are desktop-targeted, gated by `VITE_BUILD_TARGET === 'desktop'`. The web
build keeps its current behavior on the default (dark) token values.

## Risks / Trade-offs

- **Flash of wrong theme on launch** → pre-paint inline bootstrap sets
  `data-theme` before first paint (D3); covered by a spec scenario.
- **Pixel-font legibility at small sizes** → reserve VT323 / Silkscreen for
  display/labels; dense text uses mono (dark) or a readable sans (light) (D6).
- **Asset bundle / installer size growth** → compress sprites/sleeves, ship only
  a vetted subset, and lazy-load where possible.
- **Interaction with the fixed 1920×1080 CanvasScaler** → bevels and scanlines
  are authored at canvas scale; both themes are verified inside the scaler at
  representative resolution presets.
- **IP / redistribution of vendored art** → accepted by the owner; mitigated by
  attribution (credits + reworded footer) and takedown-on-request posture. This
  is a deliberate deviation from the prior "no proprietary assets" stance, called
  out in the proposal.
- **Legacy CSS churn / visual regressions** → migrate incrementally surface by
  surface; the `/style-guide` route is the manual visual-QA harness, optionally
  backed by Playwright screenshot diffs of both themes; the default-dark theme
  keeps the launcher/board close to today's look, limiting blast radius.
- **Themes drifting out of parity over time** → token completeness (every role
  defined in both themes) plus the style guide make divergence visible.

## Migration Plan

Incremental, no data migration:

1. Land the token layer + `fonts.css` + `themeStore` + `ThemeProvider` + the
   pre-paint bootstrap, with dark tokens approximating today's look (near-zero
   visible change).
2. Build the primitive library + the vendored asset layer + the `/style-guide`
   route.
3. Migrate the launcher onto primitives in both themes.
4. Migrate the game board (`.ib-*`) onto tokens/primitives in both themes.
5. Replace the legacy navy chrome/headers on the deck-builder, lobby,
   mode-select, deck-select, deck-library, and matching pages.
6. Reword the landing footer + add the credits surface.

Rollback: revert the change; because the default theme is dark and dark tokens
track the current palette, reverting returns the client to its prior appearance
without state migration.

## Open Questions

- Light-theme dense-body sans: a system sans vs. a bundled Win95-style face
  (e.g. a "W95FA"-like font) — pick during implementation for legibility.
- Persistence backend on desktop: `localStorage` for v1; revisit a native Tauri
  store later if the webview store proves insufficient.
- Exact sleeve/sprite subset to vendor and the bundle-size budget.
- Whether the web build later exposes the theme switcher too (currently
  desktop-gated).
