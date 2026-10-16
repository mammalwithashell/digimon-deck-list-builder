from pathlib import Path

from tools.rules.renumber_map import build_map, load_units

REPO = Path(__file__).resolve().parents[3]
OLD = load_units(REPO / "docs/digimon-rules/cr-3.6.txt")
NEW = load_units(REPO / "qa/rules/aegis/comprehensive-4.3.md")
MAP = build_map(OLD, NEW)


def test_known_renumbers():
    m = MAP["renumbered"]
    assert m["16-40"] == "16-41" and m["16-6"] == "16-7" and m["16-23"] == "16-24"


def test_reworded_pairs_carry_similarity():
    # <Piercing> timing text changed (and 3.6 carries a stray page number), so
    # 16-6-4 is paired with 16-7-4 but classified reworded, not renumbered.
    rew = {(r["old"], r["new"]): r["similarity"] for r in MAP["reworded"]}
    assert 0.6 <= rew[("16-6-4", "16-7-4")] < 0.97
    assert "16-6-4" not in MAP["renumbered"]


def test_reversal_is_not_silently_renumbered():
    # 15-7-4 reverses meaning (may-execute -> cannot-execute); its text is below
    # the 0.6 fuzzy floor so it surfaces as removed + added, never renumbered.
    assert "15-7-4" in MAP["removed"] and "15-7-4" in MAP["added"]
    assert "15-7-4" not in MAP["renumbered"]


def test_added_includes_new_keywords():
    assert {"16-42", "16-43", "16-44", "16-45", "16-46", "16-47"} <= set(MAP["added"])


def test_build_map_is_deterministic_and_accounted():
    assert build_map(OLD, NEW) == MAP
    s = MAP["stats"]
    assert s["old"] == len(OLD) == 1070 and s["new"] == len(NEW) == 1119
    assert s["paired"] == len(MAP["renumbered"]) + len(MAP["reworded"]) + MAP["unchanged"]
    assert s["paired"] + len(MAP["added"]) == s["new"]
    assert s["paired"] + len(MAP["removed"]) == s["old"]
