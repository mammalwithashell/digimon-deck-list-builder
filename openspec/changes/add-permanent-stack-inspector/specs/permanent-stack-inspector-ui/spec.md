## ADDED Requirements

### Requirement: Right-click opens the permanent inspector
The game UI SHALL open a permanent detail panel when the player right-clicks (context-menu) any permanent — on their own field, the opponent's field, or the breeding area — and SHALL suppress the browser's native context menu. The inspector MUST be available at any point in the game, including while a selection prompt or attack selection is active, because it is a read-only view that submits no action.

#### Scenario: Right-click own permanent
- **WHEN** the player right-clicks one of their own battle-area permanents
- **THEN** the detail panel opens for that permanent AND no browser context menu appears AND no game action is submitted

#### Scenario: Right-click opponent permanent
- **WHEN** the player right-clicks an opponent battle-area permanent
- **THEN** the detail panel opens for that permanent

#### Scenario: Right-click during a selection prompt
- **WHEN** a selection prompt is active and the player right-clicks a permanent
- **THEN** the detail panel opens AND the pending selection is unaffected

#### Scenario: Right-click does not trigger play/attack
- **WHEN** the player right-clicks a permanent that would, on left-click, be a legal attacker or target
- **THEN** the inspector opens AND no attack or play action is initiated

### Requirement: Inspector shows stack and runtime state
The detail panel SHALL display the permanent's digivolution stack (top card plus each source, in stack order), its active keywords (distinguishing innate from granted), its DP and security-attack values, per-source effect/inherited text, and the active-modifier list. Hidden opponent sources SHALL render as an obscured placeholder rather than leaking card identity.

#### Scenario: Stack and keywords render
- **WHEN** the inspector is open for a stacked permanent that has keywords
- **THEN** the panel lists each stack source AND shows the permanent's keywords with granted keywords visually distinguished from innate ones

#### Scenario: Hidden source obscured
- **WHEN** the inspector is open for an opponent permanent whose source identity is not available in the filtered state
- **THEN** that source renders as an obscured placeholder (e.g. "???") and no hidden card id is shown

### Requirement: Grouped active-modifier display
The detail panel SHALL render the serialized `modifiers` as a grouped, labelled list — grouping immunities, restrictions, stat changes, and granted keywords — using a frontend type-to-label map. A modifier type with no mapping SHALL render under a generic group rather than break the panel.

#### Scenario: Modifiers grouped and labelled
- **WHEN** a permanent has an immunity modifier, a +DP stat-change modifier, and a restriction modifier
- **THEN** the panel shows each under its group with a human-readable label (e.g. "Cannot be deleted", "DP +3000", "Can't suspend")

#### Scenario: Unknown modifier type tolerated
- **WHEN** the serialized `modifiers` contains a type with no frontend label mapping
- **THEN** the panel still renders, showing that entry under a generic/"Other" group

### Requirement: Inspector dismissal
The detail panel SHALL be dismissible via the Escape key, its close control, or by inspecting another permanent, and SHALL not obstruct continued play once closed.

#### Scenario: Escape closes the panel
- **WHEN** the inspector is open and the player presses Escape
- **THEN** the panel closes
