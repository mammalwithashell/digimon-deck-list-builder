import json
from pathlib import Path
from types import SimpleNamespace
import zipfile

import pytest

from tools.cloud_search import card_eval_worker as worker
from digimon_gym.agents.card_eval import CardEvalCollector
from digimon_gym.agents import anchored_eval


def test_schedule_baseline_interval_new_model_and_restart():
    first = {"step": 0, "sha256": "a"}
    second = {"step": 1, "sha256": "b"}
    assert worker.due_checkpoint([first], {}, 0, 7200) == (first, "new_checkpoint")
    state = {"attempted": ["a"], "next_due_unix": 7200}
    assert worker.due_checkpoint([first], state, 7199, 7200) is None
    assert worker.due_checkpoint([first], state, 7200, 7200) == (first, "interval")
    assert worker.due_checkpoint([first, second], state, 1, 7200) == (second, "new_checkpoint")
    state["attempted"].append("b")
    assert worker.due_checkpoint([first, second], state, 1, 7200, True) == (second, "final")
    state["final_attempted"] = True
    assert worker.due_checkpoint([first, second], state, 99999, 7200, True) is None


def test_failed_checkpoint_is_retried_until_the_attempt_cap():
    first = {"step": 0, "sha256": "a"}
    second = {"step": 1, "sha256": "b"}
    state = {}
    for attempt in range(worker.MAX_ATTEMPTS):
        assert worker.due_checkpoint([first, second], state, 0, 7200) == (first, "new_checkpoint")
        worker.record_result(state, first, "new_checkpoint", {"status": "failed"})
    # Capped: the schedule moves on instead of retrying one bad file forever.
    assert state["failures"]["a"] == worker.MAX_ATTEMPTS
    assert worker.due_checkpoint([first, second], state, 0, 7200) == (second, "new_checkpoint")
    worker.record_result(state, second, "new_checkpoint", {"status": "failed"})
    worker.record_result(state, second, "new_checkpoint", {"status": "complete"})
    assert state["attempted"] == ["a", "b"] and "b" not in state["failures"]


def test_unchanged_checkpoint_is_not_rehashed(tmp_path, monkeypatch):
    path = tmp_path / "inputs/seed.zip"
    path.parent.mkdir()
    with zipfile.ZipFile(path, "w") as archive:
        archive.writestr("weights", "ready")
    cache, hashed = {}, []
    real = worker.sha256
    monkeypatch.setattr(worker, "sha256", lambda p: hashed.append(p) or real(p))
    now = path.stat().st_mtime + 100
    first = worker.checkpoints(tmp_path, now, cache=cache)
    assert worker.checkpoints(tmp_path, now, cache=cache) == first
    assert len(hashed) == 1
    with zipfile.ZipFile(path, "w") as archive:
        archive.writestr("weights", "changed!")
    import os
    os.utime(path, ns=(path.stat().st_atime_ns, path.stat().st_mtime_ns + 10**9))
    assert worker.checkpoints(tmp_path, now + 10, cache=cache)[0]["sha256"] != first[0]["sha256"]
    assert len(hashed) == 2


def test_partial_checkpoint_is_not_evaluated(tmp_path):
    path = tmp_path / "inputs/seed.zip"
    path.parent.mkdir()
    path.write_text("unfinished")
    assert worker.checkpoints(tmp_path, path.stat().st_mtime + 100) == []
    with zipfile.ZipFile(path, "w") as archive:
        archive.writestr("weights", "ready")
    assert worker.checkpoints(tmp_path, path.stat().st_mtime + 1) == []
    assert len(worker.checkpoints(tmp_path, path.stat().st_mtime + 100)) == 1


def test_continuation_keeps_report_ids_and_refreshes_engine_evaluation():
    state = dict(final_attempted=True, attempted=['a', 'b'], next_window=7, next_due_unix=999)
    worker.advance_generation(state, 1)
    assert state['next_window'] == 7 and state['attempted'] == ['a', 'b']
    checkpoint = dict(step=1, sha256='b')
    assert worker.due_checkpoint([checkpoint], state, 100, 7200) == (checkpoint, 'interval')
    state['next_due_unix'] = 1000
    worker.advance_generation(state, 1)
    assert state['next_due_unix'] == 1000
    assert worker.due_checkpoint([checkpoint], state, 100, 7200, True) == (checkpoint, 'final')


def test_window_timeout_is_recorded_without_mutating_inputs(tmp_path, monkeypatch):
    import subprocess
    root = tmp_path / "experiment"
    (root / "inputs").mkdir(parents=True)
    model = root / "inputs/seed.zip"
    model.write_bytes(b"frozen weights")
    out = tmp_path / "evaluation"
    out.mkdir()
    calls = []
    def timeout(command, **kwargs):
        calls.append(command)
        raise subprocess.TimeoutExpired(command, 10)
    monkeypatch.setattr(worker.subprocess, "run", timeout)
    args = SimpleNamespace(panels=[("meta", "heldout.json"), ("pet", "pets.json")],
                           games=24, seed=1, profile="standard_lite_v2", timeout=10)
    result = worker.evaluate_window(root, out, {"path": str(model), "sha256": worker.sha256(model), "step": 0}, 0, "new_checkpoint", args)
    assert result["status"] == "failed" and len(result["failures"]) == 2
    assert model.read_bytes() == b"frozen weights"
    assert len(list(root.rglob("*"))) == 2
    assert all("--card-eval-dir" in c for c in calls)
    assert not (out / "window-000000/candidate.zip").exists()


