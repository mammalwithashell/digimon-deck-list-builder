"""Cheap complete-source condition report: all rows, exact bindings, no approvals."""
from dataclasses import replace
import json

import pytest

from test_card_data_assembly_projection import source
from test_card_data_compatibility_selections import conflict, selection
from tools.card_data.models import ScopeManifest, ScopeObservationRef, SourceName
from tools.card_data.sources.official import OfficialScopeSnapshot
from tools.card_data.serialization import content_digest, observation_digest
from tools.card_data.errors import CardDataError


def snapshots():
    _, observations, choice = conflict()
    other = source("EX12-063")
    observations[other.observation_id] = other
    def snap(rows, lane, name):
        manifest = ScopeManifest(schema_version=1, source=lane, scope="full-pool:" + name,
            source_locations=("https://example.test/" + name,), first_acquired_at="2026-10-12",
            parser_version=rows[0].parser_version, source_schema_version=rows[0].source_schema_version,
            observation_ids=tuple(r.observation_id for r in rows),
            observation_refs=tuple(ScopeObservationRef(r.observation_id, r.extracted["card_id"],
                r.extracted["card_id"]) for r in rows), complete=True,
            expected_count=len({r.extracted["card_id"] for r in rows}))
        return OfficialScopeSnapshot(manifest, tuple(rows))
    secondary = replace(other, source=SourceName.DIGIMONCARD_IO,
        parser_version="secondary-fixture-v1", extracted=dict(other.extracted, card_id="EX12-065"))
    secondary = replace(secondary, observation_id=observation_digest(secondary))
    return (snap(list(observations.values()), SourceName.OFFICIAL, "official-catalog"),
        snap([secondary], SourceName.DIGIMONCARD_IO, "digimoncard-io-catalog"), choice)


def test_full_aggregate_report_keeps_all_official_rows_and_secondary_only_membership():
    from tools.card_data.condition_review_report import build_condition_review_report
    from tools.card_data.source_manifest import build_full_pool_source_manifest
    official, secondary, _ = snapshots()
    report = build_condition_review_report(official, secondary)
    aggregate = build_full_pool_source_manifest((official.manifest, secondary.manifest))
    assert report["aggregate_source_revision"] == aggregate.manifest_revision
    assert report["official_card_count"] == 2
    assert report["official_observation_count"] == 3
    assert report["secondary_card_count"] == 1
    assert report["secondary_observation_count"] == 1
    assert report["aggregate_card_count"] == 3
    assert report["secondary_only_card_ids"] == ["EX12-065"]
    assert [r["card_id"] for r in report["cards"]] == ["EX12-046", "EX12-063"]
    assert report["unresolved_card_count"] == 1
    assert report["status"] == "blocked"
    blocked = report["cards"][0]
    assert blocked["status"] == "unresolved"
    assert len(blocked["value_groups"]) == 2
    binding = blocked["review_binding"]
    assert binding["source_manifest_revision"] == aggregate.manifest_revision
    assert binding["candidate_set_digest"] == content_digest(binding["candidate_values"])
    assert "selected_observation_id" not in binding
    assert "reviewer" not in binding
    assert report["selections_created"] == 0
    assert report["cards"][1]["status"] == "consensus"


def test_current_reviewed_selection_is_reported_but_never_created_or_auto_approved():
    from tools.card_data.condition_review_report import build_condition_review_report
    official, secondary, payload = snapshots()
    base = build_condition_review_report(official, secondary)
    payload["source_manifest_revision"] = base["aggregate_source_revision"]
    report = build_condition_review_report(official, secondary, selections=(selection(payload),))
    assert report["unresolved_card_count"] == 0
    assert report["cards"][0]["status"] == "reviewed"
    assert len(report["cards"][0]["value_groups"]) == 2  # rejected source values remain visible
    assert report["cards"][0]["selection_id"] == payload["selection_id"]
    assert report["selections_created"] == 0
    assert report["training_readiness_assessed"] is False
    assert report["runtime_cutover_approved"] is False


