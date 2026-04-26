# digimon_gym

RL environment + training agents. **No FastAPI, no DB** — this package must remain importable from the standalone training CLI.

## Surface

- `digimon_gym.py` — `DigimonEnv` (Gymnasium); drives the engine through PyO3 when `DIGIMON_BACKEND=rust`
- `agents/` — RL training modules
  - `pilot_training.py` — MLP / LSTM training entrypoint
  - `gauntlet.py` — `MetaGauntlet` opponent sampling
  - `deck_pool.py`, `league_wrapper.py`, `training_metrics.py`
  - `maskable_recurrent/` — custom recurrent + mask PPO
  - `architect_*.py` — Q-DeckRec deck-optimization agents
- `inference/onnx_policy.py` — ONNX inference (no PyTorch dependency)

## Boundaries (working rules 11–12)

- **No imports from `server.db.*`.** This package is consumed by both the training CLI and the hosted API; pulling DB code in would break the standalone training install.
- DB-coupled training pieces (`training_worker`, `gauntlet_orchestrator`) live under [`server/workers/`](../server/workers/) instead.

## Commands

```bash
# Pilot training (MLP)
python -m digimon_gym.agents.pilot_training --timesteps 500000

# LSTM
python -m digimon_gym.agents.pilot_training --lstm --timesteps 500000

# Self-play / gauntlet
python -m digimon_gym.agents.pilot_training --self-play --timesteps 1000000
python -m digimon_gym.agents.pilot_training --gauntlet --timesteps 500000

# Env smoke check
python -c "from digimon_gym.digimon_gym import DigimonEnv; env=DigimonEnv(); obs,info=env.reset(); print(obs.shape, info['action_mask'].shape)"
```

## Reference

- [`docs/TRAINING_RUNBOOK.md`](../../docs/TRAINING_RUNBOOK.md), [`AGENTS.md`](../../AGENTS.md) — wrapper chain, gauntlet, pipeline
- [`docs/TENSOR_SPEC.md`](../../docs/TENSOR_SPEC.md), [`docs/ACTION_SPEC.md`](../../docs/ACTION_SPEC.md) — observation + action contracts

## State threading

When evaluating LSTM policies, **reset state to `None` at episode boundaries** (working rule 6). The same rule applies to ONNX policies (rule 10).
