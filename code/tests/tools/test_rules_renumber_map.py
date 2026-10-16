import difflib
from pathlib import Path

from tools.rules.parse_rules import RuleUnit, parse_rules
from tools.rules.renumber_map import FUZZY_MIN as FUZZY_MIN_FOR_TEST
from tools.rules.renumber_map import SAME_ID_REWRITE_MIN as SAME_ID_MIN_FOR_TEST
from tools.rules.renumber_map import build_map

REPO = Path(__file__).resolve().parents[3]
OLD = parse_rules(REPO / "docs/digimon-rules/cr-3.6.txt")
NEW = parse_rules(REPO / "qa/rules/aegis/comprehensive-4.3.md")
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


def test_reversal_is_reworded_not_renumbered():
    # 15-7-4 kept its number but reversed meaning (text similarity 0.53, under the
    # 0.6 fuzzy floor): pass 4 pairs it as reworded at the same id.
    rew = {(r["old"], r["new"]) for r in MAP["reworded"]}
    assert ("15-7-4", "15-7-4") in rew
    assert "15-7-4" not in MAP["renumbered"]
    assert "15-7-4" not in MAP["added"] and "15-7-4" not in MAP["removed"]


def test_same_id_rewrite_pass_four_on_synthetic_units():
    def unit(i, t):
        return RuleUnit(i, int(i.split("-")[0]), t, t, "x")

    def ratio(a, b):
        return difflib.SequenceMatcher(None, a, b).ratio()

    base = "the quick brown fox jumps over the lazy dog today"
    rewritten = "the quick red owl sleeps under a busy mouse tonight"
    unrelated = "one red dog runs by the quiet fox soon"
    # Pin the ratios so the fixtures cannot drift out of the pass-4 band.
    assert FUZZY_MIN_FOR_TEST > ratio(base, rewritten) >= SAME_ID_MIN_FOR_TEST
    assert ratio(base, unrelated) < SAME_ID_MIN_FOR_TEST

    # In [0.4, 0.6): only pass 4 can pair it, and it lands in reworded.
    m = build_map([unit("1-1", base)], [unit("1-1", rewritten)])
    assert [(r["old"], r["new"]) for r in m["reworded"]] == [("1-1", "1-1")]
    assert m["added"] == [] and m["removed"] == []

    # Below 0.4 the same id stays in both added and removed.
    m = build_map([unit("1-1", base)], [unit("1-1", unrelated)])
    assert m["reworded"] == [] and m["renumbered"] == {}
    assert m["added"] == ["1-1"] and m["removed"] == ["1-1"]


def test_added_includes_new_keywords():
    assert {"16-42", "16-43", "16-44", "16-45", "16-46", "16-47"} <= set(MAP["added"])


def test_build_map_is_deterministic_and_accounted():
    assert build_map(OLD, NEW) == MAP
    s = MAP["stats"]
    assert s["old"] == len(OLD) == 1070 and s["new"] == len(NEW) == 1119
    assert s["paired"] == len(MAP["renumbered"]) + len(MAP["reworded"]) + MAP["unchanged"]
    assert s["paired"] + len(MAP["added"]) == s["new"]
    assert s["paired"] + len(MAP["removed"]) == s["old"]
