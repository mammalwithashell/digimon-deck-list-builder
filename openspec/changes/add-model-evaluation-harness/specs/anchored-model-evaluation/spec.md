## ADDED Requirements

### Requirement: Fixed-reference evaluation tiers
The evaluation harness SHALL evaluate a policy against a fixed reference frame whose opponents do not change as the learner trains, organized into tiers: Tier 0 random, Tier 1 greedy (scripted, observation-profile-agnostic), Tier 2 frozen champion model snapshots, and Tier 3 held-out scenarios with pinned decks and seeds.

#### Scenario: Anchored evaluation does not move with the learner
- **WHEN** the same policy checkpoint is evaluated against the anchored suite at two different times
- **THEN** the reference opponents (random, greedy, the named champions, the pinned scenario decks/seeds) are byte-for-byte identical across both evaluations

#### Scenario: Greedy tier is profile-agnostic
- **WHEN** the anchored suite evaluates two policies trained under different observation profiles against the Tier 1 greedy opponent
- **THEN** both face the same greedy opponent and their greedy win rates are reported on the same scale

### Requirement: Seat-balanced matchups
Every anchored matchup SHALL be played from both seats (the evaluated policy as first player and as second player) with results averaged, so first-player advantage does not bias the reported metric.

#### Scenario: Both seatings are played
- **WHEN** the suite runs a matchup with N games per cell
- **THEN** the policy plays an equal number of games as first player and as second player, and the reported win rate is the seat-averaged value

### Requirement: Frozen-model anchors in the held-out suite
The held-out evaluation suite SHALL support a frozen model snapshot as an opponent kind (loaded from a saved SB3 checkpoint), in addition to the existing greedy and random opponent kinds, and SHALL verify the snapshot's observation profile and tensor-layout hash are compatible before play.

#### Scenario: Incompatible champion is rejected
- **WHEN** a held-out suite references a frozen-model anchor whose tensor-layout hash differs from the evaluated policy's profile
- **THEN** the suite fails fast with an error naming both layout hashes rather than producing a misleading score

#### Scenario: Compatible champion is played
- **WHEN** a held-out suite references a frozen-model anchor with a matching layout hash
- **THEN** the policy is evaluated against that frozen snapshot with seat-balanced matchups

### Requirement: Default anchored suite for runs
Training runs SHALL be able to enable, via configuration, a default anchored suite consisting of greedy plus the current champion panel, evaluated on a stable schedule, with results written alongside the existing eval artifacts.

#### Scenario: Default anchored suite emits a stable signal under self-play
- **WHEN** a self-play run has the default anchored suite enabled
- **THEN** the run records an anchored win rate vs greedy (and champions) that is independent of the degenerate mirror win rate

### Requirement: Champion registry
The harness SHALL maintain an explicit, versioned registry of frozen champion model snapshots, each recording its name, source run and checkpoint, observation profile, tensor-layout hash, and creation date, serving as permanent evaluation benchmarks.

#### Scenario: A champion is registered with provenance
- **WHEN** a model is added to the champion registry
- **THEN** the registry entry records its name, source run/checkpoint, observation profile, tensor-layout hash, and creation date

### Requirement: Gated champion promotion
The harness SHALL support a promotion rule under which a candidate model is added to the champion registry only if it beats the current champion panel by at least a configurable margin (default 55%) over a seat-balanced match, and SHALL also allow explicit manual promotion.

#### Scenario: Candidate below the gate is not promoted
- **WHEN** a candidate wins fewer than the configured margin of seat-balanced games against the champion panel
- **THEN** the candidate is not added to the registry and the existing champions are unchanged

#### Scenario: Candidate above the gate is promoted
- **WHEN** a candidate wins at least the configured margin against the champion panel
- **THEN** the candidate is added to the registry as a new champion with full provenance
