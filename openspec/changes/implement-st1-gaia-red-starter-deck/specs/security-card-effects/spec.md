## ADDED Requirements

### Requirement: Effects can modify own Security Digimon DP
The engine and DSL SHALL support effects that modify the DP of the affected player's own Security Digimon during security battles. The modifier SHALL be consulted when that player is the defender and a security card with DP is revealed. The modifier SHALL support positive and negative values, duration expiry, and sources that are Digimon, Tamer, Option, inherited, or security effects as permitted by the authored YAML.

#### Scenario: Defender-side security DP buff applies during security battle
- **WHEN** player 1 has an active own-Security-Digimon DP modifier of +7000
- **AND** player 0 attacks player 1's security and reveals a Digimon card
- **THEN** the revealed Security Digimon's battle DP is increased by 7000 for that security battle

#### Scenario: Modifier does not affect opponent security battles
- **WHEN** player 1 has an active own-Security-Digimon DP modifier
- **AND** player 1 attacks player 0's security
- **THEN** player 0's revealed Security Digimon does not receive player 1's own-security modifier

#### Scenario: Modifier expires at printed duration
- **WHEN** an own-Security-Digimon DP modifier is created with an end-of-turn or end-of-opponents-next-turn expiry
- **AND** the relevant turn boundary passes
- **THEN** future security battles no longer include that modifier

#### Scenario: Security effect can create immediate own-security DP modifier
- **WHEN** a security Option effect grants the defender's Security Digimon a DP modifier for the turn
- **THEN** subsequent security battles that turn consult the modifier
- **AND** the currently resolving security card follows normal security effect disposal rules
