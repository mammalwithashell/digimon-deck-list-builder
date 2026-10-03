"""Meta coverage report: weighting, readiness ladder, unlock order, plan gates.

Everything here runs on small synthetic inputs; the tool's real-data run is a
one-second CLI (`PYTHONPATH=code python -m tools.meta_coverage`).
"""

from collections import Counter

import pytest

from tools.meta_coverage import (
    NOT_READY_STATUSES,
    Deck,
    append_history,
    build_report,
    card_stage,
    deck_weights,
    evaluate_gate,
    evaluate_plan,
    has_printed_text,
    is_dcgo_verified,
    milestones,
    normalize_card_id,
    not_ready_card_ids,
    parse_event_date,
    render_dashboard,
    render_markdown,
    resolve_metric,
    unlock_order,
)


def _exam(confirmed=0, unmeasured=0, diverged=0, unreachable=0):
    total = confirmed + unmeasured + diverged + unreachable
    return {
        "total_clauses": total,
        "by_verdict": {"confirmed": confirmed, "diverged": diverged, "unreachable": unreachable,
                       "unavailable": 0, "unmeasured": unmeasured},
        "clause_ids": [f"X#effect#{i}" for i in range(total)],
    }


def _deck(archetype, counts, when="2026-09-10"):
    return Deck(archetype=archetype, date=when, source="test", placement=None, counts=Counter(counts))


@pytest.mark.parametrize("raw, expected", [
    ("2026-09-09", "2026-09-09"),
    ("2026-09-09T10:00:00", "2026-09-09"),
    ("9/5/2026", "2026-09-05"),
    ("26-09-05", "2026-09-05"),
    ("", None),
    (None, None),
    ("TBD", None),
])
def test_parse_event_date_formats(raw, expected):
    assert parse_event_date(raw) == expected


def test_normalize_card_id_strips_suffix_and_fixes_padding():
    known = {"ST10-04", "BT1-087", "BT26-081"}
    assert normalize_card_id("BT26-081_P1", known) == "BT26-081"
    assert normalize_card_id("st10-4", known) == "ST10-04"
    assert normalize_card_id("BT1-87", known) == "BT1-087"
    # Unknown ids pass through so they can be reported as not ingested.
    assert normalize_card_id("EX13-001", known) == "EX13-001"


def test_sample_weights_are_equal_and_sum_to_one():
    decks = [_deck("A", {"X": 1}) for _ in range(4)]
    weights, info = deck_weights(decks, None)
    assert info["mode"] == "sample"
    assert weights == [0.25] * 4


def test_digilab_weights_give_each_named_deck_its_printed_share():
    decks = [_deck("A", {"X": 1}), _deck("A", {"X": 1}), _deck("B", {"X": 1}), _deck("C", {"X": 1})]
    spec = {"archetypes": [
        {"name": "Deck A", "share_pct": 50, "library_archetypes": ["A"]},
        {"name": "Deck B", "share_pct": 10, "library_archetypes": ["B"]},
    ]}
    weights, info = deck_weights(decks, spec)
    assert info["mode"] == "digilab"
    assert sum(weights) == pytest.approx(1.0)
    assert weights[0] + weights[1] == pytest.approx(0.5)
    assert weights[2] == pytest.approx(0.1)
    # The unnamed archetype takes the unnamed 40% of the field.
    assert weights[3] == pytest.approx(0.4)


def test_named_deck_without_lists_is_reported_and_renormalised_away():
    decks = [_deck("A", {"X": 1}), _deck("C", {"X": 1})]
    spec = {"archetypes": [
        {"name": "Deck A", "share_pct": 40, "library_archetypes": ["A"]},
        {"name": "Deck D", "share_pct": 40, "library_archetypes": ["D"]},
    ]}
    weights, info = deck_weights(decks, spec)
    assert info["unrepresented"] == ["Deck D"]
    assert sum(weights) == pytest.approx(1.0)
    # A carries 40 and C the unnamed 20, renormalised over the represented 60.
    assert weights[0] == pytest.approx(40 / 60)
    assert weights[1] == pytest.approx(20 / 60)


def test_stage_ladder():
    assert card_stage(False, True, False, _exam(confirmed=2)) == "missing"
    assert card_stage(True, True, True, _exam(confirmed=2)) == "known_gap"
    assert card_stage(True, False, False, _exam(confirmed=2)) == "untested"
    assert card_stage(True, True, False, _exam(confirmed=1, unmeasured=1)) == "tested"
    assert card_stage(True, True, False, _exam(confirmed=1, diverged=1)) == "tested"
    assert card_stage(True, True, False, _exam(confirmed=1, unreachable=1)) == "verified"


