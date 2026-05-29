## 1. Source Review and Fixtures

- [x] 1.1 Read `data/cards.json` and `code/digimon-engine/cards/st3/*.json` for all `ST3-01` through `ST3-16` printed text, metadata, traits, colors, levels, costs, DP, and digivolution requirements.
- [x] 1.2 Inspect current DSL precedents for Recovery, DP modifiers, SecurityAttackChange, Blocker, security-option disposition, Tamer security play, and security-Digimon DP modifiers.
- [x] 1.3 Determine the repository's established location for canonical starter deck fixtures or starter deck library entries.
- [x] 1.4 Confirm whether current DSL predicates can distinguish "opponent's Digimon deleted by dropping to 0 DP" for `ST3-01` and `ST3-04`; document a reusable gap before implementation if exact support is missing.

## 2. Behavioral Tests

- [x] 2.1 Add `code/digimon-engine/tests/cards_behavioral/st3/mod.rs` and register it from `code/digimon-engine/tests/cards_behavioral/main.rs`.
- [x] 2.2 Add structural/load tests for vanilla ST3 cards: `ST3-02`, `ST3-03`, `ST3-06`, and `ST3-10`.
- [x] 2.3 Add inherited-effect tests for `ST3-01`, `ST3-04`, `ST3-05`, and `ST3-08`, covering printed timing, conditions, OPT where printed, and resulting DP or memory changes.
- [x] 2.4 Add face-up/triggered Digimon tests for `ST3-07`, `ST3-09`, and `ST3-11`, covering Blocker, attack memory loss, Recovery gating, and attack-time DP reduction.
- [x] 2.5 Add Tamer/security tests for `ST3-12`, covering opponent-turn security-Digimon DP increase and `[Security]` free play.
- [x] 2.6 Add Option main/security tests for `ST3-13`, `ST3-14`, `ST3-15`, and `ST3-16`, covering target selection, DP changes, SecurityAttackChange changes, "activate main effect" security behavior, and add-to-hand security behavior.
- [x] 2.7 Add starter-deck composition and registration tests verifying the canonical 54-card ST3 list and all card IDs appear in the implemented-card set.

## 3. Production YAML

- [x] 3.1 Author production YAML for vanilla and metadata-only ST3 cards with correct card metadata and digivolution requirements.
- [x] 3.2 Author production YAML for inherited Digimon effects (`ST3-01`, `ST3-04`, `ST3-05`, `ST3-08`) using exact printed timing and trigger conditions.
- [x] 3.3 Author production YAML for face-up Digimon effects (`ST3-07`, `ST3-09`, `ST3-11`) using existing keyword, Recovery, memory, and DP-modifier vocabulary.
- [x] 3.4 Author production YAML for `ST3-12` T.K. Takaishi, including security play and opponent-turn security-Digimon DP modifier behavior.
- [x] 3.5 Author production YAML for ST3 Options (`ST3-13` through `ST3-16`) with faithful main and security effects.
- [x] 3.6 Ensure no ST3 YAML uses a no-op placeholder, hidden auto-selection, or `raw_rust` escape for behavior that current DSL can express.

## 4. Starter Deck Loadability

- [x] 4.1 Add the canonical worldwide ST-3 Heaven's Yellow deck list to the established fixture/library location identified in task 1.3.
- [x] 4.2 Add a test or smoke path that loads the ST-3 fixture and verifies exact card counts: 54 total with the published per-card quantities.
- [x] 4.3 Add a Rust-backed initialization smoke check using the canonical ST-3 list without missing-card or unimplemented-card errors.

## 5. Gap Tracking and Documentation

- [x] 5.1 Update `docs/RUST_ENGINE_GAPS.md`, `qa/archetype-qa/engine-gaps.md`, or `qa/dsl-vocab-gaps.md` for any ST3 clause that remains blocked by a genuinely missing reusable primitive.
- [x] 5.2 Mark only blocked behavioral assertions as ignored, with ignore reasons citing the matching reusable gap ID.
- [x] 5.3 Update any ST3/validated-card ledger or QA report used by the repository to reflect implemented vs blocked status accurately.

## 6. Verification

- [x] 6.1 Run focused ST3 behavioral tests in `code/digimon-engine/tests/cards_behavioral/st3`.
- [x] 6.2 Run relevant DSL and card-pack compilation checks for `code/digimon-engine`.
- [x] 6.3 Run an implemented-card registry check proving `ST3-01` through `ST3-16` are returned by `load_implemented_card_ids()`.
- [x] 6.4 Run the Rust-backed ST3 deck smoke test from task 4.3.
- [x] 6.5 Confirm no action-space size, tensor profile size/hash, PyO3 API, or model metadata contract changed as part of this card/deck implementation.
