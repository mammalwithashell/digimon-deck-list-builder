# Substrate Reality-Check Plan

**Date:** 2026-05-14
**Status:** Draft for review — supersedes the implementation framing in
[2026-04-29 archetype engine DSL gap roadmap](2026-04-29-archetype-engine-dsl-gap-roadmap-design.md)
and [2026-05-03 latest archetype DSL/engine gap closure](2026-05-03-latest-archetype-dsl-engine-gap-closure-design.md).
Those documents remain useful as the historical capability-slice view.

## Context

A direct audit of the engine source on 2026-05-14 (with submodule DCGO finally
initialized as the behavioral tiebreaker) found that the substrate the prior two
specs treat as "to-build" is largely already built:

| Substrate module                       | Lines | Status                |
|----------------------------------------|------:|-----------------------|
| `code/digimon-engine/src/replacement.rs` | 1,316 | Built (Track B + extensions) |
| `code/digimon-engine/src/option_lifecycle.rs` | 360 | Built (its own module) |
| `code/digimon-engine/src/aura.rs`       |   387 | Built (Track H)       |
| `code/digimon-engine/src/selection.rs`  |   788 | Built — 19 `SelectionKind` variants |
| `code/digimon-engine/src/trigger_context.rs` |  154 | Built — full result-log fields |
| `code/digimon-engine/src/effect_context/mod.rs` | 4,579 | Public API surface |
| `code/digimon-engine/src/effect_context/selections.rs` | 2,714 | Every selection wrapper |
| `code/digimon-engine/src/effect_queue.rs` | 2,757 | Dispatch in place    |

Concrete primitives the prior specs claim are "open" but actually exist (verified
with file:line citations):

- `select_reveal_buckets` — `effect_context/selections.rs:738` (Slice 1)
- `may_attack_now`, `may_attack_now_optional`, `may_attack_now_optional_with_upgrade`,
  `cancel_attack` — `effect_context/mod.rs:4393–4421` (Slice 2)
- `refire_effect_from_permanent`, `refire_target_effect` — `effect_context/mod.rs:653, 677` (Slice 9)
- `select_partition_sources`, `select_opponent_permanents_by_dp_budget`,
  `select_own_breeding_permanent`, `select_union_zone`, `select_ordered_permutation`,
  `select_count_capped_multi` — all in `effect_context/selections.rs`
- `TriggerContext.selected_results` (typed `ResultBinding` vec),
  `moved_card_sets`, `effect_initiated`, `dna_origin`, `deleted_object`,
  `provenance_token`, `option_last_field_state` — `trigger_context.rs:114–139`
- `ProvenanceToken(u64)` — `trigger_context.rs:28`
- `SelectionKind::RevealBucket / UnionZone / OrderedPermutation /
  CountCappedMultiSelect / Replacement / SourceMulti / DpBudget /
  BreedingPermanent / EffectChoice` — all in `selection.rs:86–149`

Meanwhile, two real signals tell us where the work actually is:

```
  Active gap tags referenced in tests:        216 distinct
  Ignored behavioral test annotations:        595 across 164 files
  `raw_rust` escape uses in production YAML:  110 in 39 cards
  Production cards with YAML:                 ~270 of ~4,000
```

Distribution by tag-reference count (top of the pile):

