## ADDED Requirements

### Requirement: Every Rocks card has faithful DSL implementation

Every unique card in the Rocks decklist pool (as resolved from `data/deck_library.json`) SHALL have a production DSL YAML file under `code/digimon-engine/cards/<set>/` whose effects faithfully implement the full printed card text from `data/cards.json` — every clause, timing, and player choice. No clause may be omitted, stubbed, or auto-resolved. Where the printed text exposes a choice, that choice SHALL surface through `pending_selection` so the RL action space sees it (CLAUDE.md §17).

#### Scenario: Card pool fully authored

- **WHEN** the Rocks card pool is resolved from `data/deck_library.json`
- **THEN** each unique card ID has a corresponding `code/digimon-engine/cards/<set>/<CARD-ID>.yaml` file
- **AND** the previously substrate-blocked clauses of BT21-021, P-130, EX11-065, BT20-055, and BT23-096 are each fully authored

#### Scenario: Card clauses are complete, not approximated

- **WHEN** a Rocks card's YAML is reviewed against its printed text in `data/cards.json`
- **THEN** every printed clause (main, inherited, security, when-digivolving, alt-path) is represented in the YAML
- **AND** no clause is replaced by a no-op, a hidden auto-selection, or a coarser proxy

### Requirement: Every Rocks card has behavioral test coverage

Every Rocks card SHALL have a behavioral test file under `code/digimon-engine/tests/cards_behavioral/<set>/` that exercises its card text via `DebugRunner`. Tests SHALL be written before or alongside the YAML they cover (TDD, CLAUDE.md §18), with one positive and one negative test per condition.

#### Scenario: Behavioral test exists per card

- **WHEN** the Rocks card pool is enumerated
- **THEN** each card has a behavioral test file covering its printed clauses
- **AND** the test suites `cards_behavioral`, `dsl`, `combat`, and `option_flow` pass with no regressions

### Requirement: No behavioral test is ignored for an already-closed gap

No Rocks behavioral test SHALL carry an `#[ignore]` marker that cites a substrate gap which is already closed in the current engine/DSL. Each `#[ignore]` marker that remains SHALL cite a substrate gap that is verifiably still open, confirmed by inspecting the current engine code — not by trusting a tracker document.

#### Scenario: Stale ignore markers are pruned

- **WHEN** every `#[ignore]` marker in a Rocks behavioral test is checked against the current engine and DSL source
- **THEN** any marker citing a gap that is already closed is removed and its test re-enabled
- **AND** the re-enabled test passes

### Requirement: BT21-021 inherited Rush aura is authored faithfully

BT21-021's inherited `[Your Turn]` Rush aura SHALL gate the `<Rush>` grant on the carrier permanent holding the `[Xros Heart]` trait, authored with the existing `source_permanent_trait_has` predicate (which resolves against the carrier for inherited clauses — confirmed at `predicate.rs:369`). No new predicate is required; the previously-ignored test SHALL be re-enabled and pass.

#### Scenario: Aura applies only when carrier has the required trait

- **WHEN** BT21-021 is an inherited (digivolution-source) card under a carrier Digimon that has the `[Xros Heart]` trait
- **THEN** the carrier gains `<Rush>` during its controller's turn

#### Scenario: Aura does not apply when carrier lacks the trait

- **WHEN** BT21-021 is an inherited card under a carrier Digimon that does NOT have the `[Xros Heart]` trait
- **THEN** the carrier does not gain `<Rush>`

### Requirement: Effect-initiated move-from-breeding DSL verb

The DSL SHALL provide a step that lowers to `EffectContext::move_from_breeding_by_effect`, with an optional accept/decline prompt and a `filter` constraining which breeding-area permanent is eligible. The choice SHALL surface through `pending_selection`. This closes the P-130 `[On Play]` clause ("You may move 1 of your level 3 or higher Digimon from the breeding area to the battle area").

#### Scenario: Player may move an eligible breeding Digimon

- **WHEN** P-130 is played and the controller has a level 3 or higher Digimon in the breeding area
- **THEN** the controller is offered a choice to move it to the battle area
- **AND** accepting moves the Digimon and fires the move observers; declining leaves the breeding area unchanged

