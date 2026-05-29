## ADDED Requirements

### Requirement: Bottom-source trash SHALL soft-fail on absent or insufficient sources

Any engine or DSL primitive that trashes the bottom N source cards from a target permanent SHALL follow the source-trash soft-fail contract. It MUST NOT panic when the target permanent is missing, when the target stack is empty, when the target has no source cards, or when fewer than N source cards are available. It SHALL trash all live available source cards up to N and silently no-op for the remainder.

#### Scenario: Target has no source cards

- **WHEN** bottom-source trash resolves against a permanent whose stack contains only a top card
- **THEN** no card moves to trash
- **AND** the engine does not panic or install any fallback prompt

#### Scenario: Target has fewer sources than requested

- **WHEN** bottom-source trash requests two source cards from a target with only one source card
- **THEN** the one source card is trashed
- **AND** the missing second source silently no-ops
- **AND** the top card remains on the permanent

#### Scenario: Target permanent is stale

- **WHEN** bottom-source trash resolves after the target permanent was removed by an intervening effect
- **THEN** the primitive returns or resolves as a no-op
- **AND** no observer dispatch, source movement, or panic occurs for that stale target

#### Scenario: Valid trash dispatches source-trash observers

- **WHEN** bottom-source trash removes one or more live source cards
- **THEN** each removed source card moves to its owner's trash
- **AND** the normal `OnDigivolutionCardTrashed` observer context is fired for each actually trashed source
