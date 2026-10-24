"""Monitor reports committed work and never confuses diagnostic or eval panels."""
import json
import os
import sys
import zipfile

import pytest

from tools.cloud_search import wandb_monitor as monitor


def write(root, path, value):
    target = root / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(json.dumps(value), encoding="utf-8")


def test_committed_throughput_eta_and_stale_heartbeat(tmp_path):
    write(tmp_path, "experiment_state.json", {"stage": "generation"})
    write(tmp_path, "generation/gen0/scheduler_progress.json", {
        "status": "running", "requested_games": 100, "committed_games": 20,
        "committed_rows": 1000, "truncated_games": 1, "workers": 8,
        "pending_chunks": 12, "active": [{"games": 4}],
        "elapsed_seconds_this_invocation": 200, "heartbeat_unix": 1000})
    fresh = monitor.collect(tmp_path, 1010)
    assert fresh["selfplay/positions_per_second"] == 5
    assert fresh["selfplay/games_per_hour"] == 360
    assert fresh["selfplay/eta_hours"] == pytest.approx(800 / 3600)
    assert fresh["selfplay/committed_games"] == 20  # active games excluded
    assert fresh["selfplay/truncation_rate"] == .05
    stale = monitor.collect(tmp_path, 1200)
    assert stale["selfplay/heartbeat_stale"] == 1
    assert "selfplay/eta_hours" not in stale


def test_diagnostics_and_evaluation_scopes_stay_separate(tmp_path):
    write(tmp_path, "diagnostics/scaling/benchmark.json", {
        "all_target_arrays_equal": True,
        "cases": [{"workers": 8, "positions_generated_per_second": 6},
                  {"workers": 8, "positions_generated_per_second": 8}]})
    directory = tmp_path / "generation/gen0"
    directory.mkdir(parents=True)
    (directory / "anchored_eval.txt").write_text("vs greedy 10-20-0 WR=33.3% (n=30)")
    (tmp_path / "pet_anchored_eval.txt").write_text("vs greedy 20-10-0 WR=66.7% (n=30)")
    result = monitor.collect(tmp_path, 100)
    assert result["diagnostics/workers_8_positions_per_second"] == 7
    assert not any(key.startswith("selfplay/") for key in result)
    assert result["evaluation/meta_validation/greedy/wins"] == 10
    assert result["evaluation/pet_same_list/greedy/wins"] == 20


def test_summary_loss_replaces_rounded_stdout(tmp_path):
    log = tmp_path / "log"
    log.write_text("epoch 0: policy_loss 1.2346 value_loss 0.2000\n" * 2)
    assert len(monitor.epoch_events(tmp_path, log)) == 1
    write(tmp_path, "generation/gen0/summary.json", {"train": {"epochs": [
        {"epoch": 0, "policy_loss": 1.234567, "value_loss": .2}]}})
    assert monitor.epoch_events(tmp_path, log)[0]["train/policy_loss"] == 1.234567


def test_checkpoint_must_be_stable_and_valid(tmp_path):
    path = tmp_path / "generation/gen0/model.zip"
    path.parent.mkdir(parents=True)
    path.write_bytes(b"partial zip")
    os.utime(path, (0, 0))
    assert monitor.stable_checkpoint_files(tmp_path, 100) == []
    with zipfile.ZipFile(path, "w") as archive:
        archive.writestr("weights", "checkpoint")
    os.utime(path, (95, 95))
    assert monitor.stable_checkpoint_files(tmp_path, 100) == []
    assert monitor.stable_checkpoint_files(tmp_path, 200) == [path]


def test_spool_recovers_interrupted_append_but_rejects_earlier_corruption(tmp_path):
    path = tmp_path / "spool"
    path.write_bytes(b'{"seq": 0}\n{"seq":')
    assert monitor.recover_spool(path) == [{"seq": 0}]
    assert path.read_bytes() == b'{"seq": 0}\n'
    path.write_bytes(b'bad\n{"seq": 1}\n')
    with pytest.raises(json.JSONDecodeError):
        monitor.recover_spool(path)


def test_spooling_restart_preserves_sequence_and_does_not_write_experiment(tmp_path, monkeypatch):
    root, state = tmp_path / "experiment", tmp_path / "monitor"
    root.mkdir()
    write(root, "deployment-plan.json", {"generation": {"games": 3300},
          "server": {"ip": "private"}, "secret": "do not publish"})
    before = {p: p.read_bytes() for p in root.rglob("*") if p.is_file()}
    monkeypatch.setattr(sys, "argv", ["monitor", "--root", str(root),
                        "--state-dir", str(state), "--once"])
    monitor.main()
    monitor.main()
    assert [r["seq"] for r in monitor.recover_spool(state / "metrics.jsonl")] == [0, 1]
    assert before == {p: p.read_bytes() for p in root.rglob("*") if p.is_file()}
    assert "private" not in json.dumps(monitor.wandb_config(root))
    assert "do not publish" not in json.dumps(monitor.wandb_config(root))