#### Scenario: Ineligible breeding Digimon is not offered

- **WHEN** P-130 is played and the only breeding-area Digimon is below level 3
- **THEN** no move selection is installed

### Requirement: Union-zone cost selector across hand and digivolution sources

The DSL SHALL provide a cost selector that draws candidates from the union of the controller's hand and the digivolution-card stacks of the controller's Digimon, applying a single trait/name filter across both zones, and trashing the chosen card as a cost. This closes the EX11-065 `[Start of Your Main Phase]` clause ("By trashing 1 [Mineral] or [Rock] trait card from your hand or your Digimon's digivolution cards, gain 1 memory").

#### Scenario: Cost can be paid from hand

- **WHEN** EX11-065's start-of-main clause activates and the controller has a `[Mineral]` or `[Rock]` card in hand
- **THEN** that hand card is a legal trash candidate, and trashing it gains 1 memory

#### Scenario: Cost can be paid from a digivolution source

- **WHEN** EX11-065's start-of-main clause activates and a controller Digimon has a `[Mineral]` or `[Rock]` digivolution-card source
- **THEN** that source card is a legal trash candidate, and trashing it gains 1 memory

#### Scenario: Clause does not fire with no eligible card

- **WHEN** EX11-065's start-of-main clause activates and the controller has no `[Mineral]` or `[Rock]` card in hand or in any digivolution source
- **THEN** no selection is installed and no memory is gained

### Requirement: Face-up security lifecycle primitives

The engine and DSL SHALL provide (a) a no-choice step that flips an opponent's top face-down security card face up, and (b) an observer timing that fires when one of the controller's Digimon checks a face-up security card. This closes both omitted clauses of BT20-055 Invisimon.

#### Scenario: De-Digivolve rider flips opponent security face up

- **WHEN** BT20-055's `[On Play]` / `[When Digivolving]` clause resolves
- **THEN** the opponent's top face-down security card is flipped face up with no player choice

#### Scenario: Face-up security check triggers self-security placement

- **WHEN** one of the BT20-055 controller's Digimon checks a face-up security card during the controller's turn
- **THEN** the controller may place the top card of BT20-055 face-up at the bottom of their security stack

### Requirement: Delay-on-attack trigger support

The engine and DSL SHALL support a `<Delay>` clause whose placing trigger is an attack event: delay-lowering SHALL map attack timings (e.g. `OnAllyAttack`) to an `OnEvent` delay trigger, combat dispatch SHALL fan attack events out to event-gated delayed options, and the delay activation condition SHALL be able to read the attacking Digimon's traits. This closes the BT23-096 `[Your Turn]` CS-attack `<Delay>` clause.

#### Scenario: Attack by a CS Digimon arms the Delay

- **WHEN** a `[CS]` trait Digimon controlled by the BT23-096 player attacks
- **THEN** the BT23-096 `<Delay>` clause becomes armed and is activatable on a later turn per the `<Delay>` keyword rules

#### Scenario: Attack by a non-CS Digimon does not arm the Delay

- **WHEN** a Digimon without the `[CS]` trait attacks
- **THEN** the BT23-096 `<Delay>` clause is not armed by that attack

### Requirement: Verdict ledger and gap trackers reflect verified Rocks state

After this change, `qa/qa-reports/validated_cards_dsl.json` SHALL record the verified status of every Rocks card, and the gap trackers SHALL be reconciled: gaps closed by this change SHALL be moved to `qa/resolved-gaps.md`, and any Rocks gap that remains open SHALL be confirmed open against current engine code.

#### Scenario: Ledger matches test reality

- **WHEN** the Rocks entries in `validated_cards_dsl.json` are compared against `cargo test --test cards_behavioral` results
- **THEN** every card marked `IMPLEMENTED` has all its behavioral tests passing with no `#[ignore]` for a closed gap
- **AND** no card is marked `BLOCKED` or `PARTIAL` for a gap that this change has closed

#### Scenario: Closed gaps are archived

- **WHEN** a substrate gap (B1–B5) is closed by this change
- **THEN** its entry is moved from `qa/dsl-vocab-gaps.md` / `qa/archetype-qa/engine-gaps.md` to `qa/resolved-gaps.md` with a resolution note and test command
