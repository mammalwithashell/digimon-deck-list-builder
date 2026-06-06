"""Tests for the ingest-diff (tasks 1.2 / 1.3 / 1.4).

Regression-guards the EX12-style drift case: a local snapshot missing most of a
set must report the missing cards as `added` so authoring never proceeds against
a stale snapshot.
"""

from tools.author_set.ingest_diff import SetDiff, diff_sets, merge_diff


def _card(cid, **over):
    base = {
        "card_id": cid,
        "card_name_eng": "Name",
        "card_kind": "Digimon",
        "level": 4,
        "dp": 4000,
        "play_cost": 5,
        "card_colors": ["Red"],
        "type_eng": "Dragon",
        "evo_costs": [],
        "effect_description_eng": "",
        "inherited_effect_description_eng": "",
        "security_effect_description_eng": "",
    }
    base.update(over)
    return base


def test_ex12_style_drift_reports_missing_as_added():
    # Local has 1 of the set; pull has 3 -> 2 must surface as `added`.
    local = {"EX12-001": _card("EX12-001")}
    pulled = [_card("EX12-001"), _card("EX12-002"), _card("EX12-003")]
    diff = diff_sets(local, pulled, "EX12")
    assert diff.added == ["EX12-002", "EX12-003"]
    assert diff.removed == []
    assert diff.unchanged == 1
    assert not diff.is_empty


def test_settled_set_no_drift_is_empty():
    cards = [_card("BT17-001"), _card("BT17-002")]
    local = {c["card_id"]: c for c in cards}
    diff = diff_sets(local, cards, "BT17")
    assert diff.is_empty
    assert diff.unchanged == 2


def test_changed_field_detected_and_volatile_ignored():
    local = {"BT17-001": _card("BT17-001", effect_description_eng="OLD", rarity="C")}
    pulled = [_card("BT17-001", effect_description_eng="NEW", rarity="SR")]
    diff = diff_sets(local, pulled, "BT17")
    # effect text drift is reported; rarity (volatile) is not.
    assert len(diff.changed) == 1
    assert diff.changed[0]["field"] == "effect_description_eng"
    assert diff.changed[0]["local"] == "OLD"
    assert diff.changed[0]["pulled"] == "NEW"


def test_prefix_isolation_bt1_vs_bt10():
    local = {"BT1-001": _card("BT1-001"), "BT10-001": _card("BT10-001")}
    pulled = [_card("BT1-001")]
    diff = diff_sets(local, pulled, "BT1")
    # BT10-001 must not be counted as removed from the BT1 set.
    assert diff.removed == []
    assert diff.unchanged == 1


def test_merge_writes_added_and_changed_only():
    local = {"EX12-001": _card("EX12-001", effect_description_eng="OLD")}
    pulled = [_card("EX12-001", effect_description_eng="NEW"), _card("EX12-002")]
    diff = diff_sets(local, pulled, "EX12")
    merged, written = merge_diff(local, pulled, diff)
    assert written == 2  # EX12-002 added + EX12-001 changed
    assert merged["EX12-001"]["effect_description_eng"] == "NEW"
    assert "EX12-002" in merged


def test_removed_cards_are_reported_not_deleted():
    local = {"BT17-001": _card("BT17-001"), "BT17-002": _card("BT17-002")}
    pulled = [_card("BT17-001")]
    diff = diff_sets(local, pulled, "BT17")
    assert diff.removed == ["BT17-002"]
    merged, _ = merge_diff(local, pulled, diff)
    assert "BT17-002" in merged  # not deleted


def test_offline_diff_summary():
    d = SetDiff(set_prefix="EX12", offline=True)
    assert d.offline
    assert "OFFLINE" in d.summary()
