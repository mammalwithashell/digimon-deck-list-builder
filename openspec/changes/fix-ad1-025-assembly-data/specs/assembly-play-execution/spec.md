## ADDED Requirements

### Requirement: Assembly play is offered when materials are satisfiable from trash

The engine SHALL offer a hand card's `[Assembly]` play (in the action mask) when, for each of the card's assembly material elements, the controller's trash holds at least the required number of distinct matching cards, AND the reduced play cost can be made payable. Assembly SHALL be optional (a legal but not forced play). A single trash card SHALL NOT count toward more than one element.

#### Scenario: Assembly offered when trash has the materials

- **WHEN** a hand card has an `[Assembly]` alt-path requiring 1 `[WarGreymon]` + 1 `[MetalGarurumon]` and the controller's trash holds at least one of each
- **THEN** the Assembly play is a legal action

#### Scenario: Assembly not offered when trash lacks a material

- **WHEN** the controller's trash is missing one of the required named materials
- **THEN** the Assembly play is NOT offered

#### Scenario: Declaration legal when the reduced cost can be paid

- **WHEN** the controller's memory cannot afford the card's base cost but CAN afford the cost reduced by the Assembly amount
- **THEN** declaring the Assembly play is a legal game action (declare-then-pay against the reduced cost)

### Requirement: Per-element trash selection surfaces and requires the exact count

For each assembly material element, the engine SHALL install a selection over the controller's trash (candidates = trash cards matching the element filter, minus those already chosen for prior elements), requiring exactly the element's count to be picked, with the selecting player being the controller. The selection SHALL surface through `pending_selection` (no auto-resolution), so the choice reaches the RL action space.

#### Scenario: Material choice is surfaced, not auto-selected

- **WHEN** the controller's trash holds two `[WarGreymon]` and one `[MetalGarurumon]` and the Assembly play is taken
- **THEN** the engine installs a selection for the controller to choose WHICH `[WarGreymon]` to place (and which `[MetalGarurumon]`)
- **AND** exactly the element count must be chosen per element

#### Scenario: Exact count enforced

- **WHEN** an element requires 1 material
- **THEN** the selection accepts exactly 1 (cannot end below the count)

### Requirement: Selected materials placed at the digivolution-stack bottom; play cost reduced

On resolution the engine SHALL place the selected materials at the BOTTOM of the played card's digivolution stack (under it), only when exactly the total element count was chosen, and SHALL reduce the play cost by the Assembly reduction amount before payment.

#### Scenario: Materials placed under and reduced cost paid

- **WHEN** the WarGreymon and MetalGarurumon are chosen and the Assembly play resolves for a base-cost-15 card with reduction 6
- **THEN** both materials are placed at the bottom of the played card's digivolution stack
- **AND** the controller pays a cost of 9 (15 − 6)
- **AND** the played card's own `[On Play]` / `[When Digivolving]` and keyword effects fire as for any play

### Requirement: AD1-025 carries `[Assembly]` in data and as an alt_path; Q5 pins

AD1-025 Omnimon's card data SHALL include its printed `[Assembly] -6 [WarGreymon] x [MetalGarurumon]` keyword (via `data/card_overrides.json`), `cards/ad1/AD1-025.yaml` SHALL declare an `assembly` alt_path (materials WarGreymon + MetalGarurumon, `zones: [trash]`, `stack_under`, reduction 6) alongside the existing `dna_digivolve`, and judge-quiz Q5 SHALL be un-`#[ignore]`-d and pass.

#### Scenario: AD1-025 playable via Assembly

- **WHEN** Player A has WarGreymon and MetalGarurumon in trash and declares the AD1-025 `[Assembly]` play
- **THEN** the play is legal (declare-then-pay against the reduced cost), the two materials are placed under Omnimon, and the reduced cost is paid

#### Scenario: Q5 pins

- **WHEN** the executor, data, and alt_path land
- **THEN** `c_declare_then_pay::q5_assembly_declaration_legal_when_cost_can_be_made_payable` is un-ignored and asserts the Assembly declaration is a legal game action
- **AND** `G-ASSEMBLY-PLAY-EXECUTION` is moved from `qa/archetype-qa/engine-gaps.md` to `qa/resolved-gaps.md`, and the Q5 entry in `qa/qa-reports/judge-quiz.md` / `card-resolution.md` is updated from BLOCKED to PASS
