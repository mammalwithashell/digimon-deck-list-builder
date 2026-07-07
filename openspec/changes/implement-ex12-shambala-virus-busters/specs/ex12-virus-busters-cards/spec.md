# Spec: ex12-virus-busters-cards

## ADDED Requirements

### Requirement: Virus Busters slice fully implemented
All 21 Virus Busters-slice EX12 cards (EX12-001, -005, -007, -010, -013, -014, -016, -017, -018, -021, -024, -032, -035, -037, -040, -042, -044, -066, -069, -073, -077) SHALL be implemented as YAML DSL specs in `code/digimon-engine/cards/ex12/` with every printed clause faithfully modeled per the no-approximations policy: no stubs, no auto-selections, every player choice exposed through the pending-selection surface.

#### Scenario: Card pool coverage
- **WHEN** the slice is complete
- **THEN** each of the 21 card IDs has a YAML spec that compiles into the embedded registry and a verdict entry in `qa/qa-reports/validated_cards_dsl.json` (IMPLEMENTED, or PARTIAL citing a real tracker entry)

### Requirement: DUAL Siriusmon faithful to both faces
EX12-018 Siriusmon/Planet Punch SHALL be modeled as a DUAL card following the shipped dual-YAML shape (ST23-09/ST24-07/BT25-043 precedent): the Digimon face (keywords Progress/Piercing/Security A. +1, the top-or-bottom placement clause with its per-source −2000 DP rider) and the Option face (Use Req. [VB] trait, the highest-DP delete + may-attack Main, Arts Digivolve), with the Option face's colors Red/Yellow per the verified card face.

#### Scenario: Placement clause offers top or bottom
- **WHEN** the [When Digivolving]/[When Attacking] effect places a qualifying card from hand or trash
- **THEN** the player chooses top vs bottom digivolution-card position per placed card (both positions RL-visible), and the −2000 DP rider scales with the carrier's digivolution-card count

#### Scenario: Option face resolves and Arts Digivolve is available
- **WHEN** Planet Punch is used with the [VB]-trait Use Requirement satisfied
- **THEN** the Main body resolves (delete 1 highest-DP opponent Digimon, then 1 of the controller's Digimon may attack) and the card offers Arts Digivolve disposition instead of trashing

### Requirement: Virus Busters per-card behavioral coverage
Each implemented Virus Busters card SHALL have a DebugRunner behavioral test suite in `code/digimon-engine/tests/cards_behavioral/ex12/` covering structural clause shape, positive/negative paths per condition, decline paths for optional choices, once-per-turn lockouts where printed, and event-log assertions for costs.

#### Scenario: Suite green at wave merge
- **WHEN** a wave containing the card merges
- **THEN** `cargo test --test cards_behavioral -- <card_id_lower>` passes with zero failures and zero unexplained ignores

### Requirement: Virus Busters interaction capstone
After per-card suites are green, the slice SHALL receive multi-card interaction tests plus the four static archetype tests, covering at least one full partner digivolution line into its Lv.6/Lv.7 payoff (e.g. Gammamon line into Siriusmon; Agumon/Gabumon lines into Omnimon) and the [VB]-trait synergy cards (EX12-066 Hiro, EX12-069 Virus Busters).

#### Scenario: Interaction suite green
- **WHEN** the capstone lands
- **THEN** the Virus Busters interaction suite passes and a verdict is recorded in `qa/qa-reports/archetype_interactions.json`
