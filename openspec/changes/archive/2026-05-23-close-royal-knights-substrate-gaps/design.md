## Context

The Royal Knights audit and later Track J work left the archetype in a mixed state: some blockers are real, some substrate has already landed, and several card YAML/test comments still describe closed gaps as missing. The highest-impact examples are King Drasil breeding-source selection/play, Jesmon token and may-attack flows, BT13-112 Omnimon source-play payoff, and BT17-018 Gallantmon: Crimson Mode DP-budget deletion.

Current verified state:

- `select_material` / `select_materials` can target a breeding-area carrier through the `BREEDING_TARGET` sentinel and `BREEDING_SOURCE_SELECT` action range.
- `play_from_materials` can consume source bindings and supports On Play suppression.
- `select_opponent_dp_budget` and `select_opponent_play_cost_budget` exist as DSL vocabulary and engine pending-selection primitives.
- `select_own_breeding_permanent` filters candidates, but its engine selection is currently mandatory (`is_optional: false`), which blocks printed "you may" breeding-target clauses.
- Several Royal Knights production YAML files remain stubs or raw-rust approximations despite substrate now existing for part or all of their printed behavior.

The no-approximations policy is the governing constraint: every printed legal choice must surface through actions or pending selections. This change must not add hidden auto-selections to make card authoring easier.

## Goals / Non-Goals

**Goals:**

- Close the small reusable substrate gaps still blocking faithful Royal Knights card YAML.
- Reconcile stale Royal Knights gap comments against current engine/DSL capabilities before implementing card bodies.
- Migrate now-expressible Royal Knights cards from stubs or raw-rust approximations to native DSL.
- Add card-shaped behavioral coverage for the Royal Knights cards that consume the reusable primitives.
- Update gap trackers so open entries describe current missing primitives, not closed substrate.

**Non-Goals:**

- No expansion of `ACTION_SPACE_SIZE` or tensor contracts unless a task proves an unavoidable new action range is required and updates all action/tensor docs and exports in the same change.
- No RL retraining, model export, frontend work, or hosted training changes.
- No new Python legacy card scripts.
- No broad Royal Knights archetype balancing or deck optimization work.

## Decisions

### D1 - Start With Gap Reconciliation

Before changing card behavior, perform a code-verified reconciliation of each Royal Knights `#[ignore]`, YAML `BLOCKED` comment, raw-rust escape, and tracker entry. A tracker entry is not enough evidence that a primitive is still missing.

Alternative considered: implement from the May 3 Royal Knights rollup directly. Rejected because later Track J and Medusamon work closed several primitives, and implementing from stale notes would either duplicate substrate or preserve unnecessary raw Rust.

### D2 - Add Optional Breeding Selection as a Narrow Primitive

Extend the breeding-permanent selection path with optionality instead of wrapping every printed "you may target King Drasil in breeding" card in custom effect-choice scaffolding. The author-facing shape should mirror other selection steps: when optional, PASS declines the placement/play branch and the remaining tail behavior must match printed text.

Alternative considered: model optionality by adding a preceding `select_effect_choice` accept/decline step in each card. Rejected because it fragments a single printed choice into card-local ceremony and makes tail handling easier to get wrong.

### D3 - Prefer Existing Source-Selection and Budgeted-Selection Primitives

Royal Knights source plays should consume `select_material` / `select_materials` and `play_from_materials`; BT17-018-style deletes should consume `select_opponent_dp_budget`. New primitives should only be introduced where current code cannot express the printed choice.

Alternative considered: keep raw-rust bridges for complex cards. Rejected where native pending-selection machinery now exists, because raw Rust hides authoring patterns from future card batches and keeps stale gap debt alive.

### D4 - Treat Event-Bound Grants as Reusable Substrate

Cards such as BT23-072 need to grant keywords to a Digimon that was just played. The design should use event-context bindings and normal keyword/modifier expiry rather than card-local target rediscovery after the event.

Alternative considered: grant to any matching current field Digimon selected after the event. Rejected because the printed text targets one of the played Digimon, and target rediscovery can drift if multiple Digimon enter or leave during the trigger chain.

### D5 - Card Coverage Follows Substrate Closure

Each card migration must include an active behavioral test that proves the printed player choices and negative cases. Stubs can remain only when a code-verified gap is still open and is named in the test ignore reason and tracker.

Alternative considered: migrate YAML first and backfill tests after. Rejected because Royal Knights has a history of stale "implemented slice" comments; tests are the only reliable ledger.

## Risks / Trade-offs

- **Stale docs mislead the implementation** -> Mitigate with an explicit reconciliation task that reads current source before each card migration.
- **Optional breeding selection tail behavior diverges between PASS and no-candidate paths** -> Mitigate with tests for accept, decline, and no-candidate cases before using the primitive in cards.
- **BT13-007 ordering may reveal a new fidelity issue** -> Mitigate by testing whether current deterministic `for_each` ordering matches rules expectations; if ordering is player-visible, plan it as a narrow ordered-placement follow-up.
- **Event-bound keyword grants may over-target** -> Mitigate with tests where multiple matching Digimon exist but only the played/event-bound Digimon is eligible.
- **Action-space drift** -> Mitigate by reusing existing selection ranges; if a new range is unavoidable, stop and expand the action/tensor contract docs and exports in the same change.

## Migration Plan

1. Reconcile Royal Knights stale gaps and classify each card as substrate-gap, card-authoring, raw-rust-migration, or out-of-scope.
2. Land reusable substrate fixes with failing engine/DSL tests first.
3. Migrate card YAML in small batches, starting with cards that consume already-closed primitives.
4. Re-enable ignored behavioral tests as their cards become faithful.
5. Update `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md`, `qa/archetype-qa/engine-gaps.md`, and the Royal Knights rollup with closure notes and remaining true blockers.

Rollback is normal git revert. No persisted data migration is required.

## Open Questions

- Does BT13-007's start-main placement require a player-visible ordering selection for all tucked cards, or is the current deterministic ordering acceptable under project rules?
- Does `play_from_materials` currently expose enough binding information to grant Rush only to cards played by a specific effect without over-granting?
- Which Royal Knights stubs remain genuinely blocked after reconciliation versus merely unauthored?
