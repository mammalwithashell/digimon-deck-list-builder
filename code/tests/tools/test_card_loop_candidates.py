"""`python -m tools.card_loop candidates` -- which meta archetype to run next.

Synthetic fixtures (a tiny deck library, YAML stubs, a verdict-store
directory, a small interaction denominator, a DCGO checkout with a few
scripts) prove the measures: tiering, score ordering, era spread, the
interaction join, and that a blocking ledger status (PARTIAL) counts against
readiness. One smoke test runs the command on the real repo data with a time
budget.
"""
from __future__ import annotations

import json
import os
import re
import time
from datetime import date
from pathlib import Path

import pytest

from tools.card_loop import candidates as cand
from tools.clause_coverage.exam_binding import clause_text_sha256

ROOT = Path(__file__).resolve().parents[3]

# --------------------------------------------------------------------------
# fixture world
# --------------------------------------------------------------------------

#: card -> printed effect text ("" = vanilla). Every card here is in cards.json.
TEXT = {
    # Alpha: fully implemented; the exam ledger is partly filled in.
    "BT90-001": "[On Play] Draw 1.",
    "BT90-002": "[On Play] Draw 1.\n[When Digivolving] Trash 1 card in your hand.",
    "BT90-003": "",
    "BT90-004": "[Your Turn] This Digimon gets +1000 DP.",
    # Bravo: one core card (BT91-001) has no YAML; its gap is coherent (BT91).
    "BT91-001": "[On Play] Draw 1.",
    "BT91-002": "",
    "BT91-003": "",
    # Charlie: 12 cards to implement scattered over 6 sets.
    **{f"{s}-0{n}": "" for s in ("BT92", "BT93", "BT94", "BT95", "BT96", "BT97") for n in (1, 2)},
    # A card DCGO has no script for (its clauses are planned `unavailable`).
    "BT90-009": "[On Play] Draw 1.",
}

ALPHA = ["BT90-001", "BT90-002", "BT90-003", "BT90-004"]
BRAVO = ["BT91-001", "BT91-002", "BT91-003", "BT90-001"]
CHARLIE = [c for c in TEXT if c[:4] in {"BT92", "BT93", "BT94", "BT95", "BT96", "BT97"}]


def _clause_row(cid: str, verdict: str, text: str, **kw) -> dict:
    return {"clause_id": cid, "card_id": cid.split("#")[0], "verdict": verdict,
            "text_sha256": clause_text_sha256(text), "recorded_at": "2026-10-01T00:00:00Z", **kw}


def _iv(iid: str, cards: list[str], verdict: str, sha: str, **kw) -> dict:
    return {"interaction_id": iid, "card_ids": cards, "source": iid.split(":")[0],
            "kind": "positive", "verdict": verdict, "text_sha256": sha,
            "recorded_at": "2026-10-02T00:00:00Z", **kw}


def _library(archetypes: dict[str, dict]) -> dict:
    """`archetypes[name] = {"lists": [[ids]], "top_cut": [bool], "dates": [str]}`."""
    out = {}
    for name, spec in archetypes.items():
        lists = spec["lists"]
        top = spec.get("top_cut", [True] * len(lists))
        dates = spec.get("dates", ["2026-09-01"] * len(lists))
        out[name] = {
            "archetype_name": name,
            "stats": {"times_played": len(lists), "top_cut_count": sum(top),
                      "meta_share": 0.0, "conversion_rate": 0.0, "avg_placement": 0.0, "sources": {}},
            "decklists": [
                {"deck_id": f"{name}-{i}", "source": "test", "decklist": json.dumps(cards),
                 "is_top_cut": t, "event_date": d}
                for i, (cards, t, d) in enumerate(zip(lists, top, dates))
            ],
        }
    return {"version": 1, "archetypes": out}


