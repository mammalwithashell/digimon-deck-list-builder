"""A deployment failure must not proceed into expensive generation work."""
import json
from pathlib import Path
from types import SimpleNamespace

import pytest
from tools.cloud_search import run_experiment as experiment


@pytest.fixture
def launch(monkeypatch, tmp_path):
    inputs = tmp_path / "inputs"
    inputs.mkdir()
    (tmp_path / "deployment-plan.json").write_text(json.dumps({"inputs": {}, "generation": {"games": 3300}}))
    (inputs / "seed.onnx.meta.json").write_text(json.dumps({"value_semantics": "terminal_outcome"}))
    monkeypatch.setattr(experiment, "ROOT", tmp_path)
    monkeypatch.setattr(experiment, "INPUTS", inputs)
    monkeypatch.setattr(experiment, "load_playable_card_ids", lambda: set())
    monkeypatch.setattr(experiment.GeneralistDeckPool, "from_snapshot", lambda *a, **k: SimpleNamespace(archetypes={}))
    monkeypatch.setattr(experiment, "validate_contract", lambda *a: None)
    monkeypatch.setattr(experiment, "validate_checkpoint", lambda *a: None)
    from sb3_contrib import MaskablePPO
    monkeypatch.setattr(MaskablePPO, "load", lambda *a, **k: object())
    calls = []

    def fake_run(*args):
        args = list(map(str, args))
        calls.append(args)
        if args[0].endswith("benchmark_search_workers.py"):
            path = Path(args[args.index("--out") + 1])
            path.mkdir(parents=True)
            (path / "benchmark.json").write_text(json.dumps({"all_target_arrays_equal": True,
                "cases": [{"workers": 4, "positions_generated_per_second": 2.0}]}))
    monkeypatch.setattr(experiment, "run", fake_run)
    monkeypatch.setattr(experiment.subprocess, "run", lambda *a, **k: None)
    return tmp_path, calls


def test_hash_mismatch_blocks_all_compute(launch):
    root, calls = launch
    (root / "inputs/decks.json").write_text("changed")
    (root / "deployment-plan.json").write_text(json.dumps({"inputs": {"decks.json": "wrong"}}))
    with pytest.raises(ValueError, match="Input hash mismatch"):
        experiment.main()
    assert calls == []


@pytest.mark.parametrize("truncated,passes", [(6, False), (0, True)])
def test_health_gate_controls_generation(launch, monkeypatch, truncated, passes):
    root, calls = launch

    def fake_stage(*args, **kwargs):
        directory = args[1]
        directory.mkdir(parents=True)
        (directory / "manifest.json").write_text(json.dumps({"games": 100, "truncated_games": truncated, "rows": 1000}))
        return [directory]
    monkeypatch.setattr(experiment, "run_selfplay_stage", fake_stage)
    if passes:
        experiment.main()
        generation = [c for c in calls if c[0].endswith("run_selfplay_generation.py")]
        assert len(generation) == 1
        assert generation[0][generation[0].index("--games") + 1] == "3300"
        assert json.loads((root / "experiment_state.json").read_text())["stage"] == "complete"
    else:
        with pytest.raises(RuntimeError, match="health gate failed"):
            experiment.main()
        assert not any(c[0].endswith("run_selfplay_generation.py") for c in calls)


def test_resume_requires_a_failed_run(launch):
    root, _ = launch
    with pytest.raises(RuntimeError, match="--resume requires"):
        experiment.main(["--resume"])
    experiment.main.__globals__["atomic_json"](root / "started.json", {})
    experiment.stage("generation")
    with pytest.raises(RuntimeError, match="--resume requires"):
        experiment.main(["--resume"])


def test_resume_skips_completed_stages_and_resumes_selfplay(launch, monkeypatch):
    root, calls = launch

    def fake_stage(*args, **kwargs):
        directory = args[1]
        directory.mkdir(parents=True)
        (directory / "manifest.json").write_text(json.dumps({"games": 100, "truncated_games": 0, "rows": 1000}))
        return [directory]
    monkeypatch.setattr(experiment, "run_selfplay_stage", fake_stage)
    original = experiment.run

    def interrupted(*args):
        original(*args)
        if str(args[0]).endswith("run_selfplay_generation.py"):
            (root / "generation/gen0").mkdir(parents=True)
            raise RuntimeError("worker timeout")
    monkeypatch.setattr(experiment, "run", interrupted)
    with pytest.raises(RuntimeError, match="worker timeout"):
        experiment.main()
    experiment.stage("failed", error="worker timeout")
    assert json.loads((root / "completed_stages.json").read_text()) == ["batch_parity", "worker_scaling", "health_check"]

    calls.clear()
    monkeypatch.setattr(experiment, "run", original)
    experiment.main(["--resume"])
    assert [c[0].rsplit("/", 1)[-1].rsplit("\\", 1)[-1] for c in calls] == ["run_selfplay_generation.py"]
    assert calls[0][-1] == "--resume-selfplay"
    assert json.loads((root / "experiment_state.json").read_text())["stage"] == "complete"
    assert (root / "resumed.json").exists()
