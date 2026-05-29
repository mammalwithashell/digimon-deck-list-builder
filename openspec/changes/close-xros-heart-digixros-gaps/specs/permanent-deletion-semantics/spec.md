## ADDED Requirements

### Requirement: Material Save resolves during deletion or leave-battle-area timing

The engine SHALL treat `<Material Save X>` as an optional deletion/removal-timed keyword. It MUST be offered when its carrier would be deleted or otherwise leave the battle area by an applicable effect, and it MUST NOT be exposed as a `[Main]` activated effect.

#### Scenario: Material Save offered when carrier would be deleted

- **WHEN** a permanent with `<Material Save 2>` would be deleted and has eligible source cards plus a legal Tamer destination
- **THEN** the controller is offered the optional Material Save choice during the deletion/removal timing window
- **AND** accepting the choice proceeds to source selection before the deletion commits

#### Scenario: Material Save declined

- **WHEN** the controller declines the Material Save choice
- **THEN** no source cards are placed under a Tamer by Material Save
- **AND** the deletion continues through the normal deletion flow

#### Scenario: Material Save not present in main phase

- **WHEN** the controller is in their main phase with a permanent carrying `<Material Save 2>`
- **THEN** the action mask does not expose a main-phase action solely for Material Save

### Requirement: Material Save selects eligible recipe sources from the deletion snapshot

When `<Material Save X>` resolves, the engine SHALL select up to X source cards from the carrier's pre-removal source snapshot. Eligible cards MUST satisfy the carrier's printed DigiXros recipe filters. The carrier's top card MUST NOT be eligible unless it is also present as a source in the snapshot.

#### Scenario: Recipe-ineligible source is masked out

- **WHEN** a permanent with `<Material Save 2>` has one `Shoutmon` source and one unrelated source in its deletion snapshot
- **AND** its DigiXros recipe includes `Shoutmon` but not the unrelated card
- **THEN** the Material Save source-selection mask includes the `Shoutmon`
- **AND** the unrelated source is masked out

#### Scenario: Source limit is enforced

- **WHEN** `<Material Save 1>` resolves with three eligible recipe sources
- **THEN** the controller can select no more than one source card for the Material Save placement

#### Scenario: No eligible source skips source selection

- **WHEN** `<Material Save 2>` would resolve but the deletion snapshot contains no recipe-eligible source cards
- **THEN** no source-selection prompt is installed
- **AND** the deletion continues normally

### Requirement: Material Save places selected sources under a chosen Tamer

The engine SHALL place selected Material Save source cards under a legal Tamer controlled by the same player. If multiple legal Tamers exist, choosing the Tamer SHALL be a pending selection. If no legal Tamer exists, Material Save SHALL not move any source cards.

#### Scenario: Controller chooses among multiple Tamers

- **WHEN** Material Save resolves and the controller has two legal Tamer destinations
- **THEN** the controller receives a pending selection for the destination Tamer
- **AND** the selected source cards are placed under the chosen Tamer in a deterministic order

#### Scenario: No Tamer destination

- **WHEN** Material Save would resolve but the controller has no legal Tamer destination
- **THEN** Material Save does not install a source-selection prompt
- **AND** no source cards move under a Tamer

### Requirement: Leave-battle-area source rescue effects use snapshots and pending selections

Effects that trigger when a Xros Heart or Blue Flare permanent leaves the battle area and move or play its prior sources SHALL use the permanent's pre-removal snapshot and pending selections. They MUST NOT read from a removed battle-area permanent after source movement has committed.

#### Scenario: Source rescue selects from snapshot

- **WHEN** a permanent leaves the battle area and a rescue effect may place prior sources under a Tamer
- **THEN** the selectable cards come from that permanent's pre-removal source snapshot
- **AND** ineligible snapshot cards are masked out according to the effect's printed filter

#### Scenario: Source replay plays selected snapshot card

- **WHEN** a leave-battle-area effect instructs the player to play one selected prior source without paying the cost
- **THEN** the selected card is taken from the pre-removal snapshot's moved-card location
- **AND** the play is represented through existing free-play engine helpers and action-mask-visible selections