```
  REFS  TAG                                            CATEGORY
  ────  ─────────────────────────────────────────────  ──────────────────
   139  G-OPT-TRIGGERED                                substrate edge
   107  G-INHERITED-DISPATCH                           substrate edge
    81  G-DECLARATIVE-KEYWORD                          DSL lowering
    65  G-PRED-DP-LTE                                  DSL eval-arm missing
    39  G-EVENT-TARGET-OWNER                           DSL eval-arm missing
    38  G-PLACE-SELF-AS-OPTION-PERMANENT               DSL verb
    37  G-ALT-PATH-CONDITION                           DSL schema gap
    27  G-PLAY-COST-LTE                                DSL eval-arm missing
    23  G-IGNORE-COLOR-MASK                            DSL
    22  G-BEFORE-PAY-COST-DIGIVOLVE-TARGET             substrate edge
    21  G-FORMULA-SOURCE-DP                            DSL formula
    21  G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-…       substrate edge
    21  G-DSL-SOURCE-NAME-CONTAINS                     DSL eval-arm
    19  G-DSL-DISTINCT-TAMER-COLORS-FORMULA            DSL formula
    18  G-DELAY-START-OF-TURN                          substrate edge
    17  G-DSL-UNION-PLAY-FREE                          DSL verb
    15  G-DSL-PLACE-TOP-SOURCE-AS-BOTTOM               DSL verb
    15  G-DSL-INHERITED-SUBSTITUTE-RETURN-TRASH        DSL verb
    15  G-COST-REDUCE-ALLY-DIGIVOLVE                   DSL
    14  G-BEFORE-PAY-COST-GAIN-MEMORY                  substrate edge
```

Classifying the active tag pile by leverage:

```
  Category                  Tag refs   % of total
  ────────────────────────  ────────   ──────────
  Substrate edges           ~280         ~38%
  DSL pipeline residue      ~430         ~58%
    (spec parses but evaluator missing)
  Card-author residue        ~30          ~4%
```

The 58% "DSL pipeline residue" share is the single largest finding. It is the
same anti-pattern repeating across ~150 tags: the `digimon-dsl` crate parses a
field, `digimon-engine/src/dsl_cards/predicate.rs` (or sibling lowering files)
has the matching `CompiledPredicate` / `CompiledStep` variant, but the
`eval_predicate` / step executor doesn't have an arm for it — so a card script
silently treats the predicate as `true` or `false`. The schema and parse tests
pass; only behavioral tests catch the silent default.

## Goals

1. Re-baseline `docs/RUST_ENGINE_GAPS.md` against the actual engine state so the
   roadmap and the trackers stop diverging.
2. Close the DSL pipeline residue (the 58% slice) with a substrate-level
   invariant that prevents the anti-pattern from recurring.
3. Close the small set of real substrate edge cases (G-OPT-TRIGGERED,
   G-INHERITED-DISPATCH residue, BeforePayCost target binding, Delay start-of-
   turn, OPT reset across turn cycle).
4. Scale out card YAML authoring under the no-approximations rule once the DSL
   pipeline is complete, retiring `raw_rust` escapes.

## Non-Goals

- This plan does not author cards or batch-implement archetypes — that work
  belongs to `/batch-implement-cards-rust-dsl` runs.
- It does not change `ACTION_SPACE_SIZE`, tensor layout, PyO3 exports, or RL
  contracts.
- It does not retire the Python engine — the migration follows independently.
- It does not retire the 2026-04-29 / 2026-05-03 specs; their slice framing
  remains useful for card-author-facing readiness conversations.

## Cross-cutting contracts

Every phase below obeys:

- The no-approximations rule (CLAUDE.md §17–18): every player-visible choice
  flows through `PendingSelection` or an action-mask bit.
- Source priority for card / keyword / rules questions: printed card text →
  `docs/RULES_CONTEXT.md` → fandom wiki → DCGO submodule (now initialized).
- Failing test first. New substrate variants are added with the test that proves
  the variant evaluates correctly before any DSL surface lands.
- Tracker hygiene: every closed entry moves to `qa/resolved-gaps.md` with PR
  citation and test command in the same change.

## Plan

```
  PHASE 0  Re-baseline       ── audit, retire stale claims
  PHASE 1  DSL pipeline      ── close ~150 eval-arm gaps + add lint
  PHASE 2  Substrate edges   ── close ~10 narrow engine gaps
  PHASE 3  YAML scale-out    ── card authoring + retire raw_rust
```

Each phase below names its acceptance gate and the artifact it produces.

### Phase 0 — Re-baseline

**Why first.** Phase 1's batching depends on knowing what's actually open. We
should not write a fix-plan against entries that landed in Tracks A–J.

**Inputs.**

- `docs/RUST_ENGINE_GAPS.md` (~3,300 lines, ~38 named primitives in the
  "At a glance" table)
