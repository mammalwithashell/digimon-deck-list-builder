"""An image transition compares only complete, identical new-engine cohorts."""
import copy
import json
from pathlib import Path
import zipfile

import pytest

from tools.cloud_search import evaluate_checkpoint_transition as transition
from tools.selfplay_scheduler import sha256


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value), encoding="utf-8")


@pytest.fixture
def archived(tmp_path, monkeypatch):
    source = tmp_path / "archive"
    for name in ("inputs/seed.zip", "inputs/development.zip", "generation/gen3/model.zip",
                 "generation/gen4/model.zip"):
        path = source / name
        path.parent.mkdir(parents=True, exist_ok=True)
        with zipfile.ZipFile(path, "w") as archive:
            archive.writestr("fixture", name)
    write(source / "inputs/combined_snapshot.json", dict(eligible_archetypes=["a", "b"], decks=[
        dict(archetype="a", deck_id="a1"), dict(archetype="b", deck_id="b1")]))
    write(source / "inputs/anchors.json", dict(version=1, champions=[
        dict(name="parent-checkpoint4", weights_path="/experiment/inputs/seed.zip", tensor_layout_hash="layout"),
        dict(name="development-fixed", weights_path="/experiment/inputs/development.zip", tensor_layout_hash="layout")]))
    write(source / "deployment-plan.json", dict(image="old-image", files={
        p.relative_to(source).as_posix(): sha256(p) for p in (source / "inputs").iterdir()}))
    write(source / "experiment_state.json", dict(stage="failed"))
    write(source / "evaluations/old-report.json", dict(status="complete", cohort_scheme="archetype-paired-v1"))
    write(source / "generation/gen4/replay.json", dict(old=True))
    image = "image@sha256:" + "a" * 64
    monkeypatch.setenv("DIGIMON_DEPLOYED_IMAGE", image)
    args = dict(image=image, source_plan_sha256=sha256(source / "deployment-plan.json"),
                checkpoint_hashes={i: sha256(source / f"generation/gen{i}/model.zip") for i in (3, 4)},
                seed=777, games_per_archetype=4)
    return source, tmp_path / "assessment", args


def report_for(root, plan, index):
    games = []
    for row in json.loads((root / "cohort.json").read_text()):
        games.append(dict(seed=row["seed"], winner_id=1, terminal_reason="security_attack",
                          context={k: v for k, v in row.items() if k not in ("seed", "opening_player")}))
    return dict(schema_version=1, checkpoint_step=index, games=games,
                cards=[], routes=[], route_decisions=[], effect_sequences=[], provenance=dict(
                    status="complete", engine_image=plan["image"], cohort_scheme=plan["protocol"],
                    evaluation_mode="policy_only", panel=transition.PANEL,
                    checkpoint_index=index, candidate_sha256=plan["checkpoints"][str(index)]["sha256"],
                    checkpoint_sha256=plan["checkpoints"][str(index)]["sha256"],
                    base_seed=plan["seed"], games_per_archetype=plan["games_per_archetype"],
                    requested_games_per_anchor=plan["games_per_anchor"],
                    snapshot_sha256=plan["files"]["inputs/combined_snapshot.json"],
                    anchors_sha256=plan["files"]["inputs/anchors.json"],
                    anchor_model_hashes=plan["anchor_model_hashes"],
                    runtime_cards_sha256=plan["runtime_cards_sha256"],
                    observation_profile="standard_lite_v2", tensor_layout_hash="layout",
                    tensor_size=9434, tensor_version=2, feature_schema_version="standard_lite_v2.4",
                    action_space_size=2192, action_schema_version=2, card_eval_event_version=2,
                    deterministic=True))


def test_prepare_freezes_only_pinned_models_inputs_and_current_cohort(archived):
    source, root, args = archived
    before = {p.relative_to(source): sha256(p) for p in source.rglob("*") if p.is_file()}
    plan = transition.prepare(source, root, **args)
    assert plan["protocol"] == "archetype-paired-v2"
    assert plan["games_per_checkpoint"] == 24
    assert plan["automatic_promotion"] is False
    assert not (root / "evaluations").exists()
    assert not list(root.rglob("replay.json"))
    registry = json.loads((root / "inputs/anchors.json").read_text())
    assert all(Path(c["weights_path"]).is_relative_to(root) for c in registry["champions"])
    cohort = json.loads((root / "cohort.json").read_text())
    assert {r["opening_player"] for r in cohort} == {1, 2}
    assert {p.relative_to(source): sha256(p) for p in source.rglob("*") if p.is_file()} == before
    with pytest.raises(FileExistsError):
        transition.prepare(source, root, **args)


