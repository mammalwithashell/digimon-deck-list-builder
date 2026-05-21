## 1. Phase 0 — Gating re-audit (no engine/card code)

- [x] 1.1 Enumerate every `#[ignore]`'d test across the 24 BG Imperial card test files (bt3, bt12, bt16, bt17, bt20, st9, ex1, lm, p) and record the cited gap ID for each
- [x] 1.2 For each cited gap ID, verify against current `code/digimon-dsl/src/` and `code/digimon-engine/src/` whether the primitive exists; classify each clause as `stale-ignore`, `genuine-gap`, or `out-of-scope`
- [x] 1.3 Resolve design Open Questions: confirm whether any BG card needs DNA-origin material/result payloads beyond basic `dna_origin`; confirm whether `G-OPTIONAL-SELECTION-CONTINUE-TAIL` (ST9-06/LM-030) is closed by Track H; determine whether `G-DECLARATIVE-KEYWORD` (BT20-016) is a stale ignore or a real firing bug
- [x] 1.4 Update `qa/archetype-qa/dsl/bg-imperial-cross-archetype-gaps-2026-05-03.md` and the BG Imperial entries of `qa/dsl-vocab-gaps.md` to the verified state; move confirmed-resolved primitives to `qa/resolved-gaps.md` with passing `cargo test` commands
- [x] 1.5 Record the per-card classification table in the change folder as the input contract for Phases 2–5

## 2. Tier 1 — DSL-only predicate leaves

- [x] 2.1 Add `stack_size_lte_source` boolean predicate: `PredicateSpec` field + `CompiledPredicate` field + `compile.rs` wiring + `validator.rs` registration + runtime eval branch in `code/digimon-engine/src/dsl_cards/predicate.rs` comparing candidate vs source `card_sources.len()`
- [x] 2.2 Add `carrier_has_keyword` predicate that resolves the carrier handle from the inherited-effect dispatch context and calls `Game::has_keyword`
- [x] 2.3 Add `self_digivolution_contains_trait` predicate evaluating the source permanent's digivolution stack via `Permanent`/`CardSource` trait helpers
- [x] 2.4 Add `is_carrier_of_source` aura target-filter leaf so `kind: aura` can restrict the target set to the source carrier; wire it into `lower_aura`
- [x] 2.5 Add the opponent/any-scoped `effect_suspended` predicate variant alongside `effect_suspended_any_own_digimon` (`effect_suspended_any_opponent_digimon`)
- [x] 2.6 Add DSL-parse/compile unit tests and engine eval tests for each new predicate (13 tests in `code/digimon-engine/tests/dsl/group7_predicate_batch.rs`)
- [x] 2.7 Run full `cargo test` (digimon-dsl 43 pass; engine `--test dsl` 624 pass; full engine suite 2526 pass — 4 pre-existing `bt17_095`/`bt17_102` failures verified on baseline stash, no regression); resolved-gaps.md moves folded into final sweep 6.4

## 3. Tier 2 — Engine-touching DSL verbs

- [x] 3.1 Add `select_opponent_sources` step: opponent-side mirror of `select_own_sources` (exact-N / up-to-N, PASS-after-minimum, `filter:`, `target:`, stable refs); new `EffectContext::select_opponent_sources` helper
- [x] 3.2 Add `move_trash_card_to_deck_top` verb — single `select_trash`-bound card → owner's deck top; owner routing confirmed
- [x] 3.3 Add `returned_card_matching` filtered result predicate (distinct field from the bare-bool `any_returned_card` alias); result log records returned card handles, `card_data_for_handle` resolves identity zone-agnostically
- [x] 3.4 BT17-077 player-choice-of-trash — verified `select_effect_choice` + `if` already composes with `return_all_trash_to_deck_bottom: { of: you|opponent }`; no new gap
- [x] 3.5 Add parse/compile/eval tests (DSL `parse_source_selection_steps.rs` + `parse_zone_movement_steps.rs`; engine `dsl/phase2g_select_sources.rs` + `dsl/zone_movement_verbs.rs`)
- [x] 3.6 Full engine + DSL suites green (engine `--test dsl` 633 pass; full engine suite 2526 pass, only 4 pre-existing `bt17` failures); tracker sweep folded into 6.4

