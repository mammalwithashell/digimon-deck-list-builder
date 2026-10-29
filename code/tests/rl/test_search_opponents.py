"""Historical search opponents are frozen inputs, never old training targets."""
import json
from pathlib import Path

import pytest

from tools import search_opponents as history
from tools import run_selfplay_generation as job


def model(root, name, **meta):
    path = root / f"{name}.onnx"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(str(path))
    history.metadata_path(path).write_text(json.dumps({
        **{k: f"test-{k}" for k in history.CONTRACT_KEYS},
        "value_semantics": "terminal_outcome", **meta}))
    return path


def pool_for(path, probability=0.5):
    return dict(schema=history.SCHEMA, selfplay_probability=probability,
                opponents=[history.checkpoint_entry("older", path)])


def test_default_never_reads_models_or_changes_outputs(tmp_path):
    assert history.freeze_pool(tmp_path, 0, None) is None
    assert list(tmp_path.iterdir()) == []


def test_explicit_checkpoint_freezes_hashes_and_resume_is_read_only(tmp_path):
    current, older = model(tmp_path, "current"), model(tmp_path, "old")
    args = dict(checkpoints=[f"old={older}"], selfplay_probability=0.25)
    frozen = history.freeze_pool(tmp_path, 3, current, **args)
    before = frozen.stat().st_mtime_ns
    assert json.loads(frozen.read_text()) == pool_for(older, 0.25) | {
        "opponents": [history.checkpoint_entry("old", older)]}
    assert history.freeze_pool(tmp_path, 3, current, resume=True, **args) == frozen
    assert frozen.stat().st_mtime_ns == before
    with pytest.raises(ValueError, match="differs"):
        history.freeze_pool(tmp_path, 3, current, resume=True, checkpoints=args["checkpoints"], selfplay_probability=0.5)
    with pytest.raises(ValueError, match="remove"):
        history.freeze_pool(tmp_path, 3, current, resume=True)
    older.write_text("mutated checkpoint")
    with pytest.raises(ValueError, match="differs"):
        history.freeze_pool(tmp_path, 3, current, resume=True, **args)


@pytest.mark.parametrize("probability", [-0.1, 1.1, float("nan"), float("inf"), True])
def test_invalid_mixture(tmp_path, probability):
    current, older = model(tmp_path, "current"), model(tmp_path, "old")
    with pytest.raises(ValueError, match="selfplay_probability"):
        history.validate_pool(pool_for(older, probability), tmp_path, current)


@pytest.mark.parametrize("weight", [0, -1, float("nan"), float("inf"), True])
def test_invalid_weights(tmp_path, weight):
    current, older = model(tmp_path, "current"), model(tmp_path, "old")
    pool = pool_for(older)
    pool["opponents"][0]["weight"] = weight
    with pytest.raises(ValueError, match="weights"):
        history.validate_pool(pool, tmp_path, current)


@pytest.mark.parametrize("mutation", ["model", "metadata", "contract", "value", "duplicate", "current", "missing"])
def test_invalid_artifact_pool(tmp_path, mutation):
    current, older = model(tmp_path, "current"), model(tmp_path, "old")
    pool = pool_for(older)
    if mutation == "model": older.write_text("changed")
    elif mutation == "metadata": history.metadata_path(older).write_text("{}")
    elif mutation in ("contract", "value"):
        older = model(tmp_path, "old", **({"feature_schema_version": "wrong"} if mutation == "contract"
                                         else {"value_semantics": "shaped_return"}))
        pool = pool_for(older)
    elif mutation == "duplicate": pool["opponents"].append({**pool["opponents"][0], "id": "alias"})
    elif mutation == "current": pool = pool_for(current)
    elif mutation == "missing": older.unlink()
    with pytest.raises((ValueError, FileNotFoundError)):
        history.validate_pool(pool, tmp_path, current)


def completed(root, index):
    folder = root / f"gen{index}"
    path = model(folder, "model")
    path.with_suffix(".zip").write_text("checkpoint")
    (folder / "summary.json").write_text(json.dumps(dict(generation=index)))
    return path


def test_recent_window_excludes_current_future_and_incomplete_generations(tmp_path):
    for i in range(7): completed(tmp_path, i)
    (tmp_path / "gen3/summary.json").unlink()
    frozen = history.freeze_pool(tmp_path, 5, tmp_path / "gen4/model.onnx", history_window=2)
    pool = json.loads(frozen.read_text())
    assert [e["id"] for e in pool["opponents"]] == ["generation-1", "generation-2"]
    completed(tmp_path, 3)
    with pytest.raises(ValueError, match="differs"):
        history.freeze_pool(tmp_path, 5, tmp_path / "gen4/model.onnx", history_window=2, resume=True)


