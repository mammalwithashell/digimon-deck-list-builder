## Why

DCGO lets a player right-click any permanent to inspect its full digivolution stack and every keyword, stat change, immunity, and restriction currently active on it. Our UI has no such affordance: the inspector panel component (`CardOverlay`) exists and renders a stack + keywords + DP + effect text, but in the Rust engine path — now the source of truth — `to_ui_json`'s `perm_data` hard-codes **all** of that to neutral defaults (`keywords: []`, `keywordBreakdown: {innate:[],gained:[]}`, `securityAttackModifier: 0`, base-only `dpBreakdown`, empty source/inherited effect text). And the only way to open the panel at all is a weak left-click *fallback* that fires only when no game action matches, so it is effectively unreachable mid-game. A player (or a human reviewing an RL game) cannot see why a Digimon can't be deleted, can't suspend, has +3000 DP, or gained Blocker.

## What Changes

- **Populate the stubbed runtime fields** in `serialization.rs::to_ui_json` `perm_data` from live game state instead of neutral defaults:
  - `keywords` + `keywordBreakdown.innate`/`.gained` (printed/inherited vs. modifier-granted), via `Game::has_keyword`, `face_keywords`/`inherited_keywords`, and the modifier registry.
  - `securityAttackModifier` from `Game::security_attack_keyword_bonus` + summed `SecurityAttackChange` modifiers.
  - `dpBreakdown` with real `total` (`Game::effective_dp`) and `temporary` (= total − base); base stays the printed DP.
  - each stack source's `mainEffectText`/`inheritedEffectText` and the permanent-level `inheritedEffects` array, from `CardData.{effect_text,inherited_text}` (this absorbs the unstarted `surface-runtime-card-state` change — see Impact).
- **New `modifiers` field** on the serialized permanent: a structured list of every display-relevant active `ModifierEntry` (`permanent_modifiers_iter`), each emitted as `{ type, value, expiry, sourceCardId }` — e.g. `CannotBeDeleted`, `CannotSuspend`, `CannotActivateWhenDigivolvingEffects`, `ChangeDp(+3000)`, granted-keyword modifiers. Rust emits structured data only; the frontend owns labels/grouping.
- **Frontend buff UI**: extend `PermanentInfo`/types with `modifiers`, add a `MODIFIER_DISPLAY` map (type → label + group: Immunity / Restriction / StatChange / Granted / Other), and render a grouped, color-coded "Active Modifiers" section in `CardOverlay` below the keyword block.
- **Right-click trigger**: `PermanentSlot` fires an `onInspect` callback on `onContextMenu` (preventDefault); threaded through `GameBoard` → battle/breeding areas for both players. `GamePage` opens the inspector from it regardless of `pendingSelection`/attack state. The existing left-click fallback, hover preview, and Escape/close-button dismissal are retained.

## Capabilities

### New Capabilities
- `permanent-runtime-state-serialization`: `to_ui_json` exposes a battle-area permanent's live runtime state for UI consumers — active keywords (with innate/granted breakdown), security-attack modifier, DP total/temporary breakdown, per-source and active inherited effect text, and a structured list of active modifiers (immunities, restrictions, stat changes, granted keywords) with value/expiry/source.
- `permanent-stack-inspector-ui`: the in-game UI lets a player open a detail panel (via right-click, at any time) for any permanent — own field, opponent field, or breeding — showing its digivolution stack, keywords, DP, effect text, and active-modifier list.

### Modified Capabilities
<!-- None. `live-game-surface` explicitly excludes `to_ui_json` (it owns the tool-facing `view` module). No existing spec covers the `to_ui_json` permanent fields. The unstarted `surface-runtime-card-state` change is superseded, not a modified spec. -->

## Impact

- **Engine (Rust)**: `code/digimon-engine/src/serialization.rs` (`perm_data` — reads existing `Game`/`card_data`/modifier registry; no new data sources). A stable type-string mapping for `ModifierType` (and `Expiry`) for emission. No change to action space, RL tensor, or `view` module.
- **PyO3 / response shape**: the permanent dict gains a `modifiers` array and existing fields change from empty to populated; consumers tolerate the additive field. `state_filter.py` must treat `modifiers` like other public permanent fields (battle-area permanents are public; opponent hidden sources already render as `???`).
- **Frontend**: `code/frontend/src/types/game.ts`, `utils/constants.ts` (`MODIFIER_DISPLAY`), `components/game/CardOverlay.tsx`, `components/board/PermanentSlot.tsx`, `GameBoard`/`BattleArea`/`BreedingArea`, `pages/GamePage.tsx`.
- **Supersedes `surface-runtime-card-state`** (unstarted): that change's effect-text work (Layers A/B) is folded in here; it should be archived/cancelled to avoid two changes editing the same function. Its broken premise (DP/keywords "already work") is corrected here — those fields are populated, not assumed.
- **Cross-engine**: aligns Rust `to_ui_json` with what legacy Python serialization produced; note in `docs/RUST_PYTHON_PARITY.md` if a divergence is closed.
- **Verification**: Rust serialization unit tests (constructed permanent with granted keyword + `CannotBeDeleted` + DP buff + `CannotSuspend` asserts correct `keywords`/`keywordBreakdown`/`securityAttackModifier`/`dpBreakdown`/`modifiers`); frontend tests for the grouped modifier render and the `onContextMenu` → inspect wiring; manual/Playwright check that right-click opens a populated panel on own + opponent permanents.
