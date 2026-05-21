# Phase 0 — BG Imperial gap re-audit (verified 2026-05-20)

Input contract for Phases 2–5. Every `#[ignore]`'d test and cited gap ID across
the 24 BG Imperial cards, verified against current `code/digimon-dsl/src/` and
`code/digimon-engine/src/`.

## Headline

Of ~15 distinct gap IDs cited by BG Imperial card YAML / tests, **only 6 are
genuine substrate gaps** — all small/medium DSL-side leaves or verbs. The rest
are *stale-ignore* (primitive already shipped; card needs re-authoring) or
*stale-verdict* (card already authored; tracker not refreshed). **Tier 3
(DNA-origin material/result payloads) is not needed by any BG Imperial card.**

## Audit correction (2026-05-21)

The genuine-substrate table below initially listed 9 gaps. During implementation,
a second review found **4 of them (S1, S4, S5, S6) were not genuine gaps** — the
audit checked whether each predicate *name* existed, not whether an equivalent
*capability* existed. Pre-existing engine vocabulary already covers them:

- **S1 `stack_size_lte_source`** → `materials_count_lte: { formula: { source_material_count: {} } }`.
- **S4 `carrier_has_keyword`** → `has_keyword` already resolves against the carrier
  permanent for inherited clauses.
- **S5 `is_carrier_of_source`** → `kind: aura` with `scope: inherited` + `target: {}`.
- **S6 `self_digivolution_contains_trait`** → `source_permanent_trait_has` (EX1-014).

Only **S2, S3, S7, S8, S9** were genuine and landed. Consequently EX1-014 and
BT3-002 were already correct on the base engine (stale verdict/comments), and
BT16-027 is implementable via the existing formula path. Treat the S-table below
as superseded by this correction.

## Per-card classification

| Card | Verdict | `#[ignore]` tests | Cited gap(s) | Classification |
|---|---|---|---|---|
| BT3-002 | PARTIAL | 0 | G-DSL-CARRIER-HAS-KEYWORD (YAML over-fire workaround) | **genuine-gap** (Tier 1) |
| BT12-002 | IMPLEMENTED | 0 | — | done |
| P-117 | IMPLEMENTED | 0 | stale doc comment (G-BEFORE-PAY-COST-DIGIVOLVE-TARGET — resolved) | stale-comment |
| EX1-014 | PARTIAL | 0 | G-DSL-AURA-TARGET-SOURCE-PERMANENT, G-DSL-SELF-DIGIVOLUTION-CONTAINS-TRAIT | **genuine-gap** (Tier 1) |
| ST9-09 | IMPLEMENTED | 0 | — | done |
| BT12-022 | IMPLEMENTED | 0 | — | done |
| BT12-050 | IMPLEMENTED | 0 | — | done |
| BT12-021 | IMPLEMENTED | 0 | — | done |
| BT12-047 | IMPLEMENTED | 0 | — | done |
| ST9-05 | IMPLEMENTED | 0 | — | done |
| BT12-028 | BLOCKED | 0 | none — YAML fully authored (Track E verbs) | **stale-verdict** → confirm IMPLEMENTED |
| BT16-025 | PARTIAL | 2 | G-DSL-STACK-SIZE-LTE-SOURCE; G-DSL-EFFECT-SUSPENDED-RESULT | **genuine-gap** (Tier 1) |
| ST9-06 | BLOCKED | 0 | none — YAML fully authored (`select_own_sources`/`play_selected_sources_free`) | **stale-verdict** → confirm IMPLEMENTED |
| BT12-031 | PARTIAL | 5 | G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME; G-DSL-SELF-COLOR-COUNT-GTE | **stale-ignore** (re-author; tiny residual on min:0 alt-cost shape) |
| BT16-027 | PARTIAL | 2 | G-PRED-STACK-SIZE-LTE-SOURCE | **genuine-gap** (Tier 1, same leaf as BT16-025) |
| BT16-028 | PARTIAL | 6 | G-IS-EFFECT-INITIATED (×3, resolved); test-side issue (×3) | **stale-ignore** (re-author + fix tests) |
| BT20-020 | PARTIAL | 4 | G-OPT-TRIGGERED (resolved); G-FORMULA-SOURCE-DP (resolved); test-side (×2) | **stale-ignore** (re-author + fix tests) |
| BT17-077 | PARTIAL | 0 | G-RETURN-ALL-TRASH (partial); G-ANY-RETURNED-CARD; player-choice trash | **genuine-gap** (Tier 2) |
| BT3-093 | IMPLEMENTED | 0 | — | done |
| LM-030 | PARTIAL | 4 | G-ZONE-SELECTED-TRASH-TO-DECK-TOP; G-PRED-DP-LTE; G-OPTIONAL-SELECTION-CONTINUE-TAIL | **genuine-gap** (Tier 2) + 2 likely-resolved (verify) |
| BT17-097 | PARTIAL | 1 | G-DSL-UNION-PLAY-FREE (resolved — `play_union_bound_free`) | **stale-ignore** (re-author from zone-choice workaround) |
| BT3-103 | PARTIAL | 5 | G-COST-REDUCE-ALLY-DIGIVOLVE family | **out-of-scope** (design D1 non-goal) |
| BT16-085 | PARTIAL | 2 | G-EVENT-CARD-COLOR-IS; G-SELECT-OPPONENT-SOURCES | **genuine-gap** (Tier 1 + Tier 2) |
| BT20-016 | PARTIAL | 4 | G-EVENT-TARGET-OWNER (×3); G-DECLARATIVE-KEYWORD (×1) | **stale-ignore** (re-author) + 1 uncertain |

