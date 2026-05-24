## ADDED Requirements

### Requirement: Standard De-Digivolve stops at level 3

The Rust engine SHALL resolve standard `<De-Digivolve N>` effects so they do not trash a current top card whose effective level is 3 or lower. This applies whether the requested amount is literal or formula-valued, and it SHALL match the DCGO `IDegeneration` stop shape where the loop exits once the target permanent's current level is 3.

#### Scenario: Higher stack peels to level 3

- **WHEN** a standard De-Digivolve 3 effect resolves against a battle-area stack `[Lv3, Lv4, Lv5, Lv6]`
- **THEN** the engine SHALL trash `Lv6`, `Lv5`, and `Lv4`
- **AND** the target permanent SHALL remain in the battle area with `Lv3` as its top card

#### Scenario: Digi-Egg source under level 3 remains hidden

- **WHEN** a standard De-Digivolve 3 effect resolves against a battle-area stack `[Digi-Egg Lv2, Lv3, Lv4]`
- **THEN** the engine SHALL trash `Lv4`
- **AND** the engine SHALL NOT trash `Lv3`
- **AND** the target permanent SHALL remain in the battle area with `Lv3` as its top card
- **AND** the Digi-Egg SHALL remain only as a digivolution source

#### Scenario: Formula-valued standard De-Digivolve uses the same floor

- **WHEN** a standard `de_digivolve` DSL step uses `amount_fn` that evaluates to 3
- **AND** the target stack is `[Digi-Egg Lv2, Lv3, Lv4]`
- **THEN** the engine SHALL resolve the effect as a De-Digivolve 3 attempt with the standard level 3 floor
- **AND** the target permanent SHALL remain topped by `Lv3`

### Requirement: Exposed battle-area Digi-Egg automatically leaves the field

After any De-Digivolve or stack/source mutation that can change a battle-area permanent's top card, the engine SHALL enforce that a Digi-Egg card cannot remain as the top card of a battle-area permanent. If a battle-area permanent's top card becomes a Digi-Egg, that permanent SHALL leave the field through the rules-aligned cleanup path, the Digi-Egg top card SHALL move to its owner's trash, and no permanent slot with the Digi-Egg top card SHALL remain in battle area.

#### Scenario: Source removal exposes a Digi-Egg top card

- **WHEN** an effect removes the only non-Digi-Egg card from a battle-area permanent whose remaining stack is a Digi-Egg
- **THEN** the engine SHALL remove that permanent from the battle area
- **AND** the Digi-Egg SHALL be moved to its owner's trash
- **AND** later action masks and state views SHALL NOT expose a bare Digi-Egg permanent in the battle area

#### Scenario: Cleanup does not affect breeding area

- **WHEN** a Digi-Egg is face-up in the breeding area
- **THEN** the Digi-Egg SHALL remain in the breeding area unless another legal effect or phase action moves it
- **AND** the battle-area Digi-Egg cleanup SHALL NOT remove or trash it

#### Scenario: Cleanup does not remove Digi-Egg sources

- **WHEN** a battle-area permanent has a non-Digi-Egg top card with a Digi-Egg beneath it as a digivolution source
- **THEN** the Digi-Egg source SHALL remain in that stack
- **AND** the cleanup SHALL NOT move the Digi-Egg source to trash

### Requirement: BT24-041 Minervamon matches DCGO De-Digivolve shape

BT24-041 Minervamon's shared `[On Play][When Digivolving][On Deletion]` effect SHALL select one opponent Digimon and apply standard De-Digivolve 1 for each of the controller's Digimon, counted at resolution time after any optional Iliad card is played from hand. The Rust implementation MAY use a formula-valued amount, but it MUST preserve the standard level 3 floor.

#### Scenario: Optional Iliad play contributes to Minervamon count

- **WHEN** BT24-041 resolves its shared effect
- **AND** the controller plays an eligible Iliad Digimon from hand with the optional first clause
- **THEN** the newly played Digimon SHALL count toward the De-Digivolve amount

#### Scenario: Minervamon does not expose opponent Digi-Egg

- **WHEN** BT24-041 resolves with a computed De-Digivolve amount of 3 against an opponent stack `[Digi-Egg Lv2, Lv3, Lv4]`
- **THEN** only `Lv4` SHALL be trashed from the opponent stack
- **AND** the opponent permanent SHALL remain topped by `Lv3`
- **AND** no bare Digi-Egg permanent SHALL appear in the opponent's battle area
