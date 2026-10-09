"""families@1 probe generator over REAL extracted clause text (design D6).

Every family is exercised on printed cards from the committed card data, with
false-positive guards beside each positive: a probe that fires where it should
not inflates the gating denominator, so the guards matter as much as the hits.
Each test first pins the clause text it relies on, so a re-scrape that moves
the text fails with a readable precondition instead of a puzzling probe diff.
"""
from __future__ import annotations

import re
from pathlib import Path

import pytest

from tools.card_loop.interactions.probes import (
    FAMILY_NAMES_V1,
    FAMILY_VERSION,
    FAMILY_VERSIONS,
    IMMUNITY_KEYWORDS,
    NEGATIVE,
    OPTIONAL_KEYWORDS,
    POSITIVE,
    WOULD_KEYWORDS,
    Family,
    generate_probes,
    keyword_base,
    natural_key,
    parse_probe_id,
    probe_id,
    probes_for_clause,
    strip_reminders,
)

CARDS = [
    "AD1-005", "AD1-019", "AD1-023", "BT1-017", "BT1-023", "BT1-049", "BT1-082", "BT7-056",
    "BT8-084", "BT10-031", "BT11-040", "BT11-061", "BT13-077", "BT21-029", "EX12-035",
    "EX12-047", "EX12-065", "ST1-12", "ST19-14",
]


@pytest.fixture(scope="module")
def clauses() -> dict[str, dict]:
    from tools.clause_coverage.extract import run

    return {c["id"]: c for c in run(CARDS, "probe tests")["clauses"]}


def fam(clauses, cid: str) -> dict[str, str]:
    """{family: why} for one clause."""
    return {p.family: p.why for p in probes_for_clause(clauses[cid])}


def text(clauses, cid: str) -> str:
    return clauses[cid]["text"].lower()


# --- optional_decline --------------------------------------------------------


def test_optional_decline_fires_on_you_may(clauses):
    assert text(clauses, "BT21-029#effect#2").startswith("you may delete")
    assert fam(clauses, "BT21-029#effect#2")["optional_decline"] == "text:may"


def test_optional_decline_fires_on_an_optional_cost(clauses):
    # "..., by deleting 1 other Digimon ..., prevent that deletion." (§15-6)
    assert "by deleting 1 other digimon" in text(clauses, "BT11-040#inherited#0")
    assert fam(clauses, "BT11-040#inherited#0")["optional_decline"] == "text:by deleting"


def test_optional_decline_fires_on_an_opt_cost_keyword_and_an_optional_timing(clauses):
    assert clauses["EX12-035#effect#0"]["keyword"] == "Evade"
    assert fam(clauses, "EX12-035#effect#0")["optional_decline"] == "keyword:evade"
    assert "Blast Digivolve" in clauses["AD1-005#effect#2"]["timings"]
    assert fam(clauses, "AD1-005#effect#2")["optional_decline"] == "timing:Blast Digivolve"


def test_blocker_is_optional_although_the_table_calls_it_persistent(clauses):
    assert clauses["AD1-005#effect#4"]["keyword"] == "Blocker"
    assert fam(clauses, "AD1-005#effect#4") == {"optional_decline": "keyword:blocker"}


def test_no_optional_language_means_no_optional_decline_probe(clauses):
    # [On Deletion] Return 1 of your opponent's lowest level Digimon ... -- mandatory.
    t = text(clauses, "EX12-065#effect#4")
    assert "may" not in t and "up to" not in t and " by " not in t
    assert "optional_decline" not in fam(clauses, "EX12-065#effect#4")
    # [Your Turn] All of your Digimon get +1000 DP.
    assert "optional_decline" not in fam(clauses, "ST1-12#effect#0")


def test_a_mandatory_keyword_gets_no_probes_at_all(clauses):
    assert clauses["EX12-047#effect#0"]["keyword"] == "Piercing"
    assert fam(clauses, "EX12-047#effect#0") == {}


