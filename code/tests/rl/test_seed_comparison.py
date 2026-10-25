import copy

import pytest

from tools.cloud_search.run_seed_comparison import compare_reports


def games(winners):
    return [dict(seed=20 + 2*i + i % 2, winner_id=w, terminal_reason="security_attack",
                 context=dict(panel="pet_same_list", anchor="greedy", agent_archetype="a",
                              opponent_archetype="b", agent_deck_id="a1", opponent_deck_id="b1"))
            for i,w in enumerate(winners)]


def test_paired_uncertainty_clusters_opening_seats_and_draws_are_half_points():
    control = games([1, 2, 1, 2])
    candidate = games([1, 1, 1, 1])
    report = compare_reports(control, candidate, samples=200)
    assert report["aggregate"]["difference"] == .5
    assert report["aggregate"]["ci95"] == [.5, .5]
    assert report["aggregate"]["paired_games"] == 4
    assert report["aggregate"]["noninferior_at_five_points"]
    assert compare_reports(games([2, 2]), games([None, None]), samples=200)["aggregate"]["difference"] == .5


def test_pairs_refuse_mismatched_cohorts_and_incomplete_games():
    control = games([1, 2])
    candidate = copy.deepcopy(control)
    candidate[0]["seed"] += 100
    with pytest.raises(ValueError, match="cohorts differ"):
        compare_reports(control, candidate)
    candidate = copy.deepcopy(control)
    candidate[0]["terminal_reason"] = "truncated"
    with pytest.raises(ValueError, match="incomplete"):
        compare_reports(control, candidate)
    with pytest.raises(ValueError, match="opening-seat pair"):
        compare_reports(control[:1], control[:1])


def test_all_corpora_must_qualify_before_any_fit_or_search(tmp_path, monkeypatch):
    import json
    from tools.cloud_search import run_seed_comparison as comparison
    source = tmp_path / "inputs"
    source.mkdir()
    for name in comparison.INPUTS:
        (source / name).write_text("{}")
    calls = []
    def collect(_snapshot, _out, *, arm, **_kwargs):
        calls.append(("collect", arm))
        if arm == "development":
            raise RuntimeError("qualification cap")
    monkeypatch.setattr(comparison, "collect", collect)
    monkeypatch.setattr(comparison, "fit", lambda *a, **k: calls.append(("fit", None)))
    monkeypatch.setattr(comparison.subprocess, "run", lambda *a, **k: calls.append(("search", None)))
    with pytest.raises(RuntimeError, match="qualification cap"):
        comparison.run(tmp_path, seed=7, image="example@sha256:abc")
    assert calls == [("collect", "greedy"), ("collect", "development")]
    assert json.loads((tmp_path / "comparison_state.json").read_text())["stage"] == "failed"
    for arm in comparison.ARMS:
        assert json.loads((tmp_path / "arms" / arm / "experiment_state.json").read_text())["stage"] == "failed"
        assert json.loads((tmp_path / "evaluations" / arm / "status.json").read_text())["status"] == "stopped"


def test_matched_controller_runs_both_checkpoints_and_preserves_shared_anchor(tmp_path, monkeypatch):
    import json
    from pathlib import Path
    import model_contract
    from sb3_contrib import MaskablePPO
    from tools.cloud_search import run_seed_comparison as comparison
    source = tmp_path / "inputs"
    source.mkdir()
    for name in comparison.INPUTS:
        (source / name).write_text("{}")
    calls = []
    def collect(_snapshot, out, *, arm, **_kwargs):
        out.mkdir(parents=True)
        calls.append(("collect", arm))
    def fit(out, seed):
        calls.append(("fit", out.name))
        for suffix in (".zip", ".onnx", ".onnx.meta.json", ".calibration.json"):
            (out / f"seed{suffix}").write_text("fixture")
    def search(command, **kwargs):
        run_dir = Path(command[command.index("--run-dir") + 1])
        calls.append(("search", run_dir.parent.name))
        assert command[command.index("--sims") + 1] == "320"
        assert command[command.index("--worlds") + 1] == "4"
        assert command[command.index("--eval-batch-size") + 1] == "4"
        assert "--fail-on-truncation" in command
        (run_dir / "gen1").mkdir()
        (run_dir / "gen1/model.zip").write_text("new fixture")
    def evaluate(root, out, checkpoint, window, reason, args):
        calls.append(("evaluate", root.name, window))
        assert args.games_per_archetype == 200
        anchor = json.loads((root / "inputs/anchors.json").read_text())["champions"][0]
        assert anchor["weights_path"] == str(tmp_path / "corpora/greedy/seed.zip")
        folder = out / f"window-{window:06d}"
        folder.mkdir(parents=True)
        for panel, _ in args.panels:
            (folder / f"{panel}.log").write_text("evaluation fixture")
        return dict(status="complete")
    monkeypatch.setattr(comparison, "collect", collect)
    monkeypatch.setattr(comparison, "fit", fit)
    monkeypatch.setattr(comparison.subprocess, "run", search)
    monkeypatch.setattr(comparison, "evaluate_window", evaluate)
    monkeypatch.setattr(comparison, "read_games", lambda _: games([1, 2]))
    monkeypatch.setattr(MaskablePPO, "load", lambda *a, **k: object())
    monkeypatch.setattr(model_contract, "validate_checkpoint", lambda *a: dict(tensor_layout_hash="fixture"))
    comparison.run(tmp_path, seed=7, image="example@sha256:abc")
    assert calls[:6] == [(stage, arm) for stage in ("collect", "fit") for arm in comparison.ARMS]
    assert len([c for c in calls if c[0] == "evaluate"]) == 6
    assert len([c for c in calls if c[0] == "search"]) == 3
    assert json.loads((tmp_path / "comparison_state.json").read_text())["stage"] == "complete"
    assert (tmp_path / "paired-checkpoint-1.json").exists()
