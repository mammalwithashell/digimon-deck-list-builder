## ADDED Requirements

### Requirement: Per-decision move-quality annotation
The system SHALL annotate each decision in a recorded game with a quality grade derived from the win-probability delta between the move played and the search's preferred move, using thresholds mapping to brilliant / great / best / inaccuracy / mistake / blunder.

#### Scenario: A clearly inferior move is graded a blunder
- **WHEN** the search strongly prefers a different move and the played move drops win probability beyond the blunder threshold
- **THEN** the decision is annotated "blunder" with the played value, best value, and best action recorded

#### Scenario: The brilliant heuristic
- **WHEN** the played move is the search-best, carries a low policy prior (non-obvious), and is the only move preserving the win
- **THEN** the decision is eligible for the "brilliant" annotation

### Requirement: Decision-time (belief-aware) and hindsight grades are distinct
The system SHALL compute a decision-time grade (belief-aware: averaged over worlds sampled from the player's infoset, requiring a deck prior) and a hindsight grade (evaluated on the single true recorded world, requiring no prior), and SHALL not conflate them.

#### Scenario: A move fair at decision time but unlucky in hindsight
- **WHEN** a move is strong across worlds sampled from the player's infoset but loses only because the opponent held a specific concealed counter
- **THEN** the decision-time grade is favorable while the hindsight grade reflects the actual loss, and both are surfaced separately

#### Scenario: Hindsight grade needs no prior
- **WHEN** annotating a recording for which no opponent deck prior is available
- **THEN** the hindsight grade is still computable from the true recorded world
