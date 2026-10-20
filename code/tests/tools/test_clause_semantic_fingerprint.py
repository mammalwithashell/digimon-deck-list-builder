"""Frozen semantic binding regressions: timing spans govern sibling keywords."""
from copy import deepcopy
import json

import pytest


def zone():
    return [dict(id="EX12-062#inherited#0", card_id="EX12-062", zone="inherited",
                 label="timing", kind="timing", timings=["When Attacking", "Once Per Turn"],
                 keyword=None, text="", source="bundle"),
            dict(id="EX12-062#inherited#1", card_id="EX12-062", zone="inherited",
                 label="draw", kind="keyword", timings=[], keyword="Draw", text="1",
                 source="bundle")]


@pytest.mark.parametrize("change", ["timing", "opt", "draw", "optional"])
def test_zone_semantics_invalidate_every_sibling_without_changing_ids(change):
    from tools.clause_coverage.semantic_fingerprint import bind_clause_semantics
    before = zone()
    after = deepcopy(before)
    if change == "timing":
        after[0]["timings"][0] = "On Play"
    elif change == "opt":
        after[0]["timings"].pop()
    elif change == "draw":
        after[1]["text"] = "2"
    else:
        after[1]["text"] = "You may draw 1"
    left, right = bind_clause_semantics(before), bind_clause_semantics(after)
    assert [c["id"] for c in left] == [c["id"] for c in right]
    assert all(a["semantic_sha256"] != b["semantic_sha256"] for a, b in zip(left, right))
    assert before == zone(), "binding must not mutate source evidence"


def test_provenance_and_labels_are_not_semantics_but_crlf_normalizes():
    from tools.clause_coverage.semantic_fingerprint import bind_clause_semantics
    before = zone()
    before[1]["text"] = '＜Draw＞ "1"\r\nnext\rline'
    after = deepcopy(before)
    after[1].update(label="new label", source="image-cache", image_path="new/path",
                    text='＜Draw＞ "1"\nnext\nline')
    assert [c["semantic_sha256"] for c in bind_clause_semantics(before)] == [
        c["semantic_sha256"] for c in bind_clause_semantics(after)]


@pytest.mark.parametrize("damage", ["id", "gap", "duplicate", "missing_kind"])
def test_incomplete_or_inconsistent_zone_is_not_bound(damage):
    from tools.clause_coverage.semantic_fingerprint import bind_clause_semantics
    rows = zone()
    if damage == "id":
        rows[0]["id"] = "BT1-001#effect#0"
    elif damage == "gap":
        rows = rows[1:]
    elif damage == "duplicate":
        rows.append(deepcopy(rows[0]))
    else:
        del rows[0]["kind"]
    with pytest.raises(ValueError):
        bind_clause_semantics(rows)


def test_frozen_cross_language_vectors():
    from pathlib import Path
    from tools.clause_coverage.semantic_fingerprint import semantic_digest
    vectors = json.loads((Path(__file__).parents[1] / "fixtures/semantic_fingerprints_v1.json").read_text(encoding="utf-8"))
    for vector in vectors:
        assert semantic_digest(vector["payload"]) == vector["sha256"]


