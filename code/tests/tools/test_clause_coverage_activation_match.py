"""Tests for `tools.clause_coverage.activation_match` -- the pure matcher
between one DCGO `effect_activation` row's `effect_description` and a
card's extracted denominator clauses.
"""

from __future__ import annotations

from tools.clause_coverage.activation_match import (
    COST_LINE_MARKERS,
    is_blind_spot_kind,
    match_activation_row,
    normalize_text,
)


def _timing_clause(clause_id: str, timings: list[str], text: str) -> dict:
    return {"id": clause_id, "kind": "timing", "timings": timings, "keyword": None, "text": text}


def _keyword_clause(clause_id: str, keyword: str, text: str = "") -> dict:
    return {"id": clause_id, "kind": "keyword", "timings": [], "keyword": keyword, "text": text}


def _untimed_clause(clause_id: str, text: str) -> dict:
    return {"id": clause_id, "kind": "untimed", "timings": [], "keyword": None, "text": text}


# -- normalize_text -----------------------------------------------------


def test_normalize_text_collapses_whitespace_and_newlines():
    assert normalize_text("Draw  a\ncard.\r\n") == normalize_text("draw a card.")


def test_normalize_text_folds_bullets_to_a_common_form():
    a = normalize_text("effects below:\n・Draw a card.")
    b = normalize_text("effects below:\r\n• Draw a card.")
    assert a == b


def test_normalize_text_folds_curly_apostrophes():
    assert normalize_text("opponent’s Digimon") == normalize_text("opponent's Digimon")


def test_normalize_text_casefolds():
    assert normalize_text("DRAW A CARD") == normalize_text("draw a card")


def test_normalize_text_empty_input():
    assert normalize_text("") == ""
    assert normalize_text(None) == ""


# -- is_blind_spot_kind ---------------------------------------------------


def test_keyword_kind_is_blind_spot():
    assert is_blind_spot_kind(_keyword_clause("A#effect#0", "Piercing")) is True


def test_ordinary_timing_kind_is_not_blind_spot():
    assert is_blind_spot_kind(_timing_clause("A#effect#0", ["On Play"], "Draw 1 card.")) is False


def test_cost_line_markers_are_blind_spot():
    for marker in COST_LINE_MARKERS:
        clause = _timing_clause("A#effect#0", [marker], "cost text")
        assert is_blind_spot_kind(clause) is True, marker


def test_untimed_kind_is_not_blind_spot():
    assert is_blind_spot_kind(_untimed_clause("A#effect#0", "some text")) is False


# -- match_activation_row: timing clauses ---------------------------------


def test_exact_single_marker_timing_match():
    clauses = [_timing_clause("A#effect#0", ["On Play"], "Draw 1 card.")]
    outcome = match_activation_row("[On Play] Draw 1 card.", clauses)
    assert outcome.matched_clause_ids == ["A#effect#0"]
    assert outcome.unmatched_spans == []


def test_activation_marker_subset_of_compound_denominator_clause_matches():
    """DCGO can fire one timing out of a printed compound-marker group
    individually (a card printed "[On Play] [When Digivolving] <X>" logs
    two SEPARATE per-timing activations) -- the denominator's single
    compound clause must still match each one."""
    clauses = [_timing_clause("A#effect#0", ["On Play", "When Digivolving", "When Attacking"], "Do a thing.")]
    outcome = match_activation_row("[When Digivolving] Do a thing.", clauses)
    assert outcome.matched_clause_ids == ["A#effect#0"]


def test_disjoint_timing_markers_do_not_match_even_with_shared_marker():
    """Two clauses on the same card sharing ONE marker (e.g. both include
    'Once Per Turn') but otherwise disjoint marker sets and different
    bodies must not cross-match -- this is the real EX12-017/EX12-044
    disambiguation shape from vb-corpus3."""
    clauses = [
        _timing_clause("A#effect#0", ["On Play", "When Digivolving", "Once Per Turn"], "Delete 1 Digimon."),
        _timing_clause("A#effect#1", ["Counter", "Once Per Turn"], "DNA digivolve 2 of your Digimon."),
    ]
    outcome = match_activation_row("[Counter] [Once Per Turn] DNA digivolve 2 of your Digimon.", clauses)
    assert outcome.matched_clause_ids == ["A#effect#1"]


