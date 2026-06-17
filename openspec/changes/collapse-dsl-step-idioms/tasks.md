# Tasks — collapse DSL step idioms

## 1. Generalize the `then:` action-tail (do first — lowest risk, highest reach)
- [ ] 1.1 Add `then: Vec<StepSpec>` to the field/zone select arg structs: `SelectFieldArgs` (select_*_permanent), `SelectZoneArgs` (select_hand/select_trash/select_security). Mirror the existing seven-struct lowering exactly (tail runs with the selection binding in scope).
- [ ] 1.2 Compile + lower the new tails; behavioral test per newly-supporting selector (select+delete, select+suspend, select_trash+add_to_hand) asserting parity with the longhand form.
- [ ] 1.3 Confirm tail steps that require choices still install their own `pending_selection` (RL-visibility test).
- [ ] 1.4 (Optional) Add thin aliases `discard_from_hand` / `recover_from_trash` ONLY if they materially out-read the `then:` form; otherwise skip to avoid re-introducing sprawl.

## 2. `reveal_search` composite
- [ ] 2.1 Make `add_to_hand_from_reveal` (+ `trash_from_reveal`/`return_to_deck_from_reveal` siblings) accept a multi-card `CardList` (`max: N`); fix EX5-015's single-card limitation.
- [ ] 2.2 Add `reveal_search { of, count, buckets: [{filter, to: hand|trash|deck, max, optional, prompt}...], remainder: top|bottom|choose }`; lower to reveal_top_deck → per-bucket select → per-bucket move → place_remainder.
- [ ] 2.3 Port the existing reveal-idiom corner-case tests (empty pool, all-optional declined, remainder ordering, no_duplicate_cards across buckets) onto `reveal_search`.
- [ ] 2.4 Migrate a few representative searcher cards (e.g. BT9-092, EX5-015) to `reveal_search` as parity fixtures.

## 3. Security-placement overhaul
- [ ] 3.1 Add `StackPosition::Choice` + `compile_stack_position` (`compile.rs:278-285`) handling; lower to a binary top/bottom `pending_selection`.
- [ ] 3.2 Behavioral test: `position: choice` offers top/bottom and places accordingly; migrate BT25-038 clause A to it and confirm the ~100→~20 line reduction with parity.
- [ ] 3.3 Consolidate the place-on-security arg structs (`step.rs:1587-1626` + `place_on_security` 1587-1595) into one source-polymorphic verb (`source:` = hand binding | permanent/material binding | self) with a `SecurityReplacementDisposition` enum (`none|cancel|handle|observed`).
- [ ] 3.4 Behavioral parity on EVERY current place-on-security user before deleting the old verbs; keep the replacement dispositions legal only inside `kind: replacement`. Fix the bottom+handle asymmetry.
- [ ] 3.5 Delete the superseded place-on-security verbs.

## 4. `link_card_to_self` → `link_cards` migration
- [ ] 4.1 Confirm `link_cards` covers single-card self-host + chosen-host; close `G-DSL-LINK-N-CARDS-PER-HOST` / `G-DSL-LINK-FROM-ANY-OWN-DIGIMON-SOURCES` if a residual gap exists.
- [ ] 4.2 Migrate the 11 cards (ST22-12, BT21-023/073/101, BT25-052/056/060/069/070/072/089) to `link_cards` with per-card parity tests.
- [ ] 4.3 Delete `link_card_to_self` + its lowerer.

## 4.5 Link substrate + EX11-027 (folded in from fix-dsl-substrate-rot-and-bugs)
- [ ] 4.5.1 Add `G-DSL-LINK-RELINK-STANDING-PERMANENT`: an engine primitive (DCGO `IPlacePermanentToLinkCards` analog) + DSL verb to move a standing battle-area permanent to become a link card on a chosen own Digimon.
- [ ] 4.5.2 Add `G-DSL-LINK-HOST-FILTER`: host filter + link-requirement enforcement on `link_card_to_self { to: ChosenOwnDigimon }` (exclude the source permanent; check `CanLinkToTargetPermanent`).
- [ ] 4.5.3 Add `G-DSL-LINK-HETEROGENEOUS-CHOICE`: model a single RL selection that is an either/or between two distinct link operations (self-permanent relink vs hand-card link), with a branch offered only when its precondition holds.
- [ ] 4.5.4 Add `G-DSL-REPLACEMENT-LINK-CARD-TO-BOTTOM-SOURCE`: a `kind: replacement` cost that places a chosen link card as the carrier's bottom digivolution card to cancel a leave (engine fn analogous to `trash_own_link_card_and_cancel_leave` but calling `AddDigivolutionCardsBottom`).
- [ ] 4.5.5 Migrate EX11-027 Maquinamon off raw_rust to the new vocab; behavioral tests for the on_play link choice + the leave-prevention replacement; remove `ex11_027_*` from the phase0/roundtrip `StubRegistry`.
- [ ] 4.5.6 Promote the `dsl-substrate-integrity` loader guard in `code/digimon-engine/src/cards.rs` from warn-mode to a hard error (panic on unregistered raw_rust ref) now that the pack has zero unregistered refs; add the engine-level "unregistered ref fails load" test (fix-dsl-substrate-rot-and-bugs §1.2).
- [ ] 4.5.7 Move the `G-DSL-LINK-*` entries from `qa/dsl-vocab-gaps.md` to `qa/resolved-gaps.md`.

## 5. Docs + verification
- [ ] 5.1 Regenerate the vocab block; confirm removed verbs drop out + new verbs appear; `dsl-vocab-doc-drift --check` green.
- [ ] 5.2 Update `RUST_DSL_AGENT_GUIDE.md` §5 (searcher pattern, `then:`-tail idiom, security placement).
- [ ] 5.3 RL guard: action-mask/tensor for a representative collapsed card matches its longhand equivalent (no action-space/tensor size change).
- [ ] 5.4 Full DSL + behavioral + action-mask suites green.