def test_empty_window_and_uniform_bootstrap_fail_before_freezing(tmp_path):
    current = model(tmp_path, "seed")
    with pytest.raises(ValueError, match="empty"):
        history.freeze_pool(tmp_path, 0, current, history_window=3)
    with pytest.raises(ValueError, match="uniform"):
        history.freeze_pool(tmp_path, 0, None, history_window=3)
    with pytest.raises(ValueError, match="Missing frozen"):
        history.freeze_pool(tmp_path, 0, current, checkpoints=[f"old={model(tmp_path, 'old')}"], resume=True)
    assert not (tmp_path / "gen0/opponent_pool.json").exists()


def test_relative_manifest_paths_and_source_weight_are_preserved(tmp_path):
    current, older = model(tmp_path, "current"), model(tmp_path, "old")
    pool = pool_for(older, 0.75)
    pool["opponents"][0].update(path=older.name, weight=4)
    source = tmp_path / "pins.json"
    source.write_text(json.dumps(pool))
    frozen = history.freeze_pool(tmp_path, 0, current, source=source)
    result = json.loads(frozen.read_text())
    assert result["opponents"][0]["path"] == str(older.resolve())
    assert result["opponents"][0]["weight"] == 4
    assert result["selfplay_probability"] == 0.75
    pool["opponents"][0]["weight"] = 8
    source.write_text(json.dumps(pool))
    with pytest.raises(ValueError, match="differs"):
        history.freeze_pool(tmp_path, 0, current, source=source, resume=True)


def test_scheduler_binds_all_opponent_assets_and_preserves_search(monkeypatch, tmp_path):
    current, older = model(tmp_path, "current"), model(tmp_path, "old")
    frozen = history.freeze_pool(tmp_path, 0, current, checkpoints=[f"old={older}"])
    from tools import selfplay_scheduler
    captured = {}
    monkeypatch.setattr(selfplay_scheduler, "run_queue", lambda **kwargs: captured.update(kwargs) or [])
    job.run_selfplay_stage("engine", tmp_path, str(current), False, tmp_path / "decks.json",
        6, 2, 320, 4, "pimc", 1, 16, 1200, 4096, None, "implemented", opponent_pool=frozen)
    command = captured["command"]
    assert command[command.index("--opponent-pool") + 1] == str(frozen)
    assert command[command.index("--sims") + 1] == "320"
    assert command[command.index("--worlds") + 1] == "4"
    assert command[command.index("--eval-batch-size") + 1] == "4"
    assets = captured["identity"]["assets"]
    for key, path in history.pool_assets(frozen, current).items():
        assert assets[key] == history.sha256(path)
    older.write_text("changed after freezing")
    with pytest.raises(ValueError, match="hash mismatch"):
        job.run_selfplay_stage("engine", tmp_path, str(current), False, tmp_path / "decks.json",
            6, 2, 320, 4, "pimc", 1, 16, 1200, 4096, None, "implemented", opponent_pool=frozen)


@pytest.mark.parametrize("uniform,scheduling", [(True, "dynamic"), (False, "static")])
def test_history_rejects_legacy_static_or_uniform(tmp_path, uniform, scheduling):
    with pytest.raises(ValueError, match="model-based dynamic"):
        job.run_selfplay_stage("engine", tmp_path, "current", uniform, tmp_path / "decks.json",
            6, 2, 320, 4, "pimc", 1, 16, 1200, 4096, None, "implemented",
            opponent_pool=tmp_path / "pool.json", scheduling=scheduling)


def test_coverage_distinguishes_current_and_historical_actor_rows(tmp_path):
    manifest = dict(games=3, deck_names=["A", "B"], game_decks=[[0, 1], [1, 0], [0, 0]],
        opponent_games=[dict(opponent_id="old", learner_seat=0, first_player=0, training_rows_by_seat=[8, 0]),
                        dict(opponent_id="old", learner_seat=0, first_player=1, training_rows_by_seat=[6, 0]),
                        dict(opponent_id="current", learner_seat=None, first_player=0, training_rows_by_seat=[7, 9])])
    path = tmp_path / "manifest.json"
    path.write_text(json.dumps(manifest))
    result = history.summarize_opponents([tmp_path])
    assert result["learner_appearances"] == {"A": 3, "B": 1}
    assert result["games_by_opponent"] == {"old": 2, "current": 1}
    assert result["historical_learner_opening_order"] == {"first": 1, "second": 1}
    assert result["training_rows"] == 30
    manifest["opponent_games"][0]["training_rows_by_seat"][1] = 1
    path.write_text(json.dumps(manifest))
    with pytest.raises(ValueError, match="leaked"):
        history.summarize_opponents([tmp_path])
