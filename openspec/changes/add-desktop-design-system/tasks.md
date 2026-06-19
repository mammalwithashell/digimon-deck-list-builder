## 1. Design module scaffolding & token layer

- [x] 1.1 Create `code/frontend/src/design/` with subdirs `tokens/`, `theme/`, `components/`, `assets/`, plus an `index.ts` barrel.
- [x] 1.2 Author `tokens/tokens.css` with the full role-token set on `:root, [data-theme="dark"]` and a `[data-theme="light"]` override block (surfaces, ink, screen, accent, signal, status, `--player`/`--opp`, `--frame-cut`, `--bevel-*`, `--border-w`, font roles, type scale). Every role MUST be defined in both blocks.
- [x] 1.3 Add a token-completeness check (a test or lint script) asserting every role token present in the dark block is also present in the light block.
- [x] 1.4 Import `tokens.css` once at the app root so the variables are globally available.

## 2. Fonts

- [x] 2.1 Add `design/tokens/fonts.css` loading VT323 + IBM Plex Mono (dark) and Silkscreen + the chosen readable sans (light); wire the font role tokens to these families.
- [x] 2.2 Verify dense-text roles map to mono (dark) / readable sans (light), and display/label roles map to VT323 / Silkscreen.

## 3. Theme switching infrastructure

- [x] 3.1 Add `design/theme/themeStore.ts` (Zustand) holding `theme: 'dark' | 'light'` with `setTheme` + `toggle`, persisted to `localStorage`, default `dark`.
- [x] 3.2 Add `design/theme/ThemeProvider.tsx` that syncs the store to `document.documentElement.dataset.theme` and exposes `useTheme()`; mount it at the app root in `App.tsx`.
- [x] 3.3 Add a pre-paint bootstrap (inline in `index.html` or at the very top of the entry module) that reads the persisted theme and sets `data-theme` before first paint; persist the default on first run.
- [x] 3.4 Verify no flash: persisted-light relaunch paints light on the first frame; no-persisted relaunch paints dark.

## 4. Primitive component library — structural

- [x] 4.1 Implement `Backdrop` (dark: scanlines + grid-floor + vignette; light: beige desktop + dot/grid), driven by `[data-theme]` CSS over tokens.
- [x] 4.2 Implement `Frame`/`Panel` (dark: glow border + `clip-path` cut; light: raised box-shadow bevel + square corners).
- [x] 4.3 Implement `TitleBar`, `Window` (= TitleBar + body), and `StatusBar`.
- [x] 4.4 Implement `NavRail` and `Screen` (inset LCD/CRT surface).
- [x] 4.5 Implement `Button` with `primary` / `ghost` / `accent` / `danger` variants.
- [x] 4.6 Confirm all structural primitives reference only role tokens and contain no theme-conditional chrome at the call site.

## 5. Primitive component library — domain

- [x] 5.1 Implement `AnalyzerFrame` with a Digimon-sprite art slot + stat-chip stats (dark: phosphor terminal readout; light: DIGITALMONSTER frame).
- [x] 5.2 Implement `StatChip`, `Badge`, and `DeckColorBadge`.
- [x] 5.3 Implement `CardTile`, `CardBack`, and `ThemeSwitch` (CRT power-toggle in dark, Win95 checkbox in light).
- [ ] 5.4 Restyle the existing `MemoryGauge` to consume tokens and render in both themes with theme-stable player/opponent colors. [token-based MemoryGauge primitive built; board adoption happens in group 9]

## 6. Vendored asset layer

- [x] 6.1 Vendor the card sleeves (from WE-Kaito's digimon-tcg-simulator) and Digimon pixel sprites (from Project Drasil) under `design/assets/{sleeves,sprites}/`, compressed; vet the subset against the bundle-size budget. [starter pack: 9 sleeves of 97 upstream + 1 sprite; bulk-add later]
- [x] 6.2 Author `design/assets/manifest.ts` mapping logical id → asset reference + name + source/credit; expose typed lookups.
- [x] 6.3 Implement `CardSleeve` and `DigimonSprite` slot components that resolve from the manifest, render with `image-rendering: pixelated`, and fall back procedurally on unknown ids (no broken images).
- [x] 6.4 Verify assets are theme-stable (identical pixels across a theme switch). [rendered as `<img>` with no theme filter; visual QA in 12.x]
- [x] 6.5 Add `CREDITS` (in-app credits surface + repo `CREDITS.md`) attributing WE-Kaito and Project Drasil; make the surface reachable in-app.

## 7. In-app style guide

- [x] 7.1 Add `design/StyleGuidePage.tsx` rendering every primitive in both dark and light themes side by side.
- [x] 7.2 Register the `/style-guide` route gated by `VITE_BUILD_TARGET === 'desktop'` (lazy-loaded; absent from non-desktop builds).

## 8. Apply to the launcher

- [ ] 8.1 Rebuild the launcher (`LauncherPage` + `launcher.css`) on the primitives (NavRail, Window/Panel, Button, deck list using `CardSleeve` tiles), consuming tokens; remove the bespoke `--bg-*` palette.
- [ ] 8.2 Place the `ThemeSwitch` in the launcher top bar; verify a live switch updates launcher chrome without navigation.

## 9. Apply to the game board

- [ ] 9.1 Refactor the board `.ib-*` styles in `index.css` to consume role tokens; route the board backdrop/screen through `Backdrop`/`Screen`.
- [ ] 9.2 Map in-game player/opponent elements to the theme-stable `--player`/`--opp` tokens; route card backs through `CardSleeve` and analyzer popovers through `AnalyzerFrame`.
- [ ] 9.3 Verify both themes render correctly inside the fixed 1920×1080 `CanvasScaler` at representative resolution presets, and a live theme switch keeps team colors stable.

## 10. Replace legacy navy page chrome

- [ ] 10.1 Replace the chrome/headers of `DeckBuilderPage`, `RoomLobbyPage`, `ModeSelectPage`, `DeckSelectPage`, `DeckLibraryPage`, and `MatchingPage` with primitives consuming tokens (kill the navy-blue rounded look); leave dense internals for phase 2.
- [ ] 10.2 Remove now-dead page-local palette vars (`--bld-*`, inline navy hex) superseded by tokens.

## 11. Landing footer + settings

- [x] 11.1 Reword the `code/landing/index.html` footer so it no longer claims "no proprietary assets" and instead acknowledges bundled community art + references the credits.
- [x] 11.2 Add a `ThemeSwitch` row to the settings area. [added to GraphicsSettingsPage]

## 12. Verification & QA

- [ ] 12.1 Manually walk the `/style-guide` page in both themes; confirm every primitive renders correctly.
- [ ] 12.2 Confirm `prefers-reduced-motion: reduce` disables dark-theme CRT looping animations.
- [ ] 12.3 Confirm the theme switcher + `/style-guide` route are absent from a non-desktop build, and the web build renders on default dark tokens.
- [ ] 12.4 (Optional) Add Playwright screenshot diffs of the launcher, board, and style guide in both themes.
- [ ] 12.5 Spot-check the desktop bundle/installer size delta from the vendored assets stays within budget.
