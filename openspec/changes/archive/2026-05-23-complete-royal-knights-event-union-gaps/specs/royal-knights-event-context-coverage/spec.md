## ADDED Requirements

### Requirement: Event observers expose normalized trigger payloads

The engine and DSL SHALL expose normalized trigger payloads for Royal Knights observer effects that react to cards being played, digivolved, removed from security, added to security, or trashed from security.

#### Scenario: Played Digimon payload is available to predicates and effects

- **WHEN** a Digimon is played by normal play or by an effect
- **THEN** observer predicates SHALL be able to inspect the played Digimon's controller, level, traits, name, play cause, and resulting battle-area permanent
- **AND** effect bodies SHALL be able to target the played permanent without re-querying unrelated matching permanents

#### Scenario: Security movement payload is available to observers

- **WHEN** a card is removed from security, added to security, or trashed from security
- **THEN** observer predicates SHALL be able to inspect the affected player, moved card, source zone, destination zone, and movement cause
- **AND** effects SHALL NOT infer security movement from only the current security count

#### Scenario: Digivolution payload identifies same-level X evolution

- **WHEN** a Digimon digivolves into a card with the same level as its previous top card because of an X Antibody style path
- **THEN** observer predicates SHALL be able to identify the digivolving permanent, previous top card, new top card, level relationship, and X-evolution cause

### Requirement: Event-target predicates preserve self and level scoping

The DSL SHALL support predicates that compare the event target to the effect source and that branch on event-target level without matching unrelated permanents.

#### Scenario: Self-scoped observer fires only for its own event

- **WHEN** an effect source has an observer that requires the event target to be the source permanent
- **THEN** the observer SHALL fire when the source permanent caused the matching event
- **AND** the observer SHALL NOT fire when another matching Digimon caused the same event type

#### Scenario: Opponent-played level branch resolves from the played Digimon

- **WHEN** an opponent plays a Digimon and a Royal Knights observer branches on that Digimon's level
- **THEN** the branch SHALL use the level of the Digimon from the event payload
- **AND** unrelated opponent Digimon in battle SHALL NOT affect the branch result

### Requirement: Event-bound effects target only event participants

Triggered Royal Knights effects SHALL be able to grant keywords, delete targets, apply attack permission, or attach cards using the permanents identified by the triggering event.

#### Scenario: Keyword grant targets the played Digimon

- **WHEN** a Royal Knights observer triggers from a matching Digimon being played
- **THEN** a keyword grant that refers to that Digimon SHALL apply only to the played permanent from the trigger payload

#### Scenario: Attack permission follows the printed trigger participant

- **WHEN** a Royal Knights observer permits one of the controller's Digimon to attack after another Digimon is played
- **THEN** the attack choice SHALL be exposed as a player-visible choice
- **AND** the trigger payload SHALL remain available until that choice resolves

### Requirement: Event observers maintain legal action masking

Event-context observers SHALL expose every player-visible choice through pending selections or existing action masks, and SHALL keep PASS legal only when the printed effect is optional or the selection minimum is satisfied.

#### Scenario: Optional observer choice can be declined

- **WHEN** an event-context observer opens an optional target choice with at least one legal target
- **THEN** PASS SHALL be legal
- **AND** resolving PASS SHALL decline the observer without applying its success body

#### Scenario: Mandatory observer choice cannot pass before minimum selection

- **WHEN** an event-context observer opens a mandatory target choice and at least one legal target exists
- **THEN** PASS SHALL NOT be legal before the required minimum number of selections is satisfied
