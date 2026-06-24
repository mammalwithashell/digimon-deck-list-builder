# deck-specialist-league Specification

## Purpose
TBD - created by archiving change add-deck-specialist-league. Update Purpose after archive.
## Requirements
### Requirement: Per-deck specialist training scoped and warm-started

The league SHALL train one specialist per configured deck, each run scoped to
exactly one deck, and each round SHALL warm-start from the deck's **own latest
round checkpoint** — accumulating experience across rounds — independently of the
promotion gate. Round 1 SHALL warm-start from the provided generalist checkpoint
(the deck has no prior round); round k>1 SHALL warm-start from the deck's round
k-1 checkpoint. The warm-start source MUST NOT be the registry champion, so a
deck that fails the gate ("kept") still continues its own training chain rather
than restarting from the generalist. Specialist artifacts SHALL be written under a
per-deck path (`models/specialists/<deck>/`). A `--warmstart` option SHALL select
`accumulate` (default — own checkpoint) or `champion` (legacy — registry
champion).

#### Scenario: Specialist is deck-scoped and warm-started from the generalist in round 1
- **WHEN** the league launches round 1 for deck `ST-1 Gaia Red`
- **THEN** that run is scoped to `ST-1 Gaia Red` only and is initialized from the generalist checkpoint, writing to `models/specialists/st-1/r1/`

#### Scenario: A non-promoting deck accumulates across rounds
- **WHEN** deck `ST-1 Gaia Red` trains round 2 after its round 1 failed the promotion gate (was "kept")
- **THEN** round 2 SHALL warm-start from `models/specialists/st-1/r1`'s final checkpoint (continuing the round-1 weights), NOT from the generalist, so the round-1 training is not discarded

#### Scenario: Gate still governs the pool, not the warm-start
- **WHEN** a kept deck trains a later round under `--warmstart accumulate`
- **THEN** its warm-start is its own latest checkpoint, while its opponents are still drawn from the **gated registry champions** (the best-known per-deck snapshots), so accumulating the warm-start never injects in-progress weights into any opponent pool

#### Scenario: Legacy warm-start is reproducible
- **WHEN** the league is launched with `--warmstart champion`
- **THEN** each round warm-starts from the registry champion for that deck (the pre-change behavior), enabling A/B comparison against `accumulate`

#### Scenario: Missing prior checkpoint falls back safely
- **WHEN** round k>1 under `--warmstart accumulate` cannot locate the deck's round k-1 checkpoint (e.g. a `--from-round` resume or retention pruning)
- **THEN** the league SHALL warm-start from the registry champion with a clear warning rather than aborting, so correctness is preserved and accumulation is best-effort

### Requirement: Round-based training against a frozen snapshot pool

The league SHALL be organized into rounds. Within a round, every specialist SHALL train only against a **frozen** pool of snapshots (it MUST NOT train against the live, currently-updating weights of other specialists). At the end of each round the league SHALL snapshot every specialist into the pool before the next round begins.

#### Scenario: A round trains against frozen snapshots
- **WHEN** round N is in progress
- **THEN** each specialist's opponents are fixed snapshots taken at the start of the round (or earlier), and no specialist observes another specialist's mid-round weight updates

#### Scenario: Snapshots are taken at the round barrier
- **WHEN** round N completes for all specialists
- **THEN** the league records a snapshot of each specialist into the pool, and round N+1 samples from the updated pool

### Requirement: PFSP opponent sampling

The league SHALL sample each specialist's opponents from the frozen pool using prioritized fictitious self-play (PFSP) — opponents the specialist is losing to more often SHALL be sampled more frequently — reusing the existing `LeagueOpponentWrapper` PFSP mode.

#### Scenario: Losing matchups are up-weighted
- **WHEN** a specialist's tracked win rate against pool opponent A is lower than against opponent B
- **THEN** opponent A SHALL be sampled with higher probability than opponent B

### Requirement: Mirror coverage

Each specialist's opponent pool SHALL include frozen snapshots piloting the specialist's **own** deck, so the specialist trains its mirror matchup.

