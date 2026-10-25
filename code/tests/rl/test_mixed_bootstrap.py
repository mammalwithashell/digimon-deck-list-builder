from collections import Counter

import pytest

from tools.cloud_search.mixed_bootstrap import balanced_indices, make_schedule


def test_exact_416_game_teacher_archetype_and_opening_balance():
    groups = {f"arch-{i}": [f"deck-{i}-a", f"deck-{i}-b"] for i in range(13)}
    schedule = make_schedule(groups, seed=41)
    assert len(schedule) == 416
    assert [g["game_id"] for g in schedule] == list(range(416))
    counts = Counter((g["archetypes"][s], g["teachers"][s], s, g["opening_seat"] == s)
                     for g in schedule for s in (0, 1))
    assert len(counts) == 13 * 2 * 2 * 2
    assert set(counts.values()) == {8}
    assert set(d for g in schedule for d in g["decks"]) == {d for ds in groups.values() for d in ds}
    by_list = Counter((g["decks"][s], g["teachers"][s], s, g["opening_seat"] == s)
                      for g in schedule for s in (0, 1))
    assert len(by_list) == 26 * 2 * 2 * 2
    assert set(by_list.values()) == {4}, "teacher must not be confounded with list variant"


def test_controls_preserve_decks_seeds_ids_and_assignment_is_order_independent():
    groups = {"a": ["a2", "a1"], "b": ["b1"]}
    mixed = make_schedule(groups, seed=7)
    for arm in ("greedy", "development"):
        control = make_schedule(groups, seed=7, arm=arm)
        assert [{k: v for k, v in g.items() if k != "teachers"} for g in mixed] == [
            {k: v for k, v in g.items() if k != "teachers"} for g in control]
    assert make_schedule(dict(reversed(list(groups.items()))), seed=7) == mixed


def test_sampling_equalizes_seats_and_preserves_game_boundaries():
    movers = [1] * 200 + [2] * 3
    indices = balanced_indices(movers, 64, seed=1)
    assert Counter(movers[i] for i in indices) == {1: 64, 2: 64}
    assert balanced_indices(movers, 64, seed=1) == indices
    with pytest.raises(ValueError, match="no decision rows"):
        balanced_indices([1] * 10, 64, seed=1)


def test_reject_unbalanced_round_counts():
    with pytest.raises(ValueError):
        make_schedule({"a": ["a"]}, seed=1, rounds=7)


def test_fit_rejects_failed_or_tampered_corpora_before_training(tmp_path, monkeypatch):
    from tools.cloud_search.mixed_bootstrap import digest, fit, write_json
    from tools import english_meta_search_job
    calls = []
    monkeypatch.setattr(english_meta_search_job, "bootstrap", lambda *a, **k: calls.append(k))
    provenance = dict(seed=7, arm="mixed", schedule=[dict(game_id=0)])
    write_json(tmp_path / "provenance.json", provenance)
    write_json(tmp_path / "qualification.json", dict(qualified=True, games=[dict(game_id=0)]))
    shard = tmp_path / "bootstrap_shards/game-00000/obs.npy"
    shard.parent.mkdir(parents=True)
    shard.write_bytes(b"original bytes")
    import hashlib
    write_json(tmp_path / "bootstrap_shards/manifest.json", dict(truncated_games=0, games=1,
        bootstrap_provenance_sha256=digest(provenance), shards=[dict(dir="game-00000",
        sha256={"obs.npy": hashlib.sha256(shard.read_bytes()).hexdigest()})]))
    write_json(tmp_path / "failure.json", dict(error="decision cap"))
    with pytest.raises(ValueError, match="unqualified"):
        fit(tmp_path, 7)
    (tmp_path / "failure.json").unlink()
    shard.write_bytes(b"altered bytes")
    with pytest.raises(ValueError, match="hash mismatch"):
        fit(tmp_path, 7)
    assert not calls, "neither policy nor value fitting may consume failed/tampered data"
