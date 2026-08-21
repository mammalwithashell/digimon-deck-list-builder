"""Tests for `tools.clause_coverage.text_split` — the pure clause splitter."""

from __future__ import annotations

from tools.clause_coverage.text_split import split_clauses


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
