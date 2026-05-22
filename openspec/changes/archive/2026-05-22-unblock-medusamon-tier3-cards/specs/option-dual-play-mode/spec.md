## ADDED Requirements

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
