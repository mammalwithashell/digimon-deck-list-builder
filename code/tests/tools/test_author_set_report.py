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


def test_case_insensitive_prefix():
    rep = build_report("bt17", _cards(), do_pull=False)
    assert len(rep.card_ids) == 102
    assert rep.set_prefix == "BT17"
