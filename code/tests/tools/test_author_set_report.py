"""Tests for the release-set dry-run report (tasks 8.3 / 9.1)."""

import json
import io

from data_paths import CARDS_JSON

from tools.author_set.report_set import build_report


def _cards():
    data = json.load(io.open(CARDS_JSON, encoding="utf-8"))
    return {c["card_id"]: c for c in data} if isinstance(data, list) else data


def test_dry_run_bt17_resolves_triages_and_clusters():
    rep = build_report("BT17", _cards(), do_pull=False)
    assert len(rep.card_ids) == 102
    assert rep.diff is None  # no pull requested
    assert len(rep.partition.slices) >= 4
    # every card accounted for across slices + orphans
    covered = sum(s.size for s in rep.partition.slices) + len(rep.partition.orphans)
    assert covered == 102
    # report renders without error and names the set
    text = rep.format()
    assert "/author-set BT17" in text
    assert "keyword gate" in text


def test_dry_run_blocks_flag_property_consistent():
    rep = build_report("BT22", _cards(), do_pull=False)
    # A full run is blocked by either a flagged keyword OR a subsystem auto-ingest.
    assert rep.blocks_full_run == (
        bool(rep.keywords.flag_for_human) or bool(rep.keywords.auto_ingest_subsystem)
    )
    # BT22's [Link] is a subsystem keyword -> the assess bucket, and it blocks.
    assert "link" in rep.keywords.auto_ingest_subsystem
    assert rep.blocks_full_run


def test_subsystem_keyword_excludes_only_actual_users_not_whole_slice():
    """BT25's [Link] is a subsystem keyword, but only 2 of the ~18 Aegiomon-slice
    cards actually use it (BT25-075 Vulcanusmon links; BT25-102 Factorial Area
    grants Link +1). The report must scope the exclusion to those cards so the
    other ~16 TS cards in the slice stay authorable. dcgo_root="" forces the
    text-fallback path so the test is deterministic without a DCGO checkout."""
    rep = build_report("BT25", _cards(), do_pull=False, dcgo_root="")
    assert "link" in rep.keywords.auto_ingest_subsystem
    affected = set(rep.subsystem_affected.get("link", []))
    # The two Aegiomon-slice cards that genuinely use Link are excluded ...
    assert {"BT25-075", "BT25-102"} <= affected, affected
    # ... but the ~16 Aegiomon-slice gods/options that merely belong to the same
    # slice are NOT LINK users — the whole-slice over-exclusion is gone. (Some are
    # excluded by orthogonal flag_for_human lexicon-miss noise like "animal"; that
    # is a separate gate limitation, so we assert against the link set directly.)
    for god in ("BT25-018", "BT25-020", "BT25-025", "BT25-028", "BT25-033",
                "BT25-039", "BT25-043", "BT25-044", "BT25-053", "BT25-059",
                "BT25-077", "BT25-084", "BT25-094", "BT25-097", "BT25-099"):
        assert god not in affected, god
    # The report names the specific Link-excluded cards and says the rest of
    # their slices are authorable (instead of a blanket whole-slice block).
    text = rep.format()
    assert "`link` excludes ONLY" in text
    assert "the rest of their slices are authorable" in text


def test_case_insensitive_prefix():
    rep = build_report("bt17", _cards(), do_pull=False)
    assert len(rep.card_ids) == 102
    assert rep.set_prefix == "BT17"
