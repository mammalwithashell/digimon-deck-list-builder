## 1. Store + persistence (uiStore)

- [x] 1.1 Add a `Motion` type (`'full' | 'reduced' | 'off'`) and a `MOTIONS` array + `MOTION_STORAGE_KEY` (`'desktop.motion'`) in `code/frontend/src/stores/uiStore.ts`, following the `botSpeed`/`railCollapsed` pattern.
- [x] 1.2 Add `deriveDefaultMotion()` that returns `'reduced'` when `window.matchMedia('(prefers-reduced-motion: reduce)').matches`, else `'full'` (defensive try/catch → `'full'`).
- [x] 1.3 Add `loadPersistedMotion()` (validate against `MOTIONS`, fall back to `deriveDefaultMotion()`) and `persistMotion()`.
- [x] 1.4 Add a `liveBackground` boolean with `LIVE_BG_STORAGE_KEY` (`'desktop.liveBackground'`), `loadPersistedLiveBackground()` (default `true`), and `persistLiveBackground()`.
- [x] 1.5 Add `motion`, `liveBackground` state + `setMotion`, `setLiveBackground` actions to the `UiStore` interface and store, persisting on each mutation.
- [x] 1.6 Add a `effectiveLiveBackground` derived selector/helper: `motion === 'full' && liveBackground`.
- [x] 1.7 Export the new keys + loaders on `__uiStoreInternals` for tests.

## 2. Global motion attribute + bootstrap

- [x] 2.1 Add an `applyMotionAttribute(motion)` helper (sets `document.documentElement.dataset.motion`) next to the store; call it from `setMotion`.
- [x] 2.2 Add a pre-paint bootstrap script in `code/frontend/index.html` that resolves the effective motion (persisted → `prefers-reduced-motion` → `'full'`) and sets `data-motion` before the bundle loads — mirroring the existing `data-theme` bootstrap; keep it defensive (try/catch).
- [x] 2.3 Ensure the attribute is re-asserted on mount (in `ThemeProvider` or an equivalent small provider/effect) so a store value the bootstrap didn't apply still wins.

## 3. Effective-motion accessor for downstream features

- [x] 3.1 Expose the effective motion level via a `useUiStore` selector (e.g. `useMotion()`) and document that cursor-lighting / live-atmosphere / digivolve-cut-in changes must read it rather than re-deriving.

## 4. Gate the existing animation library

- [x] 4.1 Enumerate every `@keyframes` / `animate-*` class in `code/frontend/src/index.css` and the `.ds-backdrop` animations in `code/frontend/src/design/components/components.css`; classify each as ambient/looping vs functional one-shot (record the classification as a comment block).
- [x] 4.2 Replace the lone `@media (prefers-reduced-motion: reduce)` guard on `ds-crt-scan` with a `data-motion` gate so there is one mechanism.
- [x] 4.3 Add CSS rules so ambient/looping effects do not animate under `[data-motion="reduced"]` and `[data-motion="off"]`.
- [x] 4.4 Add CSS rules so functional one-shot animations still play under `[data-motion="reduced"]` and are neutralized (instant) under `[data-motion="off"]`.
- [x] 4.5 Verify no animation references `prefers-reduced-motion` directly anymore (single gate); the media query is used only for default derivation in the store/bootstrap.

## 5. Graphics Settings UI

- [x] 5.1 Add a Motion control row (3-way segmented control or select matching the existing row styling) to `code/frontend/src/pages/GraphicsSettingsPage.tsx`, wired to `motion`/`setMotion`, with `data-testid`s.
- [x] 5.2 Add a Live-background toggle row (same toggle style as Fullscreen), wired to `liveBackground`/`setLiveBackground`, with a `data-testid`.
- [x] 5.3 Confirm both controls apply immediately (no reload) and read back the persisted value after a simulated relaunch.

## 6. Tests

- [x] 6.1 `uiStore` tests: motion default derivation (mock `matchMedia` both ways), persistence round-trip, invalid-value fallback, `liveBackground` persistence, and `effectiveLiveBackground` gating logic.
- [x] 6.2 Bootstrap-parity test: the `index.html` motion storage key matches the store's `MOTION_STORAGE_KEY` literal (mirror the existing theme bootstrap parity test).
- [x] 6.3 `GraphicsSettingsPage` test: both controls render, changing them calls the store setters, and the active value is reflected.

## 7. Verification

- [ ] 7.1 Manual pass per theme × per motion level (`full`/`reduced`/`off`): confirm ambient effects stop at `reduced`, functional feedback survives `reduced` and is instant at `off`, and no flash of motion on load.
- [x] 7.2 Run the frontend test suite and typecheck; confirm green.
- [x] 7.3 Update `openspec` status to complete and validate the change (`openspec validate add-effects-and-motion-settings`).