@pytest.mark.parametrize("change", ["checkpoint", "plan", "input", "image", "stage", "nested"])
def test_prepare_refuses_unapproved_source_before_creating_output(archived, monkeypatch, change):
    source, root, args = archived
    if change == "checkpoint":
        args["checkpoint_hashes"][4] = "0" * 64
    elif change == "plan":
        args["source_plan_sha256"] = "0" * 64
    elif change == "input":
        (source / "inputs/combined_snapshot.json").write_text("{}")
    elif change == "image":
        monkeypatch.setenv("DIGIMON_DEPLOYED_IMAGE", "different")
    elif change == "stage":
        write(source / "experiment_state.json", dict(stage="selfplay"))
    else:
        root = source / "new-assessment"
    with pytest.raises(ValueError):
        transition.prepare(source, root, **args)
    assert not root.exists()


@pytest.mark.parametrize("change", ["protocol", "image", "hash", "seed", "anchor", "missing",
    "duplicate", "incomplete", "empty_reason", "winner", "deck", "seat", "error", "metadata", "contract",
    "missing_winner", "boolean_winner", "cards"])
def test_report_rejects_noncomparable_or_partial_results(archived, change):
    source, root, args = archived
    plan = transition.prepare(source, root, **args)
    report = report_for(root, plan, 3)
    if change == "protocol": report["provenance"]["cohort_scheme"] = "archetype-paired-v1"
    elif change == "image": report["provenance"]["engine_image"] = "old"
    elif change == "hash": report["provenance"]["candidate_sha256"] = "wrong"
    elif change == "seed": report["provenance"]["base_seed"] += 1
    elif change == "anchor": report["provenance"]["anchor_model_hashes"] = {}
    elif change == "missing": report["games"].pop()
    elif change == "duplicate": report["games"][-1] = copy.deepcopy(report["games"][0])
    elif change == "incomplete": report["games"][0]["terminal_reason"] = "step_limit"
    elif change == "empty_reason": report["games"][0]["terminal_reason"] = ""
    elif change == "winner": report["games"][0]["winner_id"] = 9
    elif change == "deck": report["games"][0]["context"]["opponent_deck_id"] = "changed"
    elif change == "seat": report["games"][0]["seed"] += 1
    elif change == "error": report["games"][0]["evaluation_error"] = "crash"
    elif change == "contract": report["provenance"]["tensor_layout_hash"] = "changed-layout"
    elif change == "missing_winner": report["games"][0].pop("winner_id")
    elif change == "boolean_winner": report["games"][0]["winner_id"] = True
    elif change == "cards": report["provenance"]["runtime_cards_sha256"] = "changed-cards"
    else: report.pop("effect_sequences")
    with pytest.raises(ValueError):
        transition.validate_report(report, root, plan, 3)


def test_selection_uses_separate_stricter_anchor_intervals():
    checkpoints = {str(i): {"sha256": str(i)} for i in (3, 4)}
    intervals = {"parent-checkpoint4": [-.02, .05], "development-fixed": [-.01, .04], "greedy": [-.01, .02]}
    estimates = {k: dict(ci95=v) for k, v in intervals.items()}
    result = transition.select_checkpoint(estimates, checkpoints)
    assert result["checkpoint_index"] == 4 and result["automatic_promotion"] is False
    estimates["parent-checkpoint4"]["ci95"][0] = -.021
    assert transition.select_checkpoint(estimates, checkpoints)["checkpoint_index"] == 3
    estimates["parent-checkpoint4"]["ci95"][0] = 0
    estimates["greedy"]["ci95"][0] = -.011
    assert transition.select_checkpoint(estimates, checkpoints)["checkpoint_index"] == 3


def test_runner_retains_canonical_reports_and_never_promotes(archived, monkeypatch):
    source, root, args = archived
    plan = transition.prepare(source, root, **args)
    calls = []
    def child(command, **kwargs):
        index = int(command[command.index("--checkpoint-step") + 1])
        assert command[command.index("--seed") + 1] == "777"
        assert command[command.index("--games-per-archetype") + 1] == "4"
        assert kwargs["env"]["OMP_NUM_THREADS"] == "1"
        folder = Path(command[command.index("--card-eval-dir") + 1])
        write(folder / f"checkpoint_{index}_fixture.json", report_for(root, plan, index))
        calls.append(index)
    monkeypatch.setattr(transition.subprocess, "run", child)
    result = transition.run(root, plan, workers=2, timeout=60, bootstrap_samples=100)
    assert sorted(calls) == [3, 4]
    assert result["status"] == "complete" and result["selection"]["checkpoint_index"] == 4
    assert result["protocol"] == "archetype-paired-v2"
    assert result["comparison"]["by_anchor"]["parent-checkpoint4"]["paired_games"] == 8
    assert len(list(root.glob("evaluations/checkpoint-*/reports/checkpoint_*.json"))) == 2
    assert json.loads((root / "result.json").read_text())["automatic_promotion"] is False
    assert transition.validate_assessment(root / "result.json", source, args["image"]) == result


