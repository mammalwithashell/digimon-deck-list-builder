"""Reward profiles + composable components for pilot training.

This package implements the `reward-profiles` capability from openspec
change `add-reward-profiles`. The pieces fit together as:

    DigimonEnv.step()
        | drains occurrences via RewardEventBus (event_bus.py)
        v
    RewardProfileWrapper (wrapper.py)
        | resolves the active profile from info["deck1_archetype"]
        | invokes each component's compute(occurrences, episode_state)
        | applies per-component and per-profile budgets (budget.py)
        v
    scalar reward + info["reward_breakdown"]

Public surface (consumed by `digimon_gym.agents.pilot_training`):

- `Occurrence` types  — `occurrences.py`
- `RewardComponent`   — `component.py`
- Component registry  — `registry.py`
- Budget engine       — `budget.py`

See `docs/REWARD_PROFILES.md` for the YAML schema and component catalog.
"""