def test_body_disambiguates_clauses_sharing_every_marker():
    """Two different clauses on the same card can legitimately share the
    exact same timing marker (e.g. both are 'When Attacking' alone) --
    body text is what tells them apart."""
    clauses = [
        _timing_clause("A#effect#0", ["When Attacking"], "Opponent's Digimon gets -4000 DP for the turn."),
        _timing_clause("A#effect#1", ["When Attacking"], "This Digimon may digivolve with cost reduced by 2."),
    ]
    outcome = match_activation_row("[When Attacking] Opponent's Digimon gets -4000 DP for the turn.", clauses)
    assert outcome.matched_clause_ids == ["A#effect#0"]


def test_empty_body_compound_clause_matches_on_markers_alone():
    """A compound timing marker immediately followed by an embedded
    keyword produces an empty-text timing span on BOTH sides (denominator
    and activation) -- marker-subset is what disambiguates this shape,
    not body content."""
    clauses = [
        _timing_clause("A#effect#0", ["On Play", "When Digivolving", "Once Per Turn"], ""),
        _keyword_clause("A#effect#1", "De-Digivolve 2", "reminder text"),
    ]
    outcome = match_activation_row(
        "[When Digivolving] [Once Per Turn] <De-Digivolve 2> 1 of your opponent's Digimon.", clauses
    )
    assert set(outcome.matched_clause_ids) == {"A#effect#0", "A#effect#1"}
    assert outcome.unmatched_spans == []


def test_wording_variance_within_threshold_still_matches():
    """Small independent wording differences between DCGO's own rendering
    and the extracted printed text (observed directly in vb-corpus3, e.g.
    'set your memory to 3' vs 'set it to 3') must still match."""
    clauses = [_timing_clause("A#effect#0", ["Start of Your Turn"], "If you have 2 or less memory, set it to 3.")]
    outcome = match_activation_row(
        "[Start of Your Turn] If you have 2 or less memory, set your memory to 3.", clauses
    )
    assert outcome.matched_clause_ids == ["A#effect#0"]


def test_unrelated_body_does_not_match_despite_shared_marker():
    clauses = [_timing_clause("A#effect#0", ["On Play"], "Draw 1 card.")]
    outcome = match_activation_row("[On Play] Delete 1 of your opponent's Digimon with 6000 DP or less.", clauses)
    assert outcome.matched_clause_ids == []
    assert len(outcome.unmatched_spans) == 1
    assert outcome.unmatched_spans[0].kind == "timing"


# -- match_activation_row: keyword clauses ---------------------------------


def test_keyword_matches_on_name_ignoring_body_differences():
    """DCGO's rendered keyword activation can omit/differ on the
    parenthetical reminder text this package's extractor keeps -- keyword
    identity is the name alone."""
    clauses = [_keyword_clause("A#effect#0", "Raid", "(When this Digimon attacks, full reminder text here.)")]
    outcome = match_activation_row("<Raid> (shorter or different reminder text)", clauses)
    assert outcome.matched_clause_ids == ["A#effect#0"]


def test_keyword_name_mismatch_is_unmatched():
    clauses = [_keyword_clause("A#effect#0", "Blocker")]
    outcome = match_activation_row("<Barrier> (reminder text)", clauses)
    assert outcome.matched_clause_ids == []
    assert len(outcome.unmatched_spans) == 1
    assert outcome.unmatched_spans[0].kind == "keyword"


# -- match_activation_row: untimed clauses ---------------------------------


def test_untimed_clause_matches_on_body_similarity():
    clauses = [_untimed_clause("A#security#0", "Place this card in the battle area.")]
    outcome = match_activation_row("Place this card in the battle area.", clauses)
    assert outcome.matched_clause_ids == ["A#security#0"]


def test_blank_denominator_text_never_matches_untimed_activation():
    """An unresolved image-required security placeholder (blank text) must
    never spuriously absorb an unrelated activation."""
    clauses = [_untimed_clause("A#security#0", "")]
    outcome = match_activation_row("Place this card in the battle area.", clauses)
    assert outcome.matched_clause_ids == []
    assert len(outcome.unmatched_spans) == 1


# -- match_activation_row: no candidates at all ----------------------------


def test_no_candidates_for_card_reports_fully_unmatched():
    outcome = match_activation_row("[On Play] Draw 1 card.", [])
    assert outcome.matched_clause_ids == []
    assert len(outcome.unmatched_spans) == 1
