## Why

The shipped desktop app (v0.4.0) diverged from its own style guide: the nav rail and its green/navy accents only exist on the launcher home screen, and they vanish the moment you open Play or Decks. The design system already encodes the intended look — `NavRail` lights the active item with `--accent` (neon green in dark, Win95 navy in light) and `Window` renders the themed header bars — but the menu screens are bespoke and never route through those primitives. The result is a home screen that matches the style guide and a set of inner screens that do not.

## What Changes

- Introduce a single persistent, collapsible navigation shell that wraps **all desktop menu screens** (Home, Play, Decks, Patch Notes, Graphics, Models). The in-game board stays full-bleed.
- Drive the active nav item from the current route so the accent (green in dark / navy in light) tells the user which page they are on — not always "Home".
- Make the rail **context-aware**: a page may contribute its own sub-navigation into the rail. The deck library folds its Folders/Formats controls into the rail (under an expandable "Decks") and drops its separate in-page sidebar.
- Give the rail a collapse toggle that reduces it to an icon-only strip; the choice persists across launches.
- Replace bespoke panel headers with the design-system `Window` chrome (green terminal header in dark / navy title bar in light) on the menu screens: the launcher Play & My-Decks panels, the Play/format chooser, and the deck-library panels.
- Retire the launcher rail's `--player` (orange) active styling in favor of the shared `--accent` token, so the app matches the style guide by construction.
- The dense deck builder / analyzer, the in-game board, and the hosted-web `NavBar` are **out of scope** and unchanged.

## Capabilities

### New Capabilities
- `desktop-menu-shell`: A persistent, collapsible desktop navigation shell built from the design-system `NavRail` / `Window` primitives. Covers: which routes the shell wraps (menu screens, not the board), route-driven active-item highlighting in `--accent`, page-contributed contextual sub-navigation (e.g. deck-library Folders/Formats), collapse-to-icon-rail with persisted state, and themed window chrome on menu-screen panels.

### Modified Capabilities
<!-- None. The deck library's folder/format filtering behavior is unchanged; only the location of its controls moves into the shell, which the new capability covers. -->

## Impact

- **Frontend (desktop build only):**
  - New `components/layout/MenuShell.tsx` (+ css) and a small `RailContext` for page-contributed sub-nav.
  - `App.tsx` routing: menu routes grouped under one `<MenuShell>` layout; `/game/:id` left outside it; the launcher's shell chrome moves out of `LauncherShell` into `MenuShell`.
  - `DeckLibraryPage.tsx` loses its `library-sidebar` and contributes Folders/Formats to the rail.
  - `ModeSelectPage.tsx` and launcher panels (`LauncherActions`, `LauncherDeckPanel`) adopt the `Window` component.
  - `launcher.css` token cleanup (drop the `--player`-based active rule).
  - Reuses existing design-system `NavRail` / `NavRailItem` / `Window` (no new primitives).
- **Below the new `TitleBar`, inside `CanvasScaler`** — the shell must respect both.
- **No backend, engine, RL, or hosted-web changes.** The web `NavBar` path is untouched.
- **Tests:** active-path matcher, `MenuShell` render + collapse persistence, `RailContext` contribution, and window-header presence in both themes; existing page tests stay green.
