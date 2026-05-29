## 1. Full-Pool Baseline

- [x] 1.1 Re-run `python code/tools/resolve_deck.py "Mastemon (Tribal)" --json` with UTF-8 output and confirm the 93-card pool, 55 decklists, and 20-card best-deck unique set.
- [x] 1.2 Produce a coverage snapshot for the full pool: production YAML count, behavioral-test count, best-deck coverage, missing YAML cards, and missing-test cards.
- [x] 1.3 Record baseline `ACTION_SPACE_SIZE`, `standard_lite_v2`, and `standard_compact_v1` metadata before substrate work begins.
- [x] 1.4 Update Mastemon QA notes with the full-pool baseline and the planned substrate blocker groups.

## 2. Source-Placement Observer Substrate

- [x] 2.1 Add failing DSL/engine tests proving an effect-created `[CS]` Digimon source placement emits an observer event with placed-card and host-permanent context.
- [x] 2.2 Add negative tests proving normal digivolution/setup source presence does not satisfy the effect-created source-placement predicate.
- [x] 2.3 Implement source-placement event dispatch in the shared placement helpers and expose DSL timing/predicates for placed-card trait and host context.
- [x] 2.4 Verify pending-selection follow-up bodies work for CS observers without adding action IDs.
- [x] 2.5 Implement and test the CS source-placement cards in the Mastemon pool that depend on this substrate.

## 3. Choice-Shaped Security Cost Substrate

- [x] 3.1 Add failing tests for a top-or-bottom security trash cost that offers both choices and gates the body after successful payment.
- [x] 3.2 Add tests for one-card, empty-security, declined-cost, and prevented-cost paths.
- [x] 3.3 Implement DSL schema, compile, validation, lowering, and engine helper support for the choice-shaped security cost.
- [x] 3.4 Implement and test `BT15-038` and `BT15-042` using the new cost gate.

## 4. Aggregate Play-Cost Budget Selection

- [x] 4.1 Add failing tests for selecting multiple trash cards whose total play cost is within a budget, including stop-early behavior.
- [x] 4.2 Add tests proving candidates over the remaining budget are masked out and unselected cards remain in their origin zone.
- [x] 4.3 Implement DSL and engine support for visible-zone aggregate play-cost budget selection and batch free play from true origins.
- [x] 4.4 Implement and test `EX8-064` Boltboutamon's trash play budget branch.

## 5. Conditional Locks And Identity Mutation

- [x] 5.1 Add failing tests for Venusmon-style conditional attack and timing suppression keyed to opponent Digimon with Security Attack.
- [x] 5.2 Implement the smallest reusable modifier/filter support needed for conditional attack-target and timing suppression.
- [x] 5.3 Implement and test `BT10-042` Venusmon, including Security Attack -1 and the matching lock behavior.
- [x] 5.4 Add failing tests for temporary original-name mutation and expiry.
- [x] 5.5 Implement the narrow temporary original-name mutation support needed by KingSukamon-style effects.
- [x] 5.6 Implement and test `BT11-043` KingSukamon, or explicitly record any remaining non-name blocker found during authoring.

## 6. Security Follow-Up And Existing-Pattern Cards

- [x] 6.1 Confirm `on_discard_security` can activate a card's `[Main]` body from effect-trashed security without treating the Option as normally used from hand.
- [x] 6.2 Promote `BT13-106` from behavioral-test-only coverage to production YAML plus passing tests.
- [x] 6.3 Implement and test remaining high-frequency security/search/reveal cards that use existing substrate, including `BT14-003`, `BT13-034`, `BT1-087`, `ST20-05`, `EX6-030`, `BT14-084`, and `BT15-034`.
- [x] 6.4 Implement and test remaining keyword/static/inherited cards that use existing substrate, including `ST10-02`, `ST10-12`, `BT8-071`, `BT11-080`, `BT13-003`, `EX6-016`, `BT7-032`, `BT8-077`, `BT8-035`, `BT21-004`, `BT4-084`, `BT4-111`, and `EX2-003`.

## 7. Remaining Full-Pool Card Batches

- [x] 7.1 Implement and test remaining mid-frequency cards after substrate closure: `BT22-031`, `BT22-046`, `BT22-056`, `ST10-14`, `ST10-06`, `BT23-027`, and `EX4-005`.
- [x] 7.2 Implement and test remaining low-frequency Option and Tamer cards: `BT6-100`, `P-225`, `BT22-101`, `BT6-089`, `BT10-101`, `BT14-093`, and `EX7-064`.
- [x] 7.3 Implement and test remaining low-frequency Digimon cards: `P-221`, `BT16-088`, `EX10-051`, `BT18-082`, `BT23-037`, `BT14-102`, `BT14-037`, `BT8-082`, and `LM-043`.
- [ ] 7.4 Re-scan the resolved pool and close any remaining production YAML or behavioral-test gaps, or record explicit accepted exclusions.

## 8. Verification And Gap Hygiene

- [ ] 8.1 Run focused DSL tests for every new substrate area: source-placement observers, choice-shaped security costs, aggregate play-cost budget selection, conditional locks, and temporary identity mutation.
- [ ] 8.2 Run focused `cards_behavioral` tests for every newly authored Mastemon full-pool card.
- [ ] 8.3 Run broader regression suites covering `cards_behavioral`, `dsl`, `dna_digivolve`, `digivolve`, `security_card_effects`, and mask/tensor contract checks touched by this change.
- [ ] 8.4 Re-check `ACTION_SPACE_SIZE` and active tensor layout metadata against the baseline from task 1.3.
- [ ] 8.5 Update `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md`, `qa/archetype-qa/engine-gaps.md`, and `qa/archetype-qa/mastemon-tribal/readiness.md` for every closed or deferred gap.
- [ ] 8.6 Run `openspec validate unblock-mastemon-full-pool-rust-dsl --strict` and resolve validation issues.