def test_zero_clauses_only_verifies_a_card_that_prints_nothing():
    empty = _exam()
    assert is_dcgo_verified(empty, has_printed_text({})) is True
    # An empty denominator from a lossy source must not read as verified.
    assert is_dcgo_verified(empty, has_printed_text({"effect_description_eng": "[On Play] Draw 1."})) is False
    assert is_dcgo_verified(None) is False


def test_unlock_order_prefers_the_last_blockers_of_heavy_decks():
    weights = [0.5, 0.3, 0.2]
    missing = [{"X"}, {"Y", "Z"}, {"Z"}]
    order = unlock_order(weights, missing)
    assert [row["card_id"] for row in order] == ["X", "Z", "Y"]
    assert [round(row["cumulative_playable_pct"], 6) for row in order] == [50.0, 70.0, 100.0]
    assert milestones(order, 0.0) == [
        {"target_pct": 25, "cards": 1}, {"target_pct": 50, "cards": 1}, {"target_pct": 75, "cards": 3},
        {"target_pct": 90, "cards": 3}, {"target_pct": 100, "cards": 3},
    ]


def test_resolve_metric_and_gates():
    report = {"headline": {"decks": {"playable_pct": 17.0}},
              "by_set": [{"set": "BT26", "meta_cards_implemented": 0}]}
    assert resolve_metric(report, "headline.decks.playable_pct") == 17.0
    assert resolve_metric(report, "by_set[BT26].meta_cards_implemented") == 0
    assert resolve_metric(report, "by_set[EX13].meta_cards_implemented") is None

    up = evaluate_gate({"label": "playable", "metric": "headline.decks.playable_pct", "op": ">=", "target": 75}, report)
    assert up["met"] is False and up["progress"] == pytest.approx(17 / 75)
    down = evaluate_gate({"label": "zero", "metric": "by_set[BT26].meta_cards_implemented", "op": "<=", "target": 0}, report)
    assert down["met"] is True
    missing = evaluate_gate({"label": "absent", "metric": "by_set[EX13].meta_cards_implemented", "target": 1}, report)
    assert missing["met"] is False and missing["progress"] is None
    manual = evaluate_gate({"label": "by hand", "met": True}, report)
    assert manual["met"] is True

    plan = evaluate_plan({"milestones": [{"id": "M1", "gates": [up, manual]}]}, report)
    assert plan["milestones"][0]["gates_met"] == 1


def test_training_gate_flags_cards_by_status_not_label():
    tracker = {
        "A1": {"archetype": "three-musketeers", "status": "IMPLEMENTED"},
        "B1": {"archetype": "Toho Braves", "status": "AUDITED-OK"},
        "B2": {"archetype": "Toho Braves", "status": "PARTIAL"},
        "C1": {"archetype": "store-champs-june-2026", "status": "BLOCKED"},
        "D1": {"archetype": "judge-quiz cluster E (Q15)", "status": "AUDITED-DRIFT"},
        "E1": {"archetype": "Glowing Dawn (BEATBREAK)", "status": "audited-missing-tests"},
    }
    assert not_ready_card_ids(tracker) == {"B2", "C1", "D1", "E1"}


def test_not_ready_statuses_mirror_the_training_deck_pool():
    gauntlet = pytest.importorskip("digimon_gym.agents.gauntlet")  # needs the PyO3 engine
    assert NOT_READY_STATUSES == gauntlet._NOT_READY_DSL_STATUSES


