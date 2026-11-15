"""convert_raw.py: every "use this optional effect?" answer (OptionalSkill.SetUseOptional) reaches the replay.

The mod stamps an answer with the engine action the recording seat's agent sent (`engine_action`); a human seat's
answers, and some agent answers, carry none. Dropping the recording seat's unstamped answers (only the remote seat's
were kept) lost declines: the replay's implied-accept folds then turned a DCGO "No" into an engine "Yes". An unstamped
answer is kept as `generic_bool` with `unstamped: true`, whichever seat gave it; the replay applies it only to that
seat's optional-activation gate (ReplayDriver::classify_unstamped_answer) and halts where it would be lost.

Synthetic raw rows only: no recording (they carry private deck order) is read.
"""
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
_spec = importlib.util.spec_from_file_location(
    "dcgo_convert_raw_optional_under_test", ROOT / "code" / "dcgo-melon-recorder" / "convert_raw.py")
convert_raw = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(convert_raw)

YOU, REMOTE = 1, 0


def _game(*rows):
    deck = ["ST1-03"] * 10
    return [
        {"method": "Snapshot.SetRedraw", "t": "t", "you": YOU, "is_ai": False, "players": [
            {"id": 0, "hand": deck[:5], "library": deck[5:], "egg_library": []},
            {"id": 1, "hand": deck[:5], "library": deck[5:], "egg_library": []},
        ]},
        {"method": "TurnStateMachine.SetRedraw", "args": [0, False]},
        {"method": "TurnStateMachine.SetRedraw", "args": [1, False]},
        {"method": "TurnStateMachine.SetBreedingPhase", "tp": YOU, "turn": 1, "args": [YOU, False]},
        *rows,
        {"method": "TurnStateMachine.Surrender", "tp": YOU, "turn": 1, "args": [YOU]},
    ]


def _optional(seat, use, **extra):
    row = {"method": "OptionalSkill.SetUseOptional", "tp": YOU, "turn": 1, "args": [seat, use]}
    row.update(extra)
    return row


def _bools(out):
    return [r for r in out if r.get("type") == "selection" and r.get("prompt") == "generic_bool"]


def test_recording_seats_unstamped_decline_is_kept():
    out, _ = convert_raw.convert(_game(_optional(YOU, False)), cards={})
    assert [(r["actor"], r["bool_value"], r.get("unstamped")) for r in _bools(out)] == [(YOU, False, True)]
    assert "engine_action" not in _bools(out)[0]


def test_recording_seats_unstamped_accept_is_kept():
    out, _ = convert_raw.convert(_game(_optional(YOU, True)), cards={})
    assert [(r["actor"], r["bool_value"], r.get("unstamped")) for r in _bools(out)] == [(YOU, True, True)]


def test_stamped_and_remote_answers_are_unchanged():
    # Regression guard: an answer stamped with an engine action stays a stamped row; the remote seat's unstamped one
    # stays unstamped.
    out, _ = convert_raw.convert(_game(_optional(YOU, True, engine_action=62), _optional(REMOTE, False)), cards={})
    rows = [(r["actor"], r["bool_value"], r.get("engine_action"), r.get("unstamped")) for r in _bools(out)]
    assert rows == [(YOU, True, 62, None), (REMOTE, False, None, True)]
    steps = [r["step"] for r in out if r.get("type") in ("action", "selection")]
    assert steps == list(range(len(steps)))
