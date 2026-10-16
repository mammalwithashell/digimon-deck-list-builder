"""The isolated recorder patch must use real legality and explicit decisions."""
from pathlib import Path
import importlib.util

import pytest


def module():
    path = Path(__file__).parents[2] / "tools/dcgo_option_observation.py"
    spec = importlib.util.spec_from_file_location("dcgo_option_observation", path)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


def test_recorder_only_passes_context_at_decision_chokepoint():
    m = module()
    original = "selection prefix\nDigimon.Harness.StateDumper.Dump();\n" + m.RECORDER_OLD
    result = m.patch_recorder(original)
    assert result.count("StateDumper.Dump();") == 1
    assert result.count("StateDumper.Dump(actor, source);") == 1


def test_dumper_uses_live_legality_for_only_deciding_main_seat():
    m = module()
    original = "\n".join(old for old, _ in m.DUMPER_REPLACEMENTS)
    result = m.patch_dumper(original)
    assert 'decisionSource == "main_phase" && p != null && p.PlayerID == decisionActor' in result
    assert 'card.IsOption && card.CanPlayFromHandDuringMainPhase' in result
    assert 'legalOptions.Sort(System.StringComparer.Ordinal)' in result
    assert 'legalOptions.Distinct' not in result
    assert 'CanPlayCardTargetFrame' not in result
    assert 'Dump(int decisionActor = -1, string decisionSource = null)' in result


@pytest.mark.parametrize("patcher", ["patch_dumper", "patch_recorder"])
def test_source_drift_refuses_partial_patch(patcher):
    with pytest.raises(ValueError, match="expected exactly one"):
        getattr(module(), patcher)("unrecognized upstream source")


def test_project_must_be_separate_from_reference(tmp_path):
    m = module()
    base = tmp_path / "base"
    base.mkdir()
    with pytest.raises(ValueError, match="separate"):
        m.validate_project(base, base)
    nested = base / "nested"
    nested.mkdir()
    with pytest.raises(ValueError, match="separate"):
        m.validate_project(nested, base)
    isolated = tmp_path / "isolated"
    isolated.mkdir()
    m.validate_project(isolated, base)