def test_runner_failure_keeps_evidence_without_selection(archived, monkeypatch):
    source, root, args = archived
    plan = transition.prepare(source, root, **args)
    def child(command, **kwargs):
        index = int(command[command.index("--checkpoint-step") + 1])
        folder = Path(command[command.index("--card-eval-dir") + 1])
        report = report_for(root, plan, index)
        if index == 4: report["games"][0]["terminal_reason"] = "crash"
        write(folder / f"checkpoint_{index}_fixture.json", report)
    monkeypatch.setattr(transition.subprocess, "run", child)
    with pytest.raises(ValueError):
        transition.run(root, plan, workers=1, timeout=60, bootstrap_samples=100)
    result = json.loads((root / "result.json").read_text())
    assert result["status"] == "failed" and "selection" not in result
    assert len(list(root.glob("evaluations/checkpoint-*/reports/checkpoint_*.json"))) == 2


def test_source_input_changed_during_copy_is_rejected(archived, monkeypatch):
    source, root, args = archived
    copy_file = transition.shutil.copy2
    def drifting_copy(src, dst):
        if src.name == "combined_snapshot.json":
            src.write_text('{"unexpected": true}')
        return copy_file(src, dst)
    monkeypatch.setattr(transition.shutil, "copy2", drifting_copy)
    with pytest.raises(ValueError, match="Input changed while copying"):
        transition.prepare(source, root, **args)
    assert not (root / "plan.json").exists()


def test_runtime_and_frozen_input_changes_prevent_evaluation(archived, monkeypatch):
    source, root, args = archived
    plan = transition.prepare(source, root, **args)
    (root / "inputs/development.zip").write_text("changed")
    monkeypatch.setattr(transition.subprocess, "run", lambda *a, **k: pytest.fail("must not evaluate"))
    with pytest.raises(ValueError, match="Frozen assessment input changed"):
        transition.run(root, plan)
    assert not (root / "evaluations").exists()


@pytest.mark.parametrize("change", ["checkpoint_path", "checkpoint_hash", "anchor_hash"])
def test_plan_cannot_rebind_frozen_models(archived, change):
    source, root, args = archived
    plan = transition.prepare(source, root, **args)
    if change == "checkpoint_path":
        plan["checkpoints"]["4"]["path"] = "checkpoints/checkpoint-3.zip"
    elif change == "checkpoint_hash":
        plan["checkpoints"]["4"]["sha256"] = plan["checkpoints"]["3"]["sha256"]
    else:
        plan["anchor_model_hashes"]["development-fixed"] = "wrong"
    write(root / "plan.json", plan)
    with pytest.raises(ValueError, match="binding"):
        transition.validate_frozen(root, plan)


@pytest.mark.parametrize("change", ["source", "plan", "report", "selection", "comparison", "path", "protocol"])
def test_assessment_handoff_revalidates_ancestry_reports_and_selection(archived, monkeypatch, change):
    source, root, args = archived
    plan = transition.prepare(source, root, **args)
    def child(command, **kwargs):
        index = int(command[command.index("--checkpoint-step") + 1])
        folder = Path(command[command.index("--card-eval-dir") + 1])
        write(folder / f"checkpoint_{index}_fixture.json", report_for(root, plan, index))
    monkeypatch.setattr(transition.subprocess, "run", child)
    result = transition.run(root, plan, workers=1, timeout=60, bootstrap_samples=20)
    if change == "source":
        (source / "generation/gen4/model.zip").write_text("changed")
    elif change == "plan":
        saved = json.loads((root / "plan.json").read_text())
        saved["seed"] += 1
        write(root / "plan.json", saved)
    elif change == "report":
        (root / result["reports"]["3"]["path"]).write_text("{}")
    elif change == "selection":
        result["selection"]["checkpoint_index"] = 3
    elif change == "comparison":
        result["comparison"]["by_anchor"]["greedy"]["difference"] = 1.0
    elif change == "protocol":
        artifact = result["reports"]["3"]
        path = root / artifact["path"]
        report = json.loads(path.read_text())
        report["provenance"]["cohort_scheme"] = "archetype-paired-v1"
        write(path, report)
        artifact["sha256"] = sha256(path)
    else:
        result["reports"]["3"]["path"] = "../outside.json"
    write(root / "result.json", result)
    with pytest.raises(ValueError):
        transition.validate_assessment(root / "result.json", source, args["image"])
