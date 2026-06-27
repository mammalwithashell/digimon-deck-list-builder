## 1. Warm-start resolver (core change, test-first)

- [x] 1.1 Add a unit test for warm-start resolution: round 1 → generalist; round k>1 under `accumulate` → `<save-dir>/<deck>/r{k-1}` latest checkpoint; under `champion` → registry champion; missing `r{k-1}` ckpt under `accumulate` → registry champion + warning. (Pure path logic; no training run needed.)
- [x] 1.2 Implement a `_resolve_warmstart(spec, deck, rnd, registry)` helper that returns the init-from path per 1.1, replacing the inline `init_from = registry[deck]` in the round driver.
- [x] 1.3 Thread the resolved path into `build_specialist_argv` (it currently receives `init_from`); confirm the gate/registry/pool-emission code paths are untouched.

## 2. CLI flag + spec threading

- [x] 2.1 Add `warmstart: str = "accumulate"` to `LeagueSpec`; add `--warmstart {accumulate,champion}` (default `accumulate`) to the argparser and wire `warmstart=args.warmstart` into the `LeagueSpec` construction.
- [x] 2.2 Validate the flag value; document the A/B intent (`champion` reproduces pre-change behavior) in the flag help.

## 3. Checkpoint retention / round-chain safety

- [x] 3.1 Add a test that `keep_last_per_deck` retention never deletes a round's final checkpoint before the next round's warm-start consumes it (the round-chain link is preserved).
- [x] 3.2 Adjust retention so non-promoting decks' final accumulated checkpoint survives the run (available for post-hoc `anchored_eval_cli`); the per-deck deliverable remains the gated registry champion (unchanged).

## 4. Pool-invariant guard

- [x] 4.1 Add/extend a test asserting the opponent pool is built from gated registry champions only (best-known per deck) — i.e. accumulating the warm-start does NOT change which snapshots become opponents.

## 5. Docs

- [x] 5.1 Fix the `train_specialist_league.py` module docstring + round-loop comments to describe accumulate-from-own-checkpoint (so "warm-started from its round-(k-1) checkpoint" is accurate) and note the `--warmstart` toggle.
- [x] 5.2 Update `docs/TRAINING_RUNBOOK.md` (deck-specialist-league section) + the relevant memory: warm-start accumulates by default; gate governs pool/registry only; pass `--warmstart champion` for legacy.

## 6. Verification

- [x] 6.1 `--dry-run` the league for ≥2 rounds and confirm the emitted per-specialist argv shows `--init-from <deck>/r{k-1}/...` for a (simulated) kept deck under `accumulate`, and `--init-from <registry champion>` under `champion`.
- [x] 6.2 Run the full test suite for the league driver; confirm gate/registry/matrix behavior is unchanged (only warm-start resolution differs).

## 7. Reconciliation note

- [x] 7.1 At apply time, reconcile the `deck-specialist-league` base spec: it currently lives in the in-progress `add-deck-specialist-league` change (not archived). Either fold this delta into that change or archive it first so this change's MODIFIED requirement applies cleanly.
