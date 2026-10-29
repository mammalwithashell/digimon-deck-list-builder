## Context

Search generations collect games under frozen weights, then fit a policy. PPO already attaches CardEvalCollector to the inner DigimonEnv during WinRateCallback. Anchored evaluation uses that same environment but lacks collector wiring. The live cloud training container has no network, and its separate W&B monitor owns publication.

## Goals / Non-Goals

**Goals:** shared card semantics, inspectable periodic reports, explicit checkpoint/progress identity, consistent fixed evaluation panels, isolated cloud deployment, durable retryable publication.

**Non-Goals:** measuring search-tree hypothetical actions, changing generation size or training targets, claiming repeated frozen-policy windows show learning, or changing the existing PPO cadence.

## Decisions

1. Extend anchored evaluation with an optional collector and per-game contexts. Player 1 remains the candidate. Group by panel and anchor as well as matchup, preserving old PPO reports. Require the exported engine event version before collection. On exceptions, retain partial reports and mark incomplete games, never successful zero metrics.
2. Run a separate worker against read-only experiment files. Defaults: startup, every 7200 seconds, each new stable model, and final completion. Each window uses 24 games per anchor per panel, fixed seed/decks and alternating first player. This is diagnostic sampling, not a promotion gate. A frozen checkpoint's repeated deterministic windows are labeled unchanged and have distinct window IDs; they must not be pooled into independent evidence. A persistent schedule prevents duplicate restarts and catches new checkpoints before the interval expires.
3. Copy and hash a stable ZIP into private evaluation storage before loading; hash snapshots/anchors and record evaluation mode `policy_only`, generation/checkpoint index, collected training games, elapsed time and candidate hash. Checkpoint index is the chart step; collection progress is a separate field. Existing generation checkpoints are the update boundaries; no intrusive per-epoch fitting hooks.
4. Worker executes one bounded subprocess at a time, saves reports before publication, limits CPU/memory, retries after an interval on failure, and never writes training files. The W&B monitor is the only network writer. It publishes reports with its own monotonically increasing W&B steps, tracks confirmed artifact uploads, and waits for final evaluator status before finishing. PPO publication retains implicit W&B steps.
5. Add dashboard filters for panel, anchor, mode and window so incompatible panels and repeated same-checkpoint windows are not silently combined.

## Risks / Trade-offs

- Additional CPU load → low CPU share and one-CPU limit; no search budget reduction. Wall-clock training can slow modestly while evaluations run.
- Pathological games → subprocess timeout, incomplete report/failure status, no training restart.
- Sparse per-card samples → expose denominators and diagnostic labels, keep full end-of-generation strength eval unchanged.
- Partial checkpoint writes → stability delay, ZIP validation, immutable copied hash before loading.
- Upload interruption → local reports retained, successful artifacts deduplicated; a crash between remote acknowledgment and local commit can duplicate an artifact, with stable window ID for identification.

## Migration Plan

Test local collector and schedule/publisher behavior, then run a small real-engine panel. Deploy Python overlay separately with source hashes on the existing matching engine image. Start the isolated evaluator and restart only the logger. Rollback stops evaluator and reverts logger overlay; training continues unchanged.
