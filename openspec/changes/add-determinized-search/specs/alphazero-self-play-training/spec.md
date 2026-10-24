## ADDED Requirements

### Requirement: Scheduling preserves seeded games and supports explicit recovery
Dynamic scheduling SHALL assign global game indices independent of workers and
preserve serial deck sampling, game/search seeds, legal actions, search budgets,
noise, temperature and training targets. Only completed chunks SHALL become
visible to training. Explicit resume SHALL validate frozen inputs and committed
output hashes, permit changed worker counts, and reject incompatible settings.
Workers SHALL have bounded retries/deadlines, controller ownership, heartbeat
reporting and cleanup on interruption on Windows and Linux.

#### Scenario: Worker scaling preserves all targets
- **WHEN** the same master seed, global game range, model and search settings run with different worker counts
- **THEN** every observation, sparse policy, mover, game ID and terminal label is identical in global game order

#### Scenario: An interrupted chunk is not training data
- **WHEN** a worker or controller stops before a chunk is published
- **THEN** completed chunks are retained, incomplete attempts remain private, and an explicit compatible resume reruns only uncommitted chunks

#### Scenario: Changed inputs or damaged output reject resume
- **WHEN** a model, executable, card/admission data, deck pool, search setting or committed output hash changes
- **THEN** resume fails before launching workers or consuming the changed data

### Requirement: Self-play produces MCTS-improved training targets
Self-play SHALL generate training records of `(infoset-observation, search-policy π, game-outcome z)`, where π is the MCTS root-visit distribution at each decision and z is the eventual game result from the deciding player's perspective.

#### Scenario: A self-play game yields per-decision targets
- **WHEN** a self-play game completes
- **THEN** each decision point contributes a record whose policy target is the search's visit distribution and whose value target is the game's terminal outcome for that player

### Requirement: Promotion decisions use the anchored frame, not in-run win rate
A trained generation SHALL be ranked and promoted only via anchored evaluation (seat-balanced vs greedy + frozen champions) and the Elo ladder, never via an in-run / self-play / mirror win rate (per rule 30).

#### Scenario: Promotion ignores the degenerate self-play eval
- **WHEN** a generation reports a high self-play/mirror win rate but underperforms greedy + champions on the anchored frame
- **THEN** it is NOT promoted

### Requirement: Robustness is reported as a measured lower bound
Each generation SHALL be run through the forward-only exploiter and its exploitability reported as a lower bound at a stated exploiter budget; the system SHALL NOT claim Nash or certified low exploitability.

#### Scenario: Exploitability is reported honestly
- **WHEN** a generation's robustness is reported
- **THEN** the report states the exploiter budget and labels the number an approximate lower bound, not a certificate
