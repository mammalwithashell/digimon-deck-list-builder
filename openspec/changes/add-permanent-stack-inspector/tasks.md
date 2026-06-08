## 1. Engine: stable type-string mappings

- [x] 1.1 Add an explicit `ModifierType` → stable wire-string mapping (`match`, not `{:?}`) in `serialization.rs`, covering display-relevant permanent-scoped variants; non-emitted variants return `None`.
- [x] 1.2 Add an explicit `Expiry` → stable wire-string mapping for use in the modifier objects.
- [x] 1.3 Unit test: every emitted `ModifierType` variant maps to a non-empty stable string; the mapping is total over the emitted set (guards against silent drift when variants are added). (Covered by `active_modifier_list_emits_structured_entries` exercising the emitted-type contract.)

## 2. Engine: populate runtime permanent fields in `perm_data`

- [x] 2.1 Thread the permanent's `PermanentHandle` and `&Game` into `perm_data` (drop the `_game` underscore) so live queries are available. (`Option<PermanentHandle>` — `None` for breeding.)
- [x] 2.2 Populate `keywords` + `keywordBreakdown.innate`/`.gained` from `face_keywords`/`inherited_keywords` and grant-modifier presence (via `Game::has_keyword` and the modifier registry); exclude parameterised SA keywords from chips.
- [x] 2.3 Populate `securityAttackModifier` from `Game::security_attack_keyword_bonus` + summed `SecurityAttackChange` modifiers.
- [x] 2.4 Populate `dpBreakdown.base` (printed), `.total` (`Game::effective_dp`), `.temporary` (total − base); keep `sources` empty per Non-Goals.
- [x] 2.5 Populate each stack source's `mainEffectText`/`inheritedEffectText` from `CardData.{effect_text,inherited_text}`, and the permanent-level `inheritedEffects` from non-top sources (top card excluded), attributable to each source.

## 3. Engine: active-modifier list emission

- [x] 3.1 Add a `modifiers` array to the serialized permanent: iterate `ModifierRegistry::permanent_modifiers_iter(handle)`, emit `{ type, value, expiry, sourceCardId }` for each variant with a stable string (skip the rest); resolve `sourceCardId` from `source_permanent` when present.
- [x] 3.2 Ensure `modifiers` is always present (empty array, not omitted) when there are no active modifiers.

## 4. Engine tests + binding

- [x] 4.1 `DebugRunner` serialization test: a permanent with a granted keyword + a `CannotBeDestroyed` modifier + a +3000 DP modifier + `CannotSuspend` serializes correct `keywords`, `keywordBreakdown`, `securityAttackModifier`, `dpBreakdown`, and `modifiers` entries. (`code/digimon-engine/tests/ffi_parity/perm_inspector.rs`.)
- [x] 4.2 Tests for the boundary cases in the spec: single-card permanent → empty `inheritedEffects` and `modifiers`; no-modifier permanent → `temporary` 0 and `total == base`.
- [x] 4.3 Shape-stability regression test pinning the documented keys/types of the serialized permanent (`permanent_has_documented_keys`).
- [ ] 4.4 Rebuild PyO3 binding (`maturin develop`) and confirm `to_ui_json` returns populated fields via a quick state fetch. (No PyO3 *code* change needed — the binding converts the serde_json value to a PyDict generically, so the new `modifiers` key + populated fields flow through on rebuild. `maturin` is not installed in this worktree; rebuild deferred to an environment that has it.)

## 5. Server: state filter

- [x] 5.1 Confirm/extend `state_filter.py` so the new `modifiers` field passes through for battle-area permanents (public info) and no hidden source identity is leaked for opponents; add/adjust a filter test. (Filter is battle-area-agnostic; passthrough confirmed by `code/tests/api/test_state_filter_modifiers.py`.)

## 6. Frontend: types + label map

- [x] 6.1 Add a `PermanentModifier` interface and `modifiers: PermanentModifier[]` to `PermanentInfo` in `code/frontend/src/types/game.ts`.
- [x] 6.2 Add `MODIFIER_DISPLAY` to `utils/constants.ts`: type → `{ label, group }` where group ∈ {Immunity, Restriction, StatChange, Other}; value-aware labels for stat changes (DP/SA/level ±N) via `utils/modifierDisplay.ts`; unmapped types fall back to `Other` (humanized). (Dropped the dead "Granted" group — granted keywords surface via `keywordBreakdown.gained`, not the modifier list.)

## 7. Frontend: inspector panel UI

- [x] 7.1 Add a grouped, color-coded "Active Modifiers" section to `CardOverlay.tsx` below the keyword block, rendering `permanent.modifiers` via `groupModifiers`/`MODIFIER_DISPLAY` with concise expiry hints.
- [x] 7.2 Verify the existing stack / keyword / DP / inherited-text rendering displays correctly now that the backend populates real data (engine tests confirm populated fields; opponent hidden sources already render as `???`).

## 8. Frontend: right-click trigger wiring

- [x] 8.1 Add an `onInspect` prop to `PermanentSlot`, fired on `onContextMenu` with `preventDefault()`; do not interfere with the existing left-click/hover behaviour.
- [x] 8.2 Thread `onInspect`/`onSlotInspect`/`onBreedingInspect` through `GameBoard` → `PlayerHalf` → `BattleArea` and `BreedingArea` for both players (player 1, player 2, breeding).
- [x] 8.3 In `GamePage`, set `inspectedPerm` from the inspect callbacks regardless of `pendingSelection`/attack state; keep the existing left-click fallback and Escape/close dismissal.

## 9. Frontend tests

- [x] 9.1 `CardOverlay` test: renders the grouped modifier list from a fixture `PermanentInfo` (immunity + stat-change + restriction), tolerates an unmapped type under "Other", and omits the section when empty.
- [x] 9.2 `PermanentSlot` test: `onContextMenu` fires `onInspect`, calls `preventDefault`, and does not trigger a play/attack action.

## 10. Wrap-up

- [x] 10.1 Archive/cancel the superseded `surface-runtime-card-state` change (its effect-text work is folded in here). Marked with a SUPERSEDED banner pointing to this change (kept, not deleted, since it's unstarted and `openspec archive` would wrongly promote its specs to main).
- [x] 10.2 Update `docs/RUST_PYTHON_PARITY.md` — marked the "Stubbed per-permanent fields" divergence mostly CLOSED, listing what's now populated and the intentional remaining stubs.
- [ ] 10.3 Manual/Playwright check: right-click opens a populated panel on own + opponent permanents and on a breeding Digimon. (Requires the running app — `npm run dev:desktop` + uvicorn :8000 + a seeded game; deferred to manual verification. All layers below it are covered by automated tests: engine serialization, state-filter passthrough, `CardOverlay`/`PermanentSlot` units, and `tsc` typecheck.)
