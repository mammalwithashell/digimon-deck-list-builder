"""Run-source routing and fail-closed dispatch: no player or worker calls."""
import json
import hashlib
from pathlib import Path
from types import SimpleNamespace

import pytest

from tools.card_loop.config import LoopConfig
from tools.card_loop.state import LedgerPaths
from tools.card_loop.stages import base, harness


def test_config_supports_separate_execution_timeout_and_source_paths():
    cfg = LoopConfig(interaction_denominator="qa/frozen/interactions.json",
                     source_manifest="qa/frozen/manifest.json", source_manifest_sha256="a" * 64, job_timeout_s=600,
                     oracle_timeout_s=720)
    assert cfg.job_timeout_s == 600 and cfg.oracle_timeout_s == 720
    assert LoopConfig().job_timeout_s == 180


@pytest.mark.parametrize("value", [0, -1, True, 1.5, 2147483648])
def test_invalid_execution_timeouts_are_rejected(value):
    with pytest.raises(ValueError, match="job_timeout_s"):
        LoopConfig(job_timeout_s=value)


def test_all_denominator_readers_and_oracle_use_configured_path(tmp_path):
    frozen = tmp_path / "frozen.json"
    frozen.write_text(json.dumps({"version": 1, "cards": {}, "interactions": {}}))
    cfg = SimpleNamespace(interaction_denominator="frozen.json", source_manifest=None,
                          escalations_dir="qa/escalations", job_timeout_s=600,
                          oracle_timeout_s=720)
    ctx = SimpleNamespace(repo=tmp_path, config=cfg, plan={}, harness_bin="harness", sources=None)
    assert LedgerPaths.for_repo(tmp_path, cfg).denominator_path == frozen
    assert base.denominator(ctx)["version"] == 1
    argv = harness.oracle_argv(ctx, "s.yaml", None, clause_text_json="clauses.json")
    assert argv[argv.index("--interaction-denominator") + 1] == "frozen.json"
    assert argv[argv.index("--job-timeout") + 1] == "600"
    assert argv[argv.index("--oracle-timeout") + 1] == "720"
    frozen.unlink()
    with pytest.raises(base.StageDeferred, match="denominator"):
        harness.oracle_argv(ctx, "s.yaml", None, clause_text_json="clauses.json")


def test_campaign_passes_selected_denominator_to_binding(tmp_path, monkeypatch):
    from tools.clause_coverage import campaign
    seen = []
    def bind(*a, **kw):
        seen.append(kw.get("interaction_denominator"))
        return {"cards": {}, "denominator": {}, "orphan_scenarios": [],
                "unrecognized_verdicts": [], "current_source_clauses": []}
    monkeypatch.setattr(campaign, "bind", bind)
    campaign.build_plan_for_pool([], [], cards_dir=tmp_path, scenarios_dir=tmp_path,
        verdicts_path=tmp_path, interaction_denominator=tmp_path / "frozen.json")
    assert seen == [tmp_path / "frozen.json"]


def test_immutable_prelude_never_adds_or_commits_even_when_invalid(tmp_path, monkeypatch):
    from tools.card_loop import driver
    calls = []
    cfg = SimpleNamespace(source_manifest="snapshot.json", clause_text_json="frozen.json")
    ctx = SimpleNamespace(repo=tmp_path, config=cfg, plan={"work_set": {"pool": ["EX12-076"]}},
                          clause_text_json=None, run_command=lambda *a: calls.append(a))
    d = object.__new__(driver.Driver)
    d.ctx, d.config, d.plan, d.repo = ctx, cfg, ctx.plan, tmp_path
    d.state = SimpleNamespace(problems=[])
    d._commit_paths = lambda *a, **k: calls.append(a)
    assert d._run_prelude() is False
    assert calls == []
    assert "snapshot" in d._prelude.lower()


