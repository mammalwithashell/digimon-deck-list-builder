## 1. Rail context + active-route matcher

- [x] 1.1 Add an `isActivePath(pathname, item)` matcher (extract/reuse the logic in `components/layout/NavBar.tsx`) into a shared util (`utils/navActive.ts`) with a unit test covering exact vs prefix matches.
- [x] 1.2 Create `RailContext` (`components/layout/RailContext.tsx`): a provider exposing `setRailSection(node)` and auto-clear, plus a `useRailSection(node)` hook that sets on mount and clears on unmount. Unit-test that contributing then unmounting adds then removes the node.

## 2. MenuShell component

- [x] 2.1 Create `components/layout/MenuShell.tsx` (+ `MenuShell.css`): desktop-only route layout rendering brand, the design-system `NavRail`/`NavRailItem`, a topbar (`ThemeSwitch` + signed-in user + build version), the contributed rail section from `RailContext`, an `<Outlet>`, and a status bar. No bespoke `.launcher-side` markup.
- [x] 2.2 Drive each `NavRailItem`'s `active` from `useLocation()` + `isActivePath` so the active item tracks the route (fixes "always Home").
- [x] 2.3 Add the collapse control: a `collapsed` state persisted to `localStorage` (via `useUiStore`), a chevron toggle, and an icon-only rail when collapsed (labels hidden, contributed sub-nav hidden while collapsed).
- [x] 2.4 Ensure `MenuShell` mounts below `TitleBar` and inside `CanvasScaler` sizing (reuse `--app-vh100`); no double scrollbar in windowed or fullscreen presets.

## 3. Routing

- [x] 3.1 In `App.tsx`, group the desktop menu routes (`/`, `/play*`, `/deckbuilder*`, `/patch-notes`, `/settings/graphics`, `/models`) under one `<Route element={<MenuShell/>}>`; keep `/game/:id` outside the group (full-bleed). (Split into `DesktopRoutes`/`WebRoutes`.)
- [x] 3.2 Move the launcher's shell chrome out of `LauncherShell` into `MenuShell`; `LauncherPage` renders only its home content into the shell `<Outlet>` (content unchanged). `LauncherShell` deleted.
- [x] 3.3 Confirm the hosted-web (`!IS_DESKTOP`) routing and `NavBar` path are untouched (web tree kept byte-for-byte).

## 4. Deck-library contextual sub-nav

- [x] 4.1 In `DeckLibraryPage.tsx`, remove the `library-sidebar` Folders/Formats markup and contribute the same controls via `useRailSection`, keeping the existing `activeFolder`/`activeFormat` filtering state and `filterAndSortDecks` behavior intact.
- [x] 4.2 Render Folders + Formats as collapsible sub-sections under the Decks rail item; selecting one updates the filter and the deck grid; leaving the page clears the contributed section.

## 5. Window chrome on menu panels

- [x] 5.1 Wrap the launcher Play and My-Decks panels (`LauncherActions.tsx`, `LauncherDeckPanel.tsx`) in the design-system `Window` component with titles ("PLAY", "MY DECKS", "SAVED DECKS").
- [x] 5.2 Wrap the `ModeSelectPage.tsx` opponent + format-chooser sections in `Window` ("OPPONENT", "FORMAT").
- [x] 5.3 Apply the themed accent title-bar to the deck-library analytics panel. NOTE: the deck library still carries hardcoded dark surfaces (an incomplete design-system migration), so the full `Window` component would clash in light theme; the accent header (green dark / navy light) is theme-safe. Full library surface reconciliation deferred — see completion notes. Deck builder / analyzer left untouched.

## 6. Token cleanup

- [x] 6.1 Remove the `--player`-based active styling from `launcher.css` (`.launcher-nav-item.active` and the light override) now superseded by the `NavRail`/`--accent` styling; delete any dead `.launcher-side*` rules no longer referenced (rail block + responsive overrides + light overrides removed; no `.active` rules remain in the file).

## 7. Tests

- [x] 7.1 `MenuShell` render test: rail + outlet present; active item reflects a stubbed route; collapse toggles and persists. (3 tests, green)
- [x] 7.2 `RailContext` test: a child contributing a section makes it appear in the shell; republish on change; unmounting removes it. (3 tests, green)
- [x] 7.3 Window-header test: asserts the `Window` title bar renders for the launcher Play/My-Decks panels and for `ModeSelectPage` Opponent/Format in both `data-theme="dark"` and `data-theme="light"`. (4 tests, green)
- [x] 7.4 Ran the affected + adjacent suites from the worktree `code/frontend` (junctioned `node_modules`; `git rev-parse --show-toplevel` confirmed the worktree). 40 tests green (navActive, RailContext, MenuShell, LauncherActions, ModeSelectPage, uiStore, CanvasScaler, tokens, GraphicsSettingsPage). Fixed a PRE-EXISTING failure in `GraphicsSettingsPage.test.tsx` (rendered a `<Link>` with no `MemoryRouter`) and a pre-existing socket leak in `ModeSelectPage.test.tsx` (mocked the wrong module — `loadPlayFormats` calls `@/api/deckApi.listFormats`). Full `tsc -b` green.

## 8. Manual verification

- [ ] 8.1 (PENDING USER) `run-desktop` in dark and light: navigate Home → Play → Decks and confirm the rail persists, the active item lights up in `--accent` (green dark / navy light), and the menu panels show the themed window headers. — Requires launching the Tauri desktop app; cannot be done headlessly in the agent environment. Covered structurally by the unit tests; needs a human visual pass.
- [ ] 8.2 (PENDING USER) Open the deck library and confirm Folders/Formats drive filtering from the rail; collapse/expand the rail and confirm the persisted state; open `/game/:id` and confirm the board is full-bleed with no rail. — Same: requires a desktop run.

## 9. Review refinements (from live desktop review)

- [x] 9.1 Remove the redundant `MenuShell` topbar (the TitleBar already owns top chrome; the rail's active item shows the page). Content now sits directly under the TitleBar.
- [x] 9.2 Move the rail collapse control to the **top** of the rail (next to the brand), as a compact icon button.
- [x] 9.3 Move the signed-in identity + theme toggle into the rail **foot** (above the BUILD/DECKS/DRAFTS stats).
- [x] 9.4 Fullscreen: hide the custom `TitleBar` (`App.tsx`) and reserve no top space in `CanvasScaler` (`reservedTop = 0` when fullscreen) so the canvas fills the whole display; restore both on exit. `CanvasScaler` test green (12).
- [x] 9.5 Fix the Graphics-page Fullscreen toggle knob overflowing its track when on — re-geometried to a 20px knob anchored 4px from the left sliding 26px inside the 56px track (both themes). Graphics test green (4).
