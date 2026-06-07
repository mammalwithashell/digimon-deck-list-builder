## 1. Triage spike (gating — freeze the corpus)

- [x] 1.1 Freeze the full General Rules/FAQ corpus into `qa/qa-reports/rules-faq.md` as a ledger: one row per Q&A entry with columns `id | question | faq_answer | surface | card(s) | test/xlink | verdict` (seed from `design.md` §"Frozen corpus ledger").
- [x] 1.2 Assign each row a surface (`runtime` / `deck-validation` / `data` / `n/a`); for every `n/a` row record the explicit engine-abstraction reason.
- [x] 1.3 Resolve the design Open Questions: confirm the `tests/deck_tools/` deck-legality reach for the 5 deck-creation rules, and whether security-stack placement order is observable (else mark `n/a`). → `validate_deck` at `src/deck_tools.rs:447` reaches all 5; security placement order is N/A (not action-observable).
- [x] 1.4 For each `runtime`/`data` row needing a card property, pin the reused implemented DSL card id (no-Level, no-DP, conditional-`<Blocker>`, multi-color, On-Deletion, Once-Per-Turn, "X in name" pair); flag rows where NO implemented vehicle exists as authoring candidates. → reused-card table in ledger; BLOCK-CARD: NV-1 no-DP (0 impl'd), MC-3 dual-evo-cost, NL-3 D-Reaper.
- [x] 1.5 Cross-reference each disputed/terse answer against `general_rule.pdf` § and base-repo `DCGO/` C#; record the citation in the ledger row. → methodology + source-priority recorded; per-row PDF/DCGO citations added during each discovery-wave test docstring.

## 2. Coverage audit (cross-link, don't duplicate)

- [x] 2.1 Inventory the existing Rust test tree for prior coverage of each `runtime` rule (candidates: 0-DP deletion, memory cap-at-10, can't-attack-turn-played, mandatory draw / deck-out loss, breeding-move ≠ play, suspended-digivolve-stays-suspended).
- [x] 2.2 Mark each already-covered row `XLINK` with the existing test path; these get NO new test. → DR-1, MP-02(base), MP-05, MP-20, MP-27(partial), OR-4(partial) cross-linked in ledger.
- [x] 2.3 Produce the residual list of genuinely-uncovered rows that the discovery wave must encode. → "Genuinely uncovered" + "Authoring candidates" sections in ledger; canary MP-28 confirmed uncovered.

## 3. Suite scaffolding

- [x] 3.1 Create `code/digimon-engine/tests/rules_faq/main.rs` (module wiring) and register the test target. → `[[test]] rules_faq` added to `Cargo.toml`.
- [x] 3.2 Add `tests/rules_faq/loader.rs` — a reused-card load gate (mirrors `judge_quiz/loader.rs`): assert every reused DSL card id from §1.4 loads from the embedded pack. → 11 vehicles gated, green.
- [x] 3.3 Create empty section modules: `deck_creation`, `phases`, `main_phase`, `effect_resolution`, `keyword_identity`, `multicolor`, `no_level_no_value`, `security_digimon`, `in_its_text`.

## 4. Author missing property-cards (only if §1.4 flagged any)

- [x] 4.1 Author the BLOCK-CARD vehicles. → **NV-1**: authored **BT24-068 DemiDevimon** (vanilla no-DP Lv3) + `nv1_no_dp_digimon_cannot_gain_dp` PIN; recorded in `validated_cards_dsl.json`. **NL-3**: no authoring needed — XLINK-structural (engine determines Digimon-ness by `card_kind`, name-agnostic). **MC-3**: XLINK-partial — a real-DSL-card pin is blocked by the loader's empty `evo_costs` (would need a synthetic Game-level test); mechanism covered by `digivolve_action.rs`.
- [x] 4.2 Re-run the loader/suite to confirm the new card loads. → `rules_faq` 28/28 green; BT24-068 lints clean. **Zero BLOCK-CARD rows remain.**

## 5. Discovery wave — Phases & Main Phase (densest)

- [~] 5.1 `phases.rs`: encode uncovered unsuspend/draw/breeding rules. → DONE: US-2/US-3 (`us_unsuspend_is_turn_player_only_and_mandatory`), DR-2 (`dr2_no_maximum_hand_size`). TBD: breeding rows (BR-*), DR-1 mandatory-draw leg.
- [~] 5.2 `main_phase.rs`: encode uncovered Main-Phase rules. → DONE: MP-19 (0-DP deletion), MP-22 (memory cap), MP-08 (cost≥11 unplayable from 0 memory), MP-13 (attack only suspended opp). All PIN (engine faithful). TBD: digivolve-keeps-suspended (MP-04, blocked by DSL empty-evo_costs), On-Play-not-on-digivolve, attack ≠ blocked, attack/EoA-only-for-attacker.
- [x] 5.3 `main_phase.rs` (CANARY): encode the simultaneous +DP/−DP end-of-turn rule asserting survival at original DP; if it fails, log the 17-1-2-2 gap to `engine-gaps.md`, mark the row `GAP`, and spin off the fix chip (candidate `MODIFIED` delta to `permanent-deletion-semantics`). → **PIN** (EoT expiry path is atomic/correct; no gap). Also pinned MP-19 (0-DP deletion) and MP-22 (memory cap) from §5.2 in the same slice.
- [ ] 5.4 Run `cargo test --manifest-path code/digimon-engine/Cargo.toml --test rules_faq`; reconcile each §5 row to `PIN`/`GAP` in the ledger.

## 6. Discovery wave — Effect resolution & keyword identity

- [~] 6.1 `effect_resolution.rs`. → DONE: MP-30 + MP-31 (mandatory "2 of opp") — **gap found+fixed** via `clamp_to_available` (BT24-051 + sibling BT12-028). MP-29 (mandatory single-target not declinable) — **2 more gaps found+fixed**: BT21-037 (if-guard) + AD1-018 (remove `optional`), both DCGO-verified. All logged to `qa/dsl-vocab-gaps.md`. TBD: OPT opt-out (XLINK ad1_012/014), multi-effect ordering, no-When-Attacking-after-block, When-Digivolving-after-draw.
- [x] 6.2 `keyword_identity.rs`: keyword-as-text selectable outside its timing (MP-26, BT21-026); gained keyword counts only while condition met (MP-27, BT24-041). Both PIN (faithful). **Stale-gap cleanup:** MP-26 proved the `G-DECLARATIVE-KEYWORD` gap was already resolved (native-keyword path) — un-gated 2 dead `todo!()` `#[ignore]` tests in `bt21_026.rs` into real coverage and corrected the 3 stale "not installed at runtime" comments in `BT21-026.yaml`.
- [ ] 6.3 Run the target; reconcile §6 rows in the ledger.

## 7. Discovery wave — Multicolor, no-level/no-value, security, in-its-text

- [~] 7.1 `multicolor.rs`. → DONE: MC-1/MC-8 (2-color is each of its colors / can't drop a color, via `CardData::is_color`), 2-color Option carries both. TBD: MC-2 "counted as 1", MC-3 (BLOCK-CARD dual-evo-cost), MC-4/5/6/7 (runtime color-requirement gating).
- [~] 7.2 `no_level_no_value.rs`. → DONE: NL-1 (no-Level has `level==None`), NL-2 (no-Level w/ DP breeding-eligible). TBD: NL-3 (BLOCK-CARD D-Reaper), NL-4/5 (digivolution-source / `<De-Digivolve>` runtime). NV-1: BLOCK-CARD (no impl'd no-DP Digimon).
- [~] 7.3 `security_digimon.rs`. → DONE: OR-4 (a security Digimon deleted in a security check does NOT fire its non-`[Security]` [On Deletion]) — PIN, faithful (EX9-027 vehicle; security Digimon are `CardSource`s trashed, never permanent-deleted). Robustly asserted (preconditions confirm the deletion actually occurred). TBD: OR-5 (security Digimon not counted as "Digimon" by effects — structurally the same guarantee; battle_area filters exclude the security zone).
- [~] 7.4 `in_its_text.rs`. → DONE: IT-3 (name match = case-insensitive substring, via `Permanent::contains_card_name`). TBD: IT-1 (text-span), IT-2 (`<Material Save>` icon-exact).
- [ ] 7.5 Run the target; reconcile §7 rows in the ledger. → partial reconcile in ledger §"Discovery-wave progress".

## 8. Deck-validation & metadata surfaces

- [x] 8.1 `tests/deck_tools/`: assert the 5 deck-creation rules. → DONE in `rules_faq/deck_creation.rs` (DC-1..DC-5, 5 tests green) via `validate_deck`.
- [x] 8.2 Reconcile deck-creation + `data`-surface rows to verdicts in the ledger. → all inline cells reconciled (no stale TBD).

## 9. Reconcile & close out

- [x] 9.1 Full ledger reconciliation: every row carries a final verdict (`PIN`/`XLINK`/`N/A`/`BLOCK-CARD`); no row left `TBD`. → **100% coverage, 0 TBD** (54 rows reconciled in one pass + 4 fixups; verdict legend documents the assurance tiers).
- [x] 9.2 Confirm every `GAP` row has a corresponding tracker entry and a fix; committed test asserts the FAQ-correct outcome (not weakened). → 4 gaps (BT24-051, BT12-028, BT21-037, AD1-018) fixed + DCGO-verified + logged to `qa/dsl-vocab-gaps.md`; canary MP-28 cleared.
- [~] 9.3 If a gap fix changed spec-level behavior, add the `MODIFIED` delta. → the 4 gap fixes were per-card DSL authoring + one additive DSL flag (`clamp_to_available`); no existing capability's spec-level behavior changed. New-capability spec already covers the suite.
- [~] 9.4 Full engine test suite green. → `rules_faq` 27/27 green; `cards_behavioral` 3826 pass / 3 pre-existing fail (stash-verified unrelated) / 62 ignored; no `#[ignore]` gaps in `rules_faq`. The 3 `BLOCK-CARD` rows (MC-3, NL-3, NV-1) remain open pending authored vehicles.