def build_world(tmp_path: Path, *, partial: bool = True, dcgo: bool = True,
                archetypes: dict | None = None) -> tuple[cand.DataPaths, Path | None]:
    tmp_path.mkdir(parents=True, exist_ok=True)
    archetypes = archetypes or {
        "Alpha": {"lists": [ALPHA * 2, ALPHA[:3] * 3]},
        "Bravo": {"lists": [BRAVO, BRAVO[:3]]},
        "Charlie": {"lists": [CHARLIE]},
    }
    lib = tmp_path / "deck_library.json"
    lib.write_text(json.dumps(_library(archetypes)), encoding="utf-8")

    cards_json = tmp_path / "cards.json"
    # BT98-001 is in cards.json with no text and has NO official entry: its
    # security zone is an image-required slot unless DCGO says otherwise.
    cards_json.write_text(json.dumps({**{c: {"card_id": c} for c in TEXT},
                                      "BT98-001": {"card_id": "BT98-001"}}), encoding="utf-8")
    overrides = tmp_path / "overrides.json"
    overrides.write_text("{}", encoding="utf-8")
    official = tmp_path / "official.json"
    official.write_text(json.dumps({"cards": {
        c: {"text_sections": [{"label": "Effect", "text": t}] if t else []} for c, t in TEXT.items()
    }}), encoding="utf-8")

    yaml_dir = tmp_path / "cards"
    for c in TEXT:
        if c == "BT91-001" or c in CHARLIE:
            continue
        d = yaml_dir / c.split("-")[0].lower()
        d.mkdir(parents=True, exist_ok=True)
        (d / f"{c}.yaml").write_text(f"id: {c}\n", encoding="utf-8")

    # Verdicts. Alpha: BT90-001 confirmed; BT90-002#0 diverged with a cited
    # dcgo_quirk (adjudicated), #1 diverged untriaged (open); BT90-004
    # confirmed. Interactions: the shared ruling qa:Q1 confirmed in both of its
    # card files; BT90-001's probe diverged untriaged; BT90-004's probe
    # confirmed against a STALE fingerprint (invalidated -> open).
    verdicts = tmp_path / "verdicts"
    verdicts.mkdir()
    q1 = _iv("qa:Q1", ["BT90-001", "BT90-002"], "confirmed", "sha-q1")
    files = {
        "BT90-001": {"version": 2, "clauses": {
            "BT90-001#effect#0": _clause_row("BT90-001#effect#0", "confirmed", "Draw 1.")},
            "interactions": {"qa:Q1": q1,
                             "probe:BT90-001#effect#0:optional_decline":
                                 _iv("probe:BT90-001#effect#0:optional_decline", ["BT90-001"],
                                     "diverged", "sha-p1")}},
        "BT90-002": {"version": 2, "clauses": {
            "BT90-002#effect#0": _clause_row("BT90-002#effect#0", "diverged", "Draw 1.",
                                             triage="dcgo_quirk", citation="general_rule.pdf 15-1-2"),
            "BT90-002#effect#1": _clause_row("BT90-002#effect#1", "diverged",
                                             "Trash 1 card in your hand.")},
            "interactions": {"qa:Q1": q1}},
        "BT90-004": {"version": 2, "clauses": {
            "BT90-004#effect#0": _clause_row("BT90-004#effect#0", "confirmed",
                                             "This Digimon gets +1000 DP.")},
            "interactions": {"probe:BT90-004#effect#0:scope:neg":
                             _iv("probe:BT90-004#effect#0:scope:neg", ["BT90-004"], "confirmed", "STALE")}},
    }
    for card, doc in files.items():
        (verdicts / f"{card}.json").write_text(json.dumps(doc), encoding="utf-8")

    denom = tmp_path / "interaction_denominator.json"
    rows = {
        "qa:Q1": {"source": "qa", "card_ids": ["BT90-001", "BT90-002"], "kind": "positive",
                  "gating": True, "text_sha256": "sha-q1"},
        "probe:BT90-001#effect#0:optional_decline": {
            "source": "probe", "card_ids": ["BT90-001"], "kind": "positive", "gating": True,
            "text_sha256": "sha-p1"},
        "probe:BT90-004#effect#0:scope:neg": {
            "source": "probe", "card_ids": ["BT90-004"], "kind": "negative", "gating": True,
            "text_sha256": "sha-p4"},
        "probe:BT90-004#effect#0:timing_gate:neg": {
            "source": "probe", "card_ids": ["BT90-004"], "kind": "negative", "gating": False,
            "text_sha256": "sha-p4"},
        "probe:BT91-001#effect#0:optional_decline": {
            "source": "probe", "card_ids": ["BT91-001"], "kind": "positive", "gating": True,
            "text_sha256": "sha-b1"},
        "probe:BT90-009#effect#0:optional_decline": {
            "source": "probe", "card_ids": ["BT90-009"], "kind": "positive", "gating": True,
            "text_sha256": "sha-x"},
    }
    cards_map: dict[str, list[str]] = {c: [] for c in TEXT}
    for iid, row in rows.items():
        for c in row["card_ids"]:
            cards_map[c].append(iid)
    denom.write_text(json.dumps({"version": 1, "cards": cards_map, "interactions": rows}),
                     encoding="utf-8")

    dsl = tmp_path / "validated_cards_dsl.json"
    dsl.write_text(json.dumps({"version": 1, "cards": {
        "BT90-003": {"status": "PARTIAL" if partial else "IMPLEMENTED"},
        "BT90-001": {"status": "AUDITED-OK"},
    }}), encoding="utf-8")
    legacy = tmp_path / "validated_cards.json"
    legacy.write_text(json.dumps({"version": 1, "cards": {
        "BT90-001": {"status": "FAIL"},  # superseded by the DSL ledger entry
    }}), encoding="utf-8")

    dcgo_root = None
    if dcgo:
        dcgo_root = tmp_path / "DCGO"
        for c in [*TEXT, "BT98-001"]:
            if c == "BT90-009":
                continue
            d = dcgo_root / "Assets" / "Scripts" / "CardEffect" / c.split("-")[0] / "Red"
            d.mkdir(parents=True, exist_ok=True)
            (d / f"{c.replace('-', '_')}.cs").write_text("class X {}\n", encoding="utf-8")

    scenarios = tmp_path / "scenarios"
    scenarios.mkdir()
    paths = cand.DataPaths(
        library=lib, yaml_dir=yaml_dir, verdicts=verdicts, scenarios=scenarios,
        denominator=denom, dsl_status=dsl, legacy_status=legacy,
        cards_json=cards_json, overrides=overrides, official=official,
    )
    return paths, dcgo_root


