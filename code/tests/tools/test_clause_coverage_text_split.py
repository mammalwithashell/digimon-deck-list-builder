"""Tests for `tools.clause_coverage.text_split` — the pure clause splitter."""

from __future__ import annotations

from tools.clause_coverage.text_split import TIMING_MARKER_NAMES, split_clauses


def test_empty_and_blank_text_yield_no_clauses():
    assert split_clauses("") == []
    assert split_clauses(None) == []
    assert split_clauses("   \n\t  ") == []


def test_compound_timing_markers_yield_one_clause_with_both_timings():
    spans = split_clauses("[When Digivolving] [When Attacking] Do a thing.")
    assert len(spans) == 1
    assert spans[0].kind == "timing"
    assert spans[0].timings == ["When Digivolving", "When Attacking"]
    assert spans[0].text == "Do a thing."


def test_non_adjacent_timing_markers_yield_separate_clauses():
    spans = split_clauses("[On Play] Draw a card. [Security] Place this card in the battle area.")
    assert len(spans) == 2
    assert spans[0].timings == ["On Play"]
    assert spans[0].text == "Draw a card."
    assert spans[1].timings == ["Security"]
    assert spans[1].text == "Place this card in the battle area."


def test_each_angle_bracket_keyword_is_its_own_clause():
    spans = split_clauses(
        "＜Progress＞ ＜Piercing＞ ＜Security A. +1＞ "
        "[When Digivolving] [When Attacking] [Once Per Turn] You may place up to 2 things."
    )
    keyword_spans = [s for s in spans if s.kind == "keyword"]
    timing_spans = [s for s in spans if s.kind == "timing"]
    assert [s.keyword for s in keyword_spans] == ["Progress", "Piercing", "Security A. +1"]
    assert len(timing_spans) == 1
    assert timing_spans[0].timings == ["When Digivolving", "When Attacking", "Once Per Turn"]
    assert timing_spans[0].text == "You may place up to 2 things."


def test_use_req_and_delay_classify_as_timing_despite_angle_brackets():
    spans = split_clauses(
        "＜Use Req. ([NSp] trait)＞ (reminder text) "
        "[Main] ＜Delay＞ (delay reminder)\n・Gain 2 memory."
    )
    assert len(spans) == 2
    assert spans[0].kind == "timing"
    assert spans[0].timings == ["Use Req."]
    assert spans[1].kind == "timing"
    assert spans[1].timings == ["Main", "Delay"]
    assert "Gain 2 memory." in spans[1].text


def test_square_bracket_non_marker_tokens_are_inline_not_boundaries():
    spans = split_clauses("[On Play] Trash a card with the [VB] trait or named [Gammamon].")
    assert len(spans) == 1
    assert spans[0].timings == ["On Play"]
    assert "[VB]" in spans[0].text
    assert "[Gammamon]" in spans[0].text


def test_plain_use_requirement_prefix_is_normalized_to_a_marker():
    spans = split_clauses("Use Requirement: VB trait\n[Main] Delete 1 Digimon.")
    assert len(spans) == 2
    assert spans[0].kind == "timing"
    assert spans[0].timings == ["Use Req."]
    assert spans[0].text == "VB trait"
    assert spans[1].timings == ["Main"]
    assert spans[1].text == "Delete 1 Digimon."


def test_no_recognized_markers_yields_single_untimed_clause():
    spans = split_clauses("Assembly -6: 3 [Gokuumon]/[Sagomon] w/different names")
    assert len(spans) == 1
    assert spans[0].kind == "untimed"
    assert spans[0].timings == []
    assert spans[0].text == "Assembly -6: 3 [Gokuumon]/[Sagomon] w/different names"


def test_start_of_your_main_phase_is_a_recognized_timing_marker():
    # Directly observed on EX12-021 -- not in the task's illustrative list,
    # documented as an extension in text_split.py / README.
    spans = split_clauses("[Start of Your Main Phase] By trashing 1 card, draw 1 and gain 1 memory.")
    assert len(spans) == 1
    assert spans[0].timings == ["Start of Your Main Phase"]


# --- Positional boundary rule: markers embedded mid-sentence stay inline ----
#
# Regression suite for the fragment class the old unconditional-splitting
# rule produced. Both directions are pinned: a marker embedded inside a
# printed sentence must NOT start a clause, and a marker that genuinely
# opens one still must.


def test_mid_sentence_timing_marker_does_not_split_the_sentence():
    # ST1-15's real security text. The old rule cut this one printed
    # sentence into "Activate this card's" + "effect." -- two fragments,
    # neither of which is a testable clause.
    spans = split_clauses("[Security] Activate this card's [Main] effect.")
    assert len(spans) == 1
    assert spans[0].kind == "timing"
    assert spans[0].timings == ["Security"]
    assert spans[0].text == "Activate this card's [Main] effect."


def test_mid_sentence_timing_marker_plural_variant_does_not_split():
    # BT8-097's real security text -- same shape, plural.
    spans = split_clauses("[Security] Activate this card's [Main] effects.")
    assert len(spans) == 1
    assert spans[0].text == "Activate this card's [Main] effects."


