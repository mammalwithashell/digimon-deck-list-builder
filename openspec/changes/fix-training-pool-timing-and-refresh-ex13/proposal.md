## Why

The 13-archetype training audit reproduced printed End of Attack effects firing
after another Digimon's attack, including Ruin Mode in Three Musketeers. The
evaluation pool also selected lists by recency alone and used one list per
archetype, limiting interpretation of its divergence from tournament results.

## What Changes

- Repair printed timing scope through shared engine/DSL primitives, preserving
  broad observers, legal choices, stable identity and clone behavior.
- Audit other timing classes in the frozen pool and refreshed list additions;
  reproduce and fix concrete additional defects with focused regression tests.
- Prepare a versioned global EX13 pool using recent competitive exact lists,
  targeting two training and two distinct held-out lists per archetype.
- Preserve original pet lists as a separate regression panel and expose any
  sourcing/admission shortfall. Do not silently replace cards or duplicate lists.
- Keep the current cloud run unchanged as diagnostic data; no automatic rollout,
  restart, retraining or promotion is part of this change.

## Capabilities

### New Capabilities

- `printed-timing-scope`: Correct source/controller/event identity for printed
  timing triggers, with controls for broad observers and interrupted resolution.
- `representative-ex13-pool`: Reproducible, provenance-rich selection of distinct
  recent EX13 training/evaluation lists across the existing 13 archetypes.
- `training-pool-timing-audit`: Explicit inventory, behavioral evidence and
  unresolved timing coverage for a frozen card pool.

### Modified Capabilities

None. Observation/action profiles and training search budgets are unchanged.

## Impact

Rust trigger collection/lowering and focused tests; Python pool preparation and
admission checks; versioned pool assets, audit evidence and operational docs.
Existing checkpoints remain shape-compatible, but repaired-engine results must
be kept separate from prior diagnostic results. No live cloud mutation.