def _report(tmp_path, **kw):
    paths, dcgo_root = build_world(tmp_path, **{k: v for k, v in kw.items()
                                               if k in ("partial", "dcgo", "archetypes")})
    return cand.build_report(paths, dcgo_root=dcgo_root, top=kw.get("top", 60),
                             min_played=kw.get("min_played", 1), core_fraction=0.7,
                             command="test", base_sha=None)


def _row(report: dict, name: str) -> dict:
    return next(r for r in report["candidates"] if r["archetype"] == name)


# --------------------------------------------------------------------------
# pure measures
# --------------------------------------------------------------------------


@pytest.mark.parametrize("raw,expected", [
    ("1/11/2026", date(2026, 1, 11)),
    ("11/16/25", date(2025, 11, 16)),
    ("2026-07-03", date(2026, 7, 3)),
    ("", None),
    (None, None),
    ("13/40/2026", None),
    ("soon", None),
])
def test_event_dates_parse_in_every_library_format(raw, expected):
    assert cand.parse_event_date(raw) == expected


def test_meta_weight_counts_top_cuts_fully_entries_at_a_quarter_and_halves_per_half_life():
    ref = date(2026, 9, 1)
    entry = _library({"X": {
        "lists": [["A-1"], ["A-1"], ["A-1"]],
        "top_cut": [True, False, True],
        "dates": ["2026-09-01", "2026-09-01", "2026-03-05"],  # 180 days before ref
    }})["archetypes"]["X"]
    m = cand.meta_weight(entry, ref, undated_recency=0.5)
    assert m["weight"] == pytest.approx(1.0 + 0.25 + 0.5)
    assert m["times_played"] == 3 and m["top_cuts"] == 2
    undated = _library({"Y": {"lists": [["A-1"]], "dates": [""]}})["archetypes"]["Y"]
    assert cand.meta_weight(undated, ref, undated_recency=0.4)["weight"] == pytest.approx(0.4)


def test_closeness_is_one_at_zero_work_halves_at_k_and_falls_monotonically():
    assert cand.closeness(0) == 1.0
    assert cand.closeness(cand.CLOSENESS_K) == pytest.approx(0.5)
    values = [cand.closeness(u) for u in range(0, 2000, 37)]
    assert all(a > b for a, b in zip(values, values[1:]))


