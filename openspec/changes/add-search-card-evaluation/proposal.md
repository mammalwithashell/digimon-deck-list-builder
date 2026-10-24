## Why

Per-card telemetry currently runs only through PPO's periodic evaluation callback. Long search/self-play experiments need the same inspectable card usage, entry routes and opportunity/choice reports throughout collection and after fitting, without modifying search targets or restarting training.

## What Changes

- Reuse the inner-environment card collector in anchored policy evaluation, with explicit panel, anchor, checkpoint and failure provenance.
- Add an independent, bounded evaluator that runs at startup, every two hours by default, and for newly completed generation checkpoints.
- Publish atomic reports, W&B card/route tables and versioned artifacts through the existing monitor; extend dashboard filters to preserve evaluation scope.
- Label unchanged model evaluations and policy-only evaluation explicitly; retain PPO's existing cadence and report compatibility.

## Capabilities

### New Capabilities
- `search-card-evaluation`: Scheduled, checkpoint-aware card evaluation for search training runs, with shared reporting and cloud isolation.

### Modified Capabilities

None.

## Impact

Anchored evaluator/CLI, card telemetry publisher/dashboard, cloud evaluation worker and monitor, tests and training documentation. No engine, observation, action, reward, search budget or training cadence changes. Evaluation consumes bounded additional CPU and has no credentials or write access to training files.
