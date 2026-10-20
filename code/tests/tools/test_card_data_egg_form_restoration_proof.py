"""Exact frozen official Egg fixtures plus proof-only adversaries."""
import copy
import hashlib
import json
from pathlib import Path

import pytest

from tools.card_data import egg_form_restoration_proof as egg
from tools.card_data.form_restoration_proof import ProofError
from tools.card_data.serialization import canonical_json_bytes, content_digest

FIXTURES = Path(__file__).parents[1] / "fixtures/card_data/egg_form_restoration"


def fixture(cid="BT10-001"):
    meta = json.loads((FIXTURES / (cid + ".json")).read_text(encoding="utf-8"))
    raw = (FIXTURES / (cid + ".html")).read_bytes()
    assert hashlib.sha256(raw).hexdigest() == meta["fixture_sha256"]
    observation = copy.deepcopy(meta["original_observation"])
    observation["raw_payload_digest"] = hashlib.sha256(raw).hexdigest()
    report = dict(old_input_revision="1" * 64, new_input_revision="2" * 64,
                  runtime_field_diffs=[dict(card_id=cid, field_path="form_eng", old_value=[],
                                           new_value=["In-Training"])],
                  corrections=[], compatibility_corrections=[], canonical_conflicts=[])
    snapshot = dict(manifest=dict(complete=True, failed_ids=[], expected_count=1),
                    observations=[observation])
    bundle = [report, snapshot, {observation["source_url"]: raw}]
    reidentify(bundle)
    return bundle


def reidentify(bundle):
    report, snapshot, pages = bundle
    for o in snapshot["observations"]:
        o["raw_payload_digest"] = hashlib.sha256(pages[o["source_url"]]).hexdigest()
        o["observation_id"] = content_digest({k: o[k] for k in
            ("raw_payload_digest", "parser_version", "source_schema_version", "extracted")},
            exclude_operational=True)
    snapshot["manifest"]["observation_ids"] = [o["observation_id"] for o in snapshot["observations"]]
    snapshot["manifest"]["observation_refs"] = [dict(observation_id=o["observation_id"],
        gameplay_card_id=o["extracted"]["card_id"], printing_id=o["extracted"]["printing"]["printing_id"])
        for o in snapshot["observations"]]
    for d in report["runtime_field_diffs"]:
        d["old_digest"], d["new_digest"] = content_digest(d["old_value"]), content_digest(d["new_value"])
        d["provenance_observation_ids"] = [o["observation_id"] for o in snapshot["observations"]
            if o["extracted"]["card_id"] == d["card_id"]]


def args(bundle):
    return dict(report_bytes=canonical_json_bytes(bundle[0]),
                snapshot_bytes=canonical_json_bytes(bundle[1]), raw_pages=bundle[2])


@pytest.mark.parametrize("cid", ["BT10-001", "BT10-002"])
def test_real_egg_literal_form_proof_only(cid):
    p = egg.prove(**args(fixture(cid)))
    assert p["predicate_version"] == "simple-printed-egg-form-v1"
    assert p["status"] == "PROOF_ONLY_NO_APPROVALS"
    assert len(p["eligible"]) == 1 and p["total_form_diffs"] == 1 and not p["excluded"]
    assert p["eligible"][0]["diff"]["new_value"] == ["In-Training"]
    assert "adjudications" not in p and not hasattr(egg, "emit_reviewed")


@pytest.mark.parametrize("change", ["digimon", "tamer", "option", "missing_type", "rookie", "hybrid",
    "wrong_level", "wrong_face_kind", "missing_face", "missing_grants", "grant", "dual", "correction",
    "compatibility_correction", "form_conflict", "adjacent_conflict", "old_null", "old_nonempty",
    "new_multi", "raw_type", "raw_form", "raw_hidden", "raw_duplicate", "raw_wrong_image",
    "printed_rule", "faq_rule", "raw_special_rule", "known_gap"])
def test_exact_egg_boundary_excludes(change):
    b = fixture(); d = b[0]["runtime_field_diffs"][0]; o = b[1]["observations"][0]; e = o["extracted"]
    raw = b[2][o["source_url"]]
    if change in {"digimon", "tamer", "option", "missing_type"}:
        e["card_type"] = {"digimon": "Digimon", "tamer": "Tamer", "option": "Option", "missing_type": ""}[change]
    elif change in {"rookie", "hybrid"}: e["form"] = change.title()
    elif change == "wrong_level": e["level"] = 3
    elif change == "wrong_face_kind": e["faces"][0]["kind"] = "digimon"
    elif change == "missing_face": e["faces"] = []
    elif change == "missing_grants": e.pop("rule_grants")
    elif change == "grant": e["rule_grants"]["forms"] = ["Hybrid"]
    elif change == "dual": e["layout"] = "dual"
    elif change in {"correction", "compatibility_correction"}:
        b[0]["corrections" if change == "correction" else "compatibility_corrections"] = [dict(card_id=e["card_id"], field_path="dp")]
    elif change in {"form_conflict", "adjacent_conflict"}:
        b[0]["canonical_conflicts"] = [dict(card_id=e["card_id"], field_path="form_eng" if change == "form_conflict" else "card_kind", approval_required=True)]
    elif change == "old_null": d["old_value"] = None
    elif change == "old_nonempty": d["old_value"] = ["In-Training"]
    elif change == "new_multi": d["new_value"] = ["In-Training", "Hybrid"]
    elif change == "raw_type": raw = raw.replace(b'Digi-Egg', b'Digimon')
    elif change == "raw_form": raw = raw.replace(b'>In-Training<', b'>Rookie<')
    elif change == "raw_hidden": raw = raw.replace(b'>In-Training<', b'><span hidden>In-Training</span><')
    elif change == "raw_duplicate": raw += raw
    elif change == "raw_wrong_image": raw = raw.replace(b'BT10-001.png', b'BT10-002.png')
    elif change == "printed_rule": raw = raw.replace(b'</div>', b'<p>[Rule] Trait: Has [Hybrid] Form.</p></div>', 1)
    elif change == "faq_rule": raw = raw.replace(b'</div>', b'<dl class="cardQA"><dt>FAQ</dt><dd>Includes (Rule) text.</dd></dl></div>', 1)
    elif change == "raw_special_rule": raw = raw.replace(b'</div>', b'<dt>Special Rule</dt><dd>Something</dd></div>', 1)
    elif change == "known_gap":
        e["card_id"] = d["card_id"] = "BT10-111"; e["printing"]["printing_id"] = "BT10-111"
    if change.startswith("raw_") or change in {"printed_rule", "faq_rule"}:
        assert raw != b[2][o["source_url"]], "adversary must actually alter fixture bytes"
    b[2][o["source_url"]] = raw; reidentify(b)
    p = egg.prove(**args(b))
    assert not p["eligible"] and len(p["excluded"]) == 1


