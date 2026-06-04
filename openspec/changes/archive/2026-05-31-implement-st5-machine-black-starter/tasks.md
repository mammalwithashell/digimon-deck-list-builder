## 1. Discovery and Test Harness

- [x] 1.1 Confirm the ST5 card pool and exact starter counts from local card data and the starter-deck source.
- [x] 1.2 Add or extend ST5 behavioral-test module wiring under `code/digimon-engine/tests/`.
- [x] 1.3 Add failing behavioral tests for ST5-04 and ST5-06 inherited end-of-opponent-turn draw behavior, including the opponent-attacked negative case.
- [x] 1.4 Add failing DSL or engine predicate tests for the reusable player Digimon attack-history condition and its turn-boundary reset semantics.

## 2. Attack-History DSL Vocabulary

- [x] 2.1 Add the attack-history predicate schema to `code/digimon-dsl/src/` with support for referencing `you` and `opponent`.
- [x] 2.2 Lower the new predicate into the Rust engine's compiled predicate representation.
- [x] 2.3 Evaluate the predicate from authoritative game attack history, not observation metadata.
- [x] 2.4 Verify the predicate works inside inherited `end_of_opponents_turn` clauses and under normal DSL negation.

## 3. Simple and Static ST5 Cards

- [x] 3.1 Author YAML and registry coverage for ST5-02, ST5-05, ST5-07, and ST5-10 vanilla Digimon.
- [x] 3.2 Author and test ST5-03 Agumon `<Blocker>`.
- [x] 3.3 Author and test ST5-08 DarkTyrannomon `<Blocker>` plus `[When Attacking] Lose 2 memory`.
- [x] 3.4 Author and test ST5-11 Megadramon inherited `<Blocker>`.
- [x] 3.5 Author and test ST5-01 Kapurimon inherited +1000 DP while the carrier has `<Blocker>`.

## 4. Keyword Grants and Digi-Burst Cards

- [x] 4.1 Author and test ST5-09 MetalGreymon granting `<Blocker>` to 1 own Digimon until the end of the opponent's next turn.
- [x] 4.2 Author and test ST5-12 Machinedramon granting `<Reboot>` to up to 2 own Digimon until the end of the opponent's next turn.
- [x] 4.3 Author and test ST5-13 BlitzGreymon static `<Security A. +1>`.
- [x] 4.4 Author and test ST5-13 BlitzGreymon `<Digi-Burst 2>` source trashing and +4000 DP buff until the end of the opponent's next turn.

## 5. Options and Security Effects

- [x] 5.1 Author and test ST5-15 Laser Eye `<De-Digivolve 1>` against up to 2 opponent Digimon.
- [x] 5.2 Author and test ST5-15 Laser Eye security effect activating its main effect.
- [x] 5.3 Author and test ST5-16 Dark Side Attack deleting 1 opponent Digimon with play cost 7 or less.
- [x] 5.4 Author and test ST5-16 Dark Side Attack security effect activating its main effect.

## 6. Conditional Inherited and Tamer Effects

- [x] 6.1 Author ST5-04 ToyAgumon inherited draw using the new attack-history predicate.
- [x] 6.2 Author ST5-06 Greymon inherited draw using the new attack-history predicate.
- [x] 6.3 Add failing positive and negative tests for ST5-14 Tai Kamiya blocker-response behavior.
- [x] 6.4 Author ST5-14 Tai Kamiya using existing attack-target-change/blocker context if tests prove it is faithful.
- [x] 6.5 If existing context cannot distinguish blocker usage faithfully, add the smallest DSL timing/context extension needed and update the ST5-14 tests.
- [x] 6.6 Author and test ST5-14 Tai Kamiya security effect playing the Tamer without paying the cost.

## 7. Starter Deck and Metadata Surfaces

- [x] 7.1 Add the exact ST-5 Machine Black starter decklist to the appropriate deck-library or fixture surface.
- [x] 7.2 Add ST5 cards to the implemented/tested-card metadata gate only after their YAML compiles and behavior tests pass.
- [x] 7.3 Add a smoke test that `digimon_engine.load_implemented_card_ids()` includes ST5-01 through ST5-16.
- [x] 7.4 Add a Rust backend reset/headless smoke test using the exact ST5 starter deck.

## 8. Verification and Tracker Reconciliation

- [x] 8.1 Run targeted Rust DSL and ST5 behavioral tests.
- [x] 8.2 Run broader affected suites for cards_behavioral, DSL compilation, option/security flow, and combat/blocker behavior.
- [x] 8.3 Update `qa/dsl-vocab-gaps.md`, `qa/archetype-qa/engine-gaps.md`, and `qa/resolved-gaps.md` for the attack-history predicate and any Tai blocker-context finding.
- [x] 8.4 Confirm no ST5 tests are ignored for unresolved gaps and no card effect is stubbed or approximated.
- [x] 8.5 Run `openspec status --change implement-st5-machine-black-starter` and confirm the change remains apply-ready.
