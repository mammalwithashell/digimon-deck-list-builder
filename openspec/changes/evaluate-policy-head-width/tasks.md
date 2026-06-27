## 1. Land the head-width knobs (mostly done this session, uncommitted)

- [x] 1.1 Add `net_arch: Optional[List[int]]` + `init_extractor_from: Optional[str]` to `TrainingConfig` with validation (positive ints; mutual exclusion with init_from/resume_from).
- [x] 1.2 Add `--net-arch` + `--init-extractor-from` CLI args and wire into the config-overrides dict in `pilot_training`.
- [x] 1.3 Apply `net_arch` in both model-construction paths (MLP + LSTM) via `_head_arch`; default `None` → SB3 `[64,64]`.
- [x] 1.4 Implement extractor-only warm-start: load shape-matching `features_extractor.*` tensors from `--init-extractor-from` into the fresh model; raise if none match.
- [x] 1.5 Manually verify: `--net-arch 256,256` builds `512→256→256`; `--init-extractor-from <seed>` warm-starts 15 extractor tensors; config validation rejects bad inputs.
- [ ] 1.6 Add unit tests under `code/tests/rl/` (net_arch validation, mutual exclusion, parsed `[256,256]` produces the expected heads, extractor-warm-start transfers extractor + leaves heads fresh).
- [ ] 1.7 Thread `--net-arch` through `code/tools/train_specialist_league.py` (LeagueSpec field + `--set net_arch=...` per specialist).
- [ ] 1.8 Commit the change (worktree branch) once tests pass.

## 2. Experiment setup

- [ ] 2.1 Build a champion pool manifest from the league2 champions in `models/specialists/` (`python code/tools/champion_admin.py emit-pool ...`).
- [ ] 2.2 Wire the league2 champions as anchored-eval references (a `models/champions/registry.json` pointing at the 6 champions) so the anchored panel has headroom beyond greedy.
- [ ] 2.3 Pick the comparison budget (start 300k/arm; extend if both still climbing) and venue (local ~1–1.5h, or the idle box).

## 3. Run the comparison

- [ ] 3.1 Train arm A `[64,64]`: `--net-arch 64,64 --init-extractor-from <seed> --opponent pool --opponent-pool-manifest <champions> --anchored-eval-freq 50000`.
- [ ] 3.2 Train arm B `[256,256]`: same as A but `--net-arch 256,256`.
- [ ] 3.3 Post-hoc anchored eval (adequate n, seat-balanced) of both final checkpoints vs greedy + champions.

## 4. Verdict + follow-up

- [ ] 4.1 Record the verdict (wider better / neutral / worse) with the anchored numbers; update memory.
- [ ] 4.2 If positive, make the chosen width the league default; if neutral/negative, log that head width is not the lever and point at the extractor-architecture follow-up (attention/set-encoder).
- [ ] 4.3 (Optional fast-follow) add an `n_epochs` sample-efficiency arm using the same protocol.
