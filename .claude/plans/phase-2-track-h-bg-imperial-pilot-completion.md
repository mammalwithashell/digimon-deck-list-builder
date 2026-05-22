# Phase 2 Track H — BG Imperial Pilot Completion

You are unblocking the BG Imperial pilot archetype (18 stuck cards as of 2026-05-17). BG Imperial is the smallest stuck pile and has the most concentrated set of substrate edges: nearly every blocker is in the `BeforePayCost` triggered family.

Independent of all other Phase 2 tracks. Has a soft consumer relationship with Track B (`activation_cost(...)` builder) — some BG Imperial cards use the `BeforePayCost` trigger to pay an activation cost, which would route through Track B's builder if both lands together. But the work in this track is its own DSL surface, not engine substrate.

## Why this matters

BG Imperial scored 18 stuck cards, the smallest pilot pile. The blocker profile is laser-focused:

| Tag | Refs | Type |
|---|---:|---|
| **G-BEFORE-PAY-COST-DIGIVOLVE-TARGET** | 8 pending | DSL: BeforePayCost predicate against the digivolve target |
| **G-BEFORE-PAY-COST-GAIN-MEMORY** | 6 pending | DSL: BeforePayCost triggered gain-memory step |
| **G-COST-REDUCE-ALLY-DIGIVOLVE** | 3 BLOCKED | DSL: cost-reduction shape for ally digivolves |
| **G-OPTIONAL-SELECTION-CONTINUE-TAIL** | 4 pending | DSL: continue-tail after declined optional selection |
| **G-PRED-DP-LTE** | 4 pending | (closed by Track A) |
| **G-EFFECT-INITIATED-DIGIVOLVE-…-PERM-TARGET** | 2 BLOCKED | substrate (also gates DNA Omnimon — Track F) |
| **G-OPT-RESET-VIA-ATTACK-CYCLE** | 1 BLOCKED | (closed by Track C) |
| **G-PLAY-FROM-HAND-FREE-BIND-AS** | 1 BLOCKED | DSL bind_as field on play-from-hand-free verb |
| Long tail (1 ref each) | ~6 | mixed |

Expected unblock after Tracks A + C + F partial absorption + this track: **~12 BG Imperial cards advanced to IMPLEMENTED**.

## Read these first (in order)

1. `CLAUDE.md` — Working Rules §17, §18.
2. `qa/archetype-qa/dsl/bg-imperial.md` and `qa/archetype-qa/dsl/bg-imperial-cross-archetype-gaps-2026-05-03.md` — archetype docs.
3. `qa/qa-reports/validated_cards_dsl.json` — `"archetype": "BG Imperial"` for PARTIAL/BLOCKED entries.
4. `qa/dsl-vocab-gaps.md` — each tag entry.
5. `code/digimon-engine/src/effect_context/mod.rs` — `scan_before_pay_cost_reduction` and `scan_before_pay_cost_reduction_for_hand_card` (cited in `effect.rs:272-274`).
6. `code/digimon-engine/src/effect.rs` — `before_pay_cost` builder at line ~525, `pay_cost_fn` at ~893.
7. `code/digimon-engine/src/dsl_cards/predicate.rs` — site for new `before_pay_cost_*` predicate variants.
8. `code/digimon-engine/src/dsl_cards/step/` — site for new triggered-cost step variants.
9. `code/digimon-engine/src/game_actions.rs::play_from_hand_with_cost_result` — site of the BeforePayCost dispatch and cost-reduction summation.
10. BG Imperial cards in `data/cards.json`: search for "BG" / "Blackgatomon" / "Imperialdramon" for printed text.

## Work to be done

### 1. `G-BEFORE-PAY-COST-DIGIVOLVE-TARGET` (8 refs)

DSL: a BeforePayCost-timed predicate that evaluates against the *digivolve target* (the candidate top-card whose play cost is being computed). Today BeforePayCost predicates evaluate against the source card or controller, not against the target being digivolved into.

Add `event_digivolve_target_*` predicate family (or `would_play_target_*`): `_trait_has`, `_name_contains`, `_level_eq/lte/gte`, `_color_has`. The eval path needs `ctx.would_play_target_card_handle()` exposed (likely already partially wired via `scan_before_pay_cost_reduction_for_hand_card` per the comments in `effect.rs:272-274`).

Variant-coverage compliance.

### 2. `G-BEFORE-PAY-COST-GAIN-MEMORY` (6 refs)

DSL: a triggered-step at BeforePayCost timing that gains memory (rather than reducing cost). Today's `before_pay_cost` builder is scoped to cost-reduction; this is a sibling that fires the same trigger but performs a memory gain — typically printed as "when you would play X, gain N memory".

Decide whether to (a) extend `before_pay_cost` builder to support arbitrary `process` closures (not just `pay_cost_fn`), or (b) add a new `Effect::before_pay_cost_observe` builder distinct from cost reduction. Recommend (b) — the cost-reduction path is hot and shouldn't be widened. Both share the same trigger dispatcher but different bodies.

DSL surface:

```yaml
- kind: before_pay_cost_observe
  active_when:
    event_digivolve_target_trait_has: Imperialdramon
  body:
    - gain_memory: 1
```

