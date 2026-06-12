# per-card-command-panel

## ADDED Requirements

### Requirement: Contextual command panel on own cards
When no targeting or interrupt flow is active, left-clicking an own hand card, own battle-area permanent, or own breeding permanent SHALL open an anchored command panel listing every currently-legal action for that card derived from the action mask — including play, digivolve (per target), DNA digivolve, attack (per target and security), effect activation (per effect), trash, and breeding moves — and SHALL show an explicit empty state when the card has no legal actions.

#### Scenario: Hand card menu
- **WHEN** the player clicks a hand card that can be played for 3 memory and digivolved onto one field Digimon
- **THEN** the panel shows a "Play — 3 memory" entry and a digivolve entry for that target, and no other action entries

#### Scenario: Permanent menu with effects
- **WHEN** the player clicks an own unsuspended Digimon that can attack security and has one activatable main-phase effect
- **THEN** the panel shows an "Attack Security" entry and a labeled effect entry

#### Scenario: No legal actions
- **WHEN** the player clicks an own card with no mask-legal actions
- **THEN** the panel opens in an empty state indicating the card has no available actions

### Requirement: Panel actions submit through the standard action path
Activating a command panel entry SHALL submit exactly the same action id(s) as the equivalent existing gesture (drag-and-drop, action bar), and multi-target entries SHALL enter the existing slot-highlight target-pick flow where the chosen target completes the action and Escape cancels.

#### Scenario: Menu and drag equivalence
- **WHEN** a digivolve is performed once via drag-and-drop and once via the command panel on an identical board
- **THEN** the submitted action id is identical in both cases

#### Scenario: Target-pick flow
- **WHEN** the player activates "Attack…" with four legal targets
- **THEN** the panel closes, legal target slots highlight, clicking one submits the attack, and Escape cancels without submitting

### Requirement: Panel coexists with existing gestures and flows
The command panel SHALL not alter right-click inspection, hover preview, or drag-and-drop; SHALL not open during pending selections, attack declaration, or block/counter interrupt flows (where left-click retains its direct meaning); and SHALL close or rebuild whenever the action mask refreshes.

#### Scenario: Selection flow unaffected
- **WHEN** a pending selection is active and the player left-clicks a highlighted card
- **THEN** the click answers the selection as today and no command panel opens

#### Scenario: Stale menu prevented
- **WHEN** the panel is open and a new game state with a changed mask arrives
- **THEN** the panel closes or rebuilds from the new mask before any entry can be activated

### Requirement: Readable action labels
Command entries SHALL be labeled with human-readable text: play cost, digivolve target names and costs, attack target names, and effect labels using engine-provided labels when available with a timing-tagged fallback.

#### Scenario: Effect label fallback
- **WHEN** an activatable effect has no engine-provided label
- **THEN** its entry shows a timing-tagged generic label (e.g. "[Main] Effect 1") rather than a bare index
