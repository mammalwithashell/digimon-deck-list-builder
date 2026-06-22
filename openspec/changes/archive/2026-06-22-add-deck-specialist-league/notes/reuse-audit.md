# Reuse audit findings (Task Group 1)

Recorded during `/opsx:apply`. Determines how Groups 2–7 are built.

## 1.1 `LeagueOpponentWrapper` (PFSP) — usable standalone? **Yes, with a caveat.**
`code/digimon_gym/agents/league_wrapper.py` is a plain `gymnasium.Wrapper`, no DB
dependency — usable outside the hosted gauntlet. PFSP mode is inverse-win-rate
(`max(0.01, 1-wr)`), uniform until ≥5 games per opponent; tracks
`_opponent_games`/`_opponent_wins` on terminal episodes.
**Caveat:** it only samples the opponent's **deck** (sets `options["deck2"]`) and
tracks PFSP stats — it does **not** load the opponent's policy. Its own comment
says "the training worker handles loading the right opponent model." So standalone
use needs a bridge that reads `current_opponent.weights_path` and loads the policy.

## 1.2 `champion_registry` + `emit-pool` — reusable for a deck-keyed registry? **Yes.**
`ChampionRegistry`/`Champion` (`champion_registry.py`) is clean, JSON-versioned,
and already carries `tensor_layout_hash` with a `compatible(hash)` gate — exactly
the layout-compat machinery the league needs. `OpponentPool.from_champion_registry`
+ `champion_admin.py emit-pool` derive a training pool manifest from it.
**Gap:** `OpponentEntry` = `{name, weights_path, algorithm, win_rate_vs_pool,
games_played}` — **no `deck` field**. The league's specialist registry must add
`deck` (+ `round`), and the round-pool manifest must carry `deck` per entry.

## 1.3 `GauntletOrchestrator` stage bodies — real or placeholder? **N/A (we go standalone).**
No `TODO`/`NotImplemented`/placeholder markers in
`code/server/workers/gauntlet_orchestrator.py`; the queue/scheduling looks real.
But it is DB-backed hosted-API infra (needs FastAPI + `TrainingJobWorker`), and the
AGENTS.md caveat is about the *job bodies* in the worker, not the orchestrator. We
deliberately do **not** depend on it — we reuse only the `LeagueOpponentWrapper`
PFSP sampler it drives. (Decision 6 in design.md.)

## 1.4 `pilot_training` flags together — supported? **Mostly; one new mode needed.**
- `--archetypes` scopes the deck pool for **both** seats (mirror-only if pinned to one deck). ✓ but see gap.
- `--init-from` warm-start. ✓ (curriculum uses it)
- `--opponent pool --opponent-pool-manifest --opponent-pool-mode` exists; `OpponentPool.sample(mode="pfsp", power=2)` supports PFSP at sample time; `make_pool_opponent_fn` samples a **policy** per episode (cached by `weights_path`). ✓ for policy, ✗ for per-opponent deck.
- `--lr`. ✓
- Concede-disabled is the v0.35 engine mask, not a flag — automatic. ✓

## Load-bearing gap → design refinement
No existing path does the league's core need: **agent pinned to its own deck X,
facing opponents that each play THEIR OWN deck with THEIR OWN policy, PFSP-sampled.**
- `--opponent pool` = policy-only (deck comes from the shared deck-pool wrapper).
- `LeagueOpponentWrapper` = deck+PFSP, no policy load.

**Resolution:** a new `pilot_training` **league opponent mode** that couples
`(policy, deck)` per opponent — load `weights_path` AND set `deck2` from the same
round-pool entry. The round-pool manifest (emitted by the new specialist registry,
Group 2) carries `deck` per entry to feed it. This is new wiring on the tested
training path → checkpoint the exact shape (extend `--opponent pool` vs new
`--opponent league`) before building Group 4. Confirms, not contradicts, the
design's "the new orchestration is the missing piece."
