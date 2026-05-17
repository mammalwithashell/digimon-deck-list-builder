# Phase 2 Track A — DSL Eval-Arm Residue Sweep

You are closing five DSL eval-arm gaps in `code/digimon-engine/src/dsl_cards/` that escaped Phase 1. Each is a small (5–30 LOC) match arm or one-line bridge that the variant-coverage lint should have caught but didn't, because in two of the five cases the `CompiledPredicate` field exists in `digimon-dsl` but the engine-side `eval_permanent_fields` (or sibling) just doesn't read it. The other three are formula or schema fields with the same shape.

This track is fully independent of every other Phase 2 work item. It is also fully independent of Track B (`activation_cost(...)` builder). Land it in a single PR; expect roughly +40 currently-`#[ignore]`'d behavioral tests to start passing without test-body edits.

## Why this matters

The 2026-05-15 audit projected Phase 1 would close ~150 DSL eval-arm tags. In practice Phase 1 landed as a variant-coverage lint + 5 predicate fields + 1 schema field, leaving these five tags open because each one was a Sonnet-author-discovered surface that postdates the audit. They all share the same anti-pattern: the spec parses cleanly, the `CompiledPredicate` / `CompiledFormula` carries the field, but the evaluator's `eval_*` function never reads it — so it silently behaves as if the predicate were always true (filter no-op) or the formula returned zero.

These are the highest-leverage cheap wins in the pilot-archetype unblock plan: closing them unsticks behavioral tests in DNA Omnimon, Medusamon, and BG Imperial without any architectural risk.

## Tags to close (in priority order)

| Tag | Refs in tests | Affected archetypes | Where |
|---|---:|---|---|
| **G-PRED-DP-LTE** | 16 | Medusamon(6), DNA Omnimon(6), BG Imp(4) | `predicate.rs::eval_permanent_fields` — `pred.dp_lte` referenced at line ~1655 but the `is_none()` guard short-circuits the check on `battle_area` permanents. |
| **G-COUNT-GTE-NOT-EVALUATED** | 7 | DNA Omnimon | `predicate.rs::eval_predicate` — `pred.count_gte` aggregate predicate parses but the `count_gte` arm at ~line 310 returns `false` for some compiled shapes. |
| **G-FORMULA-SOURCE-DP** | 5 | DNA(3), BG Imp(1), RK(1) | `formula_eval.rs` — formula leaf `source_dp` parses but evaluator has no arm. |
| **G-DSL-DISTINCT-TAMER-COLORS-FORMULA** | 8 | DNA Omnimon | `formula_eval.rs` — formula leaf `distinct_tamer_colors` parses but evaluator has no arm. |
| **G-ALT-PATH-CONDITION sweep** | 6 | DNA Omnimon(4), Puppets(2) | Already closed by PR #475. The `#[ignore = "BLOCKED: G-ALT-PATH-CONDITION"]` annotations on existing tests just need removing; tests should pass as-is. |

Expected net unblock: **~42 tests stop being `#[ignore]`'d**, with the bulk in DNA Omnimon and Medusamon.

## Read these first (in order)

1. `CLAUDE.md` — Working Rules §17 (no-approximations), §18 (TDD via DebugRunner — but the failing tests already exist for this track; "TDD" here means *un-ignore the existing failing test, then make it pass*).
2. `docs/superpowers/specs/2026-05-14-substrate-reality-check-plan.md` Phase 1 section — context for why these specific tags slipped through Phase 1.
3. `docs/superpowers/plans/2026-05-15-phase-1-dsl-pipeline-completion.md` § "Final outcome (2026-05-15)" — the variant-coverage lint added by PR #475 (`code/digimon-engine/tests/dsl_eval_arm_coverage.rs`). Understand why it didn't catch these five — it asserts field/variant names appear textually in the evaluator body, which is necessary but not sufficient (a name may appear in a sibling pre-filter without being honored in the consult site). **Do not weaken the lint to make these pass — fix the consult sites.**
4. `code/digimon-engine/src/dsl_cards/predicate.rs` — start by reading `eval_predicate_with_bindings` and `eval_permanent_fields` end-to-end. The two predicate tags live here.
5. `code/digimon-engine/src/dsl_cards/formula_eval.rs` — `evaluate_with_bindings` and `evaluate_read_with_raw_and_bindings`. The two formula tags live here.
6. `code/digimon-dsl/src/compiled.rs` — the `CompiledPredicate` struct field list (for the two predicate tags) and `CompiledFormula` enum variants (for the two formula tags). The DSL-side type is the contract.
7. `qa/dsl-vocab-gaps.md` — search for each tag for the recorded user-facing YAML shape and the suggested DSL syntax. Several entries already have `Suggested DSL syntax` blocks describing how authors expect the predicate/formula to read.
8. For each tag, the first failing test (find with `Grep "G-PRED-DP-LTE" code/digimon-engine/tests/cards_behavioral/`) — read it to understand the expected behavior before touching evaluator code.

