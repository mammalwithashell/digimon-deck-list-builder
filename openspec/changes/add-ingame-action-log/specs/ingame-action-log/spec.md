## ADDED Requirements

### Requirement: Action log is populated from the event stream

During a game, the system SHALL display a scrolling action log whose entries are derived from the engine's structured event stream. The log SHALL include entries for both the local player's and the opponent's (including bot) actions as their events arrive.

#### Scenario: A played card produces a log entry

- **WHEN** a card-played event is received during a game
- **THEN** the log shows a readable entry naming the card that was played and the acting player

#### Scenario: Bot actions appear in the log

- **WHEN** the bot opponent takes actions during its turn and emits events
- **THEN** those actions appear as log entries without requiring the local player to act

#### Scenario: Opening bot prelude events appear in the log

- **WHEN** a create-game response includes events emitted before the first local decision
- **THEN** those events are appended to the event stream and rendered in the log

#### Scenario: Empty state before any events

- **WHEN** no events have been received yet
- **THEN** the log shows an empty-state message rather than stale or blank content

### Requirement: Events are formatted into readable lines

The system SHALL convert each relevant `GameEvent` variant into one or more human-readable log lines, resolving card and player references to display names using the event payload and current game state. Events that carry no player-meaningful information MAY be omitted.

#### Scenario: Event type names are normalized before formatting

- **WHEN** events arrive with Rust/PyO3 PascalCase names or legacy lowercase names
- **THEN** the log formatter and animation consumers observe the same canonical frontend event names

#### Scenario: Common gameplay events render readable text

- **WHEN** events for digivolving, declaring an attack, checking security, activating an effect, and a memory/turn change are received
- **THEN** each produces a readable line that identifies the relevant card(s) and/or player rather than raw identifiers or enum names

#### Scenario: Card references are clickable where names are known

- **WHEN** a log line references a card whose id and name are resolvable
- **THEN** the reference is rendered using the existing card-reference affordance (clickable card name)

#### Scenario: Unknown or unrenderable event is skipped safely

- **WHEN** an event variant has no defined formatting or lacks data needed to render
- **THEN** the log omits it without error and continues processing subsequent events

### Requirement: Log derives from the canonical event stream, not a parallel logger

The action log SHALL be a projection of the structured event stream that already drives game state and animations. The system SHALL NOT depend on the engine binding's textual log buffer (`get_last_log()`), and SHALL NOT require a second, independently maintained textual logger in the engine for this capability.

#### Scenario: Log works without the engine textual log buffer

- **WHEN** the engine binding's textual log buffer returns empty
- **THEN** the in-game action log is still populated from the event stream

#### Scenario: Log stays consistent with animations

- **WHEN** an event triggers a board animation (e.g., digivolve, battle)
- **THEN** a corresponding log entry is produced from the same event, so the log and the animation reflect the same underlying action

### Requirement: Desktop responses expose the structured event stream

The desktop/Tauri gameplay commands SHALL populate their existing `events` response field by draining the in-process Rust `Game` event buffer into the frontend `GameEvent` DTO shape. This SHALL preserve the existing response contract and SHALL NOT introduce a parallel textual logger.

#### Scenario: Tauri action response includes drained events

- **WHEN** a desktop game action emits one or more Rust `GameEvent`s
- **THEN** the Tauri action/step response includes those events in the existing `events` field

#### Scenario: Draining events is one-shot

- **WHEN** the desktop event drain is called twice without another game action
- **THEN** the second drain returns no duplicate events
