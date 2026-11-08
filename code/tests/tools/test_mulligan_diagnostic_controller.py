import copy
from concurrent.futures import ThreadPoolExecutor
import json
from pathlib import Path
import threading

import pytest
from tools.cloud_search import mulligan_diagnostic_controller as m


def sample():
    node = dict(id="n1", archetype="deck", observation_sha256="obs", hand=["A", "B"])
    request = dict(worlds=4, simulations=320, qualify=True)
    row = dict(node="n1", archetype="deck", observation_sha256="obs", simulations_run=320,
               observer_and_future_rng_invariance_verified=True, selected_plan=[1],
               root_plans=[[[0], 150, 0], [[1], 170, 0]], root_visits=[[0, 150], [1, 170]], root_q=[[0, 0], [1, 0]],
               sampled_worlds=[dict(seed=i, replacement_hand=["C", "D"]) for i in range(4)],
               unique_replacement_multisets=1, keep_prior=.5, redraw_prior=.5, policy_value=0,
               root_value=0, seconds=1,
               evaluated_nodes_including_roots=dict(count=10, mean=0, min=-1, max=1))
    return request, dict(request=request.copy(), rows=[row]), [node]


@pytest.mark.parametrize("fault", ["missing", "duplicate", "observation", "budget", "invariance", "nan", "diversity", "action"])
def test_rejects_corrupt_or_incomplete_output(fault):
    request, output, nodes = sample()
    row = output["rows"][0]
    if fault == "missing":
        output["rows"] = []
    elif fault == "duplicate":
        output["rows"].append(copy.deepcopy(row))
    elif fault == "observation":
        row["observation_sha256"] = "changed"
    elif fault == "budget":
        row["simulations_run"] = 319
    elif fault == "invariance":
        row["observer_and_future_rng_invariance_verified"] = False
    elif fault == "nan":
        row["root_value"] = float("nan")
    elif fault == "diversity":
        row["unique_replacement_multisets"] = 4
    else:
        row["selected_plan"] = [2]
    with pytest.raises(ValueError):
        m.validate_output(request, output, nodes)


def test_valid_output():
    assert m.validate_output(*sample()) == 1


def test_failure_drains_existing_workers_without_new_launches(tmp_path, monkeypatch):
    (tmp_path / "attempts").mkdir()
    (tmp_path / "inputs").mkdir()
    m.write(tmp_path / "manifest.json", {})
    m.write(tmp_path / "inputs/corpus.json", {"nodes": [{}]})
    protocol = dict(workers=3, jobs=[str(i) for i in range(12)])
    monkeypatch.setattr(m, "validate_inputs", lambda root: protocol)
    gate = threading.Barrier(3)
    called = []

    def execute(root, protocol, job, nodes):
        called.append(job)
        gate.wait(timeout=5)
        return False

    monkeypatch.setattr(m, "execute", execute)
    assert m.run(tmp_path) is False
    assert set(called) == {"0", "1", "2"}
    assert m.read(tmp_path / "state.json")["state"] == "failed"
    with pytest.raises(FileExistsError):
        m.run(tmp_path)


def test_attempt_marker_prevents_native_retry(tmp_path, monkeypatch):
    for folder in ("requests", "attempts", "results", "logs"):
        (tmp_path / folder).mkdir()
    m.write(tmp_path / "requests/job.json", {})
    m.write(tmp_path / "attempts/job.json", {"prior_native_failure": True})
    monkeypatch.setattr(m.subprocess, "run", lambda *a, **k: pytest.fail("native retry"))
    with pytest.raises(FileExistsError):
        m.execute(tmp_path, {}, "job", [])


def test_full_diagnostic_cannot_be_prepared_with_failed_parity(tmp_path):
    parity = tmp_path / "parity.json"
    m.write(parity, dict(passed=False, differences=["changed action"], decisions_compared=2496))
    with pytest.raises(ValueError, match="parity gate"):
        m.prepare_full(tmp_path / "new", tmp_path / "windows", tmp_path / "linux", parity, "windows")
    assert not (tmp_path / "new").exists()


def test_comparison_ignores_only_timing():
    from tools.cloud_search.compare_mulligan_qualification import comparable
    _, a, _ = sample()
    b = copy.deepcopy(a)
    b["rows"][0]["seconds"] = 999
    assert comparable(a) == comparable(b)
    b["rows"][0]["root_value"] = 1e-12
    assert comparable(a) != comparable(b)


@pytest.mark.parametrize("field", ["features", "dependency_lock_sha256", "portable_math"])
def test_comparison_rejects_math_build_mismatch(field):
    from tools.cloud_search.compare_mulligan_qualification import matching_build_sources
    a = dict(base_commit="base", archive_sha256="archive", overlays={}, additional_overlay={}, extra_archives={},
             features=["portable-policy-math"], dependency_lock_sha256="lock", portable_math={"libm":"0.2.15"})
    b = copy.deepcopy(a)
    matching_build_sources(a, b)
    b[field] = "changed"
    with pytest.raises(ValueError, match="configuration differs"):
        matching_build_sources(a, b)