def test_you_may_inside_a_granted_keywords_reminder_is_not_the_clauses_choice(clauses):
    # "this Digimon gains <Blocker>. (When an opponent's Digimon attacks, you
    # may suspend this Digimon ...)" -- the "may" belongs to Blocker.
    t = text(clauses, "BT10-031#effect#1")
    assert "you may" in t and "gains ＜blocker＞" in t
    got = fam(clauses, "BT10-031#effect#1")
    assert "optional_decline" not in got
    assert got["granted_keyword"] == "text:gains <blocker>"


def test_up_to_n_copies_is_a_deck_rule_not_a_choice(clauses):
    assert "up to 50 copies" in text(clauses, "BT11-061#effect#0")
    assert fam(clauses, "BT11-061#effect#0") == {}


# --- scope (negative) ----------------------------------------------------------


def test_scope_this_digimon_emits_a_negative_probe(clauses):
    ps = probes_for_clause(clauses["BT10-031#effect#1"])
    scope = [p for p in ps if p.family == "scope"]
    assert [p.id for p in scope] == ["probe:BT10-031#effect#1:scope:neg"]
    assert scope[0].kind == NEGATIVE and scope[0].why == "text:this digimon"


def test_scope_fires_on_one_of_and_on_all(clauses):
    assert fam(clauses, "BT21-029#effect#2")["scope"] == "text:n of"
    assert text(clauses, "ST1-12#effect#0") == "all of your digimon get +1000 dp."
    assert fam(clauses, "ST1-12#effect#0")["scope"] == "text:all digimon"


def test_no_target_reference_means_no_scope_probe(clauses):
    assert text(clauses, "AD1-019#effect#0") == "if your opponent has a digimon, gain 1 memory."
    assert "scope" not in fam(clauses, "AD1-019#effect#0")


def test_this_digimon_inside_reminder_text_is_not_scope(clauses):
    # "1 of your Digimon gains <Security Attack +1> (This Digimon checks 1
    # additional security card) for the turn." -- the scope is "1 of your",
    # never the reminder's "This Digimon" (which the rules would try first).
    assert "(this digimon checks" in text(clauses, "BT1-017#effect#0")
    assert fam(clauses, "BT1-017#effect#0")["scope"] == "text:n of"
    # A printed keyword clause is classified by the keyword table only.
    assert "scope" not in fam(clauses, "AD1-005#effect#4")
    assert strip_reminders("1 of your digimon gains <x> (this digimon checks 1 more).") == \
        "1 of your digimon gains <x>."
    assert strip_reminders("play 1 token. (digimon/white/3000 dp/this digimon") == "play 1 token."


# --- once_per_turn_multi ------------------------------------------------------


def test_once_per_turn_fires(clauses):
    assert "Once Per Turn" in clauses["BT7-056#inherited#0"]["timings"]
    assert fam(clauses, "BT7-056#inherited#0")["once_per_turn_multi"] == "timing:Once Per Turn"


def test_no_once_per_turn_no_probe(clauses):
    assert "Once Per Turn" not in clauses["ST1-12#effect#0"]["timings"]
    assert "once_per_turn_multi" not in fam(clauses, "ST1-12#effect#0")


# --- would_replacement --------------------------------------------------------


def test_would_replacement_from_text_and_keyword(clauses):
    assert "when this digimon would be deleted" in text(clauses, "BT11-040#inherited#0")
    assert fam(clauses, "BT11-040#inherited#0")["would_replacement"] == "text:when ... would"
    assert fam(clauses, "EX12-035#effect#1")["would_replacement"] == "keyword:decode"


def test_no_would_no_replacement_probe(clauses):
    assert "would" not in text(clauses, "EX12-065#effect#4")
    assert "would_replacement" not in fam(clauses, "EX12-065#effect#4")
    assert "would_replacement" not in fam(clauses, "EX12-065#effect#1")  # Fortitude


# --- granted_keyword ----------------------------------------------------------