## Work to be done

For each of the five tags, follow this loop:

1. **Find every `#[ignore = "...G-XXX..."]` test annotation.** Grep `code/digimon-engine/tests/` for the tag.
2. **Un-ignore them.** Remove the annotation. Run the test — confirm it fails (for the four real gaps) or passes (for G-ALT-PATH-CONDITION sweep).
3. **For the four real gaps:** find the consult site in `predicate.rs` / `formula_eval.rs`. Add the missing arm or fix the silent-default. Each fix should be 5–30 LOC.
4. **Confirm the test passes** without modifying the test body. If the test needs editing to pass, that's a signal the engine fix is wrong — investigate before changing the test.
5. **Tracker hygiene** — move the closed entry from `qa/dsl-vocab-gaps.md` to `qa/resolved-gaps.md` with a "Phase 2 Track A closure — 2026-05-XX" header; cite the test names that now pass.

### Per-tag implementation notes

**G-PRED-DP-LTE.** The `dp_lte: Option<u32>` field on `CompiledPredicate` is referenced by `eval_permanent_fields` (predicate.rs ~line 1666) but the surrounding guard at line ~1655 (`if pred.dp_eq.is_none() && pred.dp_lte.is_none() && pred.dp_gte.is_none()`) is a short-circuit fast-path. Inspect: does the slow path actually consult `dp_lte` against the permanent's `effective_dp` (not raw `CardData.dp`)? The Medusamon and BG Imperial tests expect the live DP after modifiers. Cite `Game::effective_dp` for the DP source.

**G-COUNT-GTE-NOT-EVALUATED.** Look at the `count_gte` aggregate predicate (predicate.rs ~line 310). The `CompiledPredicate::count_gte` field carries a sub-spec `{ over: ZoneRef, predicate: BoolPredicate, threshold: u32 }`. Confirm the inner predicate runs against each candidate and the threshold compare is signed-vs-unsigned-safe. DNA Omnimon's failing tests are `BT24-008` / `EX9-066` shape: "if you have N or more [TRAIT] Digimon in battle area, ...".

**G-FORMULA-SOURCE-DP.** Add an arm to `evaluate_with_bindings` for the formula leaf that returns `self.source_permanent()?.effective_dp(card_data)`. The "source DP" is the DP of the resolving carrier permanent.

**G-DSL-DISTINCT-TAMER-COLORS-FORMULA.** Add a formula arm that counts distinct colors across the *acting* player's Tamer permanents in battle area. Iterate `player.battle_area`, filter `is_tamer()`, accumulate the union of `Permanent::digimon_colors` (Tamers carry colors too), return the cardinality. DNA Omnimon's `P-182 WarGreymon` and several Tamer cards expect this — see `qa/dsl-vocab-gaps.md` § "ST20-10 — Distinct-Tamer-colours-on-field BoolPredicate" for context (boolean variant is sibling).

**G-ALT-PATH-CONDITION sweep.** PR #475 wired `condition: Option<PredicateSpec>` on `AltPathSpec` and consumer wiring in `dna_digivolve.rs::find_matching_alt_path`. The `#[ignore = "BLOCKED: G-ALT-PATH-CONDITION"]` annotations on existing tests just need removing. If any test fails after un-ignoring, the predicate evaluation path may have a regression — investigate before patching.

## Acceptance gates

