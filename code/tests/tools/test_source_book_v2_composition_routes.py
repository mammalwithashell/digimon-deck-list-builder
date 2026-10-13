"""Additive route regressions; saved diagnostics establish consistency, not authority.

The full original R4 evidence is external and immutable. Set the explicit path
to run its regressions; its uncompressed bytes must match the recorded pin.
Portable route vectors added here must not masquerade as a public source replay.
"""
import copy
import gzip
import hashlib
import json
import os
from pathlib import Path

import pytest

from tools.clause_coverage import source_book_v2 as book

F = Path(__file__).parents[1] / "fixtures/card_data/source_book_v2_composition_routes"


@pytest.fixture(scope="module")
def fresh_six():
    raw = gzip.decompress((F / "fresh-six-diagnostic.json.gz").read_bytes())
    assert hashlib.sha256(raw).hexdigest() == "ce06fbf46a05af97dafe175f829073bc3c797f20045f429e73ef18e7555c61a3"
    return json.loads(raw)


def test_fresh_six_complete_diagnostic_keeps_pair_mirror_nonexecuting(fresh_six):
    result = book._compile_replayed_diagnostic(fresh_six)
    assert len(result["cards"]) == 4484
    assert result["diagnostic_evidence"] == fresh_six
    assert result["full_clause_count"] is None and result["full_gating_interaction_count"] is None
    assert result["cards"]["BT24-081"]["clause_ids"] == []
    assert result["cards"]["BT24-081"]["clause_count"] is None
    assert result["cards"]["BT21-071"]["status"] == "COMPILED_REVIEW_ONLY"
    corrected = result["cards"]["EX1-066"]
    assert corrected["status"] == "COMPILED_REVIEW_ONLY"
    assert corrected["superseded_source_occurrences"] == 7
    mirrors = [o for o in corrected["structural_obligations"] if o.get("kind") == "paired-main-face-consistency"]
    assert len(mirrors) == 1 and mirrors[0]["executable"] is False
    assert all(c["zone"] != "faces.0.effect_text" for c in result["clauses"])
    book.validate_wire_book_v2(result)


@pytest.mark.parametrize("change", ["mirror_value", "mirror_executable", "mirror_field", "mirror_duplicate",
    "unit_sidecar", "unit_kind", "unit_projection", "zone_binding", "lost_original",
    "original_text", "correction_value", "replacement_indices", "image_association",
    "images_empty", "image_single", "image_duplicate"])
def test_paired_mirror_and_originals_refuse_drift(fresh_six, change):
    row = copy.deepcopy(fresh_six["cards"]["EX1-066"])
    source = row["source_composition"]
    binding = source["correction_binding"]
    mirror = next(o for o in source["structural_obligations"] if o.get("kind") == "paired-main-face-consistency")
    unit = next(u for u in source["units"] if u.get("kind") == "reviewed-paired-main-image-correction")
    if change == "mirror_value": mirror["value"] += " changed"
    elif change == "mirror_executable": mirror["executable"] = True
    elif change == "mirror_field": mirror["field_path"] = "faces.0.security_text"
    elif change == "mirror_duplicate": source["structural_obligations"].append(copy.deepcopy(mirror))
    elif change == "unit_sidecar": unit["sidecar_sha256"] = "0" * 64
    elif change == "unit_kind": unit["kind"] = "unreviewed-correction"
    elif change == "unit_projection": unit["projected_text"] += " changed"
    elif change == "zone_binding": source["zones"]["effect"]["source_binding"] = {}
    elif change == "lost_original": binding["original_sections"].pop()
    elif change == "original_text": binding["original_sections"][0]["text"] += " changed"
    elif change == "correction_value": binding["corrections"][0]["value"] += " changed"
    elif change == "replacement_indices":
        next(r for r in source["consumption"] if r["disposition"] == "superseded-by-reviewed-correction")["replacement_unit_indices"] = []
    elif change == "image_association": binding["images"][0]["official_observation_id"] = "foreign"
    elif change == "images_empty": binding["images"] = []
    elif change == "image_single": binding["images"].pop()
    elif change == "image_duplicate": binding["images"][1] = copy.deepcopy(binding["images"][0])
    if change != "zone_binding":
        source["zones"]["effect"]["source_binding"] = copy.deepcopy(binding)
    # The descriptor helper independently refuses some structural corruption.
    with pytest.raises((ValueError, KeyError)):
        _refresh_descriptors("EX1-066", row)
        book._reconcile("EX1-066", row)


@pytest.fixture(scope="module")
def original_r4():
    path = os.environ.get("SOURCE_BOOK_V2_ORIGINAL_R4_DIAGNOSTIC")
    if path is None:
        pytest.skip("explicit immutable full R4 diagnostic path required")
    raw = gzip.decompress(Path(path).read_bytes())
    assert hashlib.sha256(raw).hexdigest() == (
        "a468f5178f7814d5ac05df5dbb90cb9d54cf2146cca35b3e61db1386afb39d33"
    )
    value = json.loads(raw)
    assert len(value["cards"]) == 4484
    assert len(value["evaluated_card_ids"]) == 99
    return value


def test_original_blocked_known_source_count_is_retained(original_r4):
    row = original_r4["cards"]["BT24-081"]
    assert row["status"] == "BLOCKED"
    assert row["span_count"] == 7
    assert row["blockers"]
    book._validate_inventory(original_r4)
    assert row["status"] == "BLOCKED" and row["span_count"] == 7