def test_granted_keyword_fires_on_gain_and_gains(clauses):
    assert "gain ＜blocker＞ and ＜retaliation＞" in text(clauses, "EX12-065#effect#3")
    assert fam(clauses, "EX12-065#effect#3")["granted_keyword"] == "text:gains <blocker>"
    assert fam(clauses, "BT1-017#effect#0")["granted_keyword"] == "text:gains <security attack>"


def test_a_keyword_filter_is_not_a_grant(clauses):
    # "Delete 1 of your opponent's Digimon with <Blocker>."
    assert "with ＜blocker＞" in text(clauses, "BT1-023#effect#0")
    assert "granted_keyword" not in fam(clauses, "BT1-023#effect#0")
    # ... nor is a printed keyword clause.
    assert "granted_keyword" not in fam(clauses, "AD1-005#effect#4")


# --- leave_play ---------------------------------------------------------------


def test_leave_play_from_timing_keyword_and_text(clauses):
    assert fam(clauses, "EX12-065#effect#4")["leave_play"] == "timing:On Deletion"
    assert fam(clauses, "EX12-065#effect#1") == {"leave_play": "keyword:fortitude"}
    assert fam(clauses, "EX12-035#effect#1")["leave_play"] == "keyword:decode"
    assert fam(clauses, "AD1-005#effect#7") == {"leave_play": "keyword:overflow"}
    assert "is deleted" in text(clauses, "BT1-049#inherited#0")
    assert fam(clauses, "BT1-049#inherited#0")["leave_play"] == "text:when ... deleted"


def test_a_prevented_leave_is_not_leave_play(clauses):
    # "... would leave the battle area, by adding ..., it doesn't leave."
    assert "it doesn't leave" in text(clauses, "AD1-023#inherited#0")
    got = fam(clauses, "AD1-023#inherited#0")
    assert "leave_play" not in got and got["would_replacement"] == "text:when ... would"
    # "would be deleted ..., prevent that deletion" (replacement, stays in play)
    assert "leave_play" not in fam(clauses, "BT11-040#inherited#0")
    assert "leave_play" not in fam(clauses, "EX12-035#effect#0")  # Evade


# --- immunity -----------------------------------------------------------------


def test_immunity_from_progress_and_text(clauses):
    assert clauses["BT21-029#effect#1"]["keyword"] == "Progress"
    assert fam(clauses, "BT21-029#effect#1") == {"immunity": "keyword:progress"}
    assert "isn't affected by" in text(clauses, "BT13-077#effect#1")
    assert fam(clauses, "BT13-077#effect#1")["immunity"] == "text:not affected"


def test_no_immunity_language_no_probe(clauses):
    assert "immunity" not in fam(clauses, "BT21-029#effect#2")


# --- timing_gate (negative) ----------------------------------------------------


def test_timing_gate_emits_only_a_negative_probe(clauses):
    ps = [p for p in probes_for_clause(clauses["ST1-12#effect#0"]) if p.family == "timing_gate"]
    assert [(p.id, p.kind, p.why) for p in ps] == [
        ("probe:ST1-12#effect#0:timing_gate:neg", NEGATIVE, "timing:Your Turn")
    ]
    assert fam(clauses, "BT1-082#effect#0")["timing_gate"] == "timing:Opponent's Turn"
    assert fam(clauses, "AD1-019#effect#0")["timing_gate"] == "timing:Start of Your Main Phase"


def test_all_turns_and_untimed_triggers_have_no_wrong_turn(clauses):
    assert clauses["EX12-065#effect#3"]["timings"] == ["All Turns"]
    assert "timing_gate" not in fam(clauses, "EX12-065#effect#3")
    assert "timing_gate" not in fam(clauses, "BT21-029#effect#2")


# --- conditions, determinism, ids ---------------------------------------------


def test_digivolution_conditions_are_never_probed(clauses):
    assert clauses["EX12-065#effect#0"]["timings"] == ["Digivolve"]
    assert probes_for_clause(clauses["EX12-065#effect#0"]) == []
    assert probes_for_clause(clauses["BT8-084#effect#0"]) == []  # DNA requirement


