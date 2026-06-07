## ADDED Requirements

### Requirement: FAQ corpus is frozen and surface-triaged

The suite SHALL maintain an authoritative ledger of every entry in the official General Rules/FAQ corpus, each classified into exactly one test surface: `runtime` (DebugRunner behavioral), `deck-validation`, `data` (metadata/registry), or `n/a` (intentionally not modeled by the engine). The ledger SHALL live at `qa/qa-reports/rules-faq.md` and SHALL be the source of truth for corpus coverage.

#### Scenario: Every FAQ entry has exactly one ledger row

- **WHEN** the ledger is reconciled against the General Rules/FAQ page
- **THEN** every Q&A entry appears as exactly one row carrying: the quoted question, the FAQ answer, its assigned surface, the card(s) used (or `—`), the test path or cross-link (or reason for `n/a`), and a verdict

#### Scenario: Not-modeled items are documented, not faked

- **WHEN** an FAQ entry describes a table procedure the engine intentionally abstracts (e.g. rock-paper-scissors first-player selection, physical security-stack placement order)
- **THEN** its row carries verdict `N/A` with an explicit reason (the engine abstraction that replaces it), and no test is written for it

### Requirement: Foundational rules are pinned with real implemented DSL cards

Every in-scope, uncovered runtime FAQ rule SHALL be encoded as a `DebugRunner` behavioral test under `code/digimon-engine/tests/rules_faq/`, organized by the FAQ's own sections, asserting the FAQ-correct outcome. Tests that need a card with a specific property SHALL reuse an already-implemented DSL card that exhibits it; a card SHALL be authored only when no implemented vehicle exists, and any such card SHALL be recorded in `validated_cards_dsl.json`.

#### Scenario: A runtime rule is pinned

- **WHEN** the suite encodes an uncovered runtime FAQ rule and the test passes
- **THEN** the test asserts the FAQ-correct outcome, its docstring quotes the FAQ question + answer + the `general_rule.pdf` §/DCGO citation, and the ledger row verdict is `PIN`

#### Scenario: Reused cards are gated before their section runs

- **WHEN** a section's tests compose a reused DSL card by id
- **THEN** a loader gate confirms that card loads from the embedded DSL pack, so a missing/un-migrated card fails loudly at fixture-build time rather than mid-test

#### Scenario: A card is authored only as a last resort

- **WHEN** a rule requires a card property (e.g. a `-` Level Digimon, a `-` DP Digimon, a conditional-keyword Digimon) that no implemented DSL card carries
- **THEN** a faithful full-text DSL card is authored to serve as the vehicle and recorded in `validated_cards_dsl.json`

### Requirement: Existing coverage is cross-linked, not duplicated

For any FAQ rule already pinned elsewhere in the test tree, the suite SHALL cite the existing test rather than re-encoding it. A new test SHALL be written only for genuinely uncovered rules.

#### Scenario: An already-covered rule is cross-linked

- **WHEN** the coverage audit finds a runtime rule already pinned by an existing test
- **THEN** the ledger row verdict is `XLINK` citing the existing test path, and no duplicate test is added

### Requirement: Discovered gaps are logged and routed, never silenced

When a discovery-wave test fails, the failure SHALL be treated as a discovered faithfulness gap: the assertion SHALL be committed as-written (asserting the FAQ-correct outcome) and the gap SHALL be logged to the appropriate shared tracker and spun off as a scoped fix. Assertions SHALL NOT be weakened to make a test pass.

#### Scenario: A failing rule becomes a logged gap

- **WHEN** a discovery-wave test asserts the FAQ-correct outcome and the engine produces a different result
- **THEN** the gap is logged to `qa/archetype-qa/engine-gaps.md` (engine) or `qa/dsl-vocab-gaps.md` (DSL vocabulary) with the FAQ + rules citation, the ledger row verdict is `GAP` referencing the tracker entry, and a scoped fix/chip is spun off

#### Scenario: The canary deletion-timing rule is pinned even before its fix

- **WHEN** the suite encodes the rule that two simultaneous end-of-turn DP modifiers (+DP and −DP) end together with no intermediate 0-DP deletion
- **THEN** the test asserts the Digimon survives at its original DP, and if the engine deletes it mid-expiry the test is committed as a logged `GAP` (candidate `MODIFIED` delta to `permanent-deletion-semantics`) rather than softened

### Requirement: Deck-creation and metadata rules use their natural harness

FAQ entries triaged to `deck-validation` SHALL be asserted against the deck-legality surface (`tests/deck_tools/`), and entries triaged to `data` SHALL be asserted against card metadata/registry state, rather than being forced through the runtime harness.

#### Scenario: A deck-creation rule is validated

- **WHEN** the suite encodes a deck-construction FAQ rule (e.g. a 50-card main deck, ≤4 copies per card number, no Digi-Eggs in the main deck)
- **THEN** it is asserted via the deck-validation surface and its ledger row surface is `deck-validation`

#### Scenario: A metadata rule is asserted on card data

- **WHEN** the suite encodes an identity/text-matching FAQ rule (e.g. a two-color Digimon is treated as all its colors; "X in its name" is a substring match while a keyword-icon match is exact)
- **THEN** it is asserted against `CardData`/registry state and its ledger row surface is `data`