@pytest.mark.parametrize("lane", ["official", "secondary"])
@pytest.mark.parametrize("change", ["missing", "extra", "duplicate", "wrong_identity", "wrong_card_ref", "wrong_count", "incomplete"])
def test_partial_or_corrupt_source_scope_cannot_claim_complete_report(change, lane):
    from tools.card_data.condition_review_report import build_condition_review_report
    official, secondary, _ = snapshots()
    if lane == "secondary": official, secondary = secondary, official
    if change == "missing": official = replace(official, observations=official.observations[:-1])
    elif change == "extra": official = replace(official, observations=(*official.observations, source("EX12-076")))
    elif change == "duplicate": official = replace(official, observations=(*official.observations, official.observations[0]))
    elif change == "wrong_identity":
        bad = replace(official.observations[0], raw_payload_digest="a" * 64)
        official = replace(official, observations=(bad, *official.observations[1:]))
    elif change == "wrong_card_ref":
        refs = official.manifest.observation_refs
        official = replace(official, manifest=replace(official.manifest,
            observation_refs=(replace(refs[0], gameplay_card_id="EX12-076"), *refs[1:])))
    elif change == "wrong_count": official = replace(official, manifest=replace(official.manifest, expected_count=official.manifest.expected_count + 1))
    else: official = replace(official, manifest=replace(official.manifest, complete=False))
    if lane == "secondary": official, secondary = secondary, official
    with pytest.raises(CardDataError): build_condition_review_report(official, secondary)


def test_scope_argument_order_and_card_focus_cannot_shrink_source_inventory():
    from tools.card_data.condition_review_report import build_condition_review_report
    official, secondary, _ = snapshots()
    with pytest.raises(CardDataError): build_condition_review_report(secondary, official)
    with pytest.raises(TypeError): build_condition_review_report(official, secondary, card_ids=["EX12-046"])


def test_report_determinism_and_no_input_mutation():
    from tools.card_data.condition_review_report import build_condition_review_report
    official, secondary, _ = snapshots()
    before = content_digest((official, secondary))
    first = build_condition_review_report(official, secondary)
    second = build_condition_review_report(replace(official, observations=tuple(reversed(official.observations))), secondary)
    assert first == second
    assert content_digest((official, secondary)) == before


def test_cli_does_not_overwrite_or_write_inside_review_tree(tmp_path, monkeypatch):
    from tools.card_data import condition_review_report as module
    official, secondary, _ = snapshots()
    monkeypatch.setattr(module, "load_review_snapshots", lambda root: (official, secondary))
    tree = tmp_path / "review-tree"
    tree.mkdir()
    output = tmp_path / "condition-review.json"
    args = ["--review-tree", str(tree), "--output", str(output)]
    assert module.main(args) == 2
    before = output.read_bytes()
    with pytest.raises(FileExistsError): module.main(args)
    assert output.read_bytes() == before
    with pytest.raises(ValueError): module.main(["--review-tree", str(tree), "--output", str(tree / "new.json")])


def test_unrelated_canonical_conflicts_remain_visible_and_unresolved():
    from tools.card_data.condition_review_report import build_condition_review_report
    official, secondary, _ = snapshots()
    first = official.observations[0]
    changed = replace(first, extracted=dict(first.extracted, type="Different printed type"))
    changed = replace(changed, observation_id=observation_digest(changed))
    official = replace(official, observations=(changed, *official.observations[1:]),
        manifest=replace(official.manifest,
            observation_ids=tuple(changed.observation_id if oid == first.observation_id else oid
                for oid in official.manifest.observation_ids),
            observation_refs=tuple(replace(ref, observation_id=changed.observation_id)
                if ref.observation_id == first.observation_id else ref for ref in official.manifest.observation_refs)))
    report = build_condition_review_report(official, secondary)
    row = next(r for r in report["cards"] if r["card_id"] == "EX12-046")
    assert any(c["field_path"] == "type_eng" and c["requires_adjudication"] for c in row["canonical_conflicts"])
    assert report["canonical_required_conflict_count"] > 0
    assert report["runtime_cutover_approved"] is False
