# ui-auto-resolution

## ADDED Requirements

### Requirement: UI-side auto-resolution never alters the engine contract
Auto-resolution SHALL be implemented entirely as UI-side automatic submission of mask-legal actions through the normal action path; the engine SHALL continue to surface every choice via `pending_selection` and the action mask, and no engine, binding, or server behavior SHALL change based on automation settings.

#### Scenario: Engine surface unchanged
- **WHEN** any combination of automation toggles is enabled
- **THEN** the engine receives only standard action submissions and its emitted action space, pending selections, and RL-visible behavior are identical to manual play

### Requirement: Single-legal-action auto-resolve
When the auto-resolve-trivial-choices toggle is enabled and a pending selection has exactly one mask-legal action, the UI SHALL submit that action automatically.

#### Scenario: Forced selection auto-submitted
- **WHEN** an effect requires selecting a card and only one selection action is legal
- **THEN** the UI submits it without showing a blocking prompt

#### Scenario: Two legal actions stay manual
- **WHEN** a pending selection has two or more mask-legal actions (including an exposed pass/decline)
- **THEN** the UI presents the prompt normally and does not auto-submit

### Requirement: Allowlisted order-only auto-resolve
When the auto-order toggle is enabled, the UI SHALL auto-submit a default order only for selection kinds on an explicit allowlist of order-only choices over visible cards (initially: bottom-of-deck placement order), and SHALL never auto-resolve ordering selections off the allowlist.

#### Scenario: Bottom-deck order auto-resolved
- **WHEN** an effect asks the player to order two revealed cards to the bottom of the deck and the toggle is on
- **THEN** the UI submits the default order automatically

#### Scenario: Unlisted ordering stays manual
- **WHEN** an ordering selection of a kind not on the allowlist arises
- **THEN** the player is prompted normally regardless of the toggle

### Requirement: Cost and hatch automation
When the respective toggles are enabled, the UI SHALL auto-select the minimum cost when an identical digivolve is offered at multiple costs, and SHALL auto-submit hatching when hatch is legal and no other meaningful action exists for the decision point.

#### Scenario: Min digivolve cost picked
- **WHEN** a digivolve is legal at cost 3 and cost 5 for the same hand card and target and auto-min-cost is on
- **THEN** the UI submits the cost-3 action

#### Scenario: Auto-hatch only when forced
- **WHEN** the breeding decision point offers hatch plus another meaningful action
- **THEN** the UI does not auto-hatch and prompts the player

### Requirement: Auditability of automated choices
Every auto-submitted action SHALL produce a log/ticker entry marked as automated, and any auto-resolution failure SHALL disable automation for the session and surface a notice rather than retry silently.

#### Scenario: Automated action visible in log
- **WHEN** any toggle causes an automatic submission
- **THEN** the game log records the action with an automation marker

#### Scenario: Failure disables automation
- **WHEN** an auto-submitted action is rejected by the engine
- **THEN** automation is disabled for the session, the player is notified, and the pending prompt is shown manually