@pytest.fixture
def snapshot(tmp_path, monkeypatch):
    # Load readers before patching extract.run: their module-level aliases must
    # not retain this fixture lambda after monkeypatch teardown.
    from tools.clause_coverage import campaign, exam_binding, extract
    from tools.clause_coverage.semantic_fingerprint import bind_clause_semantics
    from tools.card_loop.interactions import denominator as den
    from tools.card_loop.source_snapshot import SOURCE_INPUTS
    cards = ["BT1-001", "BT1-002", "BT1-003"]
    rows = bind_clause_semantics([dict(id=f"{c}#effect#0", card_id=c, zone="effect",
        kind="timing", timings=["On Play"], keyword=None, text="Draw 1.") for c in cards])
    monkeypatch.setattr(extract, "run", lambda ids, *a, **kw:
        {"clauses": [r for r in rows if r["card_id"] in ids]})
    qa = {"cards": {c: ["Q1"] for c in cards[:2]}, "qa": {"Q1": {
        "card_ids": cards[:2], "question": "A shared ruling?", "answer": "Yes."}}}
    for name in SOURCE_INPUTS:
        path = tmp_path / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("{}")
    (tmp_path / SOURCE_INPUTS[0]).write_text(json.dumps({c: {} for c in cards}))
    (tmp_path / SOURCE_INPUTS[3]).write_text(json.dumps(qa))
    promotion = den.load_promotion(tmp_path / SOURCE_INPUTS[4])
    artifact = den.build_denominator(cards, rows, qa, promotion)
    (tmp_path / "book.json").write_text(json.dumps({"cards": cards, "clauses": rows}))
    (tmp_path / "interactions.json").write_text(json.dumps(artifact))
    cfg = LoopConfig(clause_text_json="book.json", interaction_denominator="interactions.json",
                     source_manifest="manifest.json", source_manifest_sha256="0" * 64,
                     job_timeout_s=600, oracle_timeout_s=720)
    ctx = SimpleNamespace(repo=tmp_path, config=cfg, plan={"work_set": {"pool": cards[:1]}},
                          harness_bin="harness", clause_text_json=None)
    ids = sorted(artifact["cards"][cards[0]])
    manifest = {"version": 1, "pool": cards[:1], "clause_text_json": "book.json",
        "interaction_denominator": "interactions.json", "expected": {
            "clause_ids": [rows[0]["id"]], "interaction_ids": ids,
            "gating_interaction_ids": [i for i in ids if artifact["interactions"][i]["gating"]]}}
    def seal():
        from dataclasses import replace
        manifest["files"] = {n: hashlib.sha256((tmp_path/n).read_bytes()).hexdigest()
                             for n in (*SOURCE_INPUTS, "book.json", "interactions.json")}
        raw = json.dumps(manifest).encode()
        (tmp_path / "manifest.json").write_bytes(raw)
        ctx.config = replace(cfg, source_manifest_sha256=hashlib.sha256(raw).hexdigest())
    seal()
    return ctx, manifest, rows, seal


def test_valid_snapshot_preserves_cross_card_clause_and_shared_qa(snapshot):
    from tools.card_loop.source_snapshot import validate_snapshot
    ctx, manifest, rows, seal = snapshot
    s = Path(ctx.repo)/"s.yaml"
    s.write_text("card: BT1-001\nclause: BT1-001#effect#0\ncovers: ['BT1-001#effect#0', 'BT1-002#effect#0']\n")
    before = {p: p.read_bytes() for p in Path(ctx.repo).rglob("*.json")}
    validate_snapshot(ctx, scenario="s.yaml")
    s.write_text("card: BT1-001\nclause: qa:Q1\ninteraction: {id: 'qa:Q1', source: qa, kind: positive}\n")
    validate_snapshot(ctx, scenario="s.yaml")
    assert all(p.read_bytes() == b for p, b in before.items())


@pytest.mark.parametrize("damage", ["manifest", "input", "missing_card", "stale_timing", "scope_ids", "qa_membership", "universe", "unbound_input"])
def test_snapshot_damage_refuses_before_oracle_without_rewriting(snapshot, damage):
    ctx, manifest, rows, seal = snapshot
    root = Path(ctx.repo)
    (root/"s.yaml").write_text("card: BT1-001\nclause: BT1-001#effect#0\n")
    if damage == "manifest":
        (root/"manifest.json").write_text("{}")
    elif damage == "input":
        (root/"data/cards.json").write_text('{"changed":true}')
    elif damage == "missing_card":
        manifest["pool"].append("BT1-003")
        ctx.plan["work_set"]["pool"] = list(manifest["pool"])
        seal()
    elif damage == "stale_timing":
        from tools.clause_coverage.semantic_fingerprint import bind_clause_semantics
        rows[0]["timings"] = ["When Attacking"]
        rows[:] = bind_clause_semantics(rows)
    elif damage == "scope_ids":
        manifest["expected"]["clause_ids"] = []
        seal()
    elif damage == "qa_membership":
        qa = json.loads((root/"data/card_qa.json").read_text())
        qa["qa"]["Q1"]["card_ids"].append("BT1-003")
        (root/"data/card_qa.json").write_text(json.dumps(qa))
        seal()
    elif damage == "universe":
        book = json.loads((root/"book.json").read_text())
        book["cards"] = ["BT1-001"]
        (root/"book.json").write_text(json.dumps(book))
        seal()
    else:
        from dataclasses import replace
        del manifest["files"]["data/cards.json"]
        raw = json.dumps(manifest).encode()
        (root/"manifest.json").write_bytes(raw)
        ctx.config = replace(ctx.config, source_manifest_sha256=hashlib.sha256(raw).hexdigest())
    before = {p: p.read_bytes() for p in root.rglob("*.json")}
    with pytest.raises(base.StageDeferred, match="snapshot"):
        harness.oracle_argv(ctx, "s.yaml", None, clause_text_json="book.json")
    assert all(p.read_bytes() == b for p, b in before.items())


