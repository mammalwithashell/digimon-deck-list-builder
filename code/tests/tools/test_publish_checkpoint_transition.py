"""Transition uploads retain immutable evidence and resume committed artifacts."""
import json
from pathlib import Path
import sys
from types import SimpleNamespace

import pytest

from tools.cloud_search import publish_checkpoint_transition as publisher
from tools.selfplay_scheduler import sha256


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value), encoding="utf-8")


def snapshot(root):
    return {p.relative_to(root): p.read_bytes() for p in root.rglob("*") if p.is_file()}


@pytest.fixture
def assessment(tmp_path):
    root = tmp_path / "assessment"
    write(root / "cohort.json", [{"seed": 17, "anchor": "greedy"}])
    plan = dict(image="new-image", protocol="archetype-paired-v2",
                source_engine_image="old-image", seed=17, games_per_archetype=200,
                games_per_checkpoint=7800, anchor_model_hashes={"parent": "parent-hash"},
                selection_policy={"automatic_promotion": False},
                files={"cohort.json": sha256(root / "cohort.json")})
    write(root / "plan.json", plan)
    reports = {}
    for index in (3, 4):
        path = root / f"evaluations/checkpoint-{index}/reports/checkpoint_{index}.json"
        report = dict(schema_version=1, checkpoint_step=index, eval_window=index,
                      window_id=f"transition-{index}", cards=[{"card_id": "BT25-057", "attacks": index}],
                      routes=[], route_decisions=[], effect_sequences=[], provenance=dict(
                          status="complete", panel="combined_validation", candidate_sha256=f"model-{index}"))
        write(path, report)
        reports[str(index)] = dict(path=path.relative_to(root).as_posix(), sha256=sha256(path))
    write(root / "result.json", dict(status="complete", plan_sha256=sha256(root / "plan.json"),
          reports=reports, comparison={"by_anchor": {"greedy": {"difference": .02, "ci95": [-.01, .05]}}},
          selection={"checkpoint_index": 4, "checkpoint_sha256": "model-4"}))
    return root, tmp_path / "monitor"


@pytest.fixture
def sdk(monkeypatch):
    state = SimpleNamespace(inits=[], artifacts=[], waits=[], logs=[], summary={},
                            fail_table_once=None, fail_comparison_once=False)

    class Artifact:
        def __init__(self, name, type, metadata):
            self.name, self.type, self.metadata = name, type, metadata
            self.files, self.tables = {}, {}

        def add_file(self, path, name):
            self.files[name] = Path(path)

        def add(self, table, name):
            self.tables[name] = table

    class Run:
        id = "transition-test"
        url = "https://wandb.invalid/entity/project/runs/transition-test"
        summary = state.summary

        def __enter__(self):
            return self

        def __exit__(self, *args):
            return False

        def define_metric(self, *args, **kwargs):
            pass

        def log_artifact(self, artifact, aliases):
            state.artifacts.append((artifact, aliases))

            def wait():
                if artifact.type == "checkpoint-comparison" and state.fail_comparison_once:
                    state.fail_comparison_once = False
                    raise RuntimeError("simulated comparison upload failure")
                state.waits.append(artifact)

            return SimpleNamespace(wait=wait)

        def log(self, payload, **kwargs):
            if payload["card_eval/checkpoint_step"] == state.fail_table_once:
                state.fail_table_once = None
                raise RuntimeError("simulated table upload failure")
            state.logs.append((payload, kwargs))

    def init(**kwargs):
        state.inits.append(kwargs)
        return Run()

    monkeypatch.setitem(sys.modules, "wandb", SimpleNamespace(
        init=init, Artifact=Artifact, Table=lambda **kwargs: kwargs,
        Settings=lambda **kwargs: SimpleNamespace(**kwargs)))
    return state


def invoke(monkeypatch, root, state_dir, **overrides):
    args = {"assessment": root, "state-dir": state_dir, "run-id": "transition-test",
            "entity": "entity", "project": "project", **overrides}
    monkeypatch.setattr(sys, "argv", ["publisher", *[
        value for key, item in args.items() for value in (f"--{key}", str(item))]])
    publisher.main()


def test_publishes_both_reports_and_bound_comparison_without_changing_evidence(assessment, sdk, monkeypatch):
    root, state_dir = assessment
    before = snapshot(root)
    invoke(monkeypatch, root, state_dir)
    assert snapshot(root) == before
    assert sdk.inits[0]["job_type"] == "checkpoint-transition"
    assert sdk.inits[0]["resume"] == "allow"
    assert sdk.inits[0]["config"]["automatic_promotion"] is False
    assert [a.type for a, _ in sdk.artifacts] == ["card-evaluation", "card-evaluation", "checkpoint-comparison"]
    assert len(sdk.waits) == 3
    assert [payload["card_eval/checkpoint_step"] for payload, _ in sdk.logs] == [3, 4]
    assert [kwargs for _, kwargs in sdk.logs] == [{"step": 0}, {"step": 1}]
    assert all("card_eval/combined_validation/cards" in payload for payload, _ in sdk.logs)
    assert set(sdk.artifacts[-1][0].files) == {"plan.json", "result.json", "cohort.json"}
    assert sdk.summary["selected_checkpoint"] == 4
    assert sdk.summary["selected_checkpoint_sha256"] == "model-4"
    assert sdk.summary["paired/greedy/ci95_low"] == -.01
    assert sdk.summary["card_reports_uploaded"] == 2
    state = publisher.read(state_dir / "publisher.json")
    assert state["binding"]["result_sha256"] == sha256(root / "result.json")
    assert state["reports"] == {str(i): {"artifact": True, "complete": True} for i in (3, 4)}
    assert state["comparison"] is True