def test_generation_is_deterministic_sorted_and_unique(clauses):
    a = [p.to_dict() for p in generate_probes(clauses.values())]
    b = [p.to_dict() for p in generate_probes(reversed(list(clauses.values())))]
    assert a == b
    ids = [p["id"] for p in a]
    assert ids == sorted(ids, key=natural_key)
    assert len(ids) == len(set(ids))
    assert all(p["family_version"] == FAMILY_VERSION for p in a)


def test_every_family_fires_somewhere_in_this_sample(clauses):
    seen = {p.family for p in generate_probes(clauses.values())}
    assert seen == set(FAMILY_NAMES_V1)


def test_probe_ids_round_trip_and_mark_negatives():
    assert probe_id("BT7-056#effect#0", "scope", True) == "probe:BT7-056#effect#0:scope:neg"
    assert parse_probe_id("probe:BT7-056#effect#0:scope:neg") == ("BT7-056#effect#0", "scope", True)
    assert parse_probe_id("probe:BT7-056#effect#0:leave_play") == ("BT7-056#effect#0", "leave_play", False)
    assert parse_probe_id("probe:BT7-056:scope") is None
    assert parse_probe_id("qa:Q1601") is None


def test_family_polarity_is_fixed():
    pol = {f.name: f.polarity for f in FAMILY_VERSIONS[FAMILY_VERSION]}
    assert {n for n, p in pol.items() if p == NEGATIVE} == {"scope", "timing_gate"}
    assert all(p in (POSITIVE, NEGATIVE) for p in pol.values())


def test_a_family_name_may_not_be_reused_by_a_later_version():
    reg = dict(FAMILY_VERSIONS)
    reg["families@2"] = (Family("scope", NEGATIVE, lambda v: None),)
    with pytest.raises(ValueError, match="unique"):
        generate_probes([], reg)


def test_keyword_base_normalizes_printed_variants():
    assert keyword_base("Fragment ≪3≫") == "fragment"
    assert keyword_base("Fragment《3》") == "fragment"
    assert keyword_base("De-Digivolve 1") == "de-digivolve"
    assert keyword_base("Digi-Burst up to 4") == "digi-burst"
    assert keyword_base("Decode《[Aegiomon]》") == "decode"
    assert keyword_base("Security A. +1") == "security a"
    assert keyword_base("＜Material Save 2＞") == "material save"
    assert keyword_base("Recovery +1 ≪Deck≫") == "recovery"
    assert keyword_base(None) is None


# --- drift guard against the committed keyword table --------------------------


def _table() -> list[tuple[str, str, str]]:
    md = Path(__file__).resolve().parents[3] / "docs" / "digimon-rules" / "keyword-semantics.md"
    rows = []
    for line in md.read_text(encoding="utf-8").splitlines():
        m = re.match(r"^\|\s*`<([^>]+)>`[^|]*\|\s*([^|]+?)\s*\|\s*([^|]*?)\s*\|", line)
        if m:
            rows.append((keyword_base(m.group(1)), m.group(2), m.group(3)))
    assert len(rows) > 30, "keyword table not found or reshaped"
    return rows


def test_optional_keywords_match_the_rules_table():
    table_optional = {k for k, kind, _ in _table() if kind in ("Optional", "Opt-cost→Mand")}
    assert table_optional <= OPTIONAL_KEYWORDS
    assert OPTIONAL_KEYWORDS - table_optional == {"blocker", "ascension"}


def test_would_keywords_match_the_rules_tables_when_column():
    table_would = {k for k, _, when in _table() if "would" in when.lower()}
    assert table_would == WOULD_KEYWORDS


def test_progress_is_the_tables_immunity_keyword():
    progress = [when for k, _, when in _table() if k == "progress"]
    assert progress and IMMUNITY_KEYWORDS == {"progress"}
