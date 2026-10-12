"""Bound whole-pool hashing work without caching revisions across reports."""
from dataclasses import replace

import pytest

from test_card_data_condition_review_report import snapshots
from test_card_data_compatibility_selections import selection
from tools.card_data.condition_review_report import build_condition_review_report
from tools.card_data.source_manifest import AggregateSourceManifest


@pytest.mark.parametrize("with_selection", [False, True])
def test_one_full_pool_revision_hash_per_report_and_fresh_next_input(monkeypatch, with_selection):
    official, secondary, payload = snapshots()
    baseline = build_condition_review_report(official, secondary)
    payload["source_manifest_revision"] = baseline["aggregate_source_revision"]
    selections = (selection(payload),) if with_selection else ()
    expected = build_condition_review_report(official, secondary, selections=selections)
    original_getter = AggregateSourceManifest.manifest_revision.fget
    calls = []

    def counted(instance):
        calls.append(instance)
        return original_getter(instance)

    monkeypatch.setattr(AggregateSourceManifest, "manifest_revision", property(counted))
    actual = build_condition_review_report(official, secondary, selections=selections)
    assert actual == expected  # includes all rows, conflicts and the report digest
    assert len(calls) == 1, "whole-source payload must not be rehashed for every card"

    # The bound value is report-local: a new source manifest must be rehashed.
    changed = replace(secondary, manifest=replace(
        secondary.manifest, scope="full-pool:other-secondary-catalog"))
    following = build_condition_review_report(official, changed, selections=selections)
    assert len(calls) == 2
    assert following["aggregate_source_revision"] != actual["aggregate_source_revision"]
    assert all(row["review_binding"]["source_manifest_revision"] ==
               following["aggregate_source_revision"] for row in following["cards"])
    if with_selection:
        assert following["cards"][0]["status"] == "unresolved"
