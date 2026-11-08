"""Actual native execute/receipt calls through campaign.run's failure drain.

Only validate_inputs is replaced with a diagnostic two-worker job list. Native
execution, output validation, receipts, scheduling, and draining are real. This
does not qualify the scored input/admission gate.
"""
import json
import threading
from pathlib import Path

import pytest
from tools.cloud_search import frozen_mode_campaign as campaign
from test_native_pair_abort_boundary import BINARY, EVIDENCE, SCHEMA, capacity_before_native

pytestmark = pytest.mark.skipif(not BINARY or not EVIDENCE, reason="explicit native drain fixture")


def test_real_reference_failure_drains_started_peer_and_never_dispatches_successors(tmp_path, monkeypatch):
    for folder in ("requests", "results", "attempts", "logs"):
        (tmp_path / folder).mkdir()
    jobs = [dict(id=f"drain-{i}", game_seed=101, deck_indices=[0, 1], checkpoint="uniform-fixture")
            for i in range(6)]
    for i, job in enumerate(jobs):
        campaign.write(tmp_path / f"requests/{job['id']}.json", dict(
            schema=SCHEMA, id=job["id"], actor=0,
            failure="first-reference" if i == 0 else "reference-control", search_seed=143,
            runtime_sha256=campaign.digest(BINARY)))
    protocol = dict(binary=str(Path(BINARY).resolve()), workers=2, deadline_seconds=120)
    plan = dict(model_sha256="uniform-fixture-not-a-model",
                opponent_model_sha256="uniform-fixture-not-a-model")
    monkeypatch.setattr(campaign, "validate_inputs", lambda root: (protocol, jobs, plan))
    # Synchronize capacity checks before the two actual allocations so both
    # first jobs are already dispatched when either reports a failure.
    barrier = threading.Barrier(2)
    original_execute = campaign.execute
    allocations = []

    def checked_execute(root, protocol, plan, job):
        name = "controller-" + job["id"]
        capacity_before_native(name)
        capacity = campaign.read(Path(EVIDENCE) / f"capacity-{name}.json")
        assert capacity["free_ram_bytes"] >= 8 * 1024**3 + 1.5 * 2831593472 * 2
        allocations.append(job["id"])
        barrier.wait(timeout=30)
        return original_execute(root, protocol, plan, job)

    monkeypatch.setattr(campaign, "execute", checked_execute)
    with pytest.raises(RuntimeError, match="panel failed; no completed score or retries"):
        campaign.run(tmp_path)
    assert sorted(allocations) == ["drain-0", "drain-1"]
    assert sorted(p.name for p in (tmp_path / "attempts").iterdir()) == ["drain-0.json", "drain-1.json"]
    assert sorted(p.name for p in (tmp_path / "results").glob("*.rejected.json")) == [
        "drain-0.rejected.json", "drain-1.rejected.json"]
    assert not list((tmp_path / "results").glob("*.done.json"))
    assert not (tmp_path / "completion-verified.json").exists()
    assert campaign.read(tmp_path / "state.json")["state"] == "failed"
    assert campaign.read(tmp_path / "state.json")["committed_games"] == 0
    failed = tmp_path / "results/drain-0.json"
    control = tmp_path / "results/drain-1.json"
    assert not failed.exists()
    assert "closed-list unavailable event-card reference read" in (tmp_path / "logs/drain-0.err.log").read_text()
    control_events = [json.loads(line) for line in control.with_suffix(".trace.jsonl").read_text().splitlines()]
    assert control_events[-1]["event"] == "pair_returned"
    assert len(campaign.read(control)["games"]) == 2
    assert campaign.read(tmp_path / "results/drain-1.rejected.json")["error"] == "untagged schema"
    preserved = {p: p.read_bytes() for p in tmp_path.rglob("*") if p.is_file()}
    with pytest.raises(ValueError, match="existing attempts/results"):
        campaign.run(tmp_path)
    assert sorted(allocations) == ["drain-0", "drain-1"]
    assert all(p.read_bytes() == data for p, data in preserved.items())
    capacity_before_native("controller-terminal")
    campaign.write(Path(EVIDENCE) / "controller-drain-verified.json", dict(
        passed=True, fixture_root=str(tmp_path), native_sha256=campaign.digest(BINARY),
        native_allocations=2, undispatched_successors=4, committed_games=0,
        native_failed_requests=1, diagnostic_completed_requests=1, diagnostic_completed_games=2,
        done_receipts=0, rejected_receipts=2, restart_rejected=True,
        scope="Actual campaign.run scheduling, execute subprocesses, validation, receipts and drain; validate_inputs supplies a diagnostic fixture, and a barrier coordinates the first two allocations.",
        limitation="Not scored input admission or the separate multi-panel queue dispatcher.",
    ))