- `qa/archetype-qa/engine-gaps.md` (~280 lines, ~12 entries)
- `qa/dsl-vocab-gaps.md` (~73K tokens — DSL-side mirror)
- `qa/resolved-gaps.md` (archive)
- Engine source under `code/digimon-engine/src/`

**Process.**

For every "Open" or "Partial" entry in the three trackers, cross-reference
against the engine source. Verdict each entry one of `CLOSED`, `NARROW`,
`OPEN`, or `UNCLEAR`. Output a findings doc at
`docs/superpowers/audits/2026-05-14-rust-engine-gap-rebaseline.md` with one
6–10 line block per entry and a verdict summary at the top.

**Acceptance.**

- Audit doc exists and covers every Open/Partial entry.
- A follow-up PR demotes `CLOSED` entries to `qa/resolved-gaps.md` and rewords
  `NARROW` entries to describe the actual residual scope.
- Tracker hygiene sweep note added to top of `RUST_ENGINE_GAPS.md` citing this
  audit.

**Status (2026-05-15).** Audit complete. Findings at
[`docs/superpowers/audits/2026-05-14-rust-engine-gap-rebaseline.md`](../audits/2026-05-14-rust-engine-gap-rebaseline.md).

Verdict summary across ~38 named primitives in `RUST_ENGINE_GAPS.md`:

| Verdict | Count | Action |
|---|---|---|
| **CLOSED** — primitive fully exists, header severity is stale | 8 | Move to `qa/resolved-gaps.md` |
| **NARROW (effectively-CLOSED inside)** | ~20 | Split umbrella, relocate closed core, retitle residual |
| **NARROW (real residual)** | ~16 | Drop 🔴 → 🟡, rename to residual sub-shape |
| **OPEN** — claim accurate, substrate missing | 12 | Keep open; these are Phase 2 work |
| **UNCLEAR** — first-test write needed to confirm | 2 | EX9-032 + EX4-074 |

**Headline finding.** The at-a-glance severity table at lines 117–158 of
`RUST_ENGINE_GAPS.md` is systematically stale: nearly every 🔴 BLOCKING row has
✅ RESOLVED footers in its own per-entry prose. The headline lags by 3–6 weeks
behind the inline sweep notes. Roughly 28 entries are ready to relocate, which
shrinks the open-gaps section from ~50 entries to ~22.

### Phase 1 — DSL pipeline completion