def test_missing_alternate_ref_and_dissenting_alternate():
    b = fixture(); alternate = copy.deepcopy(b[1]["observations"][0])
    alternate["extracted"]["form"] = "Rookie"
    alternate["extracted"]["printing"]["printing_id"] += "_P1"
    b[1]["observations"].append(alternate); reidentify(b)
    assert not egg.prove(**args(b))["eligible"]
    b[1]["manifest"]["observation_refs"].pop()
    with pytest.raises(ProofError, match="closure"):
        egg.prove(**args(b))


def test_full_form_diff_denominator_not_eligible_subset():
    b = fixture(); d = copy.deepcopy(b[0]["runtime_field_diffs"][0])
    d["card_id"] = "BT1-003"; d["new_value"] = ["Rookie"]
    b[0]["runtime_field_diffs"].append(d); reidentify(b)
    p = egg.prove(**args(b))
    assert p["total_form_diffs"] == 2
    assert len(p["eligible"]) == len(p["excluded"]) == 1


def test_existing_review_status_does_not_shrink_proof_membership():
    b = fixture(); other = fixture("BT10-002")
    b[0]["runtime_field_diffs"] += other[0]["runtime_field_diffs"]
    b[0]["runtime_field_diffs"][0]["approved"] = True
    b[1]["observations"] += other[1]["observations"]
    b[1]["manifest"]["expected_count"] = 2
    for url, raw in other[2].items():
        b[2][url] = b[2].get(url, b"") + raw
    reidentify(b)
    p = egg.prove(**args(b))
    assert p["total_form_diffs"] == 2 and len(p["eligible"]) == 2
    assert {r["diff"]["card_id"] for r in p["eligible"]} == {"BT10-001", "BT10-002"}


@pytest.mark.parametrize("change", ["missing_raw", "bad_raw", "bad_identity", "bad_digest", "missing_provenance"])
def test_forged_input_refused(change):
    b = fixture(); o = b[1]["observations"][0]
    if change == "missing_raw": b[2].clear()
    elif change == "bad_raw": b[2][o["source_url"]] += b'corrupt'
    elif change == "bad_identity": o["observation_id"] = "f" * 64
    elif change == "bad_digest": b[0]["runtime_field_diffs"][0]["new_digest"] = "f" * 64
    elif change == "missing_provenance": b[0]["runtime_field_diffs"][0]["provenance_observation_ids"] = []
    with pytest.raises(ProofError): egg.prove(**args(b))


def test_proof_deterministic_and_dependencies_bound():
    b = fixture(); p = egg.prove(**args(b))
    assert p == egg.prove(**args(b))
    assert {"form_restoration_proof.py", "models.py", "serialization.py", "adjudications.py"} <= p["dependency_sha256"].keys()


def test_cli_proof_only_and_no_overwrite(tmp_path):
    b = fixture(); report, snapshot, pages = b
    (tmp_path / "report.json").write_bytes(canonical_json_bytes(report))
    (tmp_path / "snapshot.json").write_bytes(canonical_json_bytes(snapshot))
    url, raw = next(iter(pages.items())); (tmp_path / "raw.html").write_bytes(raw)
    (tmp_path / "index.json").write_text(json.dumps([dict(url=url, file="raw.html")]))
    argv = ["prove", "--report", str(tmp_path / "report.json"), "--snapshot", str(tmp_path / "snapshot.json"),
            "--cache-index", str(tmp_path / "index.json"), "--out", str(tmp_path / "proof.json")]
    assert egg.main(argv) == 0
    before = (tmp_path / "proof.json").read_bytes()
    assert json.loads(before)["status"] == "PROOF_ONLY_NO_APPROVALS"
    with pytest.raises(ProofError, match="overwrite"): egg.main(argv)
    assert (tmp_path / "proof.json").read_bytes() == before
    with pytest.raises(SystemExit): egg.main(["emit-reviewed", *argv[1:]])
