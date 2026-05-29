## ADDED Requirements

### Requirement: Effects can digivolve from a selected security card
An effect SHALL be able to select a card from a player's security stack and use that selected card as the source card for an effect-initiated digivolution without paying the cost when the printed card text allows it.

#### Scenario: Selected security card digivolves a target
- **WHEN** an effect selects a legal Digimon card from security and a legal target Digimon for effect-initiated digivolution
- **THEN** the selected security card is removed from security and becomes the top card of the target Digimon's stack
- **AND** the digivolution fires normal when-digivolving effects with effect-initiated provenance

#### Scenario: Declined security digivolve
- **WHEN** the selected-security digivolve effect is optional and the player declines the security selection
- **THEN** no security card is removed
- **AND** no digivolution occurs

#### Scenario: Security order is preserved
- **WHEN** a non-top security card is selected for effect-initiated digivolution
- **THEN** all non-selected security cards remain in their relative order

### Requirement: Effects can play a selected security card and continue card-local tails
An effect SHALL be able to select a card from a security stack, play that selected card without paying its cost, record whether the play succeeded, and continue card-local follow-up steps from that result.

#### Scenario: Selected security card is played
- **WHEN** an effect selects a legal card from security to play without paying its cost
- **THEN** the selected card leaves security and enters the battle area through the effect-play pipeline
- **AND** normal on-play triggers and security-loss observers are dispatched

#### Scenario: Play-success tail resolves
- **WHEN** an effect says "if you did" after playing a selected security card and the play succeeds
- **THEN** the follow-up steps resolve

#### Scenario: Play-success tail is skipped
- **WHEN** an effect says "if you did" after playing a selected security card and no card is played
- **THEN** the follow-up steps do not resolve

### Requirement: Security search effects shuffle after selection
Effects that search a security stack SHALL preserve hidden-information handling by shuffling the searched security stack after the printed effect instructs it, regardless of whether an optional card was selected.

#### Scenario: Search selection succeeds
- **WHEN** a card searches its controller's security stack, selects a matching card, and then instructs the player to shuffle security
- **THEN** the selected card is processed according to the effect
- **AND** the remaining security stack is shuffled after the selection branch completes

#### Scenario: Optional search is declined
- **WHEN** a card searches its controller's security stack, the selection is optional, and the player declines
- **THEN** no card is processed from security
- **AND** the security stack is still shuffled when the printed text says to shuffle