## Genuine substrate gaps (the real Tier 1 + Tier 2 work)

| ID | Gap | Tier | Size | Blocks | Evidence verified |
|---|---|---|---|---|---|
| S1 | `stack_size_lte_source` predicate | 1 | S | BT16-025 1a, BT16-027 | no `stack_size_lte_source` in source |
| S2 | `effect_suspended_any_opponent_digimon` result predicate | 1 | S | BT16-025 clause 2 | only `_any_own_digimon` exists |
| S3 | `event_card_color_has` predicate (has-semantics; `_only`/`_count` exist) | 1 | S | BT16-085 clause 1 color gate | no `_has` variant in predicate.rs |
| S4 | `carrier_has_keyword` predicate (inherited-clause carrier keyword) | 1 | S | BT3-002 | no source hits |
| S5 | `is_carrier_of_source` aura target-filter leaf | 1 | S | EX1-014 | no source hits |
| S6 | `self_digivolution_contains_trait` predicate | 1 | S | EX1-014 | spec field exists, no runtime eval |
| S7 | `select_opponent_sources` verb | 2 | M | BT16-085 DNA branch | no source hits; mirror of `select_own_sources` |
| S8 | Selected-trash-card → deck-top movement verb | 2 | S/M | LM-030 clause B | `DeckTop` dest enum exists; no selected-trash-to-top step |
| S9 | `any_returned_card` result-set predicate + BT17-077 player-choice-of-trash | 2 | M | BT17-077 clause 1c | no result-set predicate |

## Stale-ignore — re-author only (primitive already shipped)

- **BT12-031** — `self_color_count_gte` exists (compile.rs:572, predicate.rs:110, eval 1504/1927); `select_own_sources` `filter:` + `binding_present`/`binding_absent` exist. Tiny residual possible on the `min:0` alt-cost source shape — surface a narrow gap only if re-authoring proves it.
- **BT16-028** — `event_is_effect_initiated` exists (predicate.rs:227). 3 tests are pure test-side setup issues; YAML clauses already ship.
- **BT20-020** — `source_dp` formula exists (formula.rs:33); `G-OPT-TRIGGERED` resolved Phase 2 Track C. 2 tests are `from{}.all_of` test-side nesting issues.
- **BT17-097** — `play_union_bound_free` (step.rs:322, PUPPETS-G014) + `select_union_zone` exist; re-author from the explicit zone-choice workaround to native union play with auto-collapse.
- **BT20-016** — Clause 2: `replacement_subject_is_mine` exists (predicate.rs:265); RK-G004 proved the non-cancelling cross-permanent would-leave `kind: replacement` pattern. Re-author the `bt20_016_dna_on_deletion` raw_rust placeholder to a `kind: replacement` clause.
- **P-117** — only a stale top-of-file doc comment; G-BEFORE-PAY-COST-DIGIVOLVE-TARGET resolved Phase 2 Track H.

## Stale-verdict — confirm with a test run

