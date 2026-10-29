import argparse
import json
from pathlib import Path
from types import SimpleNamespace

from tools import probe_closed_belief_pool as probe


def test_probe_retains_failures_and_truncations_without_counting_duplicate_slots(tmp_path, monkeypatch):
    decks, cli, prior = [tmp_path / name for name in ("decks.json", "engine.exe", "prior.json")]
    cli.write_bytes(b"frozen executable")
    library = tmp_path / "inference.dll"
    library.write_bytes(b"frozen runtime")
    prior.write_text("{}")
    decks.write_text(json.dumps([
        {"name": "a", "cards": ["A", "B"]},
        {"name": "a duplicate slot", "cards": ["B", "A"]},
        {"name": "b", "cards": ["C", "D"]},
    ]))

    def fake_run(command, **kwargs):
        output = Path(command[-1])
        if output.parent.name == "case-00":
            kwargs["stdout"].write("unsupported state\n")
            return SimpleNamespace(returncode=1)
        output.mkdir()
        library.write_bytes(b"changed runtime")
        (output / "manifest.json").write_text(json.dumps({"games": 1, "truncated_games": 1}))
        return SimpleNamespace(returncode=0)

    monkeypatch.setattr(probe.subprocess, "run", fake_run)
    args = argparse.Namespace(decks=decks, cli=cli, prior=prior, out=tmp_path / "probe",
                              seed=11, sims=4, worlds=2, workers=2,
                              profile="standard_lite_v2", decision_cap=1200)
    assert not probe.run(args)
    report = json.loads((args.out / "report.json").read_text())
    assert len(report["cases"]) == 2
    assert (report["completed"], report["failed"], report["truncated"]) == (0, 1, 1)
    assert report["cases"][0]["error"] == ["unsupported state"]
    assert report["identity"]["native_libraries"]["inference.dll"]
    assert not report["engine_unchanged"], "an unchanged executable does not excuse a changed native runtime"
