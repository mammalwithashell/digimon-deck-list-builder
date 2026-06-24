## Why

The generalist pilot plateaus at ~parity (~50%) against same-strength opponents because one network splits capacity across all six starter decks (ST-1..6). A model that only ever pilots one deck out-pilots a generalist on that deck — the standard specialist-vs-generalist result. Training six per-deck specialists in a **league** (PFSP self-play against frozen snapshots of each other) breaks the parity plateau by giving each agent a moving frontier of opponents *and* dedicating full capacity to a single deck, producing stronger, deck-tuned policies for the in-app AI opponent. Much of the league machinery already exists: the `LeagueOpponentWrapper` (PFSP / meta-weighted opponent sampling, inverse-win-rate targeting), the `GauntletOrchestrator` (a 3-stage league: bootstrap → meta-train *core + supporting* agents with PFSP → round-robin eval), the champion registry, `--archetypes` deck scoping, and `--init-from` warm-start. What this change adds is **specialization by deck** rather than by *role* (the gauntlet's existing core/supporting split), driven by a lightweight standalone orchestrator (no DB), warm-started from the current generalist.

## What Changes

- **League orchestrator** that drives six deck-scoped specialist training runs against a **shared frozen snapshot pool**, round-based (train a round → snapshot all six into the pool → repeat) to avoid the non-stationarity of chasing live opponents.
- **Per-deck specialists**: each run is scoped to one starter deck (`--archetypes "ST-N ..."`), warm-started from the generalist (`--init-from`), and trained vs the pool with **PFSP** opponent sampling (weight toward matchups the specialist is losing).
- **Mirror coverage**: each specialist's opponent pool includes frozen snapshots of *its own* deck so it learns its mirror (an observed weak spot, e.g. the ST-4 Green mirror).
- **Specialist registry/manifest** keyed by deck — extends the champion-registry concept so eval, the next league round, and deployment can resolve "the specialist for deck X". Layout-hash + observation-profile tagged for compatibility.
- **Standing metric**: per-specialist anchored evaluation (seat-balanced, vs greedy + the other specialists' frozen snapshots) plus a six-by-six **deck-matchup matrix** as the league's progress signal, replacing the degenerate in-run win rate.
- **Snapshot/round cadence + convergence controls**: round size, max rounds, snapshot retention, and a stop/plateau rule; LR decay within rounds to prevent the constant-LR thrashing seen on the floor/bo3 runs.
- Runs on the **concede-disabled action mask (v0.35+)** so specialists cannot learn premature surrender.

## Capabilities

### New Capabilities
- `deck-specialist-league`: the six-way, round-based PFSP league that turns one generalist into a set of per-deck specialists — round scheduling, frozen-snapshot pool management (incl. mirror), PFSP opponent sampling, warm-start handoff, the specialist registry, and the per-specialist + matchup-matrix evaluation.

### Modified Capabilities
<!-- The league is additive: it composes existing single-learner training (generalist-pilot-pretraining), pool-opponent sampling, and the champion registry without changing their requirements. No existing spec-level behavior changes. -->

## Impact

- **Code**: new standalone league orchestrator + specialist registry under `code/digimon_gym/agents/` (composes `pilot_training`, the existing `LeagueOpponentWrapper` PFSP sampler, `champion_registry`, and the anchored-eval harness); a driver/CLI analogous to `train_starter_curriculum.py`; outputs to `models/specialists/<deck>/`.
- **Reuse**: `LeagueOpponentWrapper` (PFSP), `--archetypes` deck scoping, `--init-from` warm-start, `--opponent pool`, the anchored-eval frame, and the per-deck starter-deck coverage already in the engine. The DB-backed `GauntletOrchestrator` (`code/server/workers/gauntlet_orchestrator.py`) is the heavyweight precedent for the 3-stage shape, but is **not** required here — this change deliberately takes the standalone (no FastAPI/DB) path, and should verify whether the gauntlet's train/eval stage bodies are complete or placeholders before reusing any of it.
- **Dependencies**: the generalist pool model (warm-start seed) and the v0.35 concede-disabled mask. Sibling change covers the deployment/compute provisioning to actually run six learners.
- **Deployment surface**: the in-app/desktop AI can load the per-deck specialist matching the deck it is piloting, with the generalist as fallback (out of scope here; flagged for the deployment change).