- All four predicate/formula arms wired; the matching `#[ignore]` annotations across `code/digimon-engine/tests/cards_behavioral/` are removed and tests pass.
- The G-ALT-PATH-CONDITION sweep removes 6 `#[ignore]` annotations and all 6 tests pass without body edits.
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl_eval_arm_coverage` continues to pass (8/8). **Do not weaken the lint.**
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral` total pass count rises by ~40 with no regressions.
- Net `#[ignore]` count across `code/digimon-engine/tests/` drops by at least 30 (some unblocked tests may surface secondary failures that get re-tagged with different gaps — that's fine; count them in your PR summary).

## Constraints

- No-approximations (CLAUDE.md §17): the predicate/formula must read live engine state, not a snapshot. `dp_lte` consults `effective_dp`, not raw `CardData.dp`.
- Do not change `ACTION_SPACE_SIZE`, tensor profiles, PyO3 exports, frontend constants, or RL wrappers (Working Rule 1).
- Do not author new card YAML or modify existing card scripts in this PR — scope is engine eval-arms + test un-ignoring only. If you find a card test that fails after un-ignoring for a *different* reason than the closed tag, leave it `#[ignore]`'d with the new tag and call it out in the PR.
- Do not weaken `dsl_eval_arm_coverage` (added by PR #475 in `code/digimon-engine/tests/dsl_eval_arm_coverage.rs`). If it starts failing, the fix is to either (a) add the missing variant/field reference in the evaluator, or (b) document in the test why a wildcard is acceptable for the specific predicate/formula being added.
- Source priority (CLAUDE.md): printed card text → `docs/RULES_CONTEXT.md` → fandom wiki → DCGO. The DP-lte / count-gte tests have unambiguous printed text — no DCGO consultation should be needed.

## Verification

```
# The four arm fixes
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral

# Confirm the lint still holds
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl_eval_arm_coverage

# Smoke the rest
cargo test --manifest-path code/digimon-engine/Cargo.toml

# Ignore-count delta
git grep -c '#\[ignore' code/digimon-engine/tests | awk -F: '{s+=$2} END {print s}'
```

Pre-PR baseline: 596 ignored tests. Expected post-PR: ≤ 555.

## Tracker discipline

- `qa/dsl-vocab-gaps.md` — move closed tag entries to `qa/resolved-gaps.md` under a new "Phase 2 Track A closure — 2026-05-XX" header. Cite the PR # and the passing test names.
- `qa/archetype-qa/engine-gaps.md` — only update if a shadow entry exists for one of the closed tags. (Most do not.)
- `docs/RUST_ENGINE_GAPS.md` — no entries should need touching; this track closes DSL pipeline residue, not engine substrate. If you find an open entry that's actually closed by your work, that's a Track A discovery — call it out and move it to resolved.
- `qa/qa-reports/validated_cards_dsl.json` — entries that the unblocked tests cover can have their `status` advanced from `PARTIAL` to `IMPLEMENTED` *only if* the YAML body is genuinely complete. If unblocking surfaces a *different* gap, keep status as `PARTIAL` and update `notes` accordingly.

## Order of operations

1. G-ALT-PATH-CONDITION sweep first (no engine code; pure annotation cleanup; lowest risk, confirms tooling end-to-end).
2. G-PRED-DP-LTE second (highest test-ref count; same file as predicate work below).
3. G-COUNT-GTE-NOT-EVALUATED (same file as §2; batch the predicate.rs edits).
4. G-FORMULA-SOURCE-DP (`formula_eval.rs`).
5. G-DSL-DISTINCT-TAMER-COLORS-FORMULA (`formula_eval.rs`; batch with §4).
6. Tracker hygiene + PR.

## Out of scope (do NOT do in this PR)

- Any new `EffectContext` method.
- Any change to `effect_queue.rs`, `effect.rs`, `combat.rs`, or `pending_selection.rs`.
- New DSL verbs or `CompiledStep` variants.
- Card YAML authoring or `raw_rust` retirement.
- The `G-OPT-TRIGGERED` / `G-INHERITED-DISPATCH` substrate cluster — that's Phase 2 Track C, separately planned.
- The `.activation_cost(...)` builder — that's Phase 2 Track B, separately planned.

## Discovery rider

If, while reading test bodies, you find a tag whose fix is also a 5–30 LOC DSL eval-arm with the same anti-pattern (spec parses, evaluator silently defaults), add it to this PR as a sixth/seventh tag and update the per-tag table above. **Do not add tags whose fix requires new engine substrate, new builder methods, or new selection kinds** — those go to Track C or a new track.
