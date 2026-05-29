## ADDED Requirements

### Requirement: DSL can observe effect-created digivolution-source placement
The card-scripting DSL SHALL expose a trigger and event context for cards placed into a Digimon's digivolution sources by an effect. The trigger context MUST include the placed card, the host permanent, the host controller, the placed card owner, and whether the placement was caused by an effect, so CS cards can react to "[CS] trait Digimon cards" placed in their stacks without card-local hooks.

#### Scenario: Effect places a CS Digimon source
- **WHEN** an effect places a Digimon card with the `[CS]` trait into a Digimon's digivolution cards
- **THEN** DSL triggered clauses listening for source placement are queued for eligible observers
- **AND** predicates can match the placed card's `[CS]` trait
- **AND** predicates can match the host permanent carrying the new source

#### Scenario: Non-effect source placement does not satisfy effect-created predicate
- **WHEN** a source is present because of normal digivolution or initial test setup
- **THEN** clauses requiring effect-created source placement do not fire

#### Scenario: Source-placement observer uses pending selections
- **WHEN** a source-placement observer offers a follow-up play, digivolve, or suspend cost
- **THEN** every legal follow-up choice is surfaced through `PendingSelection`

### Requirement: DSL supports choice-shaped security costs
The DSL SHALL support a security-card cost gate that lets the player choose whether to trash the top or bottom card of their security stack when both positions are legal. The selected trash action SHALL gate the follow-up body exactly like other costs: declined, impossible, or prevented costs skip the body.

#### Scenario: Both top and bottom security are legal
- **WHEN** a clause declares a top-or-bottom security trash cost and the controller has at least two security cards
- **THEN** the controller is offered a visible choice between top security and bottom security
- **AND** the clause body runs only after the selected card is successfully trashed as the cost

#### Scenario: Only one security card exists
- **WHEN** a clause declares the same cost and the controller has exactly one security card
- **THEN** the cost can be paid by trashing that card
- **AND** the clause body runs only if the trash succeeds

#### Scenario: Security cost is declined or impossible
- **WHEN** the controller declines the cost or has no security cards
- **THEN** no security card is trashed
- **AND** the clause body does not run

### Requirement: DSL supports aggregate play-cost budget zone selection
The DSL SHALL support selecting multiple visible-zone cards whose total play cost is less than or equal to a specified budget, then playing those selected cards from their true origin zones without paying their costs. The selection MUST expose all legal subsets incrementally without greedy auto-selection.

#### Scenario: Player selects cards within budget
- **WHEN** an effect allows playing up to a total play cost from trash
- **THEN** the player can choose eligible trash cards while the remaining play-cost budget is tracked
- **AND** selected cards are played from trash without paying their costs after the selection completes

#### Scenario: Candidate exceeds remaining budget
- **WHEN** a candidate card's play cost is greater than the remaining budget
- **THEN** that candidate is not offered as a legal next pick

#### Scenario: Player picks fewer than maximum budget
- **WHEN** the player stops before spending the full play-cost budget
- **THEN** only the selected cards are played
- **AND** unselected eligible cards remain in their original zone

### Requirement: DSL can express conditional attack and timing suppression
The DSL SHALL support applying attack restrictions and triggered-effect timing suppression to only the permanents that satisfy a predicate. The capability MUST be usable for Venusmon-style text that affects opponent Digimon with Security Attack and prevents specific attacks or `[When Attacking]` / `[When Digivolving]` activations.

#### Scenario: Opponent Digimon has Security Attack
- **WHEN** a conditional lock targets opponent Digimon with Security Attack
- **THEN** only matching opponent Digimon receive the attack or timing suppression

#### Scenario: Non-matching opponent Digimon remains legal
- **WHEN** an opponent Digimon does not satisfy the lock predicate
- **THEN** its legal attacks and triggered effects are not suppressed by that lock

#### Scenario: Suppression affects masks and queued triggers
- **WHEN** a suppressed Digimon would attack a forbidden target or queue a suppressed timing
- **THEN** the action mask omits the forbidden attack
- **AND** the effect queue does not enqueue the suppressed timing for that permanent

### Requirement: DSL can express temporary original-name mutation
The DSL SHALL support temporarily changing a permanent's rules-visible original name for effects that transform a Digimon into a named Digimon until an expiry point. The mutation MUST participate in name predicates for the duration and expire cleanly.

#### Scenario: Temporary name mutation is active
- **WHEN** an effect changes an opponent Digimon's original name to `Sukamon` until the end of the opponent's turn
- **THEN** name predicates treat that permanent as `Sukamon` for the duration
- **AND** any accompanying color or base-DP modifiers remain scoped to the same expiry

#### Scenario: Temporary name mutation expires
- **WHEN** the expiry point is reached
- **THEN** the permanent's printed original name is restored for rules predicates
