## ADDED Requirements

### Requirement: Effects can use an Option from hand through the normal Option lifecycle

The engine SHALL provide an effect-driven way to use an Option card from hand without paying its cost, optionally constrained by card filters and a use-cost ceiling. The selected Option SHALL resolve through the same Option lifecycle as ordinary hand use, including `OnUseOption`, the selected or applicable Option main body, mode selection for multi-mode Options, and subtype-specific disposal or attachment.

#### Scenario: Effect uses eligible Option from hand

- **WHEN** an effect offers the controller a hand selection for `[TS]` trait Option cards with use cost less than or equal to the opponent's memory
- **AND** the controller selects an eligible Option
- **THEN** the selected Option is removed from hand and used without paying its cost
- **AND** the Option's normal use lifecycle resolves before the parent effect continues

#### Scenario: Ineligible Option is not selectable

- **WHEN** the hand contains one matching Option under the cost ceiling and one matching Option above the cost ceiling
- **THEN** only the Option under the cost ceiling appears in the pending selection/action mask

#### Scenario: Multi-mode Option preserves mode choice

- **WHEN** an effect-driven hand use selects an Option card that supports more than one play mode
- **THEN** the controller is offered the same mode-select prompt as ordinary hand use
- **AND** the chosen mode follows its normal disposal or attachment path

#### Scenario: Parent effect continues after Option resolution

- **WHEN** an effect uses an Option from hand and then has a subsequent step
- **THEN** the subsequent step runs only after the Option lifecycle has resolved or has been declined according to the effect's optionality
