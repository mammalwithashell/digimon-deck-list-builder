"""Stage executors of the card-authoring loop (design D1, D4-D7, D10, D12, D14).

One executor per state that does work; the driver looks the state up in
`EXECUTORS` and calls `run(ctx, item) -> StageOutcome` (`driver_contracts`).
"""
