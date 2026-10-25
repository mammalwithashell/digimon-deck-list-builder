## Why

The search seed currently imitates one deterministic greedy policy. That policy
prioritizes setup plays before keep-turn digivolution and often declines optional
effect choices, which can narrow the examples available to modern decks. A
second teacher can demonstrate evolution and useful hand replenishment without
turning those preferences into the learner's reward objective.

## What Changes

- Add a separately selectable `development_v1` Rust heuristic that values
  efficient evolution, evolution-line completion, and net hand resources, while
  retaining combat, memory, legality, and finite-progress safeguards.
- Mix it with unchanged `greedy_v1` in a reproducible bootstrap corpus, selecting
  a teacher per seat per game rather than switching strategies at each action.
- Preserve teacher identities, seeds, deck hashes, route opportunities/choices,
  and provenance in bootstrap artifacts and checkpoint metadata.
- Compare greedy-only, development-only, and mixed seeds under equal bootstrap
  and search budgets before choosing a future experiment's seed.
- Leave the live cloud experiment, rewards, search settings, and tensor/action
  contracts unchanged. This change defines a future bootstrap option.

## Capabilities

### New Capabilities

- `development-heuristic`: legal, information-limited evolution/resource teaching
  policy with explicit pending-choice behavior and progress safeguards.
- `mixed-heuristic-bootstrap`: reproducible teacher mixtures, balanced coverage,
  provenance, and evaluation gates for search initialization.

### Modified Capabilities

None. Existing greedy defaults and generalist PPO training retain their contracts.

## Impact

Rust `policies/`, reusable read-only cost/action metadata, PyO3 headless policy
selection, `tools/english_meta_search_job.py`, the fresh-seed initializer, and
bootstrap telemetry/tests. Reuse existing card evaluation and terminal-outcome
training surfaces; do not introduce a second card-rules interpreter or a Python
engine fallback. Existing model and shard contracts remain valid.