### 3. `G-COST-REDUCE-ALLY-DIGIVOLVE` (3 BLOCKED refs)

DSL: cost-reduction whose source is an ally Digimon being played/digivolved (rather than the source card itself). Today's `before_pay_cost` builder ties cost-reduction to the SOURCE card's hand-play; the ally-digivolve variant pivots: it's "while THIS card is on field, your other Digimon's digivolve costs are reduced by N if they're [TRAIT]".

May reduce to a `kind: aura` shape with a `digivolve_cost_modifier_fn` slot — confirm whether existing Track H aura primitives can express this, or whether a new modifier kind is needed.

### 4. `G-OPTIONAL-SELECTION-CONTINUE-TAIL` (4 refs)

When an `optional: true` selection is declined, the steps AFTER the selection in the same body should still run. Today's behavior may park the tail. Sibling of `G-SELECT-EMPTY-OUTER-TAIL` which was partly closed 2026-04-29 for `select_material` / `select_own_sources`.

Audit the selection step dispatchers in `dsl_cards/step/selections.rs` to ensure declined optional selections always run the outer tail synchronously (per the 2026-04-29 pattern). Add tests.

### 5. `G-PLAY-FROM-HAND-FREE-BIND-AS` (1 BLOCKED ref)

Per `qa/archetype-qa/engine-gaps.md` § "`play_from_hand_free` Missing `bind_as` PermanentHandle Output": add `bind_as: Option<String>` to `PlayFromHandFreeArgs` in `digimon-dsl/src/step.rs` and `CompiledStep::PlayFromHandFree` in `compiled.rs`. Execute path inserts the just-played permanent's handle into bindings.

Cross-cites BT16-085 in `validated_cards_dsl.json`.

### 6. `G-EFFECT-INITIATED-DIGIVOLVE-…-PERM-TARGET` (2 refs — substrate; coordinate with Track F)

The 2 BG Imperial refs are sibling to the 3 DNA Omnimon refs. If Track F lands first, these are absorbed. If you reach them first, see Track F's plan § 7 — implement once, share with Track F author.

### 7. Track A absorption sweep

`G-PRED-DP-LTE` (4 refs) and `G-OPT-RESET-VIA-ATTACK-CYCLE` (1 ref) are closed by Tracks A and C respectively. After those land, sweep BG Imperial test files to un-ignore the relevant tests.

### 8. Author BG Imperial production YAML

Walk the per-card list. Many BG Imperial cards share the BeforePayCost target-trait + cost-reduction shape; expect strong batch effect.

## Acceptance gates

- BeforePayCost-target predicates land with variant-coverage compliance.
- BeforePayCost-observe builder + DSL surface lands.
- Cost-reduce-ally-digivolve DSL shape lands (aura-based or new modifier).
- Optional-selection continue-tail audit + fix.
- `bind_as` on PlayFromHandFree lands.
- ≥ 8 BG Imperial cards advance to IMPLEMENTED.
- All test suites pass.

## Constraints

- No-approximations: BeforePayCost-observe firing during opponent's play-action must not auto-resolve choices in the observer's body — surface through pending_selection if the body has options.
- Working Rule 1: no `ACTION_SPACE_SIZE` change.
- Working Rule 17: BeforePayCost-observe cannot AUTO-CANCEL a play; that's a different scope (`would_play` replacement, which is part of the replacement framework).
- Source priority: printed text → Rules Manual → fandom wiki → DCGO. BeforePayCost timing semantics are documented in §11 of the Rules Manual.
- Do NOT widen `before_pay_cost` builder's cost-reduction path. Add a sibling builder.

## Verification

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral
cargo test --manifest-path code/digimon-engine/Cargo.toml --test before_pay_cost
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl_eval_arm_coverage
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

## Tracker discipline

- `qa/dsl-vocab-gaps.md` — close BeforePayCost-related entries, G-PLAY-FROM-HAND-FREE-BIND-AS, G-COST-REDUCE-ALLY-DIGIVOLVE, G-OPTIONAL-SELECTION-CONTINUE-TAIL.
- `qa/archetype-qa/engine-gaps.md` — close G-PLAY-FROM-HAND-FREE-BIND-AS entry.
- `qa/qa-reports/validated_cards_dsl.json` — advance BG Imperial cards.

## Order of operations

1. Coordinate with Tracks A, C, F (absorption sweep).
2. BeforePayCost-target predicate family.
3. BeforePayCost-observe builder + DSL.
4. Optional-selection continue-tail audit.
5. `bind_as` on PlayFromHandFree.
6. Cost-reduce-ally-digivolve (largest residual item).
7. Card authoring walk.
8. Tracker hygiene + PR(s).

## Out of scope

- BeforePayCost cost-reduction substrate (closed in Group 3 / Track C earlier work — sibling family).
- Counter Blast DNA (closed).
- Effect-spawned permanent EOT deletion rider (separate substrate, planned).

## Discovery rider

If `G-COST-REDUCE-ALLY-DIGIVOLVE` is more invasive than expected (e.g., requires a new `ModifierType::AllyDigivolveCostModifier`), defer it to a follow-up and ship the rest of this track without it.