**Status:** ✅ Complete (2026-05-15) — see [`docs/superpowers/plans/2026-05-15-phase-1-dsl-pipeline-completion.md`](../plans/2026-05-15-phase-1-dsl-pipeline-completion.md#final-outcome-2026-05-15) for the outcome summary. The variant-coverage lint test (`code/digimon-engine/tests/dsl_eval_arm_coverage.rs`) is the durable deliverable; closure was much smaller in scope than the audit projected (5 predicate fields + 1 schema field, vs ~150 expected) because prior incremental work had already closed the bulk on the formula/step/timing surfaces.

**Why second.** ~58% of active gap tags are DSL eval-arm-missing of the form
"spec parses but evaluator missing." Closing them is high-leverage low-risk
work — most are 5–20 line additions in `dsl_cards/predicate.rs`,
`dsl_cards/formula_eval.rs`, `dsl_cards/step/*.rs`, or a missing `CompiledX → X`
lowering line.

**Files likely touched.**

- `code/digimon-engine/src/dsl_cards/predicate.rs` (eval_predicate arms)
- `code/digimon-engine/src/dsl_cards/formula_eval.rs` (eval_formula arms)
- `code/digimon-engine/src/dsl_cards/lower_*.rs` (lowering)
- `code/digimon-engine/src/dsl_cards/step/*.rs` (step executors)
- `code/digimon-engine/src/dsl_cards/timing_map.rs` (`compiled_timing_to_engine`)
- `code/digimon-engine/src/dsl_cards/modifier_map.rs`
- `code/digimon-dsl/src/predicate.rs`, `formula.rs`, `step.rs`, `compile.rs`
- `code/digimon-engine/tests/dsl/eval_arm_coverage.rs` (new — coverage lint)

**Substrate invariant introduced.**

> **Every `CompiledPredicate`, `CompiledFormula`, `CompiledStep`, and
> `CompiledTiming` variant must have an exhaustive `match` arm in its
> evaluator/executor — no `_ => false` / `_ => None` / `_ => Ok(())` wildcard
> catch-alls in evaluation code.**

Enforced by a `#[deny(non_exhaustive_omitted_patterns)]` or a new compile-time
proc-macro `#[exhaustive_eval]` annotation on the eval functions. As a fallback
short-term, a CI lint test asserts that every variant name in
`CompiledPredicate` appears textually in `eval_predicate`'s match body.

This is the single most valuable engineering invariant in the plan. It is what
prevents the next 150 silent gaps from accumulating.

**Batching.** Group the ~150 tags by which eval surface they touch:

```
  Batch 1  Predicate evaluator arms   ~80 tags
           - G-PRED-DP-LTE (65) + G-EVENT-TARGET-OWNER (39)
           - G-PLAY-COST-LTE (27) + G-DSL-SOURCE-NAME-CONTAINS (21)
           - long tail of single-card-shaped predicate eval

  Batch 2  Formula evaluator arms     ~40 tags
           - G-FORMULA-SOURCE-DP (21) + G-DSL-DISTINCT-TAMER-COLORS-FORMULA (19)
           - G-BINDING-DP-FORMULA + long tail

  Batch 3  Step executor + lowering   ~30 tags
           - G-DSL-UNION-PLAY-FREE (17)
           - G-DSL-PLACE-TOP-SOURCE-AS-BOTTOM (15)
           - G-DSL-INHERITED-SUBSTITUTE-RETURN-TRASH (15)
           - G-COST-REDUCE-ALLY-DIGIVOLVE (15)

  Batch 4  Timing map + dispatcher   ~10 tags
           - residuals like G-IS-EFFECT-INITIATED (11),
             specific TriggerSource → EffectTiming threads
```

Each batch lands as a single PR:

1. Failing tests (run the existing `#[ignore]` tests as the regression — they
   are already authored)
2. Eval/exec arm implementations
3. Lint test asserting variant coverage on the touched evaluator
4. Tracker updates: every closed tag moves from `qa/dsl-vocab-gaps.md` to
   `qa/resolved-gaps.md`, and the matching `#[ignore]` annotations are removed

**Acceptance.**

- Variant-coverage CI test passes (no silent eval defaults).
- Ignored test count drops from 595 to under 200.
- Top 20 gap tags by reference count all RESOLVED or in `qa/resolved-gaps.md`.
- A small follow-up PR collapses the per-batch tracker entries into a single
  "Phase 1 closure" rollup section in `qa/resolved-gaps.md`.

### Phase 2 — Substrate edge closure

**Why third.** With the DSL pipeline complete, the remaining gaps are real
engine work. ~10–15 items, each narrow but non-trivial.

**Items (the 12 truly-OPEN entries from the 2026-05-14 audit, plus a small
set of substrate edges surfaced by test ignore-counts):**

Per the audit findings doc, the truly-open engine substrate gaps are:

1. **`<Training>` keyword** — `Keyword::Training`, `push_deck_top_under_self(face_down)`,
   `CardSource::face_down` field, `[Main]` activation extension to breeding.
   - 🔴 BLOCKING; no substrate landed.
2. **Standard Delay main-phase activation action mask exposure** — Group 5
   scheduled EOT auto-fire is the workaround; the player's `[Main]` decision
   is not exposed through the action mask.
3. **Digivolution-stack / reveal-zone name+level overlay** — `name_overlay`,
   `level_overlay`, reveal-zone synthesis primitives. BT17-102 / BT17-068.
4. **Effect-spawned permanent EOT deletion rider** — `schedule_delete_at_end_of_turn`
   and `scheduled_eot_deletions` queue. ProvenanceToken (Track A) provides the
   lookup half; cleanup half unwired.
5. **Cast-time stack-construction for cost reduction** — `play_with_cast_time_assembly`,
   separable `commit_play_to_battle_area_without_on_play`. BT15-102 Apocalymon.
6. **Conditional digivolve-target restriction** — `DigivolveTargetRestriction`,
   `CanOnlyDigivolveIntoColor`, `CanOnlyDigivolveIntoTrait`. Zero matches today.
7. **Effect-initiated play from face-up security stack** — `play_face_up_security_free`.
   P-216.
8. **Generic `.activation_cost(...)` builder hook** — for triggered abilities
   that take a non-cost-reduction activation cost. `.pay_cost` Group 3 builder is
   distinct.
9. **Player-scope mass `CannotSuspend` aura on opponent** — permanent-scope exists;
   player-scope condition-gated continuous evaluation does not.
10. **Effect play with played-Digimon On Play suppression** — `suppress_on_play`,
    `PlayOptions`. BT5-106.
11. **Narrow opponent-effect protection for DP reduction / De-Digivolve** —
    `ImmuneToOpponentDpReduction`, `ImmuneToOpponentDeDigivolve`,
    `EffectCategoryProtection`. BT16-055.
12. **Bilateral player-aura `UntilLeaveField` delivery** — `Expiry::Permanent`
    used today; `UntilLeaveField` lifecycle incomplete. BT14-009 Gotsumon.

Plus a small set of substrate edges surfaced by test ignore-counts (not all of
these are in `RUST_ENGINE_GAPS.md` — some live in `qa/dsl-vocab-gaps.md` or
`qa/archetype-qa/engine-gaps.md`):

13. **G-OPT-TRIGGERED** (139 refs) — once-per-turn enforcement for triggered
    effects in `effect_queue::run_queued_effect_inner`. Lockout exists for
    manually-activated effects; queued triggers bypass it.
14. **G-INHERITED-DISPATCH residue** (107 refs) — remaining dispatch paths
    `enqueue_from_permanent` misses for inherited-stack observers, after Phase 1
    closed the major fan-outs. Audit ergonomics: enumerate every
    `TriggerSource` and confirm inherited fan-out is wired.
15. **G-OPT-RESET-VIA-ATTACK-CYCLE** — OPT slot key tied to carrier identity
    vs. position so a full turn cycle properly resets. Currently persists across
    turn boundaries when carrier source-identity differs from trigger source.

Ergonomic / sugar items the audit lists as 🟡 ergonomic only — defer to Phase 3
or pick up opportunistically:

- `CardSource::has_type` / `Permanent::has_any_type` accessors
- `ctx.grant_security_attack_change` typed sugar (aura form is closed)
- Aggregate filter helpers / dual-tri-timing composite / on-decline callback
- ProvenanceToken cleanup-token half (lookup is wired)
- Generic `ctx.prompt_blast_digivolve`/`prompt_blast_dna_digivolve` raw_rust
  helpers (engine substrate is closed; this is sugar for card authors)

Each item lands as a focused PR: failing test → engine fix → tracker update.

**Acceptance.**

- Test ignore count drops from ~200 (Phase 1 outcome) to under 50.
- All 12 truly-OPEN substrate items + the 3 ignore-count-leveraged edges
  (G-OPT-TRIGGERED, G-INHERITED-DISPATCH residue, G-OPT-RESET-VIA-ATTACK-CYCLE)
  have RESOLVED entries in `qa/resolved-gaps.md`.
- No new substrate gap entries created during this phase (would indicate a
  Phase 1 / Phase 0 oversight).

### Phase 3 — Card YAML scale-out

**Why last.** With the DSL pipeline complete and substrate edges closed,
authoring is the only remaining work. The `/batch-implement-cards-rust-dsl`
skill exists for exactly this purpose.

**Inputs.**

- 4,085 cards in `data/cards.json`
- 270 currently with YAML
- 39 currently using `raw_rust` escapes (must be rewritten when their DSL
  prerequisites land)

**Process.**

Run `/batch-implement-cards-rust-dsl` in archetype waves, prioritizing meta
archetypes by `data/deck_library.json` representation. After each wave:

1. Inspect newly created `#[ignore]` annotations — any tag that didn't exist
   before is a new gap.
2. New tags route as follows:
   - Pure DSL eval-arm: append to Phase 1 backlog, fix immediately.
   - Substrate edge: file in `RUST_ENGINE_GAPS.md` and queue for a
     mini-Phase 2.
   - Card-local authoring: complete in the same wave.
3. Retire `raw_rust` for any card whose DSL prerequisites have landed.

**Acceptance.**

- Per-wave: validated_cards_dsl.json updated; no new `raw_rust` introductions.
- Long-term: `raw_rust` use trends to zero (acceptable interim values:
  Wave +0 = 39, +5 waves ≤ 20, +10 waves ≤ 5).
- Production YAML coverage ≥ 80% of cards in active meta decklists by end of
  campaign.

This phase is open-ended; closure happens archetype-by-archetype rather than at
a single endpoint.

## Sequencing and parallelism

Phase 0 must complete before Phase 1 batches finalize (otherwise we may write
fix-plan PRs against stale claims). Phase 1 batches can parallelize across
batches 1–4 if independent contributors are available. Phase 2 items are
mostly independent and can parallelize freely. Phase 3 waves are sequential
within an archetype but can parallelize across archetypes (the existing
batch skill already orchestrates this with per-card sub-agents).

```
   Phase 0  [audit-doc]
       │
       ▼
   Phase 1  [B1  predicate ─┐
            [B2  formula  ──┼─► variant-coverage lint   ── single rollup PR
            [B3  step exec  │
            [B4  timing   ──┘
       │
       ▼
   Phase 2  10 independent narrow PRs (parallel within Wave)
       │
       ▼
   Phase 3  archetype waves (continuous)
```

## Verification matrix

```powershell
# Phase 0 — audit findings exist
Test-Path docs/superpowers/audits/2026-05-14-rust-engine-gap-rebaseline.md

# Phase 1 — variant-coverage lint
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl_eval_arm_coverage

# Phase 1 — ignored test count is dropping
git grep -c '#\[ignore' code/digimon-engine/tests | awk -F: '{s+=$2} END {print s}'

# Phase 2 — substrate edges retired
cargo test --manifest-path code\digimon-engine\Cargo.toml --test cards_behavioral
cargo test --manifest-path code\digimon-engine\Cargo.toml --test timing_dispatch

# Phase 3 — RL parity preserved
$env:DIGIMON_BACKEND='rust'; python -m pytest code\tests\rl -v
$env:DIGIMON_BACKEND='rust'; python -m pytest code\engine_py_legacy\tests\engine\test_rust_backend_parity.py -v
```

If any phase changes `ACTION_SPACE_SIZE`, observation shape, or PyO3 exports,
stop and open a separate action/tensor contract spec before merging.

## Self-review

- **Reality grounded.** Every claim about substrate state cites file:line in the
  current engine source. The two predecessor specs (2026-04-29, 2026-05-03)
  were drafted before Tracks A–J landed; this plan reads as the post-substrate
  view.
- **Leverage-first.** Phase 1 is sequenced first after audit because it
  resolves ~58% of active gap tags at low risk. Phase 2 is narrow by design.
- **Invariant-bearing.** The variant-coverage lint introduced in Phase 1 is the
  long-term insurance against the next 150 silent gaps.
- **No-approximations preserved.** No phase introduces hidden auto-selection,
  raw-Rust escape hatches as a permanent state, or UI-only rules handling.
- **No contract churn.** No phase changes ACTION_SPACE_SIZE, tensor shape, or
  PyO3 exports unless a separate contract spec lands first.
- **DCGO posture.** With the submodule initialized, the four substrate items
  where DCGO is the behavioral tiebreaker (G-OPT-TRIGGERED, attack pipeline
  ordering, declarative keyword carrier semantics, option lifecycle disposition)
  can cite `DCGO/Assets/Scripts/CardEffect/{SET}/{COLOR}/{CARD_ID}.cs` as ground
  truth.
