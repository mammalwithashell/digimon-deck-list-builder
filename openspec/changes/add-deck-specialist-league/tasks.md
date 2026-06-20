## 1. Reuse audit (verify before building)

- [x] 1.1 Read `LeagueOpponentWrapper` and confirm its PFSP mode (inverse-win-rate, per-opponent tracking) is usable standalone (outside the DB gauntlet path) — usable, but loads no policy (deck/PFSP only); see `notes/reuse-audit.md`
- [x] 1.2 Read `champion_registry` + `champion_admin.py emit-pool` and confirm the manifest schema can be reused/extended for a deck-keyed specialist registry — reusable; gap: `OpponentEntry` carries no `deck`
- [x] 1.3 Verify in `GauntletOrchestrator` which train/eval stage bodies are real vs placeholder (per the AGENTS.md caveat) and record what, if anything, is reusable — N/A, we go standalone; only reuse the PFSP sampler it drives
- [x] 1.4 Confirm `pilot_training` supports the needed flags together: `--archetypes <one deck>`, `--init-from`, `--opponent pool`, LR decay, and the v0.35 concede-disabled mask — all present; new league opponent mode needed to couple (policy, deck) per opponent

## 2. Specialist registry

- [x] 2.1 Define the deck-keyed registry schema (`deck → {weights_path, algorithm, observation_profile, tensor_layout_hash, round}`) and a loader/writer — `code/digimon_gym/agents/specialist_registry.py` (`Specialist`, `SpecialistRegistry`)
- [x] 2.2 Implement round-pool emission from the registry (latest snapshot per deck + retained history), including each deck's own snapshots for the mirror — `emit_round_pool` / `write_round_pool` emit `LeaguePoolEntry` carrying `deck`
- [x] 2.3 Enforce layout-hash compatibility: reject snapshots whose `tensor_layout_hash` mismatches the active layout from any pool — `compatible()` / layout filter in `emit_round_pool`; tested

## 3. Standalone league orchestrator

- [x] 3.1 Scaffold a standalone driver (no FastAPI/DB), modeled on `train_starter_curriculum.py` + the champion-loop snapshot cadence — `code/tools/train_specialist_league.py` (dry-run verified)
- [x] 3.2 Implement the round loop: assemble per-deck frozen pools → launch specialists → snapshot all into the registry at the round barrier → repeat — `run_round` + round-0 generalist seed
- [x] 3.3 Support parallel and sequential execution of a round's specialists, converging to identical barrier state (registry + snapshots) — `--topology sequential|parallel`, same barrier snapshot
- [x] 3.4 Add convergence controls: round size, max rounds, snapshot retention (generous — avoid losing peak checkpoints), and a plateau/stop rule keyed on the matchup matrix — `--steps-per-round`/`--rounds`/`--keep-last-per-deck` (default 2); matrix-keyed plateau stop deferred to Group 5

## 4. Specialist run wiring

- [x] 4.1 Launch a single specialist as a deck-scoped `pilot_training` run warm-started from the generalist, opponents sampled from its round pool via PFSP — DONE + verified: `--opponent league`/`--league-pool-manifest` wired into make_env/make_vec_env/run/CLI (coupled (policy, deck) via `LeagueOpponentController` + `LeaguePoolWrapper`); deck1 pinned from the single `--archetypes`; engine-backed integration test (`test_league_env_integration.py`) drives reset+steps with a faked loader. Loading a *real* specialist `.zip` opponent still awaits seed models (Group 6)
- [x] 4.2 Wire LR decay within a round to prevent the constant-LR thrashing — DONE: `--lr-schedule {constant,linear}` on `pilot_training` (`_resolve_learning_rate` → SB3 `progress_remaining` callable); orchestrator passes `--lr-schedule`; verified lr 1e-4→0
- [x] 4.3 Ensure runs use the concede-disabled mask (v0.35+) and write artifacts to `models/specialists/<deck>/` — concede mask is automatic (v0.35 engine); orchestrator writes `models/specialists/<slug>/r<round>/`
- [x] 4.4 Per-deck best-checkpoint selection (by anchored eval) for promotion into the registry each round — satisfied by the gated barrier (5.3): each deck promotes only if it clears the anchored head-to-head, else keeps its prior

## 5. Standing evaluation

- [x] 5.1 Per-specialist seat-balanced anchored eval vs greedy + the other specialists' frozen snapshots (reuse the anchored-eval harness; adequate n) — `league_eval.build_matchup_matrix` (greedy column + every deck), reuses `play_one`/`_seat_balanced_seed`
- [x] 5.2 Assemble and persist the per-round deck-by-deck matchup matrix (each cell with sample count) — `write_matchup_matrix`; wired into the orchestrator behind `--eval-n` after each barrier
- [x] 5.3 Make promotion decisions from the anchored frame only (never the in-run win rate) — DONE: gated barrier (`_barrier`, `--promote-min-wr`) promotes a deck only if its round-r checkpoint clears a seat-balanced anchored mirror head-to-head vs its prior, else keeps the prior (a regressing round can't poison the pool); offline e2e tested

## (Code-complete: 21/24. Remaining 6.1–6.3 ARE the league runs — need a box + the generalist seed)

## 6. Bring-up: skeleton → scale

- [ ] 6.1 Walking skeleton on two decks (e.g. ST-4 vs ST-1), one round, parallel: verify snapshot→pool→PFSP→eval loop and that matrix cells move sensibly
- [ ] 6.2 Scale to all six starter decks, one round; verify both parallel and sequential paths emit the 6×6 matrix and a consistent registry
- [ ] 6.3 Multi-round league with LR decay + the plateau stop rule; confirm the matchup matrix is the driving signal

## 7. Tests + docs

- [x] 7.1 Unit tests: registry round-trip, pool emission (incl. mirror + layout-hash rejection), PFSP up-weighting of losing matchups — `test_specialist_registry.py` (8) + `test_league_opponent.py` (6) + `test_league_eval.py` (4), green
- [x] 7.2 Integration test: a tiny 2-deck, 1-round league runs end-to-end and produces a registry + matchup matrix — DONE: `test_specialist_league_orchestrator.py` fakes the training subprocess and drives seed→round→barrier→registry + the promotion gate offline; engine-backed env e2e (`test_league_env_integration.py`) covers the real wrapper chain. (Live matrix-from-trained-models is part of the Group-6 run.)
- [x] 7.3 Document the league driver in `docs/TRAINING_RUNBOOK.md`, including the parallel-vs-sequential dial and the deck-vs-role distinction from the existing gauntlet
