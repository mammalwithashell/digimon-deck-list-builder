## Context

The desktop app's navigation lives in two diverged places. The launcher (`/`) renders its own bespoke sidebar in `LauncherShell` (`components/launcher/`), styled by hand-written `.launcher-*` CSS. Every other desktop route renders through `Layout`, which on desktop deliberately renders **no** navigation (the `NavBar` is web-only). So the rail — and its accent coloring — exist only on the home screen and disappear on Play, Decks, etc.

Meanwhile the design system (`design/components/`, `design/tokens/`) already encodes the intended look and is what the in-app style guide showcases:

- `NavRailItem --active` → `inset 2px 0 0 var(--accent)` in dark (neon-green bar) and `background: var(--accent); color: var(--accent-ink)` in light (solid navy block). (`components.css:278`, `:280`)
- `Window` title → green terminal header with LED dots in dark, navy Win95 title bar with a beveled close box in light. (`components.css:126-146`)
- `--accent` resolves to `#39ff88` (dark) / `#2b2f7a` (light); the launcher rail instead hard-codes the dark active accent to `--player` (orange).

This change is a UI-layer reconciliation: route the menu screens through the existing primitives instead of inventing a look. Constraints: desktop build only (`VITE_BUILD_TARGET=desktop`); the shell sits below the newly-merged `TitleBar` and inside `CanvasScaler`'s fixed canvas; the hosted-web `NavBar` path and all engine/RL/backend code are untouched.

## Goals / Non-Goals

**Goals:**
- A single persistent, collapsible nav shell wrapping all desktop *menu* screens, built from the design-system `NavRail` / `Window` primitives.
- Active nav item driven by the current route, highlighted in `--accent` (green dark / navy light).
- A context-aware rail: pages can contribute sub-navigation; the deck library folds its Folders/Formats into the rail and drops its own sidebar.
- Collapse-to-icon-rail with the choice persisted across launches.
- Design-system `Window` chrome on the menu-screen content panels.

**Non-Goals:**
- The in-game board (`/game/:id`) — stays full-bleed, no rail.
- The deck builder / analyzer (dense DCGO-style tool) — unchanged.
- The hosted-web `NavBar` and any non-desktop build path.
- Any backend, engine, RL, or gameplay behavior.
- New design-system primitives — this change only *consumes* existing ones.

## Decisions

**1. Build the shell from design-system primitives, not by extending `.launcher-*`.**
The shell uses `NavRail` / `NavRailItem` / `Window` so the app matches the style guide by construction and there is one source of visual truth. *Alternative considered:* lift the bespoke launcher CSS into a shared component and just repoint `--player`→`--accent`. Rejected — it perpetuates two parallel rail implementations and the drift that caused this problem.

**2. A route-level `MenuShell` layout via a parent `<Route element>`.**
Menu routes are grouped under one `<Route element={<MenuShell/>}>` in `App.tsx`; the shell renders chrome + `<Outlet>`. `/game/:id` is left outside the group so it renders full-bleed. The launcher's content stays in `LauncherPage`; only its shell chrome (`LauncherShell`'s rail/topbar/statusbar) moves to `MenuShell`. *Alternatives:* render the shell inside every page (duplicative, drift-prone); or one always-mounted shell that hides itself on the board (couples the shell to route knowledge and fights `CanvasScaler`).

**3. Context-aware sub-nav via a minimal `RailContext`.**
A small React context exposes `setRailSection(node)` / clear-on-unmount; `MenuShell` renders the contributed node inside the rail under the owning item. `DeckLibraryPage` keeps owning its folder/format state but renders the controls through this context instead of `library-sidebar`. *Alternatives:* a portal/`RailOutlet` slot (more machinery for no extra benefit); hard-coding deck-library knowledge into `MenuShell` (tight coupling); keeping the page's own second sidebar (rejected during brainstorming — two bars).

**4. Active state derives from the URL (`useLocation` + an `isActivePath` matcher reused from `NavBar`).**
The URL is the single source of truth, which fixes the "always Home" artifact. *Alternative:* shell-managed active state set by each page — redundant and can desync from the route.

**5. Collapse to an icon-only rail, persisted in `localStorage`.**
Matches the desktop "site nav" idiom and preserves at-a-glance active-page indication plus quick nav. *Alternative:* fully hide behind a hamburger — loses glanceability, which is the opposite of the request.

**6. Keep theme-appropriate active treatments.**
Dark = green inset accent bar; light = solid navy fill block. This is what the DS already renders and what the brainstorm confirmed; we do not force one identical treatment across both themes.

**7. Scope window chrome to menu screens.**
Launcher Play/My-Decks panels, the Play/format chooser, and deck-library panels adopt `Window`. The dense builder is excluded to avoid high-churn, high-risk edits to a tool the request didn't name.

## Risks / Trade-offs

- **Relocating the launcher under a layout `<Outlet>` could disturb the `/` screen.** → Keep `LauncherPage` content byte-for-byte; move only shell chrome; cover with a `MenuShell` render test and a manual `run-desktop` pass on `/`.
- **`TitleBar` + `CanvasScaler` interplay (both newly merged from main).** → `MenuShell` mounts below `TitleBar` and inside the existing fixed-canvas sizing (`--app-vh100`); verify no double scrollbar and that the rail height tracks the canvas, in both windowed and fullscreen presets.
- **`RailContext` adds indirection.** → Keep the API to a single setter plus automatic clear on page unmount; document the contract in the component.
- **Collapsed rail vs contributed sub-nav.** → In collapsed (icon) mode the contributed Folders/Formats sub-nav is hidden; expanding the rail reveals it. Documented as intended behavior, not a bug.
- **Intentional light/dark divergence in active treatment** could be "corrected" by a future reviewer. → Called out explicitly here and in the spec so it's understood as deliberate.

## Migration Plan

Pure frontend, desktop-only, no data migration. Ships in the desktop build; the hosted-web build is unaffected (its `NavBar` path is unchanged). The only new persisted state is a `localStorage` collapse flag, which is additive and safely ignored if absent. Rollback = revert the routing + `MenuShell`/`RailContext`/`Window`-adoption commits; no server or schema impact.

## Open Questions

- Final rail item set / grouping (current launcher: Main = Home / Play / Decks / Patch Notes; Tools = Import / Graphics; plus desktop Models) — finalize from the live launcher items during implementation; no new destinations are introduced.
- Whether the collapsed-mode contributed sub-nav should later gain a hover flyout — deferred; default is hidden-while-collapsed.