def test_trainable_lists_are_playable_lists_without_not_ready_cards():
    cards_index = {cid: {"card_name_eng": cid, "card_kind": 0, "effect_description_eng": "text"}
                   for cid in ("AA-001", "AA-002", "AA-003")}
    decks = [
        _deck("Three Musketeers", {"AA-001": 4}),
        _deck("Three Musketeers", {"AA-001": 2, "AA-002": 2}),
        _deck("Unplayable", {"AA-003": 4}),
    ]
    weights, weighting = deck_weights(decks, None)
    report = build_report(
        decks=decks, weights=weights, weighting=weighting, cards_index=cards_index,
        implemented={"AA-001": "a.yaml", "AA-002": "b.yaml"},
        test_refs={"AA-001", "AA-002"},
        # Batch labels never name the library archetype; only card status counts.
        tracker={"AA-001": {"archetype": "store-champs-june-2026", "status": "IMPLEMENTED"},
                 "AA-002": {"archetype": "judge-quiz cluster E (Q15)", "status": "PARTIAL"}},
        exam={},
        window={"label": "TEST", "format": None, "since": "2026-09-01", "until": "2026-09-30",
                "newest_list": "2026-09-10", "sources": {"test": 3}},
    )
    decks_h = report["headline"]["decks"]
    assert decks_h["playable_lists"] == 2
    assert decks_h["trainable_lists"] == 1  # the list playing the PARTIAL card is held back
    assert decks_h["trainable_pct"] == pytest.approx(round(100 / 3, 2))
    row = next(r for r in report["archetypes"] if r["archetype"] == "Three Musketeers")
    assert row["playable_lists"] == 2 and row["trainable_lists"] == 1


@pytest.fixture
def small_report():
    cards_index = {cid: {"card_name_eng": cid, "card_kind": 0, "effect_description_eng": "text"}
                   for cid in ("AA-001", "AA-002", "AA-003", "AA-004")}
    decks = [
        _deck("Alpha", {"AA-001": 4, "AA-002": 4}),
        _deck("Beta", {"AA-001": 2, "AA-003": 2}),
    ]
    weights, weighting = deck_weights(decks, None)
    report = build_report(
        decks=decks, weights=weights, weighting=weighting, cards_index=cards_index,
        implemented={"AA-001": "a.yaml", "AA-002": "b.yaml", "AA-004": "d.yaml"},
        test_refs={"AA-001", "AA-002", "AA-004"},
        tracker={"AA-001": {"archetype": "Alpha", "status": "IMPLEMENTED"},
                 "AA-004": {"archetype": "Gamma", "status": "PARTIAL"}},
        exam={"AA-001": _exam(confirmed=2), "AA-002": _exam(unmeasured=1), "AA-003": _exam(unmeasured=3)},
        window={"label": "TEST", "format": None, "since": "2026-09-01", "until": "2026-09-30",
                "newest_list": "2026-09-10", "sources": {"test": 2}},
        tested_cards_snapshot=["AA-001"],
    )
    report["generated_at"] = "2026-10-03T00:00:00+00:00"
    report["git_head"] = "abc1234"
    return report


def test_build_report_headline(small_report):
    h = small_report["headline"]
    assert h["meta_cards"] == 3
    # Copies per deck: Alpha 8 (all implemented), Beta 4 (2 implemented), equal weight.
    assert h["implemented"]["copies_pct"] == pytest.approx(round(100 * (0.5 * 8 + 0.5 * 2) / (0.5 * 8 + 0.5 * 4), 2))
    assert h["verified"]["unique"] == 1  # only AA-001 has every clause confirmed
    assert h["decks"]["playable_pct"] == pytest.approx(50.0)
    assert h["decks"]["trainable_pct"] == pytest.approx(50.0)  # Alpha plays no PARTIAL/BLOCKED card
    assert h["clauses"]["total"] == 6 and h["clauses"]["by_verdict"]["confirmed"] == 2
    assert small_report["implement_next"][0]["card_id"] == "AA-003"
    assert small_report["implement_next"][0]["cumulative_playable_pct"] == pytest.approx(100.0)
    pool = small_report["pool"]
    assert pool["implemented"] == 3 and pool["known_gap"] == 1 and pool["allowlist_lag"] == 2


def test_renderers(small_report):
    md = render_markdown(small_report)
    assert "## Headline" in md and "Alpha" in md and "AA-003" in md
    assert "| Trainable lists |" in md
    page = render_dashboard(small_report)
    assert page.startswith("<!doctype html>")
    assert "/*__META_COVERAGE_REPORT__*/" not in page and '"meta_cards":3' in page
    assert not render_dashboard(small_report, standalone=False).startswith("<!doctype html>")


def test_append_history_skips_repeat_measurements(tmp_path):
    path = tmp_path / "history.jsonl"
    assert append_history(path, {"generated_at": "t1", "playable": 17.0}) is True
    assert append_history(path, {"generated_at": "t2", "playable": 17.0}) is False
    assert append_history(path, {"generated_at": "t3", "playable": 18.5}) is True
    assert len(path.read_text(encoding="utf-8").splitlines()) == 2
