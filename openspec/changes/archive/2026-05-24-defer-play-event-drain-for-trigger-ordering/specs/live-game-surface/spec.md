## ADDED Requirements

### Requirement: Play event broadcasts share a single trigger drain

When a Digimon enters the battle area via a play action (player-initiated main-phase play, effect-initiated play, or security-effect play), the engine SHALL enqueue the played card's own `[On Play]` triggers (timing `OnPlay`) together with the observer broadcasts `OnEnterFieldAnyone` and `OnAllyPlayed` (both keyed on `TriggerSource::EnteredField`) into the effect queue BEFORE draining. The drain SHALL run exactly once after all three trigger sources are enqueued, so simultaneous triggers from the same play event are eligible for inclusion in a single `TriggerOrder` selection bundle.

#### Scenario: Played card's own [On Play] and an observer's [All Turns] share a TriggerOrder bundle

- **WHEN** a player plays a Digimon whose card data declares an effect with timing `OnPlay`
- **AND** another permanent (own or opponent) declares an effect with timing `OnEnterFieldAnyone` whose condition matches the play event
- **AND** both triggers belong to the same controller (the turn player)
- **THEN** the next `pending_selection` after the play action resolves is a `SelectionKind::TriggerOrder` bundle containing BOTH triggers
- **AND** the controller can pick the order in which they resolve

#### Scenario: Picking the observer trigger first runs it before the played card's effect choice

- **WHEN** a `TriggerOrder` bundle from a play event lists the played card's `[On Play]` mandatory trigger and an observer's `[All Turns]` optional trigger
- **AND** the controller picks the observer trigger first
- **THEN** the observer's body runs (or inerts if its activation cost cannot be paid) BEFORE the played card's `[On Play]` body or effect-choice prompt surfaces
- **AND** the played card's `[On Play]` becomes the next prompt in the selection sequence after the observer trigger resolves

#### Scenario: Default ordering preserves prior observable sequence

- **WHEN** a `TriggerOrder` bundle from a play event includes the played card's `[On Play]`
- **AND** the controller picks the played card's `[On Play]` first (matching the default sequence prior to this change)
- **THEN** the resolution sequence is identical to the previous engine behavior — the played card's effect resolves, then observer triggers fire and drain in turn

#### Scenario: Single-trigger plays auto-fire without TriggerOrder

- **WHEN** a play event produces exactly one queued trigger (e.g. the played card has no `[On Play]` AND no observers trigger from the play)
- **OR** when the produced triggers can auto-fire because only one is queued for the active chooser at a time
- **THEN** the engine resolves them without surfacing a `TriggerOrder` selection — existing single-trigger behavior is preserved
