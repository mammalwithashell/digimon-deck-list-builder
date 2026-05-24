## MODIFIED Requirements

### Requirement: DSL supports formula-valued De-Digivolve amounts

The `de_digivolve` step SHALL accept a formula-valued amount in addition to the existing literal amount. The formula SHALL evaluate at effect resolution time using the resolving effect context, and the resulting amount SHALL be passed through the normal De-Digivolve caps, immunity checks, and configured stop-at-level floor. DSL-authored `de_digivolve` steps that omit `stop_at_level` SHALL default to the normal level 3 floor, so card YAML that represents standard printed `<De-Digivolve N>` text preserves the floor even when using `amount_fn`. Non-standard stack-clearing effects that intentionally ignore the level 3 floor SHALL use a raw Rust/helper path that explicitly calls the engine primitive with no floor.

#### Scenario: De-Digivolve amount equals own Digimon count

- **WHEN** a `de_digivolve` step uses `amount_fn` based on the controller's Digimon count
- **AND** the controller has three Digimon when the effect resolves
- **THEN** the engine attempts to De-Digivolve the selected target by 3
- **AND** normal stop-at-level, available-source caps, and immunity checks still apply

#### Scenario: Formula-valued standard De-Digivolve preserves the level 3 floor

- **WHEN** a standard printed `<De-Digivolve>` effect is authored with `amount_fn`
- **AND** the target stack contains a Digi-Egg under a level 3 card
- **THEN** the YAML-authored step SHALL preserve the standard level 3 floor
- **AND** resolving the effect SHALL NOT trash the level 3 card or expose the Digi-Egg

#### Scenario: Literal De-Digivolve remains supported

- **WHEN** a `de_digivolve` step uses the existing literal `amount` field
- **THEN** it compiles and resolves with the same behavior as before this change

#### Scenario: Non-standard unbounded stack trash remains expressible outside default DSL lowering

- **WHEN** a card's printed text requires trashing digivolution cards without the standard De-Digivolve level 3 floor
- **THEN** a raw Rust/helper implementation MAY call the engine De-Digivolve primitive with no stop-at-level floor for that non-standard effect
- **AND** that usage SHALL remain distinct from standard DSL-authored printed `<De-Digivolve>` text
