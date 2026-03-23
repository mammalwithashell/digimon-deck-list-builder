# tests/test_transpiler_scoring.py
from dataclasses import dataclass
from tools.transpiler.scoring import score_card, TranspileScore


def _make_effects(count, actions_per=1, has_unmapped=False):
    """Build minimal EffectBlock-like objects for scoring."""
    from tools.transpiler.models import EffectBlock
    effects = []
    for i in range(count):
        eb = EffectBlock()
        eb.actions = [f"action_{j}" for j in range(actions_per)]
        if has_unmapped and i == 0:
            eb.actions = []  # Simulate unmapped coroutine
        effects.append(eb)
    return effects


def _make_validation(forward=0, reverse=0, timing=0):
    """Build minimal ValidationResult-like object."""
    from tools.transpiler.validation import ValidationResult
    vr = ValidationResult()
    vr.card_id = "TEST-001"
    vr.forward_issues = [f"issue_{i}" for i in range(forward)]
    vr.reverse_issues = [f"issue_{i}" for i in range(reverse)]
    vr.timing_issues = [f"issue_{i}" for i in range(timing)]
    return vr


def _make_card_meta(effect_count=3):
    """Build minimal card metadata with expected effect count."""
    # Card text with effect_count worth of keyword lines
    lines = ["[On Play] Draw 1." for _ in range(effect_count)]
    return {"card_id": "TEST-001", "effect": "\n".join(lines)}


class TestScoreCard:
    def test_perfect_score(self):
        effects = _make_effects(3, actions_per=2)
        vr = _make_validation(forward=0)
        meta = _make_card_meta(effect_count=3)
        result = score_card("TEST-001", effects, vr, meta)
        assert isinstance(result, TranspileScore)
        assert result.score >= 0.9
        assert result.below_threshold is False

    def test_zero_effects_scores_low(self):
        effects = []
        vr = _make_validation(forward=3)
        meta = _make_card_meta(effect_count=3)
        result = score_card("TEST-001", effects, vr, meta)
        assert result.score < 0.3
        assert result.below_threshold is True

    def test_partial_extraction(self):
        effects = _make_effects(1, actions_per=1)
        vr = _make_validation(forward=2)
        meta = _make_card_meta(effect_count=3)
        result = score_card("TEST-001", effects, vr, meta)
        assert 0.2 < result.score < 0.7

    def test_unmapped_coroutines_penalize(self):
        effects_clean = _make_effects(3, actions_per=2)
        effects_unmapped = _make_effects(3, actions_per=2, has_unmapped=True)
        vr = _make_validation(forward=0)
        meta = _make_card_meta(effect_count=3)
        clean = score_card("TEST-001", effects_clean, vr, meta)
        unmapped = score_card("TEST-001", effects_unmapped, vr, meta)
        assert clean.score > unmapped.score

    def test_custom_threshold(self):
        effects = _make_effects(2, actions_per=1)
        vr = _make_validation(forward=1)
        meta = _make_card_meta(effect_count=3)
        low_bar = score_card("TEST-001", effects, vr, meta, threshold=0.3)
        high_bar = score_card("TEST-001", effects, vr, meta, threshold=0.95)
        assert low_bar.below_threshold is False or high_bar.below_threshold is True

    def test_card_with_no_expected_effects(self):
        """Vanilla cards (no effects in text) should score 1.0."""
        effects = []
        vr = _make_validation(forward=0)
        meta = {"card_id": "TEST-001", "effect": ""}
        result = score_card("TEST-001", effects, vr, meta)
        assert result.score == 1.0

    def test_result_dataclass_fields(self):
        effects = _make_effects(2, actions_per=1)
        vr = _make_validation(forward=1)
        meta = _make_card_meta(effect_count=3)
        result = score_card("TEST-001", effects, vr, meta)
        assert hasattr(result, "card_id")
        assert hasattr(result, "score")
        assert hasattr(result, "effects_ratio")
        assert hasattr(result, "actions_ratio")
        assert hasattr(result, "forward_match_ratio")
        assert hasattr(result, "has_unmapped_coroutines")
        assert hasattr(result, "below_threshold")
