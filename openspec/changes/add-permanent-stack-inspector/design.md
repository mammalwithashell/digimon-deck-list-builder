## Context

The frontend already has a DCGO-style detail panel, `code/frontend/src/components/game/CardOverlay.tsx`, that renders a permanent's digivolution stack, keywords (with an innate/granted breakdown), a DP breakdown, security-attack modifier, and per-source inherited effect text. It reads a `PermanentInfo` (`code/frontend/src/types/game.ts`) produced by the engine's `to_ui_json`.

In the Rust engine — the source of truth — `serialization.rs::perm_data` builds that `PermanentInfo` with neutral defaults: `keywords: []`, `keywordBreakdown: {innate:[],gained:[]}`, `securityAttackModifier: 0`, base-only `dpBreakdown`, and empty `mainEffectText`/`inheritedEffectText`/`inheritedEffects`. The code comments these as "neutral defaults — richness arrives with card migration." So the panel renders almost nothing on the Rust path.

Separately, the panel is only opened as a left-click *fallback* in `GamePage.handleSlotClick` — it fires only when a click matches no other game action, so it is effectively unreachable during normal play and during selections.

The engine already exposes everything needed to populate the panel live:
- `Game::has_keyword(handle, kw)` — unified native + inherited + granted keyword query (`game/queries.rs`).
- `game/mod.rs::face_keywords` / `inherited_keywords` — printed keyword sets from `CardData`.
- `Game::effective_dp(handle)` (`combat/dp.rs`) and `Game::security_attack_keyword_bonus(handle)`.
- `ModifierRegistry::permanent_modifiers_iter(handle)` — every active `ModifierEntry { modifier, value, expiry, source_permanent, .. }` on a permanent.
- `CardData.{effect_text, inherited_text}` for per-source effect text.

This change supersedes the unstarted `surface-runtime-card-state` change, whose effect-text work is folded in here (both edit the same function; keeping them separate would conflict, and that change's premise that DP/keywords already work is false).

## Goals / Non-Goals

**Goals:**
- `to_ui_json` returns runtime-accurate per-permanent state: keywords + breakdown, SA modifier, DP total/temporary, source/inherited effect text, and a new structured `modifiers` list.
- A player can open a populated detail panel for ANY permanent (own field, opponent field, breeding) via right-click, at any point in the game.
- The active-modifier list faithfully mirrors DCGO's `PermanentDetail` content (immunities, restrictions, stat changes, granted keywords), grouped and labelled in the frontend.

**Non-Goals:**
- Per-source DP attribution in `dpBreakdown.sources` (DP deltas surface as `ChangeDp` entries in the modifier list, as DCGO does). Base + temporary + total only.
- Player-scoped modifiers (memory block, draw block, play gates) — these are not buffs on a permanent and are out of scope.
- Redesigning the panel's layout/position or the hover-preview behaviour.
- New rules/engine behaviour — this is a read-only serialization + UI change.

## Decisions

### D1. Structured modifier emission, frontend-owned labels
Rust emits each active modifier as `{ type: "<ModifierType>", value: i32, expiry: "<Expiry>", sourceCardId: string|null }`. The frontend `MODIFIER_DISPLAY` map turns `type` (+ `value`) into a human label and a group (`Immunity` / `Restriction` / `StatChange` / `Granted` / `Other`).
- *Why over pre-formatted strings (DCGO's approach):* consistent with how keywords already render (`KEYWORD_DISPLAY`), keeps wording/i18n/grouping/styling in the UI layer, and lets the panel group and color-code instead of showing a flat bulleted list. Cost is a TS label map to maintain — acceptable and localized.

### D2. Stable type strings via an explicit mapping, not `Debug`
`ModifierType` → wire string uses an explicit `match` (or `serde` rename) in `serialization.rs`, not `{:?}`, so refactors/renames of the enum don't silently change the UI contract. Same for `Expiry`. Only *display-relevant, permanent-scoped* variants are emitted; internal/bookkeeping state is skipped. Unmapped variants fall back to `Other` in the frontend rather than crashing.

### D3. Keyword breakdown derivation
`keywordBreakdown.innate` = printed face + active inherited keywords (`face_keywords` ∪ live `inherited_keywords`); `.gained` = keywords present via grant modifiers (`GrantBlocker`, etc.) but not innate. `keywords` = union. Parameterised keywords (`SecurityAttackPlus(n)`) are reflected through `securityAttackModifier`, not as keyword chips, to avoid double-counting.

### D4. Right-click is additive
`PermanentSlot` gains `onContextMenu` → `onInspect(handle)`; `e.preventDefault()` suppresses the browser menu. The existing left-click fallback and hover preview stay. Inspect is available irrespective of `pendingSelection`/attack selection (a pure UI read; it never submits an action). Works for both players and breeding via the same threaded callback.

### D5. Opponent permanents
Battle-area permanents and their digivolution sources are public information, so inspecting an opponent permanent shows real data. Any source the filtered state omits (`cardId == null`) already renders as `???` in `CardOverlay`. `state_filter.py` treats the new `modifiers` field as a public permanent field (no redaction).

## Risks / Trade-offs

- **Modifier-list fidelity drift** → If a new `ModifierType` is added later without a `MODIFIER_DISPLAY` entry, it shows as a generic "Other" label. Mitigation: frontend fallback never crashes; a unit test enumerates emitted types so additions are noticed.
- **Wire-contract change** (fields go from empty to populated; new `modifiers` array) → Consumers that pinned empty values would see real data. Mitigation: additive field; a regression test pins the shape (keys present, types correct).
- **Right-click during selection could confuse** with the action UI → Mitigation: inspect is read-only and dismissible (Escape / close / click elsewhere); it never consumes a selection or mutates state.
- **Cross-engine parity** → Legacy Python populated some of these; record alignment in `docs/RUST_PYTHON_PARITY.md` to avoid a spurious divergence flag.

## Migration Plan

1. Land Rust `perm_data` population + `modifiers` emission behind no flag (read-only, additive). Rebuild PyO3 binding (`maturin develop`).
2. Add frontend types + `MODIFIER_DISPLAY` + `CardOverlay` section + right-click wiring.
3. Archive/cancel `surface-runtime-card-state` (folded in).
4. Rollback is reverting the change set; no data migration, no persisted state.

## Open Questions

- Should `inheritedEffects` reflect runtime substitution/removal (cf. `dsl-inherited-substitute-trash`) in v1, or only printed inherited text? Lean: printed text in v1; runtime-accurate active set as a follow-up if needed (carried over from `surface-runtime-card-state` Layer B).
- Expiry display granularity: show a short hint ("until end of turn") for non-permanent expiries, or omit? Lean: show a concise hint where the `Expiry` maps cleanly, omit otherwise.
