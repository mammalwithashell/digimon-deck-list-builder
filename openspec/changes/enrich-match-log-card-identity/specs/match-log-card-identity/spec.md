## ADDED Requirements

### Requirement: Card-bearing log lines render `[CARD-ID: Name]` from event identity

The human-readable match log SHALL render every card it names in the form `[CARD-ID: Name]` (e.g. `[BT25-061: Offmon]`), where both the id and the name are taken from the **event payload**, not reconstructed from live game state. Live board-state lookup MAY be used only as a fallback when the event carries no identity (see the fallback requirement). The renderer lives in `code/frontend/src/utils/gameLogFormat.ts`.

#### Scenario: Played card renders id and name from the event

- **GIVEN** a `Play` event carrying `card_id = "BT25-007"` and `card_name = "Gatchmon"`
- **WHEN** the log formats the event
- **THEN** the line SHALL contain `[BT25-007: Gatchmon]` and SHALL NOT render a bare `BT25-007`

#### Scenario: Trashed card off the board still renders its name

- **GIVEN** a `Trash` event carrying `card_id = "BT25-061"` and `card_name = "Offmon"` for a card no longer in any battle-area slot
- **WHEN** the log formats the event
- **THEN** the line SHALL contain `[BT25-061: Offmon]` (name SHALL NOT depend on the card still occupying a board slot)

### Requirement: Attack lines name the attacker and the target

The log SHALL render an attack with the attacking Digimon named as `[CARD-ID: Name]` and the target named as `[CARD-ID: Name]` when the target is a Digimon, or as the literal `security` when the attack targets the security stack. The log SHALL NOT render `slot N` for an attacker or target whose identity is present on the event. When the event carries effective DP, the log SHALL annotate each named Digimon with its DP (e.g. `[BT1-009: Greymon] (5000 DP)`) so a battle's DP comparison is visible.

#### Scenario: Attack on a Digimon names both sides

- **GIVEN** an `Attack` event with attacker `[BT1-009: Greymon]` and target `[BT25-020: Tyrannomon]`
- **WHEN** the log formats the event
- **THEN** the line SHALL name both the attacker and the target and SHALL NOT contain `slot`

#### Scenario: Battle line shows DP for both Digimon

- **GIVEN** an `Attack` event with `attacker_dp = 5000` and `target_dp = 3000`
- **WHEN** the log formats the event
- **THEN** the attacker and target SHALL each be annotated with their DP (e.g. `(5000 DP)` and `(3000 DP)`)

#### Scenario: Attack on security names attacker and reads "security"

- **GIVEN** an `Attack` event with attacker `[BT1-009: Greymon]` and no target Digimon
- **WHEN** the log formats the event
- **THEN** the line SHALL name the attacker and SHALL describe the target as `security`

### Requirement: Memory-change lines attribute their effect source

When a `MemoryChange` event carries an effect source (`source_card_id` + `source_card_name`), the log line SHALL name the source card as `[CARD-ID: Name]`. When the event carries no source (cost payment, pass, structural), the line SHALL render as it does today with no source attribution.

#### Scenario: Tamer-driven memory gain names the tamer

- **GIVEN** a `MemoryChange` event with `delta = +1` and source `[BT25-098: Cyber Engage]`
- **WHEN** the log formats the event
- **THEN** the line SHALL attribute the gain to `[BT25-098: Cyber Engage]`

#### Scenario: Sourceless memory change is unattributed

- **GIVEN** a `MemoryChange` event with no `source_card_id`
- **WHEN** the log formats the event
- **THEN** the line SHALL render the memory change with no card attribution

### Requirement: Effect-target lines name the source effect and chosen targets

The log SHALL render an `EffectTarget` event as a line naming the source effect card `[CARD-ID: Name]` and each chosen target card `[CARD-ID: Name]`. The line SHALL be produced for every `EffectTarget` event, including those with a single target.

#### Scenario: Effect targeting one Digimon produces a named line

- **GIVEN** an `EffectTarget` event with source `[BT1-009: Greymon]` and one target `[BT25-020: Tyrannomon]`
- **WHEN** the log formats the event
- **THEN** the line SHALL name both the source effect and the target

#### Scenario: Multi-target effect names every target

- **GIVEN** an `EffectTarget` event with two targets
- **WHEN** the log formats the event
- **THEN** the line SHALL name both targets

### Requirement: Reveal lines name the revealed card and its source

The log SHALL render each non-security reveal event (reveal-deck-top, trash-from-deck-top) as a line naming the revealed card `[CARD-ID: Name]` and indicating which reveal it was.

#### Scenario: Deck-top reveal names the card

- **GIVEN** a reveal event for the top of deck carrying `[BT25-052: Logimon]`
- **WHEN** the log formats the event
- **THEN** the line SHALL name `[BT25-052: Logimon]` and indicate it was revealed from the deck top

### Requirement: Renderer falls back gracefully when identity is absent

When an event carries no card identity (e.g. a recording produced before this change, or a structural event), the formatter SHALL fall back in order: event-carried name → live board-state lookup by slot → bare card id → `slot N`. The fallback SHALL NOT throw and SHALL never blank out the line.

#### Scenario: Legacy event without name falls back to board lookup then id

- **GIVEN** an `Attack` event with `source_slot = 0` but no attacker identity fields
- **WHEN** the log formats the event and the slot still holds a named card
- **THEN** the line SHALL render that card's name; **AND** when the slot is empty it SHALL render the bare id, and only `slot 0` when no id is available

### Requirement: Desktop and browser adapters populate identity fields identically

Both adapter wires — `event_to_dto` (`code/src-tauri/src/engine_commands.rs`, desktop) and `event_to_pydict` (`code/digimon-engine-py/src/lib.rs`, browser/server) — SHALL map every new identity field and new event variant onto the shared frontend `GameEvent` shape, and SHALL produce equivalent field population for the same engine event. Neither wire SHALL drop a field the other carries.

#### Scenario: Same Attack event yields the same identity on both wires

- **GIVEN** one engine `Attack` event with attacker and target identity
- **WHEN** it is converted by `event_to_dto` and by `event_to_pydict`
- **THEN** both outputs SHALL carry the attacker id+name and target id+name with matching values
