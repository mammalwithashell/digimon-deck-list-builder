## Why

The BG Imperial archetype (24 cards, all with YAML) sits at 9 IMPLEMENTED / 13 PARTIAL / 2 BLOCKED. Its gap-tracker inputs (`qa/archetype-qa/dsl/bg-imperial-cross-archetype-gaps-2026-05-03.md`, BG-relevant entries in `qa/dsl-vocab-gaps.md`) were written 2026-05-03/04 and are now badly stale: three substrate waves landed afterward (Phase 2 Tracks A–J / PR #480, DNA Omnimon completion, Puppets sweep). Direct source verification shows ~12 predicates/verbs the docs list as "open" already exist in the tree. The archetype is therefore mostly blocked by stale card YAML and `#[ignore]`'d tests, not by missing engine primitives — but the true remaining substrate set has never been pinned down.

## What Changes

- **Phase 0 (gating re-audit):** Re-verify every `#[ignore]`'d test across all 24 BG Imperial cards against current DSL/engine vocabulary. Classify each blocked clause as either *stale-ignore (re-author only)* or *genuine-gap*. Refresh `bg-imperial-cross-archetype-gaps` and the BG entries of `qa/dsl-vocab-gaps.md` so the trackers stop lying.
- Close the genuinely-remaining **DSL-only predicate/verb leaves** (engine already has the data): `stack_size_lte_source`, `carrier_has_keyword`, `is_carrier_of_source` aura target, `self_digivolution_contains_trait`, and an opponent/any-scoped `effect_suspended` predicate variant.
- Close the **engine-touching DSL verbs**: `select_opponent_sources`, a selected-trash-card → deck-top movement verb, and an `any_returned_card` result-set predicate plus the BT17-077 player-choice-of-trash branch.
- Scope and (if confirmed needed) close the **DNA-origin material/result event payload** residual (G-BG-03).
- **Decision point — explicitly in or out:** `G-COST-REDUCE-ALLY-DIGIVOLVE` (BT3-103), the one large engine gap. BT3-103 is not in either assessed BG Imperial meta list; default recommendation is **OUT of this change**, tracked separately.
- Re-author card YAML and un-`#[ignore]` tests to drive PARTIAL/BLOCKED verdicts toward IMPLEMENTED; update `validated_cards_dsl.json`.

## Capabilities

### New Capabilities
- `bg-imperial-archetype-coverage`: Faithful Rust-DSL implementation of the BG Imperial card pool — the substrate primitives its clauses require and the per-card behavioral coverage that proves each card's printed text is implemented under the no-approximations policy.

### Modified Capabilities
<!-- None. dna-omnimon-archetype-coverage and dsl-inherited-substitute-trash specs are not having their requirements changed. -->

## Impact

- **DSL crate** (`code/digimon-dsl/src/`): new predicate leaves in `predicate.rs` / `compiled.rs` / `compile.rs` / `validator.rs`; new step kinds in `step.rs`.
- **Engine DSL lowering** (`code/digimon-engine/src/dsl_cards/`): predicate evaluation in `predicate.rs`, new step lowering in `step/`, possible `EffectContext` helpers for opponent-source selection and deck-top trash movement.
- **Engine core** (`code/digimon-engine/src/`): only if the DNA-origin payload residual is confirmed in scope — `game.rs` / `effect_queue.rs` / `effect_context/` event payloads.
- **Card YAML** (`code/digimon-engine/cards/`): re-authoring of BG Imperial card files across bt3/bt12/bt16/bt17/bt20/st9/ex1/lm/p.
- **Tests** (`code/digimon-engine/tests/cards_behavioral/`): un-`#[ignore]` and add behavioral tests.
- **Trackers**: `qa/archetype-qa/dsl/bg-imperial-cross-archetype-gaps-2026-05-03.md`, `qa/dsl-vocab-gaps.md`, `docs/RUST_ENGINE_GAPS.md`, `qa/resolved-gaps.md`, `qa/qa-reports/validated_cards_dsl.json`.
- No `ACTION_SPACE_SIZE` or tensor-contract changes expected; new choices reuse existing pending-selection masks.