def test_original_link_route_reconciles_without_executing_folded_dp(original_r4):
    row = original_r4["cards"]["BT21-074"]
    before = copy.deepcopy(row)
    occurrences = book._reconcile("BT21-074", row)
    assert row == before
    assert len(occurrences) == 4
    assert [x["zone"] for x in occurrences].count("link") == 1
    assert [x["zone"] for x in occurrences].count("conditions") == 1
    assert all("+4000 DP" not in x["span"][3] for x in occurrences)
    assert len(row["source_unit_descriptors"]) == 4
    assert row["structural_obligations"]


def test_original_full_inventory_book_keeps_blocked_known_evidence(original_r4):
    result = book._compile_replayed_diagnostic(original_r4)
    assert result["diagnostic_evidence"] == original_r4
    assert len(result["cards"]) == 4484
    assert result["full_clause_count"] is None
    assert result["full_gating_interaction_count"] is None
    blocked = result["cards"]["BT24-081"]
    assert blocked["status"] == "BLOCKED"
    assert blocked["blockers"] == original_r4["cards"]["BT24-081"]["blockers"]
    assert blocked["clause_count"] is None and blocked["clause_ids"] == []
    assert not any(x["card_id"] == "BT24-081" for x in result["clauses"])
    assert result["cards"]["BT21-074"]["clause_count"] == 4


@pytest.mark.parametrize("change", [
    "boolean_count", "negative_count", "wrong_count", "missing_composition",
    "missing_units", "missing_spans", "missing_obligations", "missing_conservation",
    "missing_blocker", "not_evaluated", "invented_known_count",
])
def test_known_blocked_count_requires_complete_retained_evidence(original_r4, change):
    # Shallow copy unrelated rows: the mutation is confined to this exact row.
    diagnostic = {**original_r4, "cards": dict(original_r4["cards"])}
    row = copy.deepcopy(original_r4["cards"]["BT24-081"])
    diagnostic["cards"]["BT24-081"] = row
    if change == "boolean_count":
        row["span_count"] = True
    elif change == "negative_count":
        row["span_count"] = -1
    elif change == "wrong_count":
        row["span_count"] = 0
    elif change == "missing_blocker":
        row["blockers"] = []
    elif change == "not_evaluated":
        row.update(status="NOT_EVALUATED", blockers=[])
    elif change == "invented_known_count":
        row = copy.deepcopy(original_r4["cards"]["LM-063"])
        row["span_count"] = 0
        diagnostic["cards"]["LM-063"] = row
    else:
        key = {"missing_composition": "source_composition", "missing_units": "source_unit_descriptors",
               "missing_spans": "span_descriptors", "missing_obligations": "structural_obligations",
               "missing_conservation": "conservation"}[change]
        row.pop(key)
    with pytest.raises((ValueError, KeyError)):
        book._validate_inventory(diagnostic)


def _refresh_descriptors(card_id, row):
    from tools.clause_coverage.diagnostic_source_book import _descriptors
    values = _descriptors(card_id, row["source_composition"])
    for key, value in zip(("source_unit_descriptors", "span_descriptors",
                           "structural_obligations", "conservation"), values):
        row[key] = value
    row["span_count"] = len(row["span_descriptors"])


@pytest.mark.parametrize("change", [
    "dp_role", "dp_value", "dp_projection", "dp_section", "dp_obligation_missing",
    "dp_derivation_missing", "dp_extra_span", "condition_member_missing",
    "condition_projection", "condition_observation", "condition_empty_claim",
])
def test_link_source_changes_refuse_even_with_refreshed_descriptor_views(original_r4, change):
    row = copy.deepcopy(original_r4["cards"]["BT21-074"])
    source = row["source_composition"]
    dp = source["units"][1]
    if change == "dp_role":
        dp["role"] = "effect"
    elif change == "dp_value":
        dp["text"] = "+3000 DP"
    elif change == "dp_projection":
        dp["projected_text"] = "[Special Rule] +3000 DP"
    elif change == "dp_section":
        dp["section_index"] = 1
    elif change == "dp_obligation_missing":
        source["structural_obligations"] = [o for o in source["structural_obligations"] if o["role"] != "dp"]
    elif change == "dp_derivation_missing":
        derivation = next(iter(source["link_derivations"].values()))
        derivation["occurrences"] = [o for o in derivation["occurrences"] if o["role"] != "dp"]
    elif change == "dp_extra_span":
        source["zones"]["effect"]["spans"].append({"unit_index": 1, "span_index": 0,
            "kind": "untimed", "timings": [], "keyword": None, "text": "+4000 DP"})
    elif change == "condition_member_missing":
        source["condition_binding"]["candidate_values"] = {}
    elif change == "condition_projection":
        source["condition_binding"]["projection"] = "[Digivolve] Cost 0"
    elif change == "condition_observation":
        source["condition_binding"]["evidence_observation_ids"] = ["foreign"]
    elif change == "condition_empty_claim":
        source["condition_binding"]["a2_section_coverage_claimed"] = True
    _refresh_descriptors("BT21-074", row)
    with pytest.raises((ValueError, KeyError)):
        book._reconcile("BT21-074", row)


def test_link_all_selected_condition_occurrences_must_be_consumed(fresh_six):
    row = copy.deepcopy(fresh_six["cards"]["BT21-074"])
    source = row["source_composition"]
    assert source["zones"]["conditions"]["unit_indices"] == [3]
    source["units"].pop()
    source["zones"]["conditions"]["unit_indices"] = []
    source["zones"]["conditions"]["spans"] = []
    for consumed in source["consumption"]:
        if consumed["unit_indices"] == [3]:
            consumed["unit_indices"] = []
            consumed["disposition"] = "retained-alternative"
    _refresh_descriptors("BT21-074", row)
    with pytest.raises(ValueError):
        book._reconcile("BT21-074", row)
