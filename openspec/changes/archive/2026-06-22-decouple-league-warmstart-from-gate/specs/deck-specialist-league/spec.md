## MODIFIED Requirements

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

## ADDED Requirements

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
