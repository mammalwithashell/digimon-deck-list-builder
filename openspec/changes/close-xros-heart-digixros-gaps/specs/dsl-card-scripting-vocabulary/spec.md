## ADDED Requirements

### Requirement: DSL can declare DigiXros alt paths

The card-scripting DSL SHALL accept DigiXros alternate play paths that declare recipe material filters, allowed material origin zones, and per-material play-cost deltas. Lowering such a path SHALL create a DigiXros transaction rather than a normal alternate digivolution path.

#### Scenario: Author declares a two-material DigiXros recipe

- **WHEN** a card YAML declares a `kind: digixros` path with recipe filters for `Shoutmon` and `Ballistamon`
- **THEN** the DSL compiles the recipe into distinct material slots
- **AND** the engine offers the path only when at least one legal material-selection sequence can satisfy the recipe

#### Scenario: Author declares allowed material zones

- **WHEN** a DigiXros YAML path declares material zones `hand` and `battle_area`
- **THEN** the lowered transaction initially allows only those zones for material selection
- **AND** cards in trash or under Tamers are not legal materials unless another DSL-authored transaction modifier grants access

#### Scenario: Unsupported DigiXros field fails compilation

- **WHEN** a DigiXros YAML path declares an unsupported field
- **THEN** compilation fails with an error naming the unsupported field
- **AND** the compiler does not silently ignore the field

### Requirement: DSL can author DigiXros transaction modifiers

The card-scripting DSL SHALL provide declarative steps or triggered clauses that mutate a pending DigiXros transaction before play cost is paid. Supported mutations SHALL include granting material origin zones, adding maximum counts for those zones where needed, pre-attaching selected materials, and applying one-shot cost deltas.

#### Scenario: Author grants under-Tamer material access

- **WHEN** a Tamer YAML clause declares an optional before-pay-cost transaction modifier that grants `under_tamers` material access
- **THEN** the lowered effect offers the printed accept/decline choice at the pending play's cast-time window
- **AND** accepting the choice mutates only the current DigiXros transaction

#### Scenario: Author pre-attaches a selected source

- **WHEN** a card YAML clause declares a transaction modifier that selects a matching `Shoutmon`, pre-attaches it, applies `cost_delta: -1`, and unlocks `trash`
- **THEN** the lowered effect installs the required selection before cost is fixed
- **AND** the selected card is attached during the DigiXros transaction's source-commit step
- **AND** the trash material access applies only to that transaction

### Requirement: DSL can author Material Save from DigiXros recipes

The card-scripting DSL SHALL allow `<Material Save X>` to be authored as a deletion/removal-timed keyword that filters eligible source cards through the carrier's printed DigiXros recipe. The DSL compiler MUST NOT lower Material Save to a `[Main]` activated effect.

#### Scenario: Keyword derives eligible materials from recipe

- **WHEN** a card YAML declares a DigiXros recipe and `<Material Save 2>`
- **THEN** the lowered keyword can identify eligible source cards from the same recipe filters when the carrier would be deleted or leave the battle area

#### Scenario: Material Save is not a main-phase action

- **WHEN** a card YAML declares `<Material Save 2>`
- **THEN** the compiled card does not gain a main-phase activated action from that keyword
- **AND** Material Save is available only through the deletion/removal timing window

### Requirement: DSL Xros Heart fixtures are production-authored

The initial Xros Heart acceptance pool SHALL be represented by production YAML and Rust behavioral tests, not `_examples` files or raw-Rust placeholders. The acceptance pool SHALL include BT10-009, BT10-087, BT12-112, and BT10-013.

#### Scenario: Acceptance card has no raw Rust placeholder

- **WHEN** an acceptance-pool card YAML is compiled
- **THEN** its DigiXros, transaction-modifier, and Material Save behavior is expressed through supported DSL vocabulary
- **AND** no `raw_rust` placeholder is required for the tested behavior

#### Scenario: Example-only Xros Heart behavior is promoted or tracked

- **WHEN** an `_examples` Xros Heart YAML file demonstrates behavior required by the acceptance pool
- **THEN** that behavior is either promoted into production YAML with tests or retained as an explicit gap in the relevant tracker