@pytest.mark.parametrize("verdict", ["confirmed", "diverged", "unreachable", "unavailable", "unmeasured"])
@pytest.mark.parametrize("binding", ["legacy", "unknown", "sibling_changed", "current"])
def test_reader_preserves_historical_evidence_but_fails_closed(tmp_path, monkeypatch, verdict, binding):
    from tools.clause_coverage import exam_binding
    from tools.clause_coverage.semantic_fingerprint import bind_clause_semantics
    rows = bind_clause_semantics(zone())
    record = dict(clause_id=rows[1]["id"], verdict=verdict, reason="original reason",
                  triage="ours_wrong", citation="original citation")
    if binding != "legacy":
        record.update(semantic_fingerprint_version=99 if binding == "unknown" else 1,
                      semantic_sha256=rows[1]["semantic_sha256"])
    if binding == "sibling_changed":
        changed = zone()
        changed[0]["timings"] = ["On Play"]
        rows = bind_clause_semantics(changed)
    monkeypatch.setattr(exam_binding, "extract_run", lambda *a, **k: {"clauses": rows})
    path = tmp_path / "verdicts.json"
    path.write_text(json.dumps({"version": 3, "clauses": {record["clause_id"]: record}}))
    before = path.read_bytes()
    result = exam_binding.bind(["EX12-062"], None, path)
    row = result["cards"]["EX12-062"]["clauses"][1]
    assert row["verdict"] == (verdict if binding == "current" else "unmeasured")
    assert row["invalidated"] == (binding != "current")
    assert row["stored_verdict"] == verdict
    assert row["stored_record"] == record
    assert path.read_bytes() == before
    assert sum(result["denominator"]["by_verdict"].values()) == 2


def test_book_validation_detects_cached_timing_drift_without_rewriting(tmp_path):
    from tools.clause_coverage.book import validate_book
    from tools.clause_coverage.semantic_fingerprint import bind_clause_semantics
    rows = bind_clause_semantics(zone())
    path = tmp_path / "book.json"
    path.write_text(json.dumps({"cards": ["EX12-062"], "clauses": rows}))
    before = path.read_bytes()
    assert validate_book(path, extract=lambda *a: {"clauses": rows}) == []
    changed = zone()
    changed[0]["timings"] = ["On Play"]
    findings = validate_book(path, extract=lambda *a: {"clauses": bind_clause_semantics(changed)})
    assert len(findings) == 2
    assert all(row["kind"] == "source_changed" for row in findings)
    assert path.read_bytes() == before


def test_probe_hash_changes_on_timing_drift_without_dropping_denominator():
    from tools.card_loop.interactions.denominator import build_denominator
    before = zone()
    after = deepcopy(before)
    after[0]["timings"][0] = "On Play"
    # Keep the same timing-gate / OPT families: only the target timing changed.
    old = build_denominator(["EX12-062"], before, {}, {"families@1": True})
    new = build_denominator(["EX12-062"], after, {}, {"families@1": True})
    assert old["cards"] == new["cards"]
    assert old["summary"] == new["summary"]
    for iid, row in old["interactions"].items():
        assert row["gating"] == new["interactions"][iid]["gating"]
        assert row["text_sha256"] == new["interactions"][iid]["text_sha256"]
        assert row["semantic_sha256"] != new["interactions"][iid]["semantic_sha256"]


def test_clause_model_round_trip_retains_semantic_metadata():
    from tools.clause_coverage.models import clause_from_dict
    from tools.clause_coverage.semantic_fingerprint import bind_clause_semantics
    row = bind_clause_semantics(zone())[0]
    result = clause_from_dict(row).to_dict()
    for name in ("semantic_fingerprint_version", "semantic_sha256", "semantic_payload"):
        assert result[name] == row[name]


def test_writer_rejects_a_claimant_hash_without_source_payload(tmp_path):
    from tools.clause_coverage.exam_binding import write_interaction_verdict
    row = dict(interaction_id="qa:Q1", card_ids=["EX12-062"], source="qa", kind="positive",
               verdict="confirmed", text_sha256="a" * 64, recorded_at="2026-10-11",
               semantic_fingerprint_version=1, semantic_sha256="b" * 64)
    with pytest.raises(ValueError, match="source binding refused"):
        write_interaction_verdict(tmp_path, row)
    assert not list(tmp_path.iterdir())


@pytest.mark.parametrize("version", [None, True, 1.0, "1", 99])
def test_version_is_an_exact_integer_contract(version):
    from tools.clause_coverage.semantic_fingerprint import bind_clause_semantics, binding_error
    row = bind_clause_semantics(zone())[0]
    bad = {**row, "semantic_fingerprint_version": version}
    assert binding_error(row, bad)
    assert binding_error(bad, row)

