## ADDED Requirements

### Requirement: An activated-digivolve alt-path is offered and executes

A hand card that declares a `kind: activated_digivolve` alternate path SHALL be offered to the action space as a digivolve action against each field permanent the alt-path can target, and executing that action SHALL digivolve the hand card onto the chosen permanent at the alt-path's declared cost, ignoring printed digivolution requirements. The action reuses the existing `DIGIVOLVE` action range — the action space size does not change.

#### Scenario: A satisfiable activated-digivolve alt-path is offered

- **WHEN** a hand card has a `kind: activated_digivolve` alt-path whose `condition` passes and whose `from:` source filter matches a field permanent the controller could digivolve, and whose `extra_cost` is payable
- **THEN** a digivolve action targeting that field permanent is legal in the action mask for that hand card

#### Scenario: Executing the activated digivolve

- **WHEN** the controller takes the activated-digivolve action for a hand card onto a chosen field permanent
- **THEN** any `extra_cost` is paid first, then the hand card digivolves onto that permanent at the alt-path's declared `cost`, ignoring the card's printed digivolution requirements

#### Scenario: The alt-path condition gates the offer

- **WHEN** a hand card's activated-digivolve alt-path has a `condition` that does not currently pass
- **THEN** no activated-digivolve action is offered for that hand card

#### Scenario: Unsatisfiable extra cost is not offered

- **WHEN** a hand card's activated-digivolve alt-path requires an `extra_cost` that cannot currently be paid
- **THEN** no activated-digivolve action is offered for that hand card