- **BT12-028** — YAML fully authored (`trash_top_n_digivolution_cards_of_each`, `dna_origin`, `select_count_capped_multi`, `CannotAttack`, inherited memory). Expected → IMPLEMENTED.
- **ST9-06** — YAML fully authored (`select_own_sources from: source` + `play_selected_sources_free`). Expected → IMPLEMENTED.

## Resolved design Open Questions

1. **DNA-origin material/result payloads (Tier 3):** NOT needed. No BG Imperial card requires more than the basic `dna_origin` predicate / `on_dna_digivolve` timing — BT12-028 uses `if { dna_origin: true }`, BT16-025 uses `on_dna_digivolve` timing, BT16-085 uses `dna_origin`. **Tier 3 / tasks 4.1–4.3 are dropped; task 4.4 records this finding.**
2. **G-OPTIONAL-SELECTION-CONTINUE-TAIL (LM-030):** Phase 2 Track H landed a `select_trash` declined-optional outer-tail continuation. Treated as likely-resolved; confirm during LM-030 re-authoring.
3. **G-DECLARATIVE-KEYWORD (BT20-016 inherited `<Security A. +1>`):** UNCERTAIN. The YAML claims `EffectTiming::Declarative` is never enqueued so the inherited `grant_keyword` modifier never installs. Yet `kind: partition` declarative clauses ship (BT16-025), and DNA Omnimon completion landed `Modifiers::granted_security_attack_keyword_bonus`. This is the **one possible engine-level item** — needs a focused engine check before classification is final.

## Scope delta vs. the original task list

- **Tier 3 (tasks 19–21) dropped** — no BG card needs it.
- **Tier 1** is 6 small DSL predicate leaves (S1–S6), not engine work.
- **Tier 2** is 3 verbs/predicates (S7–S9).
- **One open engine question** — G-DECLARATIVE-KEYWORD — to resolve before Tier 1 starts.
- Card re-authoring is the bulk of the remaining effort, and ~half of it is verdict bumps / test-side fixes rather than new clauses.

## Closeout (2026-05-21)

Final BG Imperial tally: **22 IMPLEMENTED / 1 PARTIAL / 1 out-of-scope** — from
the 9 / 13 / 2 baseline.

- **IMPLEMENTED (22):** the 9 already-implemented cards + BT16-025, BT16-027,
  BT16-085, BT17-077, LM-030, BT3-002, EX1-014, BT16-028, BT20-020, BT17-097,
  BT20-016, BT12-028, ST9-06 (13 re-authored/verified).
- **PARTIAL (1):** BT12-031 — clause 1b implemented; Step C alt-cost blocked on
  genuine engine gap `G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME` (return a single
  named digivolution-stack source card to hand — no `EffectContext` method
  exists). Filed in `qa/dsl-vocab-gaps.md` with corrected engine-gap diagnosis.
- **Out-of-scope (1):** BT3-103 — `G-COST-REDUCE-ALLY-DIGIVOLVE`, deferred per
  design D1.

Substrate landed: 5 genuine gaps (`effect_suspended_any_opponent_digimon`,
`event_card_color_has`, `select_opponent_sources`, `move_trash_card_to_deck_top`,
`returned_card_matching`). 4 originally-flagged gaps were found redundant and
reverted (see "Audit correction" above). `G-DECLARATIVE-KEYWORD` open question
resolved — declarative inherited keyword grants fire correctly (stale claim).

Full engine suite: 2557 passed, 4 pre-existing `bt17_095`/`bt17_102` failures
(verified on baseline), no regressions. No `ACTION_SPACE_SIZE`/tensor changes.

## Final update (2026-05-21) — both follow-up gaps closed

Per user direction the two scoped engine gaps were then implemented:

- **G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME** — `return_card_source_to_hand` +
  `return_selected_sources_to_hand` verb → BT12-031 Step C authored.
- **G-COST-REDUCE-ALLY-DIGIVOLVE** — player-scoped one-shot paid future-digivolve
  cost reducer (`player_cost_reducer.rs`, `arm_digivolve_cost_reducer`) →
  BT3-103 Clause 0 authored.

BT12-031 and BT3-103 are now both IMPLEMENTED.

**Final BG Imperial tally: 24 / 24 IMPLEMENTED** (0 PARTIAL, 0 BLOCKED) — from the
9 / 13 / 2 baseline. Full engine suite: 2564 passed, only the 4 pre-existing
`bt17_095` / `bt17_102` failures (unrelated, on baseline), no regressions.
