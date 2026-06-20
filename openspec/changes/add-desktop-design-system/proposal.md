## Why

The desktop client's UI is visually fragmented: three siloed styling systems
coexist — the game board's OKLCH `.ib-*` palette, the deck-builder/launcher hex
`--bld-*` / `--bg-*` navy chrome, and the landing page's phosphor-CRT look — with
no shared tokens and no theme switching (the app is hard-coded dark-only). The
launcher and board already point at a strong dark "Digi-OS" terminal direction,
but the rest of the client still wears a legacy navy-blue rounded web-app look
with mismatched headers. Every page reinvents its own chrome, so there is no
reusable component layer and no single source of visual truth. This change
establishes one design system plus dual theming so the whole client speaks one
visual language with two flippable skins.

## What Changes

- **Add a role-based token layer.** Semantic CSS custom properties (roles like
  `--surface`, `--ink-0`, `--accent`, `--player`, `--frame-cut`) become the
  single source of truth for color, type, and chrome treatment. Two value sets,
  one per theme. This replaces the scattered `--ib-*`, `--bld-*`, `--bg-*`, and
  inline-hex palettes.
- **Add two named themes.** Dark "Digi-OS" (phosphor-green/amber CRT terminal,
  VT323 + IBM Plex Mono, angular `clip-path` frames + scanlines) and light
  "Adventure '99" (beige Izzy's-laptop desktop, chunky Win95 bevels, the
  DIGITALMONSTER analyzer frame, Silkscreen titles). Default theme is dark.
- **Add a theme switcher.** A `data-theme` attribute on the document root, a
  persisted Zustand `themeStore`, and a `ThemeProvider` that applies the theme
  **before first paint** (no flash-of-wrong-theme). Switching flips one
  attribute — no restyle re-render. Orange (player) / blue (opponent) game
  identity colors and the vendored art are **theme-stable** (identical in both
  themes) so the board stays legible and sprites never recolor.
- **Add a theme-agnostic primitive component library.** React primitives whose
  structure is shared and whose per-theme chrome lives in `[data-theme]` CSS:
  `Backdrop`, `Frame`/`Panel`, `Window`/`TitleBar`/`StatusBar`/`MenuBar`,
  `NavRail`, `Screen`, `Button`, `ThemeSwitch`, `Toggle`, `Field`/`Select`/
  `Slider`, `Tabs`, `Dialog`, `Badge`/`Tag`, `BootLog`, `Tooltip`,
  `AnalyzerFrame`, `CardTile`/`CardBack`/`CardSleeve`, `DigimonSprite`,
  `StatChip`, `MemoryGauge` (restyle), `DeckColorBadge`, `BrandMark`.
- **Add a vendored asset layer.** Card sleeves (from WE-Kaito's
  digimon-tcg-simulator) and Digimon pixel sprites (from Project Drasil) are
  committed under the design module with a typed manifest, slot components
  (`CardSleeve`, `DigimonSprite`) with a procedural fallback, and
  `image-rendering: pixelated`. IP/redistribution risk is accepted by the
  project owner; this deviates from the prior "no proprietary assets" stance.
- **Add an in-app `/style-guide` route** (desktop-only) rendering every
  primitive in both themes — the living kitchen-sink reference.
- **Apply the system to the launcher and game board** in both themes.
- **Replace the legacy navy-blue rounded chrome/headers** on the deck builder,
  room lobby, mode-select, deck-select, deck-library, and matching pages with
  the primitive chrome consuming tokens. Deep restyles of dense page internals
  are deferred to phase 2.
- **Add `CREDITS` + reword the landing footer.** `code/landing/index.html`'s
  "distributes no official card images or proprietary assets" line is reworded
  to acknowledge bundled community art and point at a credits screen that
  attributes WE-Kaito and Project Drasil.

Out of scope here (tracked as **phase 2**): deep both-theme migration of
remaining page internals — the deck-builder dense card grid, lobby flows, and
settings/models/admin surfaces — beyond their outer chrome.

## Capabilities

### New Capabilities

- `desktop-theming`: The token role taxonomy, the two named themes and their
  values, the `data-theme` switching mechanism, pre-paint application,
  persistence + default, theme-stable game/asset colors, and which in-scope
  surfaces must respond to the active theme.
- `desktop-ui-primitives`: The theme-agnostic React primitive component library
  (the component set, the "one component, two chromes" contract that call sites
  stay theme-agnostic) and the in-app `/style-guide` kitchen-sink route.
- `desktop-visual-assets`: The vendored sleeve + pixel-sprite asset layer — the
  committed asset pack, the typed manifest, the `CardSleeve` / `DigimonSprite`
  slot components with procedural fallback, attribution/credits, and the
  landing-footer reword.

### Modified Capabilities

<!-- None. No existing spec governs desktop theming, component chrome, the
     launcher/board visual contract, or bundled visual assets. -->

## Impact

- **Frontend (`code/frontend/src/`)** — new `design/` module (`tokens/`,
  `fonts.css`, `theme/` with `themeStore.ts` + `ThemeProvider.tsx`,
  `components/`, `assets/`, `StyleGuidePage.tsx`); `App.tsx` gains the
  `ThemeProvider` mount, the pre-paint theme application, and the `/style-guide`
  route; `index.css` `.ib-*` board styles, `launcher.css`, and the legacy page
  CSS (`DeckBuilderPage.css`, `RoomLobbyPage.css`, `ModeSelectPage.css`,
  `DeckSelectPage.css`, `DeckLibraryPage.css`, `MatchingPage.css`) are refactored
  to consume tokens. The existing fixed 1920×1080 `CanvasScaler` is unaffected;
  both themes must render correctly inside it.
- **Assets** — new committed binaries under `code/frontend/src/design/assets/`
  (sleeves, sprites) increase the bundle/installer size; lazy-load and compress.
- **Landing (`code/landing/index.html`)** — footer reword + optional credits link.
- **Docs/policy** — new `CREDITS` (in-app + repo); documents the accepted
  deviation from the "no proprietary assets" stance.
- **Build targets** — the token layer is shared, but the theme switcher and the
  `/style-guide` route are desktop-targeted and gated by
  `VITE_BUILD_TARGET === 'desktop'`. The web build keeps current behavior on the
  default (dark) token values. The desktop build remains Python-free (no runtime
  change to the Tauri shell for v1; theme persists via webview `localStorage`).
- **No engine, RL, DB, or hosted-API impact.** This change is confined to the
  frontend design layer + the landing page.
