"""Queue recovery and reproducibility contracts, using real child processes."""
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time

import pytest

from tools.selfplay_scheduler import controller_lock, run_queue


@pytest.fixture
def worker(tmp_path):
    script = tmp_path / "worker.py"
    script.write_text('''
import argparse, json, time
from pathlib import Path
p=argparse.ArgumentParser()
p.add_argument('--games',type=int);p.add_argument('--game-start',type=int)
p.add_argument('--seed',type=int);p.add_argument('--out',type=Path)
p.add_argument('--delay',type=float,default=0)
p.add_argument('--fail-once',type=Path)
p.add_argument('--fail-start',type=int,default=-1)
a=p.parse_args()
a.out.mkdir(parents=True)
time.sleep(a.delay * (2 if a.game_start == 0 else 1))
if a.game_start == a.fail_start: raise SystemExit(8)
if a.fail_once and not a.fail_once.exists():
    a.fail_once.write_text('failed once');raise SystemExit(9)
s=a.out/'shard-000';s.mkdir()
for name in ['obs','policy_offsets','policy_actions','policy_visits','z','terminated','mover','game_id']:
    (s/(name+'.npy')).write_text(f'{a.seed}:{a.game_start}:{a.games}:{name}')
(a.out/'manifest.json').write_text(json.dumps({'game_start':a.game_start,
    'games':a.games,'seed':a.seed,'rows':a.games,'decisions_total':a.games*2,
    'truncated_games':0,'shards':[{'dir':'shard-000','rows':a.games}]}))
''')
    return [sys.executable, str(script)]


def queue(worker, directory, workers=2, **kwargs):
    return run_queue(command=worker, gen_dir=directory, games=5, workers=workers,
                     chunk_games=2, seed=42, identity={"model": "frozen", "sims": 320},
                     **kwargs)


def test_dynamic_queue_publishes_remainder_once_and_resumes_with_new_worker_count(worker, tmp_path):
    out = tmp_path / "run"
    dirs = queue(worker + ["--delay", "0.05"], out)
    assert [json.loads((p / "manifest.json").read_text())["game_start"] for p in dirs] == [0, 2, 4]
    assert [json.loads((p / "manifest.json").read_text())["games"] for p in dirs] == [2, 2, 1]
    attempts = list((out / "attempts").glob("*.log"))
    assert len(attempts) == 3
    resumed = run_queue(command=["must-not-launch"], gen_dir=out, games=5, workers=4,
                        chunk_games=2, seed=42, identity={"model": "frozen", "sims": 320}, resume=True)
    assert resumed == dirs
    assert len(list((out / "attempts").glob("*.log"))) == 3
    assert json.loads((out / "scheduler_progress.json").read_text())["committed_games"] == 5


def test_resume_rejects_changed_inputs_and_corrupted_outputs(worker, tmp_path):
    out = tmp_path / "run"
    dirs = queue(worker, out)
    with pytest.raises(ValueError, match="frozen inputs"):
        run_queue(command=worker, gen_dir=out, games=5, workers=1, chunk_games=2,
                  seed=42, identity={"model": "different"}, resume=True)
    (dirs[0] / "shard-000/obs.npy").write_bytes(b"corrupt")
    with pytest.raises(ValueError, match="output changed"):
        queue(worker, out, resume=True)


def test_retry_is_bounded_and_partial_attempt_is_never_published(worker, tmp_path):
    marker = tmp_path / "fail-once"
    out = tmp_path / "run"
    dirs = queue(worker + ["--fail-once", str(marker)], out, max_retries=1, workers=1)
    assert len(dirs) == 3
    assert len(list((out / "attempts").glob("*.log"))) == 4
    assert len(list((out / "attempts").glob("game-*"))) == 5  # four logs plus failed private dir


def test_timeout_stops_children_and_can_resume_uncommitted_chunks(worker, tmp_path):
    out = tmp_path / "run"
    with pytest.raises(RuntimeError, match="timeout=True"):
        queue(worker + ["--delay", "1"], out, max_retries=0, worker_timeout_seconds=0.15)
    assert not list((out / "shards").iterdir())
    assert json.loads((out / "scheduler_progress.json").read_text())["status"] == "failed"
    assert len(queue(worker, out, resume=True)) == 3


def test_failed_later_chunk_preserves_completed_data_on_resume(worker, tmp_path):
    out = tmp_path / "run"
    with pytest.raises(RuntimeError):
        queue(worker + ["--fail-start", "2"], out, workers=1, max_retries=0)
    first = out / "shards/proc00000000/chunk_commit.json"
    before = first.read_bytes()
    assert len(list((out / "shards").iterdir())) == 1
    queue(worker, out, workers=3, resume=True)
    assert first.read_bytes() == before
    events = [json.loads(line) for line in (out / "scheduler_events.jsonl").read_text().splitlines()]
    assert sum(e["event"] == "chunk_started" and e["game_start"] == 0 for e in events) == 1


def test_fresh_invocation_cannot_overwrite_or_concurrently_own_a_queue(worker, tmp_path):
    out = tmp_path / "run"
    queue(worker, out)
    with pytest.raises(FileExistsError, match="explicit"):
        queue(worker, out)
    with controller_lock(out / "scheduler.lock"):
        with pytest.raises(RuntimeError, match="Another"):
            queue(worker, out, resume=True)


@pytest.mark.skipif(os.name == "nt", reason="POSIX SIGTERM delivery; Windows timeout/drain tested separately")
def test_linux_sigterm_drains_children_and_marks_stopped(worker, tmp_path):
    out = tmp_path / "signal-run"
    harness = tmp_path / "controller.py"
    harness.write_text("from tools.selfplay_scheduler import run_queue\n"
        "try:\n"
        f" run_queue(command={worker + ['--delay', '30']!r}, gen_dir=__import__('pathlib').Path({str(out)!r}), "
        "games=2, workers=1, chunk_games=1, seed=1, identity={})\n"
        "except InterruptedError: pass\n")
    controller = subprocess.Popen([sys.executable, str(harness)])
    worker_pid = None
    try:
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            progress = out / "scheduler_progress.json"
            if progress.exists():
                active = json.loads(progress.read_text())["active"]
                if active:
                    worker_pid = active[0]["pid"]
                    break
            time.sleep(0.05)
        assert worker_pid is not None
        controller.send_signal(signal.SIGTERM)
        assert controller.wait(timeout=15) == 0
        assert json.loads(progress.read_text())["status"] == "stopped"
        with pytest.raises(ProcessLookupError):
            os.kill(worker_pid, 0)
        assert not list((out / "shards").iterdir())
    finally:
        if controller.poll() is None:
            controller.kill()
            controller.wait()
        if worker_pid is not None:
            try:
                os.kill(worker_pid, signal.SIGTERM)
            except ProcessLookupError:
                pass


@pytest.mark.parametrize("kwargs", [{"chunk_games": 0}, {"workers": 0}, {"games": 2**31}])
def test_invalid_queue_settings_do_not_launch(tmp_path, kwargs):
    settings = dict(command=["must-not-launch"], gen_dir=tmp_path, games=5, workers=2,
                    chunk_games=2, seed=1, identity={}) | kwargs
    with pytest.raises(ValueError):
        run_queue(**settings)
