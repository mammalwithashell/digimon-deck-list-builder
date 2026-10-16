"""Selection capability is captured before choices and scoped to the acting seat."""
import importlib.util
import os
from pathlib import Path
import pytest


def module():
    spec = importlib.util.spec_from_file_location("selection_observation", Path(__file__).parents[2] / "tools/dcgo_selection_observation.py")
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


def source(m, kind):
    return "\n".join(old for old, _ in getattr(m, kind.upper() + "_REPLACEMENTS"))


def test_capture_is_instance_nullable_reset_and_before_ui():
    m = module()
    result = m.patch_selector(source(m, "selector"))
    assert "bool? _selectionCanDecline = null;" in result
    assert "static bool?" not in result
    assert result.index("_selectionCanDecline = null;", result.index("IEnumerator Activate()")) < result.index("if (_maxCount == 0)")
    assert result.index("if (active())") < result.index("_selectionCanDecline = _canNoSelect();") < result.index("if (_isSecurity)")
    assert "catch (System.Exception) { _selectionCanDecline = null; }" in result
    assert result.count("selectionCanDecline: _selectionCanDecline") == 2
    assert result.count("_selectionCanDecline = _canNoSelect();") == 1


def test_recorder_carries_context_without_affecting_main_phase():
    m = module()
    result = m.patch_recorder(source(m, "recorder") + "\nStateDumper.Dump(actor, source);")
    assert "bool? selectionCanDecline = null" in result
    assert 'prompt == "SelectCardEffect" ? selectionCanDecline : null' in result
    assert "StateDumper.Dump(actor, source);" in result


def test_dumper_emits_false_and_omits_unknown_or_other_seat():
    m = module()
    result = m.patch_dumper(source(m, "dumper"))
    assert "selectionCanDecline.HasValue && p != null && p.PlayerID == selectionActor" in result
    assert 'selectionCanDecline.Value ? "true" : "false"' in result
    assert 'AppendStrings(sb, "legal_option_plays", legalOptions);' in result
    assert result.count('selection_can_decline') == 1


@pytest.mark.parametrize("kind", ["selector", "recorder", "dumper"])
def test_missing_duplicate_and_already_patched_fail_closed(kind):
    m = module()
    patch = getattr(m, "patch_" + kind)
    original = source(m, kind)
    for bad in ("unknown", original + original, patch(original)):
        with pytest.raises(ValueError, match="anchor"):
            patch(bad)


def projects(tmp_path, m):
    project, reference = tmp_path / "project", tmp_path / "reference"
    for relative, patcher in m.PATCHES:
        kind = patcher.__name__.removeprefix("patch_")
        for root in (project, reference):
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(source(m, kind), encoding="utf-8")
    return project, reference


def test_overlap_and_shared_file_rejected(tmp_path):
    m = module()
    project, reference = projects(tmp_path, m)
    with pytest.raises(ValueError, match="separate"):
        m.plan(reference, reference)
    relative = m.PATCHES[0][0]
    target = project / relative
    target.unlink()
    os.link(reference / relative, target)
    with pytest.raises(ValueError, match="independent"):
        m.plan(project, reference)


def test_plan_is_dry_and_application_has_exact_backups(tmp_path):
    m = module()
    project, reference = projects(tmp_path, m)
    originals = {rel: (project / rel).read_bytes() for rel, _ in m.PATCHES}
    planned = m.plan(project, reference)
    assert all((project / rel).read_bytes() == data for rel, data in originals.items())
    receipt = m.apply_plan(planned, tmp_path / "receipt", project, reference)
    assert receipt["applied"]
    for rel, data in originals.items():
        assert (tmp_path / "receipt" / Path(rel).name).read_bytes() == data
        assert (reference / rel).read_bytes() == data
        assert (project / rel).read_bytes() != data
    with pytest.raises(ValueError):
        m.plan(project, reference)


def test_bad_later_anchor_cannot_partially_apply(tmp_path):
    m = module()
    project, reference = projects(tmp_path, m)
    first = project / m.PATCHES[0][0]
    before = first.read_bytes()
    (project / m.PATCHES[-1][0]).write_text("drift", encoding="utf-8")
    with pytest.raises(ValueError):
        m.plan(project, reference)
    assert first.read_bytes() == before


def test_partial_install_is_rejected(tmp_path):
    m = module()
    for kind in ("selector", "recorder", "dumper"):
        original = source(m, kind)
        old, new = getattr(m, kind.upper() + "_REPLACEMENTS")[0]
        with pytest.raises(ValueError, match="anchor"):
            getattr(m, "patch_" + kind)(original.replace(old, new))


def test_changed_plan_refuses_before_any_write_or_receipt(tmp_path):
    m = module()
    project, reference = projects(tmp_path, m)
    planned = m.plan(project, reference)
    first = project / m.PATCHES[0][0]
    before = first.read_bytes()
    (project / m.PATCHES[-1][0]).write_text("changed", encoding="utf-8")
    with pytest.raises(ValueError, match="source changed"):
        m.apply_plan(planned, tmp_path / "receipt", project, reference)
    assert first.read_bytes() == before
    assert not (tmp_path / "receipt").exists()


def test_receipt_cannot_modify_reference(tmp_path):
    m = module()
    project, reference = projects(tmp_path, m)
    with pytest.raises(ValueError, match="receipt"):
        m.apply_plan(m.plan(project, reference), reference / "receipt", project, reference)
    assert not (reference / "receipt").exists()


def test_bom_crlf_and_json_boolean_escaping_preserved(tmp_path):
    m = module()
    project, reference = projects(tmp_path, m)
    path = project / m.PATCHES[0][0]
    path.write_bytes(b"\xef\xbb\xbf" + path.read_text(encoding="utf-8").replace("\n", "\r\n").encode())
    planned = m.plan(project, reference)
    after = planned[0][2]
    assert after.startswith(b"\xef\xbb\xbf")
    assert b"\n" not in after.replace(b"\r\n", b"")
    result = m.patch_dumper(source(m, "dumper"))
    assert r'Append("\"selection_can_decline\":")' in result
