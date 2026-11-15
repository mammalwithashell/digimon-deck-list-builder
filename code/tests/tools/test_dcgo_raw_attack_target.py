import importlib.util
from pathlib import Path
import pytest

spec = importlib.util.spec_from_file_location("dcgo_convert_raw", Path(__file__).resolve().parents[2] / "dcgo-melon-recorder" / "convert_raw.py")
converter = importlib.util.module_from_spec(spec)
spec.loader.exec_module(converter)


@pytest.mark.parametrize("turn_player,target_is_turn_player,expected", [
    (0, True, 0), (0, False, 1), (1, True, 1), (1, False, 0),
])
def test_attack_target_side_is_the_target_owner(turn_player, target_is_turn_player, expected):
    players = [dict(id=i, hand=[], library=[], egg_library=[]) for i in (0, 1)]
    rows = [
        dict(method="Snapshot.SetRedraw", t="test", you=0, is_ai=False, players=players),
        dict(method="SelectAttackEffect.SetAttackTarget", tp=turn_player, turn=1,
             args=[turn_player, target_is_turn_player, 2]),
    ]
    output, _ = converter.convert(rows, {})
    selection = next(row for row in output if row.get("prompt") == "SelectAttackEffect")
    assert selection["targets"] == [{"player": expected, "frame": 2}]