def test_era_spread_counts_sets_and_the_top_two_share():
    era = cand.era_spread(["BT12-001", "BT12-002", "EX1-004", "BT21-003", "BT12-009"])
    assert era["sets"] == {"BT12": 3, "BT21": 1, "EX1": 1}
    assert era["n_sets"] == 3
    assert era["top2"] == ["BT12", "BT21"]
    assert era["top2_share"] == pytest.approx(4 / 5)
    assert cand.era_spread([]) == {"sets": {}, "n_sets": 0, "top2": [], "top2_share": None}


@pytest.mark.parametrize("impl,core_impl,share,tier", [
    (0, 0, None, "A"),
    (5, 0, 0.2, "A"),
    (3, 1, 1.0, "B"),       # a core card is unimplemented: not "implemented"
    (6, 0, 0.1, "B"),       # small enough for one batch whatever the spread
    (10, 0, 0.1, "B"),
    (11, 0, 0.6, "B"),      # coherent era
    (30, 0, 0.75, "B"),
    (11, 0, 0.59, "C"),     # scattered
    (31, 0, 1.0, "C"),      # too large
])
def test_tiers(impl, core_impl, share, tier):
    assert cand.tier_of(impl, core_impl, share) == tier


@pytest.mark.parametrize("row,expected", [
    ({"verdict": "confirmed"}, True),
    ({"verdict": "unreachable", "reason": "no legal line reaches it"}, True),
    ({"verdict": "unreachable", "reason": None}, False),
    ({"verdict": "unavailable", "reason": "no DCGO script"}, True),
    ({"verdict": "diverged", "triage": "dcgo_quirk", "citation": "general_rule.pdf 15-1-2"}, True),
    ({"verdict": "diverged", "triage": "dcgo_quirk", "citation": None}, False),
    ({"verdict": "diverged", "triage": "ours_wrong", "citation": "x"}, False),
    ({"verdict": "diverged"}, False),
    ({"verdict": "unmeasured"}, False),
])
def test_adjudicated_follows_the_readiness_definition(row, expected):
    assert cand.adjudicated(row) is expected


def test_the_blocking_statuses_mirror_the_training_gate():
    """The training gate's deny-list lives in `gauntlet.py`, which imports the
    engine binding and gymnasium; read it as text rather than import it."""
    src = (ROOT / "code" / "digimon_gym" / "agents" / "gauntlet.py").read_text(encoding="utf-8")
    block = re.search(r"_NOT_READY_DSL_STATUSES = frozenset\(\{(.*?)\}\)", src, re.S)
    assert block, "gauntlet.py no longer defines _NOT_READY_DSL_STATUSES"
    assert set(re.findall(r'"([A-Z-]+)"', block.group(1))) == set(cand.NOT_READY_STATUSES)


def test_the_command_is_registered():
    from tools.card_loop.__main__ import COMMANDS

    assert COMMANDS["candidates"][:2] == ("tools.card_loop.candidates", "cli_candidates")


# --------------------------------------------------------------------------
# the join over the fixture world
# --------------------------------------------------------------------------


def test_card_facts_join_yaml_status_dcgo_clauses_and_interactions(tmp_path):
    paths, dcgo_root = build_world(tmp_path)
    facts, _ = cand.collect_card_facts(list(TEXT), paths, dcgo_root)

    f1 = facts["BT90-001"]
    assert f1.has_yaml and f1.status == "AUDITED-OK" and not f1.needs_impl
    assert f1.open_clauses == []
    assert f1.open_interactions == {"probe:BT90-001#effect#0:optional_decline"}

    f2 = facts["BT90-002"]
    assert f2.clause_counts["diverged"] == 2
    assert f2.open_clauses == ["BT90-002#effect#1"], "only the cited dcgo_quirk is adjudicated"
    assert f2.open_interactions == set(), "the shared ruling is confirmed"

    f4 = facts["BT90-004"]
    assert f4.open_interactions == {"probe:BT90-004#effect#0:scope:neg"}, \
        "a stale fingerprint invalidates the verdict; the non-gating probe is not counted"

    assert facts["BT91-001"].needs_impl and not facts["BT91-001"].has_yaml
    assert facts["BT90-003"].needs_impl, "PARTIAL blocks training even with a YAML spec"

    f9 = facts["BT90-009"]
    assert f9.dcgo_script is False
    assert f9.open_clauses == [] and f9.open_interactions == set()
    assert f9.planned_unavailable == {"clauses": 1, "interactions": 1}


