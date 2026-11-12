"""Specialist league: deck pool and per-generation opponent pools."""
import json

import pytest

from tools import search_opponents as history
from tools.cloud_search import specialist_league as league


def model(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    history.metadata_path(path).write_text(json.dumps({
        **{k: f"test-{k}" for k in history.CONTRACT_KEYS}, "value_semantics": "terminal_outcome"}))
    return path


def test_build_decks_merges_aliases_dedupes_and_skips_inadmissible():
    ok = {c: {"admissible": True} for c in "abcdef"}
    campaign = [dict(name="Medusamon:sha256:1:slot1", cards=["a", "b"]),
                dict(name="Medusamon:sha256:2:slot2", cards=["b", "a"])]
    roster = {"Medusamon/Styracomon": [dict(result_id=1, cards=["c"], blockers=[]),
                                       dict(result_id=2, cards=["z"], blockers=["z"])],
              "Dynasmon": [dict(result_id=3, cards=["d"], blockers=[])],
              "Gallantmon": [dict(result_id=4, cards=["e"], blockers=[])]}
    decks = league.build_decks(campaign, roster, ok, ["Dynasmon"])
    assert [league.archetype(d["name"]) for d in decks] == ["Medusamon", "Medusamon", "Dynasmon"]
    with pytest.raises(ValueError, match="not admissible"):
        league.build_decks([dict(name="X:sha256:1", cards=["q"])], {}, ok, [])
    with pytest.raises(ValueError, match="No lists"):
        league.build_decks(campaign, roster, ok, ["Jesmon"])


def test_pools_restrict_learner_and_freeze_previous_champions(tmp_path):
    inputs = tmp_path / "inputs"
    model(inputs / "opponents/cp8.onnx", "cp8")
    model(inputs / "opponents/base.onnx", "base")
    plan = dict(archetypes=["A", "B", "C"], cp8_archetypes=["A", "B"],
                mixture=dict(gen1=dict(base_selfplay=0.8, cp8=0.2),
                             later=dict(champions=0.7, base=0.15, cp8=0.15)))
    first = league.opponent_pool(tmp_path, plan, "A", 1)
    assert first["learner_decks"] == ["A"] and first["selfplay_probability"] == 0.8
    assert [(e["id"], e["decks"]) for e in first["opponents"]] == [("cp8", ["B"])]
    for arch in "ABC":
        model(league.specialist_dir(tmp_path, arch) / "gen1/model.onnx", f"{arch}1")
    current = model(league.specialist_dir(tmp_path, "A") / "gen1/model.onnx", "A1")
    later = league.opponent_pool(tmp_path, plan, "A", 2)
    assert later["selfplay_probability"] == 0.0
    assert {e["id"]: e.get("decks") for e in later["opponents"]} == {
        "champion-b-gen1": ["B"], "champion-c-gen1": ["C"], "base": None, "cp8": ["B"]}
    assert sum(e["weight"] for e in later["opponents"]) == pytest.approx(1.0)
    history.validate_pool(later, tmp_path, current)


def test_capped_games_are_quarantined_with_manifest_bound_exclusions(tmp_path):
    import hashlib
    from tools.run_selfplay_generation import quarantine_truncations
    proc = tmp_path / "proc0"
    proc.mkdir()
    manifest = dict(truncated_games=1, deck_names=["A:sha256:1", "B:sha256:2"], game_decks=[[0, 1], [1, 0]],
                    opponent_games=[dict(game_id=4, truncated=False), dict(game_id=5, truncated=True)])
    (proc / "manifest.json").write_text(json.dumps(manifest))
    clean = tmp_path / "proc1"
    clean.mkdir()
    (clean / "manifest.json").write_text(json.dumps(dict(manifest, truncated_games=0)))
    out = quarantine_truncations([proc, clean])
    assert [(r["game_id"], r["decks"]) for r in out] == [(5, ["B:sha256:2", "A:sha256:1"])]
    exclusion = json.loads((proc / "training_exclusions.json").read_text())
    assert exclusion["manifest_sha256"] == hashlib.sha256((proc / "manifest.json").read_bytes()).hexdigest()
    assert [g["game_id"] for g in exclusion["games"]] == [5]
    assert not (clean / "training_exclusions.json").exists()
    assert quarantine_truncations([proc]) == out  # idempotent on resume
    (proc / "manifest.json").write_text(json.dumps(dict(manifest, opponent_games=[])))
    with pytest.raises(ValueError):
        quarantine_truncations([proc])


def gated_league(tmp_path):
    inputs = tmp_path / "inputs"
    model(inputs / "opponents/cp8.onnx", "cp8")
    model(inputs / "opponents/base.onnx", "base")
    (inputs / "decks.json").write_text("[]")
    for arch in "ABC":
        gen0 = league.specialist_dir(tmp_path, arch) / "gen0"
        model(gen0 / "model.onnx", "base")
        (gen0 / "model.zip").write_text("zip")
        model(league.specialist_dir(tmp_path, arch) / "gen1/model.onnx", f"{arch}1")
        (league.specialist_dir(tmp_path, arch) / "state.json").write_text(json.dumps(
            dict(next_gen=2, prev=dict(model_zip="gen1.zip", model_onnx="gen1.onnx"))))
    return dict(archetypes=["A", "B", "C"], cp8_archetypes=["A", "B"], workers=2,
                mixture=dict(gen1=dict(base_selfplay=0.8, cp8=0.2),
                             later=dict(champions=0.7, base=0.15, cp8=0.15)),
                image="img", gate=dict(games=4, sims=320, worlds=4, seed=10))


def fake_stage(outcomes):
    """run_selfplay_stage stand-in: learner wins per game id from ``outcomes[role]``."""
    def stage(**kw):
        out = kw["gen_dir"]
        role = out.name
        out.mkdir(parents=True, exist_ok=True)
        wins = outcomes[role]
        games = [dict(game_id=i, learner_seat=i % 2, winner=(i % 2) if wins[i] else 1 - i % 2,
                      truncated=False) for i in range(kw["games"])]
        (out / "manifest.json").write_text(json.dumps(dict(
            deck_names=["A:sha256:1", "B:sha256:2"], game_decks=[[0, 1] if i % 2 == 0 else [1, 0]
                                                                  for i in range(kw["games"])],
            opponent_games=games)))
        assert kw["budget_flags"] == ["--root-noise-epsilon", "0"] and kw["temperature_decisions"] == 0
        pool = json.loads(kw["opponent_pool"].read_text())
        stage.pools[role] = pool
        return [out]
    stage.pools = {}
    return stage


@pytest.mark.parametrize("challenger_wins, promoted", [([1, 1, 0, 0], False), ([1, 1, 1, 0], True)])
def test_gate_promotes_only_on_a_strictly_higher_paired_score(tmp_path, monkeypatch, challenger_wins, promoted):
    import tools.run_selfplay_generation as rsg
    plan = gated_league(tmp_path)
    stage = fake_stage(dict(challenger=challenger_wins, champion=[1, 0, 1, 0]))
    monkeypatch.setattr(rsg, "run_selfplay_stage", stage)
    league.gate(tmp_path, plan, "A", 1, 0)
    record = json.loads(league.gate_path(tmp_path, "A", 1).read_text())
    assert record["promoted"] is promoted and record["games"] == 4
    # The champion is still the base, so its games are base self-play.
    assert stage.pools["champion"]["selfplay_probability"] == 1.0
    assert [e["id"] for e in stage.pools["challenger"]["opponents"]] == ["base"]
    after = league.champion(tmp_path, "A", 1)
    assert after["generation"] == (1 if promoted else 0)
    state = json.loads((league.specialist_dir(tmp_path, "A") / "state.json").read_text())
    assert state["prev"]["model_onnx"] == after["onnx"]
    league.gate(tmp_path, plan, "A", 1, 0)  # idempotent: record reused, no new games
    assert json.loads(league.gate_path(tmp_path, "A", 1).read_text()) == record


def test_gate_rejects_unpaired_games():
    row = dict(score=1.0, learner_seat=0, truncated=False, decks=["A", "B"])
    with pytest.raises(ValueError, match="differs"):
        league.decide({0: row, 1: row}, {0: row, 1: dict(row, decks=["A", "C"])}, 2)
    with pytest.raises(ValueError, match="incomplete"):
        league.decide({0: row}, {0: row}, 2)


def test_gated_pool_uses_champions_and_merges_base(tmp_path):
    from tools.selfplay_scheduler import atomic_json
    plan = gated_league(tmp_path)
    (tmp_path / "gates/gen1").mkdir(parents=True)
    for arch, after in (("A", 0), ("B", 1), ("C", 0)):
        atomic_json(league.gate_path(tmp_path, arch, 1), dict(champion_after=league.champion(tmp_path, arch, 0)
                    if after == 0 else dict(generation=1, onnx=str(league.specialist_dir(tmp_path, arch) / "gen1/model.onnx"),
                                            zip="z")))
    # A is still the base: the base share (anchor + C's base champion) is self-play.
    pool = league.opponent_pool(tmp_path, plan, "A", 2)
    assert pool["selfplay_probability"] == pytest.approx(0.15 + 0.35)
    assert {e["id"]: e.get("decks") for e in pool["opponents"]} == {"champion-b-gen1": ["B"], "cp8": ["B"]}
    assert sum(e["weight"] for e in pool["opponents"]) == pytest.approx(1.0)
    # B is promoted: A and C (both base) merge into one base entry.
    pool = league.opponent_pool(tmp_path, plan, "B", 2)
    assert pool["selfplay_probability"] == 0.0
    weights = {e["id"]: e["weight"] for e in pool["opponents"]}
    assert set(weights) == {"base", "cp8"} and weights["base"] == pytest.approx(0.85)
    history.validate_pool(pool, tmp_path, league.specialist_dir(tmp_path, "B") / "gen1/model.onnx")


def test_claims_are_exclusive_and_reentrant(tmp_path):
    assert league.claim(tmp_path, "A", 1, "helper:x")
    assert league.claim(tmp_path, "A", 1, "helper:x")
    assert not league.claim(tmp_path, "A", 1, "controller")
    assert league.claim(tmp_path, "A", 2, "controller")


@pytest.mark.parametrize("helper_fails", [False, True])
def test_settle_runs_unclaimed_gates_and_waits_for_helpers(tmp_path, monkeypatch, helper_fails):
    import tools.run_selfplay_generation as rsg
    from tools.selfplay_scheduler import atomic_json
    plan = gated_league(tmp_path)
    plan["gate"]["remote"] = True
    stage = fake_stage(dict(challenger=[1, 1, 1, 0], champion=[1, 0, 1, 0]))
    ran = []
    monkeypatch.setattr(rsg, "run_selfplay_stage", lambda **kw: (ran.append(kw["gen_dir"].parent.name), stage(**kw))[1])
    assert league.claim(tmp_path, "B", 1, "helper:x")
    waits = []

    def helper_finishes(_seconds):
        waits.append(json.loads((tmp_path / "league_state.json").read_text())["waiting"])
        if helper_fails:  # the coordinator released the claim
            league.claim_path(tmp_path, "B", 1).unlink()
        else:
            gen1 = league.specialist_dir(tmp_path, "B") / "gen1"
            atomic_json(league.gate_path(tmp_path, "B", 1), dict(
                promoted=True, champion_after=dict(generation=1, onnx=str(gen1 / "model.onnx"),
                                                   zip=str(gen1 / "model.zip"))))
    monkeypatch.setattr(league.time, "sleep", helper_finishes)
    league.settle_gates(tmp_path, plan, 1)
    assert waits == [["B"]]
    assert sorted(set(ran)) == (["a", "b", "c"] if helper_fails else ["a", "c"])
    for arch in "ABC":
        state = json.loads((league.specialist_dir(tmp_path, arch) / "state.json").read_text())
        assert state["prev"]["model_onnx"] == league.champion(tmp_path, arch, 1)["onnx"]


def test_coordinator_picks_eligible_gates_from_the_end(tmp_path):
    from tools.cloud_search import gate_helpers
    from tools.selfplay_scheduler import atomic_json
    plan = dict(gated_league(tmp_path), generations=3)
    state = league.specialist_dir(tmp_path, "C") / "state.json"
    state.write_text(json.dumps(dict(next_gen=1)))  # C has not trained generation 1
    assert gate_helpers.next_task(tmp_path, plan, "h1") == (1, "B")
    assert gate_helpers.next_task(tmp_path, plan, "h2") == (1, "A")
    assert gate_helpers.next_task(tmp_path, plan, "h3") is None  # gen 2 needs gen 1 decided + trained
    (tmp_path / "gates/gen1").mkdir(parents=True)
    atomic_json(league.gate_path(tmp_path, "B", 1), dict(champion_after=league.champion(tmp_path, "B", 0)))
    model(league.specialist_dir(tmp_path, "B") / "gen2/model.onnx", "B2")
    rels = gate_helpers.inputs_for(tmp_path, "B", 2)
    assert {"gates/gen1/b.json", "specialists/b/generation/gen0/model.onnx",
            "specialists/b/generation/gen2/model.onnx", "inputs/opponents/base.onnx"} <= set(rels)
    assert not any("/gen1/" in r for r in rels if r.startswith("specialists"))
