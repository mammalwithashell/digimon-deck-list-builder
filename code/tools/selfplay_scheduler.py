"""Portable, bounded native self-play queue; committed chunks are immutable.

One controller owns the OS file lock. Workers only write private attempt dirs;
the controller hashes and atomically publishes successful chunks. Resume is
explicit, validates input/output hashes, and never treats partial rows as data.
"""
from __future__ import annotations

from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import time

ARRAY_NAMES = ("obs", "policy_offsets", "policy_actions", "policy_visits",
               "z", "terminated", "mover", "game_id")


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def atomic_json(path: Path, value: dict) -> None:
    temporary = path.with_name(path.name + ".tmp")
    with temporary.open("w", encoding="utf-8") as target:
        json.dump(value, target, indent=2)
        target.flush()
        os.fsync(target.fileno())
    os.replace(temporary, path)


@contextmanager
def controller_lock(path: Path):
    """Kernel locks release on crashes; no stale PID-file guessing on resume."""
    with path.open("a+b") as lock:
        if path.stat().st_size == 0:
            lock.write(b"0")
            lock.flush()
        lock.seek(0)
        try:
            if os.name == "nt":
                import msvcrt
                msvcrt.locking(lock.fileno(), msvcrt.LK_NBLCK, 1)
            else:
                import fcntl
                fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except OSError as error:
            raise RuntimeError(f"Another self-play controller owns {path}") from error
        try:
            yield
        finally:
            lock.seek(0)
            if os.name == "nt":
                msvcrt.locking(lock.fileno(), msvcrt.LK_UNLCK, 1)
            else:
                fcntl.flock(lock, fcntl.LOCK_UN)


def _validate_result(directory: Path, start: int, count: int, seed: int) -> dict:
    manifest = json.loads((directory / "manifest.json").read_text())
    if (manifest["game_start"], manifest["games"], manifest["seed"]) != (start, count, seed):
        raise ValueError(f"Worker returned wrong game range/seed: {directory}")
    if sum(shard["rows"] for shard in manifest["shards"]) != manifest["rows"]:
        raise ValueError(f"Worker row counts disagree: {directory}")
    files = {"manifest.json": sha256(directory / "manifest.json")}
    if manifest.get("information",{}).get("regime")=="closed_decklist":
        for name in ("belief_predictions.jsonl","belief_scoring_truth.jsonl"):
            files[name]=sha256(directory/name)
    for shard in manifest["shards"]:
        # Native shard names are flat, controlled names; reject path traversal.
        name = shard["dir"]
        if Path(name).name != name or not name.startswith("shard-"):
            raise ValueError(f"Invalid shard path: {name}")
        for array in ARRAY_NAMES:
            relative = f"{name}/{array}.npy"
            files[relative] = sha256(directory / relative)
    return {"game_start": start, "games": count, "seed": seed,
            "rows": manifest["rows"], "truncated_games": manifest["truncated_games"],
            "decisions": manifest["decisions_total"], "files": files}


def _read_commit(directory: Path, start: int, count: int, seed: int) -> dict:
    saved = json.loads((directory / "chunk_commit.json").read_text())
    actual = _validate_result(directory, start, count, seed)
    if actual != saved:
        raise ValueError(f"Committed self-play output changed: {directory}")
    return saved


def run_queue(*, command: list[str], gen_dir: Path, games: int, workers: int,
              chunk_games: int, seed: int, identity: dict, resume: bool = False,
              max_retries: int = 1, worker_timeout_seconds: float = 3600,
              heartbeat_seconds: float = 15, env: dict | None = None) -> list[Path]:
    if min(games, workers, chunk_games) < 1 or games > 2**31 - 1:
        raise ValueError("games, workers and chunk_games must be positive; games must fit int32")
    if max_retries < 0 or min(worker_timeout_seconds, heartbeat_seconds) <= 0:
        raise ValueError("retry count must be non-negative and timeouts positive")
    gen_dir.mkdir(parents=True, exist_ok=True)
    with controller_lock(gen_dir / "scheduler.lock"):
        return _run_locked(command, gen_dir, games, workers, chunk_games, seed, identity,
                           resume, max_retries, worker_timeout_seconds, heartbeat_seconds, env)