def test_without_a_dcgo_checkout_nothing_is_planned_unavailable(tmp_path):
    paths, _ = build_world(tmp_path, dcgo=False)
    facts, notes = cand.collect_card_facts(list(TEXT), paths, None)
    assert facts["BT90-009"].dcgo_script is None
    assert facts["BT90-009"].open_clauses == ["BT90-009#effect#0"]
    assert notes["dcgo_available"] is False


def test_clauses_are_extracted_with_dcgo_disabled_like_the_readiness_generator(tmp_path, monkeypatch):
    """The fixture DCGO has a BT98-001 script with no security block, which
    would suppress the image-required slot if extraction consulted it."""
    paths, dcgo_root = build_world(tmp_path, archetypes={"Img": {"lists": [["BT98-001"] * 4]}})
    monkeypatch.setenv("DIGIMON_DCGO_ROOT", str(dcgo_root))
    report = cand.build_report(paths, dcgo_root=dcgo_root, top=60, min_played=1, core_fraction=0.7,
                               command="test", base_sha=None)
    img = _row(report, "Img")
    assert img["clauses"]["pool"]["total"] == 1
    assert img["clauses"]["pool"]["open_image_required"] == 1
    assert "image-required" in img["rationale"]
    assert report["notes"]["image_required_clause_ids"] == ["BT98-001#security#0"]
    assert os.environ["DIGIMON_DCGO_ROOT"] == str(dcgo_root), "the caller's env is restored"


def test_the_interaction_join_counts_a_shared_ruling_once_per_archetype(tmp_path):
    report = _report(tmp_path)
    alpha = _row(report, "Alpha")
    assert alpha["interactions"] == {"total": 3, "adjudicated": 1, "open": 2, "planned_unavailable": 0}
    bravo = _row(report, "Bravo")
    # BT90-001 (shared with Alpha) contributes its two gating ids here too.
    assert bravo["interactions"]["total"] == 3
    assert bravo["interactions"]["adjudicated"] == 1


def test_clause_counts_cover_core_and_pool(tmp_path):
    alpha = _row(_report(tmp_path), "Alpha")
    pool = alpha["clauses"]["pool"]
    assert pool["total"] == 4
    assert (pool["confirmed"], pool["diverged"], pool["unmeasured"]) == (2, 2, 0)
    assert pool["adjudicated"] == 3 and pool["open"] == 1
    # core = cards in >= ceil(2 * 0.7) = 2 of 2 lists: BT90-001..003.
    assert alpha["core"] == 3
    assert alpha["clauses"]["core"]["total"] == 3


def test_a_partial_card_counts_against_readiness(tmp_path):
    archetypes = {"Solo": {"lists": [["BT90-001", "BT90-003"] * 4]}}
    blocked = _row(_report(tmp_path / "a", archetypes=archetypes), "Solo")
    clear = _row(_report(tmp_path / "b", archetypes=archetypes, partial=False), "Solo")

    assert blocked["cards"]["blocked_status"] == ["BT90-003"]
    assert blocked["units"]["implement"] == 1
    assert blocked["units"]["lists_ready_now"] == 0
    assert clear["units"]["implement"] == 0
    assert blocked["units"]["total"] == clear["units"]["total"] + 1
    assert blocked["score"] < clear["score"]


def test_a_list_with_nothing_outstanding_is_ready_now(tmp_path):
    archetypes = {"Done": {"lists": [["BT90-003"] * 4]}}
    done = _row(_report(tmp_path, archetypes=archetypes, partial=False), "Done")
    assert done["units"] == {"implement": 0, "clauses": 0, "interactions": 0, "total": 0,
                             "first_list": 0, "lists_ready_now": 1}
    assert done["closeness"] == 1.0


def test_first_list_units_follow_the_loops_completion_order(tmp_path):
    alpha = _row(_report(tmp_path), "Alpha")
    # Outstanding: BT90-001 (1 interaction), BT90-002 (1 clause), BT90-003
    # (PARTIAL), BT90-004 (1 interaction). The second list (001..003) completes
    # first; BT90-004 is only in the first list.
    assert alpha["units"]["first_list"] == 3
    assert alpha["units"]["total"] == 4


