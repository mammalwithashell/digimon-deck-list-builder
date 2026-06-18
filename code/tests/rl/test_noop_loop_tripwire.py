"""The no-op loop tripwire in DigimonEnv.step.

Fails LOUDLY (raises NoOpLoopError) when an action is applied repeatedly with no
state change — a mask/execution mismatch where the mask offers an action the
engine refuses (e.g. a CannotAttack'd Digimon "attacking", an unusable [Main]
field effect, an attack while a mandatory selection is pending). The panic rail
turns the raise into a logged + recorded + counted crash, so these surface
immediately instead of silently looping to the step limit.
"""
import numpy as np
import pytest

from digimon_gym.digimon_gym import DigimonEnv, NoOpLoopError, NOOP_LOOP_THRESHOLD


def test_tripwire_fires_loudly_on_a_noop_action():
    env = DigimonEnv()
    _, info = env.reset(seed=0)

    # Freeze the engine: step() becomes a no-op so the observation never changes
    # — exactly what a mask/execution mismatch produces (a "legal" action the
    # engine won't perform). Everything else delegates to the real runner.
    real = env.runner

    class _FrozenRunner:
        def step(self, _action):  # no-op -> state frozen
            return None

        def __getattr__(self, name):
            return getattr(real, name)

    env.runner = _FrozenRunner()

    action = int(np.argmax(info["action_mask"]))  # a mask-legal action

    err = None
    for _ in range(NOOP_LOOP_THRESHOLD + 3):
        try:
            env.step(action)
        except NoOpLoopError as exc:
            err = exc
            break
    assert err is not None, "tripwire must fire when an action no-ops repeatedly"
    msg = str(err)
    assert "NO-OP LOOP" in msg
    assert f"action {action}" in msg  # the offending action is named loudly


def test_tripwire_silent_during_normal_greedy_progress():
    """A genuinely-progressing game must NOT trip the wire (no false positives)."""
    env = DigimonEnv()
    env.reset(seed=0)
    for _ in range(NOOP_LOOP_THRESHOLD + 5):
        if env.is_game_over:
            break
        action = int(env.runner.greedy_action())
        env.step(action)  # greedy makes progress -> must never raise NoOpLoopError
