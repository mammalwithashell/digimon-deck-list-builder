## Context

The TS Olympos deck resolver snapshot identifies a representative deck with 23 unique cards. Eleven already have production Rust YAML plus behavioral tests; twelve representative cards remain unauthored or unverified: `BT24-095`, `BT10-042`, `BT24-085`, `BT24-088`, `BT24-090`, `BT24-035`, `BT24-051`, `BT24-030`, `BT24-041`, `BT24-034`, `BT24-083`, and `BT24-091`.

Most remaining cards are card-authoring work, but four reusable primitives block faithful implementation without approximations: effect-driven Option use from hand, source-stack aggregate predicates, formula-valued De-Digivolve amounts, and predicate-scoped timing suppression. DCGO was initialized at the repository-pinned submodule revision and reviewed as a behavioral reference. It does not contain per-card C# implementations for the missing TS Olympos cards at that revision, but it does show useful generic patterns for Option resolution, De-Digivolve loops, attack restrictions, and trigger wrappers.

The no-approximations policy applies: every player-visible choice must remain surfaced through pending selections and action masks. This change must not expand `ACTION_SPACE_SIZE` or alter tensor contracts as a side effect of card unlock work.

## Goals / Non-Goals

**Goals:**

- Close the four reusable Rust engine/DSL gaps needed by the representative TS Olympos deck.
- Author production YAML and behavioral tests for the twelve remaining representative TS Olympos cards.
- Preserve normal Option lifecycle semantics when an effect uses an Option from hand.
- Keep all new choices visible through existing pending-selection/action-mask machinery.
- Reconcile QA and gap documentation so training-readiness status is based on verified Rust source.

**Non-Goals:**

- Implement every card in the broad 117-card resolved TS Olympos pool in this change.
- Change observation tensor layout, action-space size, or model metadata contracts.
- Port DCGO code directly or treat DCGO as authoritative over printed card text.
- Add raw-Rust card stubs to claim readiness.

## Decisions

1. **Use existing Option lifecycle for effect-driven hand Option use.**

   Effects like `BT24-085` should not inline an Option's YAML body. They should select a legal Option card from hand, then run the same lifecycle as normal use: move to executing/pending Option state, fire `OnUseOption`, run `OptionMain` or selected mode, and dispose or attach according to subtype. This follows DCGO's `UseOptionClass` shape and prevents divergent behavior between ordinary Option use and effect-driven Option use.

   Alternative considered: add a generic "execute selected card text" step. That would bypass Option use legality, counters, disposal, Link/Delay paths, and future Option-resolution hooks.

2. **Add source-stack aggregates as predicates, not card-specific callbacks.**

   `BT24-030` needs "all opponent Digimon with the fewest digivolution cards." The current DSL already has level aggregate predicates and stack-size/material count scalar predicates. A new `materials_count_matches_aggregate` predicate should mirror `level_matches_aggregate`, support tied candidates, and compose with existing filters.

   Alternative considered: author Neptunemon with raw Rust or hardcoded source-count logic. That would be less reusable and would leave the same gap for future cards.

3. **Add `amount_fn` to `de_digivolve`.**

   The engine already supports De-Digivolve with literal amount and normal caps. `BT24-041` needs the amount to equal a live count of the controller's Digimon. Extending the DSL step with `amount_fn` reuses the existing formula evaluator and keeps the final peel operation inside `EffectContext::de_digivolve`.

   Alternative considered: repeat `de_digivolve: { amount: 1 }` in YAML behind multiple count branches. That cannot scale cleanly and would encode live game state as brittle authoring structure.

4. **Suppress timings at legality/enqueue boundaries.**

   DCGO models attack prevention as predicate-based static effects consulted by attack legality, while `[When Attacking]` and `[When Digivolving]` are normal trigger wrappers. Rust should follow that split: attack restrictions affect masks and resolution, and timing suppression prevents matching effects from being enqueued or activated for affected permanents.

   Alternative considered: insert suppression checks inside individual card effects. That would miss inherited/granted effects and would not generalize.

5. **Treat representative-deck readiness separately from broad-pool accounting.**

   The initial training unlock target is the representative TS Olympos deck. The broad resolved pool remains important and must be documented, but requiring all 117 unique cards would turn a focused archetype unlock into a much larger migration.

## Risks / Trade-offs

- **Risk: Effect-driven Option use diverges from ordinary Option use.** Mitigation: implement through shared lifecycle helpers and add tests comparing direct hand use with effect-driven use for the same Option card.
- **Risk: Timing suppression misses inherited or granted effects.** Mitigation: put the suppression check in the shared timing-dispatch path and cover face-up, inherited, and granted timing cases.
- **Risk: Formula-valued De-Digivolve amount evaluates against the wrong subject.** Mitigation: define formula context explicitly as the resolving effect source/controller, with a test for own-Digimon count and normal De-Digivolve caps.
- **Risk: Aggregate predicates include Tamers or empty stacks incorrectly.** Mitigation: compose with `kind: digimon`, evaluate material count as `stack_size - 1`, and test tied fewest counts.
- **Risk: Broad-pool cards remain unavailable after representative unlock.** Mitigation: keep broad-pool accounting in QA with the remaining count and do not mark the broad pool complete.

## Migration Plan

1. Add failing DSL/engine tests for the four primitives.
2. Implement primitives and run focused DSL/engine tests.
3. Add card behavioral tests for the twelve remaining representative TS Olympos cards.
4. Author card YAML in small batches, keeping tests enabled as each card becomes faithful.
5. Refresh resolver/QA ledgers and document broad-pool residual cards separately.
6. Run focused `cargo test` suites for DSL and TS Olympos card behavior.

Rollback is ordinary source rollback: the new DSL surfaces are additive and do not require data migration or tensor/action contract migration.

## Open Questions

- Should the training gate require only the representative TS Olympos deck or a named subset of high-frequency broad-pool cards as well?
- Should `use_option_from_hand` support cost payment in the same change, or only printed "without paying the cost" effects with an optional cost-ceiling filter?