def _run_locked(command, gen_dir, games, workers, chunk_games, seed, identity,
                resume, max_retries, timeout, heartbeat_interval, env):
    shards = gen_dir / "shards"
    shards.mkdir(exist_ok=True)
    attempts_root = gen_dir / "attempts"
    attempts_root.mkdir(exist_ok=True)
    plan_path = gen_dir / "schedule.json"
    # Concurrency deliberately excluded: changing workers cannot change games.
    plan = {"schema": "selfplay-global-game-queue-v1", "games": games,
            "chunk_games": chunk_games, "seed": seed, "identity": identity}
    if plan_path.exists():
        if not resume:
            raise FileExistsError("Schedule exists; use explicit --resume-selfplay")
        if json.loads(plan_path.read_text()) != plan:
            raise ValueError("Resume refused: frozen inputs or search/chunk settings changed")
    else:
        if resume or any(shards.iterdir()) or any(attempts_root.iterdir()):
            raise ValueError("Cannot adopt missing schedule or pre-existing outputs")
        atomic_json(plan_path, plan)

    chunks = [(start, min(chunk_games, games - start)) for start in range(0, games, chunk_games)]
    expected = {f"proc{start:08d}" for start, _ in chunks}
    if any(path.name not in expected for path in shards.iterdir()):
        raise ValueError("Unexpected outputs in scheduled shard directory")
    committed = {}
    pending = []
    for start, count in chunks:
        final = shards / f"proc{start:08d}"
        if final.exists():
            committed[start] = _read_commit(final, start, count, seed)
        else:
            pending.append((start, count))
    active = {}
    tries = {}
    started = time.monotonic()
    last_heartbeat = -heartbeat_interval
    stopped = False
    old_handlers = {}

    def event(kind, **fields):
        with (gen_dir / "scheduler_events.jsonl").open("a", encoding="utf-8") as output:
            output.write(json.dumps({"event": kind, "unix": time.time(), **fields}) + "\n")
            output.flush()

    def stop(signum, frame):
        nonlocal stopped
        stopped = True

    for signum in (signal.SIGINT, signal.SIGTERM):
        old_handlers[signum] = signal.signal(signum, stop)

    def heartbeat(status, error=None):
        rows = sum(item["rows"] for item in committed.values())
        atomic_json(gen_dir / "scheduler_progress.json", {
            "status": status, "error": error, "workers": workers, "requested_games": games,
            "committed_games": sum(item["games"] for item in committed.values()),
            "committed_rows": rows,
            "truncated_games": sum(item["truncated_games"] for item in committed.values()),
            "elapsed_seconds_this_invocation": time.monotonic() - started,
            "heartbeat_unix": time.time(),
            "active": [{"game_start": start, "games": item["count"], "pid": item["proc"].pid,
                        "elapsed_seconds": time.monotonic() - item["started"]}
                       for start, item in active.items()],
            "pending_chunks": len(pending)})

    try:
        while pending or active:
            if stopped:
                raise InterruptedError("Self-play stopped; committed chunks remain resumable")
            while pending and len(active) < workers:
                start, count = pending.pop(0)
                tries[start] = tries.get(start, 0) + 1
                # Never overwrite a previous interrupted/failed attempt.
                attempt = attempts_root / f"game-{start:08d}-{time.time_ns()}"
                log = attempt.with_suffix(".log").open("w", encoding="utf-8")
                worker_command = command + ["--games", str(count), "--game-start", str(start),
                                            "--seed", str(seed), "--out", str(attempt)]
                try:
                    proc = subprocess.Popen(worker_command, stdout=log, stderr=subprocess.STDOUT, env=env)
                except BaseException:
                    log.close()
                    raise
                active[start] = {"proc": proc, "log": log, "dir": attempt,
                                 "started": time.monotonic(), "count": count}
                event("chunk_started", game_start=start, games=count, pid=proc.pid,
                      attempt=tries[start], workers=workers)
            for start, item in list(active.items()):
                proc = item["proc"]
                rc = proc.poll()
                expired = time.monotonic() - item["started"] > timeout
                if rc is None and not expired:
                    continue
                if rc is None:
                    proc.terminate()
                    try:
                        proc.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        proc.kill()
                        proc.wait()
                    rc = proc.returncode
                item["log"].close()
                del active[start]
                if rc == 0 and not expired:
                    record = _validate_result(item["dir"], start, item["count"], seed)
                    atomic_json(item["dir"] / "chunk_commit.json", record)
                    os.replace(item["dir"], shards / f"proc{start:08d}")
                    committed[start] = record
                    event("chunk_committed", game_start=start, games=item["count"],
                          rows=record["rows"], elapsed_seconds=time.monotonic() - item["started"])
                    heartbeat("running")
                elif tries[start] <= max_retries:
                    event("chunk_retry", game_start=start, exit_code=rc, timeout=expired)
                    pending.append((start, item["count"]))
                else:
                    event("chunk_failed", game_start=start, exit_code=rc, timeout=expired,
                          attempts=tries[start])
                    raise RuntimeError(f"Chunk {start} failed after {tries[start]} attempts "
                                       f"(exit {rc}, timeout={expired}); see {item['dir']}.log")
            if time.monotonic() - started - last_heartbeat >= heartbeat_interval:
                heartbeat("running")
                last_heartbeat = time.monotonic() - started
            if active:
                time.sleep(0.1)
        heartbeat("complete")
        return [shards / f"proc{start:08d}" for start, _ in chunks]
    except BaseException as error:
        # Drain every owned child on Ctrl-C, SIGTERM, timeout or controller error.
        for item in active.values():
            if item["proc"].poll() is None:
                item["proc"].terminate()
        for item in active.values():
            try:
                item["proc"].wait(timeout=10)
            except subprocess.TimeoutExpired:
                item["proc"].kill()
                item["proc"].wait()
            item["log"].close()
        active.clear()
        heartbeat("stopped" if stopped else "failed", str(error))
        raise
    finally:
        for signum, handler in old_handlers.items():
            signal.signal(signum, handler)
