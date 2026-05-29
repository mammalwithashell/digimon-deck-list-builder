## ADDED Requirements

### Requirement: Leave-battle source rescue can use arbitrary filters
The engine SHALL support leave-battle source rescue effects that use the
permanent's pre-removal source snapshot with arbitrary authored filters, not
only the carrier's DigiXros recipe.

#### Scenario: Trait-filtered source rescue
- **WHEN** a permanent with Xros Heart and Blue Flare sources would leave the
  battle area
- **AND** its effect allows up to four Xros Heart or Blue Flare Digimon cards
  from its sources to be placed under a Tamer
- **THEN** the eligible source list comes from the pre-removal snapshot
- **AND** nonmatching sources are masked out
- **AND** selected sources are placed under the chosen Tamer after the leave
  event resolves according to the effect timing

#### Scenario: Snapshot remains valid after permanent is removed
- **WHEN** the source permanent has already moved out of the battle area during
  leave-battle resolution
- **THEN** the rescue effect can still resolve selected source cards from the
  snapshot-backed moved-card locations
- **AND** it does not read from a stale battle-area index

### Requirement: Leave-battle rescue can be optional
The engine SHALL support optional source rescue prompts for leave-battle
effects. Declining the prompt MUST leave the original leave-battle event to
continue normally.

#### Scenario: Decline source rescue
- **WHEN** a leave-battle source rescue prompt is offered
- **AND** the player declines it
- **THEN** no source cards are placed under Tamers by that effect
- **AND** the original leave-battle movement continues normally
