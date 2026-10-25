"""Search jobs must evaluate the trained profile and the intended deck pool."""
from pathlib import Path
import json
from types import SimpleNamespace

import pytest

from tools import run_selfplay_generation as job


def test_anchored_eval_receives_profile_snapshot_and_frozen_anchors(monkeypatch, tmp_path):
    calls = []
    def run(cmd, **kwargs):
        calls.append(cmd)
        return SimpleNamespace(returncode=0, stdout="  vs frozen-bootstrap 60-40-0 WR= 60.0% (n=100)", stderr="")
    monkeypatch.setattr(job.subprocess, "run", run)
    result = job.run_anchored_eval(tmp_path / "model.zip", tmp_path / "eval.txt", 100,
        tmp_path / "heldout.json", 123, "cpu", "standard_lite_v2", tmp_path / "anchors.json")
    assert result == {"ok": True, "win_rates": {"frozen-bootstrap": 60.0}}
    command = calls[0]
    assert command[command.index("--profile") + 1] == "standard_lite_v2"
    assert command[command.index("--deck-pool-snapshot") + 1] == str(tmp_path / "heldout.json")
    assert command[command.index("--champions") + 1] == str(tmp_path / "anchors.json")


def test_failed_eval_is_not_an_empty_success(monkeypatch, tmp_path):
    monkeypatch.setattr(job.subprocess, "run", lambda *args, **kwargs:
        SimpleNamespace(returncode=1, stdout="", stderr="incompatible contract"))
    result = job.run_anchored_eval(tmp_path / "model.zip", tmp_path / "eval.txt", 100,
        tmp_path / "heldout.json", 123, "cpu", "standard_lite_v2")
    assert result == {"ok": False}


def test_retry_preserves_existing_generation_artifacts(tmp_path):
    folder = tmp_path / "gen0"
    folder.mkdir()
    record = folder / "failure.json"
    record.write_text("original evidence")
    with pytest.raises(FileExistsError, match="Preserving"):
        job.run_one_generation(SimpleNamespace(run_dir=str(tmp_path)), 0, None)
    assert record.read_text() == "original evidence"


def test_selfplay_workers_receive_batch_size_without_changing_search_budget(monkeypatch, tmp_path):
    commands = []
    class Worker:
        def __init__(self, cmd, **kwargs):
            commands.append(cmd)
        def wait(self):
            return 0
    monkeypatch.setattr(job.subprocess, "Popen", Worker)
    job.run_selfplay_stage("engine", tmp_path, "model.onnx", False, tmp_path / "decks.json",
        4, 2, 320, 4, "pimc", 19, 16, 1200, 4096, None, "implemented", eval_batch_size=4, scheduling="static")
    assert len(commands) == 2
    for command in commands:
        assert command[command.index("--eval-batch-size") + 1] == "4"
        assert command[command.index("--sims") + 1] == "320"
        assert command[command.index("--worlds") + 1] == "4"


def test_zero_batch_size_does_not_launch_workers(tmp_path):
    with pytest.raises(ValueError, match="positive"):
        job.run_selfplay_stage("engine", tmp_path, "model.onnx", False, tmp_path / "decks.json",
            4, 2, 320, 4, "pimc", 19, 16, 1200, 4096, None, "implemented", eval_batch_size=0)
    assert not (tmp_path / "shards").exists()


def test_generation_resume_cannot_race_an_existing_trainer(tmp_path):
    from tools.selfplay_scheduler import controller_lock

    with controller_lock(tmp_path / "generation-0.lock"):
        with pytest.raises(RuntimeError, match="Another"):
            job.run_one_generation(SimpleNamespace(run_dir=str(tmp_path), resume_selfplay=True), 0, None)


def test_unattended_generation_stops_before_fitting_truncated_games(monkeypatch, tmp_path):
    folder = tmp_path / 'collected'
    folder.mkdir()
    (folder / 'manifest.json').write_text(json.dumps(dict(truncated_games=1)))
    monkeypatch.setattr(job, 'run_selfplay_stage', lambda **kwargs: [folder])
    args = job.build_parser().parse_args(['--run-dir', str(tmp_path / 'run'),
        '--seed-model', 'seed.zip', '--decks', 'decks.json', '--fail-on-truncation'])
    with pytest.raises(RuntimeError, match='refusing to fit'):
        job.run_one_generation(args, 1, dict(model_zip='parent.zip', model_onnx='parent.onnx'))
    assert (folder / 'manifest.json').exists()
    assert not (tmp_path / 'run/gen1/model.zip').exists()
