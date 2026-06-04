## Why

The engine's frontend UI serialization (`digimon-engine/src/serialization.rs::to_ui_json`) already exposes runtime DP and keyword breakdowns per permanent, but it hard-codes every **card-effect text** field to empty: each stack source's `mainEffectText`/`inheritedEffectText` is `""` and the permanent-level `inheritedEffects` is `[]` (the code even comments them as "neutral defaults"). As a result, any consumer that reads engine state directly — the in-game digivolution-stack inspector (`CardOverlay`), WebSocket/`state_filter` clients, and engine-state debugging — sees a stack of cards with no effect text and no indication of which inherited effects a permanent actually has from its sources. Unlike a printed-card preview (static metadata), this is meant to be the engine's authoritative, runtime-accurate view of what a permanent on the field can do.

## What Changes

- Populate each serialized **stack source's** `mainEffectText` and `inheritedEffectText` from the engine's own `card_data` (the printed text the engine already holds by `data_index`), instead of `""`.
- Populate the permanent-level **`inheritedEffects`** array to reflect the inherited effects currently conferred on the permanent by its digivolution sources (top-card excluded, matching how inherited effects work), so the inspector can show which inherited effects are active and from which source.
- Keep the existing runtime fields (`dp`, `dpBreakdown`, `keywords`, `keywordBreakdown`, `sources[].dpContribution`, etc.) unchanged — they already work; this change is scoped to the stubbed **text** fields.
- Account for engine state that changes which inherited effects apply (e.g., inherited-effect substitution / removal) where feasible, so the serialized active set reflects runtime rather than only printed text.

Out of scope (separate changes):
- The right-click **card preview** (uses static metadata, not this surface) — `add-ingame-card-preview`.
- The empty **action log** — `add-ingame-action-log`.
- The tool-facing `view` module (`FieldView`, etc.) owned by `live-game-surface` — this change touches only the frontend `to_ui_json` surface.

## Capabilities

### New Capabilities
- `serialized-card-effect-state`: The engine's frontend UI serialization (`to_ui_json`) exposes per-source and active inherited card-effect text for battle-area permanents, so the in-game stack inspector and other `to_ui_json` consumers can render what a stacked permanent does without a separate metadata lookup.

### Modified Capabilities
<!-- None. `live-game-surface` explicitly excludes `to_ui_json` (it owns the tool-facing `view` module). No existing spec covers the `to_ui_json` effect-text fields. -->

## Impact

- **Engine (Rust) serialization only:** `code/digimon-engine/src/serialization.rs` (the per-source and permanent effect-text fields in `to_ui_json`). Reads from existing `Game`/`card_data`; no new data sources.
- Consumers benefit without change: the frontend `CardOverlay` stack inspector (already renders `sources[].inheritedEffectText` and `inheritedEffects`), `state_filter.py`/WS clients, and any `to_ui_json` reader.
- No change to the action space, game logic, RL tensor, `view` module, or the HTTP/Tauri response contract shape (fields already exist; only their contents change from `""`/`[]` to real text).
- Cross-engine note: aligns the Rust `to_ui_json` with what the legacy Python serialization produced; record in `docs/RUST_PYTHON_PARITY.md` if a divergence is being closed.
- Verification: Rust serialization unit tests asserting populated text for a known stacked permanent, plus a manual/Playwright check that the in-game stack inspector shows inherited-effect text.