def test_tiers_over_the_fixture(tmp_path):
    report = _report(tmp_path)
    # BT90-003 is PARTIAL and in every Alpha list: a core card needing work.
    assert _row(report, "Alpha")["tier"] == "B"
    assert _row(_report(tmp_path / "clear", partial=False), "Alpha")["tier"] == "A"
    bravo = _row(report, "Bravo")
    assert bravo["tier"] == "B"
    assert bravo["cards"]["core_needs_impl"] == 1
    charlie = _row(report, "Charlie")
    assert charlie["tier"] == "C"
    assert charlie["era"]["n_sets"] == 6 and charlie["era"]["top2_share"] == pytest.approx(4 / 12)


def test_score_orders_by_meta_weight_times_closeness(tmp_path):
    report = _report(tmp_path)
    rows = report["candidates"]
    assert [r["rank"] for r in rows] == list(range(1, len(rows) + 1))
    scores = [r["score"] for r in rows]
    assert scores == sorted(scores, reverse=True)
    for r in rows:
        assert r["score"] == pytest.approx(
            r["meta"]["weight"] * cand.closeness(r["units"]["total"]), abs=1e-4)

    # Same pool, more meta weight -> higher score; same meta, more work -> lower.
    archetypes = {
        "Popular": {"lists": [ALPHA] * 4},
        "Niche": {"lists": [ALPHA] * 2},
        "Heavy": {"lists": [ALPHA + CHARLIE] * 4},
    }
    report = _report(tmp_path / "order", archetypes=archetypes)
    # Popular and Heavy share a meta weight (4 top cuts); Heavy has 12 more
    # cards to implement. Niche has Popular's work at half the weight.
    assert [r["archetype"] for r in report["candidates"]] == ["Popular", "Heavy", "Niche"]
    assert _row(report, "Popular")["meta"]["weight"] == _row(report, "Heavy")["meta"]["weight"]
    assert _row(report, "Popular")["units"]["total"] == _row(report, "Niche")["units"]["total"]


def test_min_played_and_top_cap_the_candidate_set_by_meta_weight(tmp_path):
    archetypes = {
        "Big": {"lists": [ALPHA] * 5},
        "Mid": {"lists": [ALPHA] * 3},
        "Tiny": {"lists": [ALPHA]},
    }
    report = _report(tmp_path, archetypes=archetypes, min_played=2, top=1)
    assert [r["archetype"] for r in report["candidates"]] == ["Big"]
    assert report["library"] == {"archetypes": 3, "eligible": 2, "scored": 1}


def test_plan_commands_and_markdown(tmp_path):
    report = _report(tmp_path)
    for r in report["candidates"]:
        assert r["plan_command"] == f'python -m tools.card_loop plan --archetype "{r["archetype"]}"'
        assert r["rationale"] and "\n" not in r["rationale"]
    md = cand.render_markdown(report)
    assert md.startswith("# Card-loop candidates")
    assert "Generated by `test`" in md
    assert "## Tier A" in md and "## Tier B" in md and "## Tier C" in md
    assert 'python -m tools.card_loop plan --archetype "Alpha"' in md
    assert "\r" not in md


# --------------------------------------------------------------------------
# the real data
# --------------------------------------------------------------------------


def test_smoke_on_the_real_repo_data_within_a_time_budget(tmp_path, capsys):
    out = tmp_path / "candidates.md"
    t0 = time.monotonic()
    rc = cand.cli_candidates(["--top", "3", "--min-played", "20", "--out", str(out)])
    elapsed = time.monotonic() - t0
    assert rc == 0
    assert elapsed < 120, f"candidates --top 3 took {elapsed:.1f}s"

    doc = json.loads(out.with_suffix(".json").read_text(encoding="utf-8"))
    assert len(doc["candidates"]) == 3
    for r in doc["candidates"]:
        assert r["tier"] in ("A", "B", "C")
        assert r["meta"]["times_played"] >= 20
        assert r["plan_command"].startswith('python -m tools.card_loop plan --archetype "')
        u = r["units"]
        assert u["total"] == u["implement"] + u["clauses"] + u["interactions"]
    md = out.read_text(encoding="utf-8")
    assert "Generated by `python -m tools.card_loop candidates --top 3 --min-played 20" in md
    captured = capsys.readouterr()
    assert "binding clauses" in captured.err, "progress goes to stderr"
    assert "binding clauses" not in captured.out