## 4. Tier 3 — DNA-origin event payloads (DROPPED per Phase 0)

- [x] 4.1 N/A — Phase 0 found no BG Imperial card needs DNA-origin material/result payloads
- [x] 4.2 N/A — see 4.1
- [x] 4.3 N/A — see 4.1
- [x] 4.4 Finding recorded in `phase-0-audit.md` § "Resolved design Open Questions" Q1: every BG card needs only basic `dna_origin` / `on_dna_digivolve` (BT12-028, BT16-025, BT16-085 verified). Tier 3 dropped; no dependent clause omitted

## 5. Card re-authoring sweep

> Done directly via dispatched TDD agents (the `batch-implement-cards-rust-dsl`
> skill's AUDIT mode is audit-only and would not modify shipping YAML).

- [x] 5.1 Re-authored stale-ignore cards: BT16-028 (event_is_effect_initiated + 3 test-side fixes), BT20-020 (source_dp formula + G-OPT-TRIGGERED stale), BT17-097 (native select_union_zone + play_union_bound_free), BT20-016 (raw_rust → kind: replacement; G-DECLARATIVE-KEYWORD confirmed stale)
- [x] 5.2 Re-authored Tier-1/predicate-consumer cards: BT16-025 (IMPLEMENTED), BT16-027 (IMPLEMENTED), BT3-002 (verified already correct), EX1-014 (verified already correct)
- [x] 5.3 Re-authored Tier-2-verb consumers: BT16-085, BT17-077, LM-030 (IMPLEMENTED); BT12-028, ST9-06 (stale BLOCKED verdicts corrected → IMPLEMENTED); BT17-097 (IMPLEMENTED); BT12-031 PARTIAL — Step C blocked on genuine engine gap G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME (filed)
- [x] 5.4 N/A — Tier 3 was dropped (Phase 0)
- [x] 5.5 BT3-103 Clause 0 left omitted; its `#[ignore]` tests still cite out-of-scope `G-COST-REDUCE-ALLY-DIGIVOLVE`
- [x] 5.6 `qa/qa-reports/validated_cards_dsl.json` updated for all 15 touched cards (14 IMPLEMENTED, BT12-031 PARTIAL)

## 6. Verification and closeout

- [x] 6.1 Full `cargo test` engine suite green — `cards_behavioral` 2526 pass; only 4 pre-existing `bt17_095`/`bt17_102` failures (verified identical on baseline stash); all other binaries green
- [x] 6.2 Confirmed no `ACTION_SPACE_SIZE` / tensor-contract change — no `tensor.rs` or `action/` files modified by Tier 1 or Tier 2
- [x] 6.3 Full engine suite green — 2557 passed, 198 ignored, only the 4 pre-existing `bt17_095`/`bt17_102` failures. Every BG Imperial behavioral test passes; the only BG `#[ignore]`'d tests remaining are BT3-103 Clause 0 (out of scope) and BT12-031 Step C (genuine engine gap, filed)
- [x] 6.4 Tracker sweep: `qa/resolved-gaps.md` § "BG Imperial substrate closeout" added (9 gaps + test commands); `qa/dsl-vocab-gaps.md` BG header marks the 9 gaps closed; `bg-imperial-cross-archetype-gaps-2026-05-03.md` has the substrate-landed note. `docs/RUST_ENGINE_GAPS.md` untouched — all 9 gaps are DSL-vocab, not engine-gap entries
- [x] 6.5 Final BG Imperial tally: **22 IMPLEMENTED / 1 PARTIAL (BT12-031) / 1 out-of-scope (BT3-103)** — up from the 9 IMPLEMENTED / 13 PARTIAL / 2 BLOCKED baseline. Recorded in `phase-0-audit.md` closeout note
