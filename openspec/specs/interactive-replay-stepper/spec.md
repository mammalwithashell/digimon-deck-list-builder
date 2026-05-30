# interactive-replay-stepper

## Purpose

Defines the unified `ReplaySession` core that both the engine MCP and the `dcgo-replay` batch tool build on: the per-step apply policy (Trust vs CheckThenApply), pausable non-fatal divergence reporting, the divergence taxonomy, partial-observability surfacing for opaque games, and the single in-engine replay path shared by interactive and batch consumers.

## Requirements

### Requirement: Per-Step Policy

`ReplaySession` SHALL support a `StepPolicy` selecting how each recorded action is applied:

- `Trust` — apply the recorded action directly via `decode_action` without a legality pre-check; capture emitted events and the state delta. This is the default for `NativeAdapter` (self-play / eval recordings are engine-generated).
- `CheckThenApply` — before applying, verify the recorded actor equals `current_decision_player()` and that the recorded `action_id` is set in the engine's legal-action mask. This is the default for `DcgoAdapter` (differential replay against the battle-tested DCGO oracle).

The default policy SHALL be derivable from the adapter; callers MAY override it.

#### Scenario: Native default trusts the stream

- **WHEN** a `ReplaySession` is built from a native recording with no explicit policy
- **THEN** the session uses `Trust` and applies each recorded action without a mask pre-check

#### Scenario: DCGO default checks before applying

- **WHEN** a `ReplaySession` is built from a DCGO recording with no explicit policy
- **THEN** the session uses `CheckThenApply` and verifies actor + mask-membership before applying each action

### Requirement: Non-Fatal Pausing Divergence

Under `CheckThenApply`, when a recorded action fails the actor or mask-membership check, the session SHALL record a `Divergence` and **pause** at that step — it SHALL NOT apply the action and SHALL NOT abort the session. The caller MAY inspect, `seek` elsewhere, restore a checkpoint, or stop. Batch callers SHALL obtain the same verdicts by running to completion and reading the divergence log; the first recorded divergence SHALL map to the corresponding batch `ReplayOutcome::Fail` variant.

#### Scenario: Divergence pauses rather than aborts

- **WHEN** a `CheckThenApply` session reaches a step whose recorded action is not in the engine's legal mask
- **THEN** a `Divergence` is recorded, the action is not applied, the cursor remains at that step, and the session remains usable for inspection and seeking

#### Scenario: Interactive and batch verdicts agree

- **WHEN** the same recording is replayed interactively (stepping until the divergence log is non-empty) and in batch (`run_to_completion`)
- **THEN** the first interactive divergence and the batch `ReplayOutcome` describe the same step and the same disagreement

### Requirement: Divergence Taxonomy

A `Divergence` SHALL carry a `kind` drawn from a fixed taxonomy: `mask_miss` (recorded action not in the legal mask), `actor` (engine expected a different decision player), `memory` / `phase` / `winner` (replayed value differs from recorded, where the recording provides it), and the opaque-only kinds `reveal_kind` (engine requested a different reveal kind than the queue's next entry) and `reveal_exhausted` (engine requested a reveal with none remaining). A `mask_miss` divergence SHALL include a sample of the engine's legal action IDs at that step. The taxonomy SHALL note that detection is one-directional: it cannot flag an action the engine over-permits relative to DCGO.

#### Scenario: mask_miss carries a legal sample

- **WHEN** a `mask_miss` divergence is produced
- **THEN** it includes the recorded `action_id` and up to ~10 legal action IDs the engine would have accepted at that step

#### Scenario: opaque reveal divergences are distinguished

- **WHEN** an opaque replay encounters a reveal-kind disagreement versus a reveal-queue exhaustion
- **THEN** the two are reported as `reveal_kind` and `reveal_exhausted` respectively, each naming the step

### Requirement: Partial-Observability Surfacing

For opaque games, the view layer SHALL surface the engine's `is_opaque_placeholder` flag on hidden cards in `PermanentView`, `SecurityView`, and `HandView`, so a consumer reads "hidden" rather than a placeholder card identity. A card that is `is_opaque_placeholder` SHALL NOT be presented with a concrete `card_id` until the reveal stream materializes it.

#### Scenario: Hidden opponent cards read as hidden

- **WHEN** an opaque-game view is requested for an unrevealed opponent zone
- **THEN** the hidden entries are marked as placeholders and do not expose a concrete card identity

#### Scenario: Revealed cards read concretely

- **WHEN** the reveal stream has materialized a previously hidden card
- **THEN** the corresponding view entry reports the concrete `card_id` and is no longer marked as a placeholder

### Requirement: Single Replay Path For Batch And Interactive

The DCGO `RecordingV1` parser SHALL live in `digimon-engine` so both the engine MCP and the `dcgo-replay` batch tool build a `DcgoAdapter` from the same code. `dcgo-replay` SHALL be reduced to a batch driver that constructs a `ReplaySession`, runs it to completion, and maps the divergence log to its existing `ReplayOutcome` and parity-report output. The parity-report output SHALL remain byte-stable (the existing determinism test SHALL continue to pass).

#### Scenario: Parity report unchanged after unification

- **WHEN** the unified `dcgo-replay` driver replays a corpus that the pre-unification tool also replayed
- **THEN** the per-card parity report is byte-identical to the pre-unification output for the same corpus

#### Scenario: One parser feeds both consumers

- **WHEN** the engine MCP loads a DCGO recording and `dcgo-replay` loads the same recording
- **THEN** both construct the game through the same in-engine `DcgoAdapter` code path
