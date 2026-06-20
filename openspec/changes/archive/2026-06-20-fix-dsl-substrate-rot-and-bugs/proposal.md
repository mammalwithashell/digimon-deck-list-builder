## Why

A 14-agent audit of the DSL substrate (gap trackers + recent sets BT22–25/EX10–11/ST24 + internal structure) found the substrate fundamentally healthy — but surfaced two **live correctness bugs**, a class of **silent failure** the loader doesn't catch, and pervasive **documentation/gap-tracker rot** that makes the codebase look less capable and the backlog look larger than they are. These are the highest-confidence, lowest-risk fixes and are independent of the larger consolidation work.

**Two live bugs (verified against source):**

- **BT13-007 — cost reduction is a silent no-op.** [`cards/bt13/BT13-007.yaml:19`](../../../code/digimon-engine/cards/bt13/BT13-007.yaml) uses `amount_fn: { raw_rust: bt13_007_royal_knight_cost_reduction }`, but that fn is registered only in the test harness — `build_registry()` has **no `register_formula` call at all**. In production the `CompiledFormula::RawRust(name)` arm in `formula_eval.rs` hits "not registered → returns 0" (a debug-only `eprintln`), so the Royal Knight play-cost reduction (printed "reduce by 4, then by 1 per digivolution card") reduces by **0**. The needed primitive already exists: `amount_fn: { base: 4, per: material_count, delta: 1 }`.
- **EX8-070 — no-approximations violation.** [`cards/ex8/EX8-070.yaml:148`](../../../code/digimon-engine/cards/ex8/EX8-070.yaml)'s `[Security]` runs an active raw_rust fn that deletes "the first Digimon at the minimum cost (lowest battle-area index)" with **no pending selection**. Printed text is "Delete 1 of your opponent's Digimon with the lowest play cost" — when several tie at the minimum, the player must choose (rule 17 / CLAUDE.md no-approximations). The `LowestPlayCost` selector (`step.rs:2245`) already exists to surface the tie through `pending_selection`.

**A silent-failure class.** The BT13-007 bug exists because an unregistered `raw_rust` name resolves to a no-op instead of failing. A pack validator (`missing_required_raw_rust_fns`) already exists but is not wired into the engine card-load path, so this can recur for any escape-hatch reference.

**Documentation/gap-tracker rot.** Of the 12 cards that grep as "raw_rust," only **2 still carry active raw_rust** (EX8-070, BT24-062); the other 9 are stale header comments on cards that are now pure DSL. Multiple recently-touched card headers (EX4-073, EX5-015, BT20-102, EX11-017) cite gap IDs that are already RESOLVED. The three gap trackers triple-log the same gaps (e.g. `OnDiscardHand` in all three). And the top-ranked "source-selection clamp/max_fn/cross_permanent" gap was **already shipped 2026-06-13** (`G-DSL-SELECT-SOURCES-FORMULA-COUNT`) yet `dsl-vocab-gaps.md:3077` and BT25-103's "blocked" status are stale. An author trusting these signals wastes time and risks re-introducing routed-around solutions.

## What Changes

- **Fix BT13-007**: replace the dead `raw_rust` `amount_fn` with the native `{ base: 4, per: material_count, delta: 1 }` formula; replace the ignored/empty behavioral test with one asserting the per-digivolution-card scaling actually applies.
- **Fix EX8-070**: replace the `[Security]` raw_rust step with `select_opponent_permanent { filter: { kind: digimon }, selector: lowest_play_cost }` + `delete_permanent`; invert the existing auto-pick assertion to assert a tie surfaces a `pending_selection`. Retires one of the two remaining active raw_rust fns.
- **Retire the last active raw_rust (BT24-062)** via a `target: self` self-modifier on `kind: aura`/`flood_gate` (small engine addition — see the companion step-idiom change if scheduled together; otherwise included here). Brings active raw_rust to **zero**.
- **Wire the loader guard**: `missing_required_raw_rust_fns` runs on engine card-load so an unregistered `raw_rust` name (step OR formula) is a **hard load error**, not a silent no-op.
- **Retire ~10 genuinely-dead verbs/predicates** (`bounce_self`, `mark_security_face_up`, `place_self_at_security`, `lose_memory_fn`, `form_is`, `source_is_tamer`, `add_digixros_cost_delta`, `add_digixros_wildcard_to_pending_transaction`, `event_target_same_level_as_previous`) after a final in-flight-card audit. Keep the wired completeness siblings.
- **Deprecate `link_card_to_self`** in the agent guide now (superseded by `link_cards`; usage is going backwards — 11 vs 3 — and new BT25 cards keep landing on it). The actual migration + deletion is in the step-idiom consolidation change.
- **Doc/gap-tracker reconciliation**: scrub the 9 stale raw_rust headers and the headers citing RESOLVED gaps; close the stale `G-DSL-SELECT-SOURCES-FORMULA-COUNT` open entry and re-check BT25-103's real blocker; de-duplicate the triple-logged `OnDiscardHand` (and similar) across the three trackers.
- **CI guard**: a check that fails when a card YAML header cites a gap ID listed in `qa/resolved-gaps.md`, so header rot self-corrects.

## Capabilities

### New Capabilities
- `dsl-substrate-integrity`: The engine fails card load on any unregistered `raw_rust` reference (no silent no-op); "delete the one with the lowest/highest <metric>" effects expose the tie choice through `pending_selection` rather than auto-picking; and card-header gap citations cannot reference already-resolved gaps (CI-guarded).

## Impact

- **Cards:** `cards/bt13/BT13-007.yaml`, `cards/ex8/EX8-070.yaml`, `cards/bt24/BT24-062.yaml`; header scrub on `cards/ex4/EX4-073.yaml`, `cards/ex5/EX5-015.yaml`, `cards/bt20/BT20-102.yaml`, `cards/ex11/EX11-017.yaml` + the other stale-header cards.
- **Engine:** wire `missing_required_raw_rust_fns` into the card-load path (`pack`/registry load); `target: self` handling in `lower_flood_gate.rs`/`lower_aura.rs` (for BT24-062); remove the retired-verb match arms in `digimon-dsl/src/{step.rs,predicate.rs}` + their lowerers + the `raw_rust/mod.rs` entries for EX8-070/BT24-062.
- **Tests:** un-ignore/replace BT13-007 + EX8-070 behavioral tests; an engine test that an unregistered raw_rust name fails load.
- **Docs/trackers:** `docs/RUST_DSL_AGENT_GUIDE.md` (deprecation note — regenerate the vocab block after verb retirement), `qa/dsl-vocab-gaps.md`, `docs/RUST_ENGINE_GAPS.md`, `qa/archetype-qa/engine-gaps.md`, `qa/resolved-gaps.md`.
- **CI:** new header-vs-resolved-gaps check.
- **No action-space/tensor/PyO3 impact.** Retiring dead vocab and fixing escape hatches does not change the RL contract; the EX8-070 fix adds a normal `pending_selection` through the existing selection machinery.

## Non-Goals

- The FormulaSpec scalar unification and predicate comparator factoring (separate `unify-dsl-scalar-and-comparators` change). `lose_memory_fn` is listed here for retirement but is also folded by that change — whichever lands first removes it.
- Generalizing `then:` tails, the `reveal_search` composite, security-placement consolidation, and the full `link_card_to_self → link_cards` migration (separate `collapse-dsl-step-idioms` change).
- Tier-3 capability gaps (mass suspend/unsuspend floodgate, BeforePayCost interactive chain, new observer timings, conditional digivolve restriction, ST24 bootstrap) — card-driven, authored when their archetypes come up.