#### Scenario: Own-deck snapshots are in the pool
- **WHEN** the pool for the `ST-4 Giga Green` specialist is assembled for a round
- **THEN** it contains at least one frozen snapshot that pilots `ST-4 Giga Green`

### Requirement: Deck-keyed specialist registry

The league SHALL maintain a registry mapping each deck to its current specialist, including `weights_path`, `algorithm`, `observation_profile`, `tensor_layout_hash`, and round index. The round's opponent pool SHALL be emitted from this registry, and downstream consumers (next round, evaluation, deployment) SHALL resolve a specialist by deck through it.

#### Scenario: Registry resolves a specialist by deck
- **WHEN** a consumer requests the specialist for deck `ST-6 Venomous Violet`
- **THEN** the registry returns its checkpoint path plus the algorithm and tensor-layout-hash needed to load it

#### Scenario: Layout-incompatible snapshot is rejected from a pool
- **WHEN** a snapshot's `tensor_layout_hash` does not match the league's active observation layout
- **THEN** it SHALL NOT be placed in an opponent pool used for training or evaluation

### Requirement: Concede is unavailable to league agents

Specialist training and league evaluation SHALL run on the concede-disabled action mask (no `CONCEDE_GAME` action), so specialists cannot learn or be credited with premature surrender.

#### Scenario: Concede never appears in a league mask
- **WHEN** any specialist or pool opponent reaches a decision point during a league game
- **THEN** the action mask does not expose the concede action

### Requirement: Standing evaluation by anchored win rate and matchup matrix

The league SHALL judge progress with seat-balanced anchored evaluation, not the in-run win rate. Each round SHALL produce, per specialist, anchored win rates against fixed references (greedy plus the other specialists' frozen snapshots) and SHALL assemble a deck-by-deck matchup matrix. Promotion of a specialist's "current" checkpoint into the registry SHALL be decided from this anchored frame.

#### Scenario: Round emits the matchup matrix
- **WHEN** round N's evaluation completes
- **THEN** a deck-by-deck matrix of seat-balanced win rates (each cell with its sample count) is recorded for that round

#### Scenario: Promotion uses anchored eval, not in-run win rate
- **WHEN** selecting which checkpoint becomes deck X's registry entry for the next round
- **THEN** the choice is made from the seat-balanced anchored win rate, never from the in-run training win rate

### Requirement: Orchestration runs parallel or sequentially

The league orchestrator SHALL support running a round's specialists either in parallel (separate processes/hosts) or sequentially on a single host, producing equivalent registry and snapshot state at the round barrier regardless of mode.

#### Scenario: Sequential and parallel rounds converge to the same barrier state
- **WHEN** a round is run sequentially on one host versus in parallel across hosts
- **THEN** both produce a snapshot of every specialist in the pool and an updated registry at the round barrier (the per-specialist results may differ, but the barrier contract is identical)

### Requirement: Accumulated specialist checkpoints retained for evaluation

The league SHALL retain each deck's accumulated round checkpoints such that a
non-promoting deck's trained specialist is not discarded and remains available on
disk for post-hoc anchored evaluation. Checkpoint retention (`keep_last_per_deck`)
MUST NOT delete a round's final checkpoint before the next round's warm-start has
consumed it. The per-deck **deliverable** SHALL remain the gated registry champion
(the model proven to win ≥0.55 head-to-head); this requirement only preserves the
accumulated specialist alongside it, it does not auto-ship sub-gate models.

#### Scenario: Kept deck's accumulated specialist survives the run
- **WHEN** deck `ST-1 Gaia Red` completes all rounds without ever promoting
- **THEN** its latest accumulated checkpoint remains under `models/specialists/st-1/` and can be scored by `anchored_eval_cli`, while the registry's deliverable for `ST-1 Gaia Red` stays the generalist (the gated best)

#### Scenario: Retention preserves the warm-start link
- **WHEN** per-deck checkpoint retention runs between rounds
- **THEN** it SHALL NOT remove the round k-1 final checkpoint that round k needs to warm-start from

