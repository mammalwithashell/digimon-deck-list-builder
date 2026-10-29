"""The no-op loop tripwire in DigimonEnv.step.

Fails LOUDLY (raises NoOpLoopError) when an action is applied repeatedly with no
state change — a mask/execution mismatch where the mask offers an action the
engine refuses (e.g. a CannotAttack'd Digimon "attacking", an unusable [Main]
field effect, an attack while a mandatory selection is pending). The panic rail
turns the raise into a logged + recorded + counted crash, so these surface
immediately instead of silently looping to the step limit.
"""
from types import SimpleNamespace

import numpy as np
import pytest

from digimon_gym.digimon_gym import DigimonEnv, NoOpLoopError, NOOP_LOOP_THRESHOLD


def _noop_message(ui, action=116):
    env = DigimonEnv.__new__(DigimonEnv)
    env.runner = SimpleNamespace(to_ui_json=lambda: ui)
    return env._describe_noop(action)


def test_noop_diagnostic_identifies_pending_field_choice_and_its_player():
    """116 selects a Token here; the turn player is not the selecting player."""
    prompt = "Delete 1 Token so this Digimon doesn't leave"
    msg = _noop_message({
        "currentPhase": 5,
        "currentPlayer": 2,
        "turnCount": 7,
        "pendingSelection": {
            "kind": "AnyField",
            "phase": 4,
            "prompt": prompt,
            "selectingPlayer": 1,
            "validIndices": [116],
        },
    })

    assert "action 116 (SELECT-AnyField)" in msg
    assert "(ATTACK)" not in msg
    assert "phase=5" in msg
    assert "currentPlayer=2" in msg
    assert "pendingSelection=yes" in msg
    assert "selectionKind='AnyField'" in msg
    assert "selectionPhase=4" in msg
    assert "selectingPlayer=1" in msg
    assert prompt in msg


def test_noop_diagnostic_keeps_normal_main_phase_attack_label():
    msg = _noop_message({
        "currentPhase": 4,
        "currentPlayer": 2,
        "turnCount": 7,
        "pendingSelection": None,
        "player2": {"battleArea": [{
            "topCardName": "Attacker",
            "modifiers": [{"type": "CannotAttack"}],
        }]},
    })

    assert "action 116 (ATTACK)" in msg
    assert "phase=4" in msg
    assert "pendingSelection=no" in msg
    assert "selectingPlayer=" not in msg
    assert "P2s0=Attacker+CannotAttack" in msg


def test_noop_diagnostic_survives_unavailable_ui_state():
    def unavailable():
        raise RuntimeError("UI unavailable")

    env = DigimonEnv.__new__(DigimonEnv)
    env.runner = SimpleNamespace(to_ui_json=unavailable)

    msg = env._describe_noop(116)

    assert "NO-OP LOOP: action 116" in msg
    assert "phase=None" in msg


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
