## Why

Search self-play currently faces only its current checkpoint. EX13 diagnostics found regression against older anchors, but have not established opponent diversity as the cause. A reproducible historical-opponent option lets us test that hypothesis without changing search quality or silently training on old policies' targets.

## What Changes

- Add opt-in weighted historical ONNX opponents, mixed with current-model self-play.
- Freeze and hash the pool for each generation, including explicitly pinned checkpoints and a recent-generation window.
- Select opponents deterministically across worker scheduling and retries; balance the learner's opening order over game pairs.
- Route each real actor's decisions to its own evaluator with unchanged search settings, emitting only learner rows against history.
- Record identities, assignments, outcomes and row counts; reject incompatible or mutated artifacts.
- Expose the feature in generation and cloud-plan configuration, with tests and operator documentation. Leave the live run stopped.

## Capabilities

### New Capabilities

- `search-checkpoint-opponents`: Reproducible search training against frozen historical models.

### Modified Capabilities

None. Default self-play and existing observation/action contracts remain unchanged.

## Impact

Native self-play driver and CLI, Python generation orchestration and scheduler identities, cloud controller options, tests, and training documentation. No training deployment or automatic model promotion.
