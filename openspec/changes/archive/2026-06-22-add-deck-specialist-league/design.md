## Context

The generalist pilot (`generalist-pilot-pretraining`) converges to ~parity (~50%) against same-strength frozen champions because one policy network is shared across six starter decks. The single-learner machinery already exists: deck scoping (`--archetypes`), warm-start (`--init-from`), pool-opponent training (`--opponent pool`), the champion registry, and the seat-balanced anchored-eval frame.

**Prior art (reuse, don't reinvent):** there is already a `LeagueOpponentWrapper` (`code/digimon_gym/agents/league_wrapper.py`) implementing PFSP / meta-weighted opponent sampling with inverse-win-rate targeting and per-opponent win tracking, and a DB-backed `GauntletOrchestrator` (`code/server/workers/gauntlet_orchestrator.py`) running a 3-stage league (Stage 1 bootstrap → Stage 2 meta-train *core + supporting* agents with PFSP → Stage 3 round-robin eval). The existing axis of specialization is **role** (core vs supporting/exploiter agents); this change introduces specialization by **deck** instead, and runs it standalone rather than through the hosted-API/DB pipeline. Caveat noted in `AGENTS.md`: some gauntlet train/eval *stage bodies have historically been placeholders* — verify before depending on them.

What is missing is the **deck-axis multi-learner league**: six deck-pinned specialists co-evolving against frozen snapshots of one another, plus the standalone orchestration, registry, and evaluation to drive and judge it. This change is the orchestration layer; the sibling change provisions the compute to run it.

Constraints: must compose existing single-learner training rather than fork it; runs on the v0.35 concede-disabled mask; the in-run win rate is not a usable cross-mode signal (anchored eval only, per `add-model-evaluation-harness`).

## Goals / Non-Goals

**Goals:**
- Turn one generalist into six per-deck specialists that out-pilot the generalist on their deck and collectively map the starter-deck metagame.
- Make the league **stable** (no live-opponent non-stationarity) and **judgeable** (anchored per-specialist + a 6×6 matchup matrix as the standing metric).
- Reuse `pilot_training`, `OpponentPool`/PFSP, and `champion_registry` — the league is an orchestrator + a registry, not a new trainer.
- Produce a deck-keyed **specialist registry** that the next round, evaluation, and (later) deployment can resolve.

**Non-Goals:**
- AlphaZero/MCTS or cloneable-engine search (separate horizon).
- Meta/constructed decks — scope is the six starters only.
- The deployment/serving wiring and the compute provisioning (sibling change).
- Changing generalist or BO3 training requirements.

## Decisions

**1. Round-based PFSP against frozen snapshots — not live co-training.** Each round, all six specialists train for a fixed budget against a *frozen* pool of recent snapshots of all six, then every specialist snapshots into the pool and the next round begins. *Why:* six policies chasing each other's *live* current weights is a non-stationary moving target that destabilizes or cycles; freezing the pool (fictitious self-play / PSRO) is the standard stabilizer and is exactly what the champion-loop already does for one learner. *Alternative rejected:* fully-online simultaneous self-play (unstable); pure round-robin without PFSP (cycles on rock-paper-scissors deck matchups).

**2. Each specialist = a deck-scoped `pilot_training` run, warm-started from the generalist.** `--archetypes "ST-N ..." --init-from <generalist> --opponent pool --opponent-pool-mode pfsp --opponent-pool-manifest <round-pool>`. *Why:* maximal reuse; the generalist supplies the basics prior so specialists fine-tune rather than relearn. *Alternative rejected:* six fresh-init specialists (waste the generalist; slow).

**3. Mirror via frozen self-snapshots.** Each specialist's round pool includes snapshots of *its own* deck so it trains its mirror (the ST-4 Green mirror is a measured weak spot). *Why:* without it the specialist never faces its own deck and the mirror stays broken.

**4. PFSP opponent sampling, weighted toward losing matchups.** Sample pool opponents with probability rising in the specialist's loss rate against them. *Why:* drives toward the metagame Nash and counters cycling; already supported by `opponent_pool_mode=pfsp`.

**5. Specialist registry keyed by deck, extending the champion registry.** A manifest mapping `deck → {weights_path, algorithm, observation_profile, tensor_layout_hash, round}`; the round pool is emitted from it (latest snapshot per deck + retained history). *Why:* one resolution surface for eval, the next round, and deployment; layout-hash tagging guards compatibility.

**6. Reuse `LeagueOpponentWrapper`; orchestrate standalone, not via the DB gauntlet.** The PFSP sampler (`LeagueOpponentWrapper`, inverse-win-rate targeting, per-opponent win tracking) is reused as-is for each specialist's opponent sampling. The orchestrator itself is a **standalone driver** (no FastAPI/DB), modeled on `train_starter_curriculum.py`'s subprocess-of-`pilot_training` pattern + the champion-loop snapshot cadence. *Why:* the recent training work is all standalone-CLI; the DB-backed `GauntletOrchestrator` is heavyweight (needs the hosted API + `TrainingJobWorker`) and has placeholder stage bodies, so adopting it would cost more than it saves. *Alternative considered:* extend `GauntletOrchestrator` with a deck axis — rejected for this iteration (revisit if/when league runs need the DB's durable queue + recovery).

**7. Orchestration is a compute *dial*, not a hard requirement of parallelism.** Because rounds train against frozen pools, the six specialists in a round can run **in parallel** (separate processes/boxes, fast) **or sequentially** on one box (cheap), syncing only at the snapshot barrier. *Why:* lets the same design run on one 16-core box or fan out.

**8. Standing metric = per-specialist anchored eval + a 6×6 matchup matrix.** Each round, evaluate every specialist seat-balanced vs greedy + the other specialists' frozen snapshots; assemble the deck-matchup matrix. *Why:* the in-run win rate is degenerate; the matrix is the league's real progress + diagnostic (it surfaces weak decks/matchups, as the generalist analysis already did).

**9. LR decay within each round + a plateau/stop rule.** *Why:* constant LR produced the thrashing/late-drift seen on the floor and bo3 runs; decay lets each round settle. Stop when the matchup matrix stops moving beyond noise across K rounds.

## Risks / Trade-offs

- **Rock-paper-scissors cycling across deck matchups** → PFSP loss-weighted sampling + Nash-averaging over the snapshot history (not just the latest); never train only vs the newest opponents.
- **Overfit to the six-starter pool** (specialists tuned to beat each other, not unseen decks/humans) → acceptable for the starter-deck-vs-human goal; flag that these are *specialists for this pool*, validate with human playtests (sibling/deployment concern).
- **6× compute + snapshot storage** → round-based sequencing keeps it runnable on one box; bound snapshot retention (keep last N per deck) — note the floor run lost its peak to `keep_last=3`, so retention must be generous enough to re-select.
- **Capacity ceiling (MLP)** → specialists may need LSTM for the harder mirrors/matchups; design keeps `--lstm` switchable per specialist; treat as an open question, not a commitment.
- **Stale generalist seed** (trained with concede enabled, v0.34) → harmless under the v0.35 mask (concede is masked out), but a fresh v0.35 generalist would be marginally cleaner.

## Migration Plan

1. **Walking skeleton on two decks** (e.g. the strongest + weakest, ST-4 vs ST-1): one round, parallel, verify the snapshot→pool→PFSP→eval loop and the matchup cells move sensibly.
2. **Scale to six decks, one round**; confirm orchestration (parallel and sequential paths) and the 6×6 matrix emit.
3. **Multi-round league** with LR decay + the plateau stop rule; promote the per-deck best checkpoint (by anchored eval) into the registry each round.
4. **Rollback**: the generalist remains the deployed baseline throughout; the league is additive and produces a separate `models/specialists/` tree, so abandoning it costs nothing.

## Open Questions

- Round size (steps/specialist/round) and number of rounds before the matchup matrix plateaus.
- Parallel vs sequential default, given the available boxes (resolved by the sibling compute change).
- MLP vs LSTM for specialists — start MLP (cheap, reuses the generalist), escalate per-deck only where the matrix shows a stuck matchup.
- Whether the generalist itself stays in every specialist's pool as a stable anchor, or only the specialist snapshots.
- Anti-cycling: PFSP weighting curve and how much snapshot history to retain in the active pool.
