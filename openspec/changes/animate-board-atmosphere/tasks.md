## 1. Reuse the atmosphere engine on the board

- [ ] 1.1 Mount `LiveAtmosphere surface="board"` in `GameBoard` on the existing negative `z-index` atmosphere band, replacing the static `BinaryWallpaper` for the rain.
- [ ] 1.2 Configure board-variant intensity (lower rain density than menus) and size the canvas to the internal 1920×1080 board so it scales with `CanvasScaler` and never reflows.
- [ ] 1.3 Retire / disable `BinaryWallpaper` once the engine's static fallback matches the current corner-binary look (gated-off parity).

## 2. Animate scanlines + grid mat

- [ ] 2.1 Add a slow vertical roll animation to `.ib-board__scanlines`, gated by `data-motion` / effective live-background.
- [ ] 2.2 Add a slow background-position drift to `.ib-board__mat`, similarly gated.
- [ ] 2.3 Keep amplitudes subtle (board < menu); confirm both fall back to static when gated off.

## 3. Gating + fallback

- [ ] 3.1 Drive all board atmosphere animation from the effective live-background gate; gated-off renders exactly the current static board (no new design needed).
- [ ] 3.2 Confirm the hidden-tab pause and FPS cap come through from the shared engine.

## 4. Z-order + non-regression

- [ ] 4.1 Verify permanents, board chrome, memory gauge, and event VFX (digivolve/battle/security/phase) all render above the atmosphere.
- [ ] 4.2 Confirm the event-driven VFX behavior (rule 15, `lastSeqRef`) is unchanged.
- [ ] 4.3 Confirm board layout is unchanged at all resolution presets.

## 5. Tests

- [ ] 5.1 Board atmosphere animates when effective live-background is on; static when off.
- [ ] 5.2 Atmosphere sits on the atmosphere `z-index` band (below permanents/VFX); ordering asserted.
- [ ] 5.3 No layout/snapshot regression on the board at the default preset.

## 6. Verification

- [ ] 6.1 Manual pass in a real match, both themes: atmosphere reads as texture, never distracts, cards/VFX clearly above it.
- [ ] 6.2 Dev perf check during an active game on the Tauri webview (capped FPS, paused when hidden, no reflow).
- [ ] 6.3 Typecheck + frontend tests green; `openspec validate animate-board-atmosphere`.