def test_anchors_and_panels_do_not_merge():
    collector = CardEvalCollector(0, 0)
    for anchor, panel in [("greedy", "meta"), ("frozen", "meta"), ("greedy", "pet")]:
        collector.context = {"anchor": anchor, "panel": panel, "evaluation_mode": "policy_only"}
        collector.start_game({}, (["A"], []), 1)
        collector.finish_game(1, "security_attack")
    rows = collector.report()["cards"]
    assert len(rows) == 3 and all(r["games"] == 1 for r in rows)


def test_collector_attaches_before_opponent_reset_and_detaches_on_error(monkeypatch):
    collector = CardEvalCollector(0, 0)
    instances = []
    class Inner:
        def __init__(self, **kwargs):
            instances.append(self)
    class Wrapper:
        def __init__(self, inner, opponent_fn):
            self.inner = inner
        def reset(self, seed):
            attached = self.inner.card_eval_collector
            assert attached is collector
            attached.start_game({}, (["A"], ["B"]), seed)
            attached.card(2, "B")["plays"] += 1
            raise RuntimeError("opponent failed")
        def close(self):
            self.closed = True
    monkeypatch.delenv("DIGIMON_EVAL_FAILURE_DIR", raising=False)
    monkeypatch.setattr(anchored_eval, "DigimonEnv", Inner)
    monkeypatch.setattr(anchored_eval, "OpponentWrapper", Wrapper)
    with pytest.raises(RuntimeError, match="opponent failed"):
        anchored_eval.play_one(None, None, ["A"], ["B"], 2, "standard_lite_v2", collector)
    assert instances[0].card_eval_collector is None
    report = collector.report()
    assert report["games"][0]["terminal_reason"] == "evaluation_end"
    assert next(r for r in report["cards"] if r["player"] == 2)["plays"] == 1
    assert all(r["incomplete"] == 1 and r["draws"] == 0 for r in report["cards"])


def test_monitor_retries_card_artifact_without_step_collision(tmp_path, monkeypatch):
    import sys
    import wandb
    from tools.cloud_search import wandb_monitor
    from digimon_gym.agents import card_eval
    root, state, cards = tmp_path / "experiment", tmp_path / "monitor", tmp_path / "cards"
    root.mkdir()
    path = cards / "window-000000/reports/checkpoint_0_id.json"
    path.parent.mkdir(parents=True)
    path.write_text(json.dumps({"window_id": "id"}))
    logged, published = [], []
    class Run:
        step, url, id = 0, "https://example.test/run", "test"
        summary = {}
        def define_metric(self, *a, **kw): pass
        def log(self, payload, step): logged.append(step)
        def finish(self, **kw): pass
    monkeypatch.setattr(wandb, "init", lambda **kwargs: Run())
    success = [False]
    def publish(report, path, run, step, wait, **kwargs):
        assert wait
        published.append(step)
        return success[0]
    monkeypatch.setattr(card_eval, "publish_report", publish)
    monkeypatch.setattr(sys, "argv", ["monitor", "--root", str(root), "--state-dir", str(state),
        "--card-eval-root", str(cards), "--online", "--entity", "test", "--project", "test", "--once"])
    wandb_monitor.main()
    assert json.loads((state / "publisher.json").read_text())["uploaded"] == 0
    success[0] = True
    wandb_monitor.main()
    wandb_monitor.main()
    assert published == [1, 1]  # failed attempt retried; successful report deduplicated
    assert logged == [0, 2, 3]
    assert len([r for r in wandb_monitor.recover_spool(state / "metrics.jsonl") if "card_report" in r]) == 1


def test_monitor_retry_after_logged_artifact_skips_a_second_version(tmp_path, monkeypatch):
    import sys
    import wandb
    from tools.cloud_search import wandb_monitor
    from digimon_gym.agents import card_eval
    root, state, cards = tmp_path / "experiment", tmp_path / "monitor", tmp_path / "cards"
    root.mkdir()
    path = cards / "window-000000/reports/checkpoint_0_id.json"
    path.parent.mkdir(parents=True)
    path.write_text(json.dumps({"window_id": "id"}))
    class Run:
        step, url, id = 0, "https://example.test/run", "test"
        summary = {}
        def define_metric(self, *a, **kw): pass
        def log(self, payload, step): pass
        def finish(self, **kw): pass
    monkeypatch.setattr(wandb, "init", lambda **kwargs: Run())
    calls = []
    def publish(report, path, run, step, wait, skip_artifact, on_artifact_logged):
        calls.append(skip_artifact)
        if not skip_artifact:
            on_artifact_logged()
        return len(calls) > 1  # first call: artifact logged, then the table log fails
    monkeypatch.setattr(card_eval, "publish_report", publish)
    monkeypatch.setattr(sys, "argv", ["monitor", "--root", str(root), "--state-dir", str(state),
        "--card-eval-root", str(cards), "--online", "--entity", "test", "--project", "test", "--once"])
    wandb_monitor.main()
    wandb_monitor.main()
    assert calls == [False, True]
    saved = json.loads((state / "publisher.json").read_text())
    assert saved["uploaded"] >= 1 and saved["card_reports_uploaded"] == 1