@pytest.mark.parametrize("change", ["failed", "plan", "report", "escaped_report", "cohort"])
def test_rejects_unbound_evidence_before_network_or_state_creation(assessment, sdk, monkeypatch, change):
    root, state_dir = assessment
    result = publisher.read(root / "result.json")
    if change == "failed":
        result["status"] = "failed"
        write(root / "result.json", result)
    elif change == "plan":
        write(root / "plan.json", {**publisher.read(root / "plan.json"), "image": "other"})
    elif change == "report":
        write(root / result["reports"]["4"]["path"], {"changed": True})
    elif change == "escaped_report":
        outside = root.parent / "outside-report.json"
        write(outside, {"outside": True})
        result["reports"]["4"] = dict(path="../outside-report.json", sha256=sha256(outside))
        write(root / "result.json", result)
    else:
        write(root / "cohort.json", [{"seed": 999}])
    before = snapshot(root)
    with pytest.raises(ValueError):
        invoke(monkeypatch, root, state_dir)
    assert sdk.inits == []
    assert not state_dir.exists()
    assert snapshot(root) == before


def test_retry_after_artifact_then_table_failure_skips_committed_artifact(assessment, sdk, monkeypatch):
    root, state_dir = assessment
    before = snapshot(root)
    sdk.fail_table_once = 3
    with pytest.raises(RuntimeError, match="state retained for retry"):
        invoke(monkeypatch, root, state_dir)
    state = publisher.read(state_dir / "publisher.json")
    assert state["reports"] == {"3": {"artifact": True}}
    assert state["comparison"] is False
    assert len(sdk.artifacts) == 1
    invoke(monkeypatch, root, state_dir)
    assert len(sdk.artifacts) == 3
    assert [payload["card_eval/checkpoint_step"] for payload, _ in sdk.logs] == [3, 4]
    committed_state = (state_dir / "publisher.json").read_bytes()
    invoke(monkeypatch, root, state_dir)
    assert len(sdk.artifacts) == 3
    assert len(sdk.logs) == 2
    assert (state_dir / "publisher.json").read_bytes() == committed_state
    assert snapshot(root) == before


def test_comparison_failure_preserves_completed_reports_for_retry(assessment, sdk, monkeypatch):
    root, state_dir = assessment
    before = snapshot(root)
    sdk.fail_comparison_once = True
    with pytest.raises(RuntimeError, match="comparison upload failure"):
        invoke(monkeypatch, root, state_dir)
    state = publisher.read(state_dir / "publisher.json")
    assert all(row["complete"] for row in state["reports"].values())
    assert state["comparison"] is False
    invoke(monkeypatch, root, state_dir)
    assert len([a for a, _ in sdk.artifacts if a.type == "card-evaluation"]) == 2
    assert len(sdk.logs) == 2
    assert publisher.read(state_dir / "publisher.json")["comparison"] is True
    assert snapshot(root) == before


@pytest.mark.parametrize("placement", ["same", "child", "parent"])
def test_state_directory_cannot_overlap_assessment_evidence(assessment, sdk, monkeypatch, placement):
    root, _ = assessment
    state_dir = {"same": root, "child": root / "monitor", "parent": root.parent}[placement]
    before = snapshot(root.parent)
    with pytest.raises(ValueError, match="separate from assessment evidence"):
        invoke(monkeypatch, root, state_dir)
    assert sdk.inits == []
    assert snapshot(root.parent) == before


@pytest.mark.parametrize("change", ["run-id", "entity", "project", "result"])
def test_retry_cannot_rebind_publisher_state(assessment, sdk, monkeypatch, change):
    root, state_dir = assessment
    invoke(monkeypatch, root, state_dir)
    committed_state = (state_dir / "publisher.json").read_bytes()
    overrides = {change: "different"} if change != "result" else {}
    if change == "result":
        write(root / "result.json", {**publisher.read(root / "result.json"), "finished_at": "changed"})
    before = snapshot(root)
    with pytest.raises(ValueError, match="different run or assessment"):
        invoke(monkeypatch, root, state_dir, **overrides)
    assert len(sdk.inits) == 1
    assert (state_dir / "publisher.json").read_bytes() == committed_state
    assert snapshot(root) == before
