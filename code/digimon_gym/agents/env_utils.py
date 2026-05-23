"""Shared environment helpers for the RL training agents."""

from __future__ import annotations

import gymnasium

from digimon_gym.digimon_gym import DigimonEnv


# `importlib.reload(digimon_gym.digimon_gym)` in some tests produces a fresh
# class object; modules that imported the name before the reload hold the stale
# class and an isinstance() check rejects otherwise-matching instances. The
# qualified name is stable across reloads, so we fall back to it.
_DIGIMON_ENV_MODULE = "digimon_gym.digimon_gym"
_DIGIMON_ENV_NAME = "DigimonEnv"


def _matches_digimon_env(env: object) -> bool:
    if isinstance(env, DigimonEnv):
        return True
    cls = type(env)
    return cls.__module__ == _DIGIMON_ENV_MODULE and cls.__name__ == _DIGIMON_ENV_NAME


def unwrap_to_digimon_env(env: gymnasium.Env) -> DigimonEnv:
    """Walk a Gymnasium wrapper stack until the underlying ``DigimonEnv`` is found.

    Termination is bounded by the depth of the wrapper chain: each iteration
    strips exactly one ``gymnasium.Wrapper`` layer, and the inner-most
    environment is always non-Wrapper.

    Raises ``RuntimeError`` if no ``DigimonEnv`` is found at any layer.
    """
    current: gymnasium.Env = env
    while isinstance(current, gymnasium.Wrapper):
        if _matches_digimon_env(current):
            return current  # type: ignore[return-value]
        current = current.env
    if _matches_digimon_env(current):
        return current  # type: ignore[return-value]
    raise RuntimeError(
        f"Could not find DigimonEnv in wrapper stack. "
        f"Innermost layer is {type(current).__name__}."
    )