def test_immutable_valid_prelude_does_not_run_append_or_commit(snapshot):
    from tools.card_loop import driver
    ctx, _, _, _ = snapshot
    ctx.run_command = lambda *a: pytest.fail("immutable prelude must not run book add")
    d = object.__new__(driver.Driver)
    d.ctx, d.config, d.plan, d.repo = ctx, ctx.config, ctx.plan, ctx.repo
    d.state = SimpleNamespace(problems=[])
    d._commit_paths = lambda *a, **k: pytest.fail("immutable prelude must not commit")
    assert d._run_prelude() is True


def test_default_oracle_execution_limit_is_unchanged(tmp_path):
    ctx = SimpleNamespace(repo=tmp_path, config=LoopConfig(), plan={}, harness_bin="harness")
    argv = harness.oracle_argv(ctx, "s.yaml", None, clause_text_json="clauses.json")
    assert argv[argv.index("--job-timeout") + 1] == "180"
    assert argv[argv.index("--oracle-timeout") + 1] == "300"


def test_manual_verdict_writer_uses_same_frozen_pair_and_refuses_drift(snapshot):
    ctx, _, _, _ = snapshot
    argv = harness.verdict_set_argv(ctx, interaction="qa:Q1", verdict="unreachable", reason="reviewed")
    assert argv[argv.index("--clause-text-json") + 1] == "book.json"
    assert argv[argv.index("--interaction-denominator") + 1] == "interactions.json"
    (Path(ctx.repo)/"book.json").write_text("{}")
    with pytest.raises(base.StageDeferred, match="snapshot"):
        harness.verdict_set_argv(ctx, clause="BT1-001#effect#0", verdict="unreachable", reason="reviewed")


@pytest.mark.parametrize("damage", ["shared_member", "unrelated_card", "dangling_row"])
@pytest.mark.parametrize("entry", ["prelude", "oracle", "manual"])
def test_full_source_universe_and_all_members_required(snapshot, damage, entry):
    from tools.card_loop import driver
    ctx, _, _, seal = snapshot
    root = Path(ctx.repo)
    book = json.loads((root/"book.json").read_text())
    artifact = json.loads((root/"interactions.json").read_text())
    if damage == "dangling_row":
        artifact["interactions"]["qa:unrelated"] = dict(artifact["interactions"]["qa:Q1"],
            card_ids=["BT1-999"])
    else:
        missing = "BT1-002" if damage == "shared_member" else "BT1-003"
        book["cards"].remove(missing)
        book["clauses"] = [r for r in book["clauses"] if r["card_id"] != missing]
        del artifact["cards"][missing]
    (root/"book.json").write_text(json.dumps(book))
    (root/"interactions.json").write_text(json.dumps(artifact))
    seal()
    (root/"s.yaml").write_text("card: BT1-001\nclause: BT1-001#effect#0\n")
    before = {p: p.read_bytes() for p in root.rglob("*.json")}
    if entry == "prelude":
        ctx.run_command = lambda *a: pytest.fail("must not append")
        d = object.__new__(driver.Driver)
        d.ctx, d.config, d.plan, d.repo = ctx, ctx.config, ctx.plan, ctx.repo
        d.state = SimpleNamespace(problems=[])
        d._commit_paths = lambda *a, **k: pytest.fail("must not commit")
        assert d._run_prelude() is False
    else:
        with pytest.raises(base.StageDeferred, match="snapshot"):
            if entry == "oracle":
                harness.oracle_argv(ctx, "s.yaml", None, clause_text_json="book.json")
            else:
                harness.verdict_set_argv(ctx, clause="BT1-001#effect#0", verdict="unreachable", reason="reviewed")
    assert all(p.read_bytes() == b for p, b in before.items())


def test_campaign_default_binds_actual_interaction_and_explicit_none_disables(snapshot, monkeypatch):
    from tools.clause_coverage import campaign, exam_binding
    ctx, _, rows, _ = snapshot
    root = Path(ctx.repo)
    (root/"s.yaml").write_text("card: BT1-001\nclause: qa:Q1\ninteraction: {id: 'qa:Q1', source: qa, kind: positive}\n")
    monkeypatch.setattr(exam_binding, "extract_run", lambda *a, **kw: {"clauses": rows})
    original_load = exam_binding.load_interaction_denominator
    def load(path):
        if path == exam_binding.DEFAULT_INTERACTION_DENOMINATOR:
            return original_load(root/"interactions.json")
        return original_load(path)
    monkeypatch.setattr(exam_binding, "load_interaction_denominator", load)
    real_bind = campaign.bind
    captured = []
    def capture(*a, **kw):
        result = real_bind(*a, **kw)
        captured.append(result)
        return result
    monkeypatch.setattr(campaign, "bind", capture)
    kwargs = dict(cards_dir=root, scenarios_dir=root, verdicts_path=root/"absent")
    campaign.build_plan_for_pool(["BT1-001"], [], **kwargs)
    assert captured[-1]["orphan_scenarios"] == []
    campaign.build_plan_for_pool(["BT1-001"], [], interaction_denominator=None, **kwargs)
    assert captured[-1]["orphan_scenarios"]
