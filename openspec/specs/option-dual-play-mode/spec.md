# option-dual-play-mode Specification

## Purpose

Define how an Option card that supports more than one play mode — a Standard
`[Main]` Option and a Link Option — surfaces a mode-select choice to its
controller when played, while preserving today's direct-play behavior for
single-mode Options. This unblocks Medusamon tier-3 Option cards.
## Requirements
### Requirement: An Option with multiple play modes surfaces a mode choice

An Option card that supports more than one play mode — a Standard `[Main]` Option and a Link Option — SHALL, when played from hand, surface a mode-select choice to the controller. Each chosen mode SHALL resolve through its own disposal path. An Option with exactly one play mode SHALL play directly, with no extra prompt, identical to today's behavior.

#### Scenario: A dual-mode Option installs a mode-select prompt

- **WHEN** an Option card that is both a Standard `[Main]` Option and a Link Option is played from hand
- **THEN** a mode-select selection installs, offering "play as a `[Main]` Option" and "plug in via Link Requirements"

#### Scenario: Choosing the Standard mode

- **WHEN** the controller selects the `[Main]` Option mode at the mode-select prompt
- **THEN** the card resolves through the Standard Option disposal path, paying the `[Main]` use cost

#### Scenario: Choosing the Link mode

- **WHEN** the controller selects the Link mode at the mode-select prompt
- **THEN** the card resolves through the Link Option disposal path — paying the link cost and attaching to a legal link host

#### Scenario: A single-mode Option plays directly

- **WHEN** an Option card that supports exactly one play mode is played from hand
- **THEN** no mode-select prompt installs and the card resolves through that single mode's disposal path, unchanged from prior behavior

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
