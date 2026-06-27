## ADDED Requirements

### Requirement: Search operates on determinized worlds via clone-and-step
The search SHALL build its tree by cloning a determinized world and applying `step(action_id)` for legal actions only (those set in the action mask), with nodes corresponding to decision points (`current_player_id` + mask) and edges to masked action ids. The search SHALL NOT read opponent-hidden zone identities outside the materialized world it is searching.

#### Scenario: Only masked actions are expanded
- **WHEN** the search expands a node
- **THEN** every edge corresponds to an `action_id` that is set in that state's action mask, and no illegal action is explored

#### Scenario: No hidden-information leakage
- **WHEN** the search runs on a determinized world
- **THEN** all hidden card identities used during rollouts come from that world's committed piles, not from any other player's true concealed state

### Requirement: Policy+value evaluator over the existing tensor and action space
The search SHALL guide selection and leaf evaluation with an evaluator mapping the engine's observation tensor to a masked policy over the 2192-action space and a scalar value for the player to move.

#### Scenario: Policy respects the mask
- **WHEN** the evaluator returns a policy for a state
- **THEN** probability mass on masked-out (illegal) actions is zero after masking

### Requirement: PIMC and IS-MCTS aggregation modes
The search SHALL support both PIMC (K independently sampled worlds searched separately, root visit counts summed) and IS-MCTS (per-iteration re-determinization sharing an infoset-keyed tree), selectable by configuration.

#### Scenario: PIMC aggregates across worlds
- **WHEN** PIMC search runs with K worlds
- **THEN** the returned policy is derived from root visit counts summed across all K world searches

### Requirement: Search finds the optimal line on perfect-information games
On a fully-known (already-determinized) game with a known optimal line, the search with adequate budget SHALL select that line.

#### Scenario: Known win is found
- **WHEN** the search runs with sufficient simulations on a perfect-information position with a forced win
- **THEN** the search's preferred action lies on the winning line
