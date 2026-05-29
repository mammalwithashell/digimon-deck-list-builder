## 1. Baseline Audit

- [ ] 1.1 Confirm the worldwide Gaia Red card-count fixture from the source decklist and local `data/cards.json` records for `ST1-01` through `ST1-16`
- [ ] 1.2 Audit existing `code/digimon-engine/cards/st1/` YAML and `code/digimon-engine/tests/cards_behavioral/st1/` tests so new work preserves current `ST1-07` behavior
- [ ] 1.3 Record any already-open reusable gaps for blocked-attacker triggers, own-security DP modifiers, and optional duplicate-prevented target picks

## 2. Starter Deck Fixture

- [ ] 2.1 Add or update the Gaia Red starter deck fixture with exact 54-card counts and separated Digi-Egg/main-deck zones
- [ ] 2.2 Add a fixture-count regression test that asserts 4 `ST1-01` Digi-Eggs, 50 main-deck cards, and only `ST1-01` through `ST1-16` IDs
- [ ] 2.3 Add a Rust smoke test that constructs a game using Gaia Red for both players without missing-card errors

## 3. Shared Blocked-Attacker Trigger Primitive

- [x] 3.1 Add failing combat or DSL tests for "when this Digimon is blocked" covering inherited carrier trigger, non-attacker non-trigger, unblocked attacks, and non-block target changes
- [x] 3.2 Extend trigger event context, predicates, or DSL timing vocabulary so authored YAML can distinguish the blocked attacker from other battle-area observers
- [x] 3.3 Implement the runtime trigger/lowering path without adding action IDs or tensor fields
- [x] 3.4 Update reusable gap trackers to close or narrow the blocked-attacker trigger gap

## 4. Shared Own-Security DP Primitive

- [x] 4.1 Add failing security/combat tests for own Security Digimon DP modifiers, opponent isolation, duration expiry, and security-effect creation
- [x] 4.2 Extend engine modifier storage and security battle DP calculation to consult defender-side own-security modifiers
- [x] 4.3 Extend DSL parsing/lowering for own Security Digimon DP modifiers with `end_of_turn` and `end_of_opponents_next_turn` durations
- [x] 4.4 Update reusable gap trackers to close or narrow the own-security DP modifier gap

## 5. ST-1 YAML Coverage

- [x] 5.1 Add production no-effect YAML specs for vanilla cards including `ST1-02`, `ST1-04`, `ST1-05`, and `ST1-10`
- [x] 5.2 Add production YAML for inherited DP, Security Attack, and keyword/aura cards including `ST1-01`, `ST1-03`, `ST1-06`, `ST1-08`, `ST1-11`, and `ST1-12`
- [x] 5.3 Update or confirm `ST1-07` YAML remains faithful while aligning file style with the new ST-1 batch
- [x] 5.4 Add production YAML for `ST1-09` using the new blocked-attacker trigger primitive
- [x] 5.5 Add production YAML for `ST1-13` and `ST1-14`, including tamer DP aura and own-security DP modifier behavior
- [x] 5.6 Add production YAML for `ST1-15` and `ST1-16`, including hand-play effects and explicit security mirrors

## 6. ST-1 Behavioral Coverage

- [x] 6.1 Add registry and no-hidden-effect tests proving every `ST1-01` through `ST1-16` card is implemented and vanilla cards have no scripted effects
- [x] 6.2 Add card-specific tests for inherited DP boosts, Security Attack formulas, Blocker and attack memory loss, and Tamer DP auras
- [x] 6.3 Add card-specific tests for `ST1-09` gaining memory only when its inherited carrier is blocked
- [ ] 6.4 Add card-specific tests for `ST1-14` main and security effects, including duration and security battle DP application
- [ ] 6.5 Add card-specific tests for `ST1-15` optional up-to-two deletion choices, duplicate prevention, and zero/one/two target paths
- [ ] 6.6 Add card-specific tests for `ST1-16` main and security deletion mirrors

## 7. Verification and Documentation

- [ ] 7.1 Run targeted Rust tests for ST-1 cards, DSL trigger behavior, own-security DP behavior, and deck fixture construction
- [ ] 7.2 Run implemented-card registry or deck-loading checks proving Gaia Red survives implemented-card filtering as a complete deck
- [ ] 7.3 Verify `ACTION_SPACE_SIZE`, active tensor layout metadata, PyO3 action exports, and frontend action constants are unchanged
- [x] 7.4 Update parity, QA, and resolved-gap documentation to reflect completed primitives and any remaining blocked behavior
- [ ] 7.5 Run `openspec status --change "implement-st1-gaia-red-starter-deck"` and ensure all apply-required artifacts remain complete