@pytest.mark.parametrize('version', [1, 2])
def test_legacy_container_never_blesses_matching_semantic_metadata(tmp_path, version):
    from tools.clause_coverage.exam_binding import _read_store_file
    from tools.clause_coverage.semantic_fingerprint import bind_clause_semantics, binding_error
    row = bind_clause_semantics(zone())[0]
    record = dict(row, verdict='confirmed')
    path = tmp_path / 'legacy.json'
    path.write_text(json.dumps({'version': version, 'clauses': {row['id']: record}}))
    loaded = _read_store_file(path)
    assert binding_error(row, loaded['clauses'][row['id']]) is not None
    assert dict(loaded['clauses'][row['id']]) == record
    loaded['version'] = 3
    path.write_text(json.dumps(loaded))
    assert binding_error(row, _read_store_file(path)['clauses'][row['id']]) is not None

@pytest.mark.parametrize('change', ['timing', 'answer', 'membership'])
def test_old_interaction_artifact_is_not_current_source(tmp_path, monkeypatch, change):
    from tools.clause_coverage import exam_binding
    from tools.card_loop.interactions import denominator as den
    qa = {'cards': {'EX12-062': ['Q1']}, 'qa': {'Q1': {'card_ids':['EX12-062'], 'question':'When?', 'answer':'Once.'}}}
    old = den.build_denominator(['EX12-062'], zone(), qa, {'families@1': True})
    current_rows, current_qa = deepcopy(zone()), deepcopy(qa)
    if change == 'timing':
        current_rows[0]['timings'][0] = 'On Play'
        iid = 'probe:EX12-062#inherited#0:once_per_turn_multi'
    else:
        iid = 'qa:Q1'
        if change == 'answer': current_qa['qa']['Q1']['answer'] = 'Twice.'
        else: current_qa['qa']['Q1']['card_ids'].append('BT1-001')
    entry = dict(old['interactions'][iid], interaction_id=iid, verdict='confirmed', reason='original evidence')
    artifact, store = tmp_path/'den.json', tmp_path/'store.json'
    artifact.write_text(json.dumps(old))
    store.write_text(json.dumps({'version':3,'interactions':{iid:entry}}))
    from tools.clause_coverage.semantic_fingerprint import bind_clause_semantics
    monkeypatch.setattr(exam_binding, 'extract_run', lambda *a, **k: {'clauses':bind_clause_semantics(current_rows)})
    monkeypatch.setattr(den, 'load_card_qa', lambda *a, **k: current_qa)
    before = store.read_bytes()
    report = exam_binding.bind_interactions(['EX12-062'], artifact, store)
    row = next(r for r in report['cards']['EX12-062']['interactions'] if r['interaction_id'] == iid)
    assert row['verdict'] == 'unmeasured'
    assert row['stored_verdict'] == 'confirmed' and row['stored_record'] == entry
    assert report['denominator']['total_interactions'] == len(old['cards']['EX12-062'])
    assert store.read_bytes() == before

    # Fresh clause retests must not let stale interactions clear readiness.
    fresh = bind_clause_semantics(current_rows)
    doc = json.loads(store.read_text())
    doc["clauses"] = {r["id"]: dict(clause_id=r["id"], card_id=r["card_id"], verdict="confirmed",
                                    semantic_fingerprint_version=1, semantic_sha256=r["semantic_sha256"])
                      for r in fresh}
    store.write_text(json.dumps(doc))
    from tools.clause_coverage.readiness import build_readiness
    ready = build_readiness(["EX12-062"], scenarios_dir=tmp_path/"scenarios", verdicts_dir=store,
                            denominator_path=artifact)
    assert ready["cards"]["EX12-062"]["status"] == "not_ready"
    assert iid in ready["cards"]["EX12-062"]["blocking"]