def test_mid_sentence_keyword_grants_stay_inline():
    # EX12-065's real [All Turns] clause. The old rule emitted a keyword
    # clause with body "and" and another whose entire body was ".".
    spans = split_clauses(
        "[All Turns] All of your [Puppet] or [TB] trait Digimon gain "
        "＜Blocker＞ and ＜Retaliation＞. "
        "[On Deletion] Return 1 of your opponent's Digimon to the bottom of the deck."
    )
    assert [s.kind for s in spans] == ["timing", "timing"]
    assert spans[0].timings == ["All Turns"]
    assert spans[0].text == (
        "All of your [Puppet] or [TB] trait Digimon gain ＜Blocker＞ and ＜Retaliation＞."
    )
    assert spans[1].timings == ["On Deletion"]
    assert all(s.text.strip(" .") for s in spans)


def test_mid_sentence_keyword_action_stays_with_its_timing_clause():
    # EX12-005's real [On Play] text -- the README's old worked example of
    # the limitation. The keyword IS this clause's action, not a clause.
    spans = split_clauses(
        "[On Play] By trashing 1 card with [Greymon] in its name from your hand, "
        "＜Draw 2＞ (Draw 2 cards from your deck.)"
    )
    assert len(spans) == 1
    assert spans[0].timings == ["On Play"]
    assert "＜Draw 2＞" in spans[0].text


def test_standalone_keyword_prefix_still_splits_into_keyword_clauses():
    # The case the original unconditional rule was written to protect:
    # EX12-018's real printed keyword-grant prefix. Each keyword sits at a
    # clause boundary (start of field / right after another marker), so
    # each is still its own clause.
    spans = split_clauses(
        "＜Progress＞ ＜Piercing＞ ＜Security A. +1＞ "
        "[When Digivolving] [When Attacking] [Once Per Turn] "
        "You may place up to 2 Digimon cards from your hand or trash."
    )
    assert [s.kind for s in spans] == ["keyword", "keyword", "keyword", "timing"]
    assert [s.keyword for s in spans[:3]] == ["Progress", "Piercing", "Security A. +1"]
    assert spans[3].timings == ["When Digivolving", "When Attacking", "Once Per Turn"]


def test_keyword_after_a_finished_sentence_still_splits():
    spans = split_clauses("[On Play] Draw 1 card. ＜Blocker＞ (This can block.)")
    assert [s.kind for s in spans] == ["timing", "keyword"]
    assert spans[1].keyword == "Blocker"


def test_zone_prefix_brace_before_a_timing_marker_is_a_boundary():
    # AD1-005 / BT14-014 shape: "{Hand}" is the zone prefix printed
    # immediately before the [Counter] clause, not prose.
    spans = split_clauses(
        "[Main] Delete 1 Digimon. {Hand} [Counter] Play this card without paying its cost."
    )
    timing = [s for s in spans if s.kind == "timing"]
    assert [s.timings for s in timing] == [["Main"], ["Counter"]]
    assert timing[1].text == "Play this card without paying its cost."


def test_quoted_granted_effect_still_starts_its_own_clause():
    # BT13-094 shape: the granting sentence opens a quote, and the granted
    # ability inside it is an independently-testable clause.
    spans = split_clauses(
        '[When Digivolving] 1 of your Digimon gains "[On Deletion] Gain 1 memory." '
        "for the turn."
    )
    assert [s.timings for s in spans] == [["When Digivolving"], ["On Deletion"]]


def test_consecutive_digivolution_cost_lines_each_start_a_clause():
    # BT22-015 shape: cost lines run together with no sentence punctuation
    # ("... : Cost 5 [DNA Digivolve] ..."), but each is its own condition.
    spans = split_clauses(
        "[Digivolve] Lv.6 w/[CS] trait: Cost 5 "
        "[DNA Digivolve] Lv.5 [Justimon] + Lv.5 [Cyberdramon]: Cost 3"
    )
    assert [s.timings for s in spans] == [["Digivolve"], ["DNA Digivolve"]]


def test_line_break_before_a_marker_is_a_boundary():
    spans = split_clauses("Reduce this card's cost by 1\n[Main] Gain 1 memory.")
    assert spans[0].kind == "untimed"
    assert spans[1].timings == ["Main"]
    assert spans[1].text == "Gain 1 memory."


def test_always_boundary_markers_track_the_cost_line_marker_set():
    """`_ALWAYS_BOUNDARY_MARKER_NAMES` and `activation_match`'s
    `COST_LINE_MARKERS` name the same family of structured cost-line
    markers. `activation_match` imports FROM `text_split`, so the constant
    cannot be shared by import -- this pins them equal instead."""
    from tools.clause_coverage.activation_match import COST_LINE_MARKERS
    from tools.clause_coverage.text_split import _ALWAYS_BOUNDARY_MARKER_NAMES

    assert _ALWAYS_BOUNDARY_MARKER_NAMES == COST_LINE_MARKERS
    assert _ALWAYS_BOUNDARY_MARKER_NAMES <= TIMING_MARKER_NAMES
