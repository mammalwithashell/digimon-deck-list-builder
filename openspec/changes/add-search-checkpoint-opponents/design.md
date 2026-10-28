## Context

The native driver uses one frozen evaluator for both players. Python's generation runner already freezes scheduler inputs, preserves failed attempts and pools past data. Reusing data is different from facing past policies. PPO's opponent pool does not configure this native path.

## Goals / Non-Goals

Provide explicit checkpoints and a rolling recent-generation pool with reproducible assignments and learner-only training targets. Preserve default self-play, search budgets, masks, acting-player observations and terminal value labels. No live deployment, promotion, PPO changes, opponent-dependent leaf inference, or new hidden-information claims.

## Decisions

- A versioned JSON pool carries a current-self-play probability and weighted opponents with stable IDs, ONNX paths and SHA256 identities for model and metadata. The native CLI validates hashes, the existing model contract, and terminal-value semantics before playing.
- Python freezes a generation-local manifest combining explicit pins and completed generations older than the current actor's checkpoint. Duplicate IDs/artifacts, empty requested pools and current-model aliases fail. Rolling history alone is unavailable at bootstrap until an older completed generation exists; supply explicit pins or start later.
- A separate seed stream selects an opponent once per global pair of games. The current model opens the first game of the pair and plays second in the other, using the game's existing randomized opening seat. Deck and game seeds remain unchanged; these are balanced opening-order assignments, not identical-deal evaluation pairs. Odd tails are reported honestly.
- At each real decision, the acting seat chooses its evaluator. That evaluator drives the entire existing search; both sides retain identical search parameters and temperature schedules. Each evaluator gets its own optional cache. This avoids changing the algorithm to opponent-model-aware tree evaluation.
- Keep the current driver's public entrypoint as a no-pool wrapper. Against history, omit only the historical actor's training rows, including navigation rows; still execute its complete searched commitment and count its decisions. Same-model games retain both seats' rows. Per-game receipts identify actor, opening seat, opponent, winner, truncation and row counts.
- Bind the frozen manifest and all model/metadata files into scheduler identity. Resume compares the requested pool to the frozen pool before scheduling; no history refresh during a generation. Reject historical pools in legacy static scheduling because its per-worker seed scheme is not globally stable.
- Cloud plans forward the same opt-in options and require their inputs in the plan's existing hash validation. No changes to default production plans or evaluation anchors.

## Risks / Trade-offs

- More resident models consume worker RAM; operators choose a bounded history window. No throughput gain is claimed.
- Historical games yield fewer learner rows per game; record rows and learner appearances, not just games, when comparing experiments.
- A weighted random mixture is not an exact per-opponent quota; paired assignments balance opening order, and aggregate receipts show realized coverage.
- More opponent diversity is a hypothesis, not an established cure for EX13 regressions. Fixed held-out anchors remain the strength measure.

## Migration Plan

Opt in only on a newly qualified run/image. Existing commands remain unchanged. Keep the current cloud run stopped. Roll back by starting a fresh generation without history; never mutate a partially collected generation's pool.
