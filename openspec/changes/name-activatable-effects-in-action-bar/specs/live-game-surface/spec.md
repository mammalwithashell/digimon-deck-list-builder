## MODIFIED Requirements

### Requirement: Legal Actions Enumeration

The system SHALL expose `LiveGame::legal_actions(player) -> Vec<DecodedAction>` returning every action ID currently legal for the specified player, each decoded into a human-readable label.

`DecodedAction` SHALL contain:

- `action_id: u16` — the raw integer the engine consumes via `step()`
- `kind: ActionKind` — enum tag (`Play`, `Digivolve`, `Attack`, `EndTurn`, `PassTurn`, `ResolveSelection`, `MulliganKeep`, `MulliganRedraw`, etc.)
- `label: String` — human-readable description (e.g., `"play hand[2]: Agumon"`, `"digivolve field[0] from hand[3]: Greymon"`)
- `effect_name: Option<String>` — for [Main]-activated effects (field, hand, trash, breeding `<Training>`, delayed-Option), the name of the *matched* effect (e.g. `"Digiburst"`), resolved by mirroring the mask builder's first-match-wins selection. `None` when the action is not an effect activation or the matched effect carries no name.
- `payload: serde_json::Value` — kind-specific structured detail

For [Main]-activated effect actions, the decoder SHALL populate `effect_name` from the matched `Effect::name` and SHALL include that effect name in `label` when present.

The decoder SHALL use the existing `build_action_mask` infrastructure to determine legality; it MUST NOT re-implement legality logic.

`legal_actions(player)` SHALL return an empty `Vec` when `player != current_decision_player()`. Returned actions SHALL be executable by the caller via `step(action_id)` at the moment of the call — callers SHALL NOT receive phantom actions for inactive players or for phases where the player cannot act.

#### Scenario: Enumerate legal plays

- **WHEN** a caller invokes `LiveGame::legal_actions(0)` during player 0's main phase
- **THEN** every action whose bit is set in `build_action_mask(0)` appears in the returned vector with a corresponding `DecodedAction` entry

#### Scenario: Labels include card names

- **WHEN** the legal actions list contains a play of a card whose `name` is `"Agumon"`
- **THEN** the corresponding `DecodedAction.label` includes `"Agumon"`

#### Scenario: Labels include effect names for activated effects

- **WHEN** the legal actions list contains a field [Main] activation of a card whose matched effect is named `"Digiburst"`
- **THEN** the corresponding `DecodedAction.effect_name` is `Some("Digiburst")` and `DecodedAction.label` includes `"Digiburst"`

#### Scenario: Activated effect with no effect name omits it

- **WHEN** the legal actions list contains a [Main] activation whose matched effect has no name
- **THEN** the corresponding `DecodedAction.effect_name` is `None` and the label still identifies the source card

#### Scenario: Pending-selection legal actions

- **WHEN** a `PendingSelection` is active and `legal_actions` is called for the selecting player
- **THEN** every legal `ResolveSelection` action appears in the list, and unrelated actions (plays, attacks, etc.) do not

#### Scenario: Inactive player returns empty

- **WHEN** a caller invokes `LiveGame::legal_actions(player)` and `player != current_decision_player()`
- **THEN** the returned `Vec` is empty, regardless of what actions the player would have if it were their turn

#### Scenario: Returned actions are step-executable

- **WHEN** a caller takes any `DecodedAction` returned by `legal_actions(player)` and immediately invokes `LiveGame::step(action.action_id)`
- **THEN** the step succeeds (`ok == true`) and produces at least one event or a phase / selection change (no silent no-ops)
