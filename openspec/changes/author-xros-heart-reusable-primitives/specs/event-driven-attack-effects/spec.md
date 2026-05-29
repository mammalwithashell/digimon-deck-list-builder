## ADDED Requirements

### Requirement: Effects can create immediate may-attack windows
The engine SHALL support effects that allow a Digimon that was just played or
digivolved to attack through a temporary attack window. The attack choice MUST
be optional when the printed text says "may".

#### Scenario: Played Digimon may attack
- **WHEN** a Tamer effect triggers because an own Xros Heart or Hero Digimon is
  played
- **AND** the player pays the printed cost such as suspending the Tamer
- **THEN** the played Digimon is offered an optional attack window
- **AND** declining the attack leaves the game state otherwise unchanged

#### Scenario: Digivolved Digimon may attack
- **WHEN** the same trigger condition is met by an own matching Digimon
  digivolving
- **THEN** the digivolved Digimon is offered the same optional attack window
- **AND** legal attack targets are still determined by normal attack legality

### Requirement: Effects can initiate attacks after resolving costs
The engine SHALL support effects that pay or resolve printed costs, then prompt
the player to attack a player with one of their Digimon. The selected attack
MUST resolve through the normal attack and combat pipeline.

#### Scenario: Unsuspend named bodies then attack player
- **WHEN** an option effect unsuspends one `Shoutmon EX6` and one
  `ShootingStarmon` as its printed cost or setup
- **THEN** the legal named permanents are chosen through pending selections
- **AND** the player then chooses one legal attacking Digimon
- **AND** the attack target is a player as printed

#### Scenario: No legal attacker after setup
- **WHEN** an effect-driven attack reaches the attack step but no legal attacker
  exists
- **THEN** no hidden attack is performed
- **AND** the effect resolves the no-attack path without panicking

### Requirement: Effect-driven attacks do not bypass attack hooks
The engine SHALL route effect-driven attacks through normal attack declaration,
blocker, collision, and attack-resolution hooks.

#### Scenario: Blocker and collision still apply
- **WHEN** an effect-driven attack is declared
- **AND** blockers or collision modifiers are active
- **THEN** the normal blocker and collision logic can affect the attack
- **AND** the attack outcome is not resolved as direct damage unless the normal
  rules allow it
