# Verification Notes

## Automated tests landed with this change

| Suite                                                  | Tests | Status |
|--------------------------------------------------------|------:|--------|
| `src/stores/uiStore.test.ts`                           |     6 | ✓      |
| `src/components/desktop/CanvasScaler.test.tsx`         |    12 | ✓      |
| `src/pages/GraphicsSettingsPage.test.tsx`              |     4 | ✓      |
| `src/hooks/usePositionTransitions.test.tsx`            |     3 | ✓      |
| **New tests total**                                    |    25 | ✓      |
| Tauri `cargo test --lib`                               |    36 | ✓      |

Pre-existing failures unrelated to this change:
`src/bootstrap/guest.test.ts` (2 failures present on the base branch as well).

## Manual QA test plan

Each item below corresponds to a 7.x manual-QA task in `tasks.md`.

### 7.3 — Cycle through all 8 presets

1. Launch desktop build.
2. Open **Desktop ▸ Graphics Settings** from the navbar.
3. Click each preset 1024×576 → 5160×2160 in order.
4. After each click, verify the window resizes to that exact size and
   the game-board scales uniformly. The 14 battle-area slots should
   always be 2 rows of 7.

### 7.4 — Fullscreen toggle

1. From Graphics Settings, pick 1920×1080.
2. Toggle Fullscreen on → window goes fullscreen on current monitor.
3. Toggle Fullscreen off → window returns to 1920×1080 windowed.
4. Pick 2560×1440, repeat steps 2–3 to confirm "off" restores the most
   recent preset, not the prior one.

### 7.5 — Persist across launches

1. From Graphics Settings, pick 2560×1440.
2. Close the app.
3. Relaunch the app. The window should open directly at 2560×1440.
4. Graphics Settings should show 2560×1440 marked as selected.

### 7.6 — Slot-shift animation

1. Start a game and play at least 3 permanents to the battle area
   (slots 1, 2, 3 visually).
2. Trigger a midfield deletion — easiest path is to let opponent
   destroy the permanent in slot 2 via attack.
3. Observe slot 3's card sliding left into slot 2 over ~250ms instead
   of teleporting.
4. Repeat with 5+ permanents and delete from the middle; survivors
   should all slide in sync.

### 7.8 — Web build regression check

1. Build the browser bundle: `npm run build` (NOT `build:desktop`).
2. Serve `dist/` and open in a regular browser.
3. The `<CanvasScaler>` should render children directly with no scaling
   transform (responsive layout intact).
4. The `/settings/graphics` route should be unreachable / 404 in the
   web build (route gated behind `IS_DESKTOP`).
5. The "Desktop ▸ Graphics Settings" nav entry should not appear.
