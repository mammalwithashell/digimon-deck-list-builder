"""Tests for the flag-for-human path (task 5.1 / 5.2)."""

from tools.author_set.gap_router import (
    FlaggedKeyword,
    blocked_cards,
    cards_using_keyword,
    route_flagged_keyword,
)


def _c(cid, eff):
    return {"card_id": cid, "effect_description_eng": eff,
            "inherited_effect_description_eng": "", "security_effect_description_eng": ""}


def test_cards_using_keyword_matches_normalized_base():
    cards = {
        "S-1": _c("S-1", "gains [Unchained]"),
        "S-2": _c("S-2", "has [App Fusion -4]"),
        "S-3": _c("S-3", "has [App Fusion -6]"),
        "S-4": _c("S-4", "no keyword here"),
    }
    assert cards_using_keyword("unchained", cards) == ["S-1"]
    # both numeric params collapse to the same base
    assert cards_using_keyword("app fusion", cards) == ["S-2", "S-3"]


def test_blocked_cards_union():
    flagged = [
        FlaggedKeyword("unchained", "EX11", ["EX11-001", "EX11-002"]),
        FlaggedKeyword("petrification", "EX11", ["EX11-002", "EX11-009"]),
    ]
    assert blocked_cards(flagged) == {"EX11-001", "EX11-002", "EX11-009"}


def test_route_writes_gap_and_plan_then_idempotent(tmp_path):
    gaps = tmp_path / "RUST_ENGINE_GAPS.md"
    plans = tmp_path / "plans"
    fk = FlaggedKeyword("zephyrlock", "BT99", ["BT99-001", "BT99-050"])

    r1 = route_flagged_keyword(fk, gaps_path=str(gaps), plans_dir=str(plans))
    assert r1["gap_appended"] and r1["plan_written"]
    body = gaps.read_text(encoding="utf-8")
    assert "zephyrlock" in body
    assert "BT99-001" in body
    assert "EXCLUDED from mass-implementation" in body
    plan_text = (plans / "author-set-bt99-zephyrlock.md").read_text(encoding="utf-8")
    assert "Needed from you" in plan_text
    assert "BT99-050" in plan_text

    # second run must not duplicate the gap entry or rewrite the plan
    r2 = route_flagged_keyword(fk, gaps_path=str(gaps), plans_dir=str(plans))
    assert not r2["gap_appended"] and not r2["plan_written"]
    assert gaps.read_text(encoding="utf-8").count("zephyrlock`") == body.count("zephyrlock`")
