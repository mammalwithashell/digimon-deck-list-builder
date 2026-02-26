"""Deterministic confidence scoring for transpiled card scripts."""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING, List

if TYPE_CHECKING:
    from tools.transpiler.models import EffectBlock
    from tools.transpiler.validation import ValidationResult

# Weights for scoring formula
W_EFFECTS = 0.40
W_ACTIONS = 0.30
W_FORWARD = 0.20
W_COROUTINE = 0.10

DEFAULT_THRESHOLD = 0.7


@dataclass
class TranspileScore:
    card_id: str
    score: float
    effects_ratio: float
    actions_ratio: float
    forward_match_ratio: float
    has_unmapped_coroutines: bool
    below_threshold: bool


def _count_expected_effects(card_meta: dict) -> int:
    """Estimate expected effect count from card text.

    Counts distinct effect lines/keywords in the card's effect field.
    Returns 0 for vanilla cards with no effect text.
    """
    effect_text = (card_meta.get("effect") or "").strip()
    if not effect_text:
        return 0
    # Count lines that contain timing keywords or effect text
    lines = [ln.strip() for ln in effect_text.split("\n") if ln.strip()]
    return max(len(lines), 1)


def score_card(
    card_id: str,
    effects: List["EffectBlock"],
    validation_result: "ValidationResult",
    card_meta: dict,
    threshold: float = DEFAULT_THRESHOLD,
) -> TranspileScore:
    """Score a transpiled card for completeness.

    Returns a TranspileScore with a 0.0-1.0 score.
    Cards scoring below *threshold* have below_threshold=True.
    """
    expected = _count_expected_effects(card_meta)

    # Vanilla card — no effects expected, nothing to transpile
    if expected == 0:
        return TranspileScore(
            card_id=card_id,
            score=1.0,
            effects_ratio=1.0,
            actions_ratio=1.0,
            forward_match_ratio=1.0,
            has_unmapped_coroutines=False,
            below_threshold=False,
        )

    # Effects ratio: how many timing blocks did the regex find?
    extracted = len(effects)
    effects_ratio = min(extracted / expected, 1.0)

    # Actions ratio: of found effects, how many have mapped actions?
    total_actions = sum(len(eb.actions) for eb in effects)
    detected_slots = max(extracted, 1)  # avoid division by zero
    actions_ratio = min(total_actions / detected_slots, 1.0) if extracted > 0 else 0.0

    # Forward match ratio: 1 - (forward mismatches / expected)
    forward_issues = len(validation_result.forward_issues)
    forward_match_ratio = max(1.0 - (forward_issues / expected), 0.0)

    # Unmapped coroutines: any effect with zero actions despite being non-factory?
    has_unmapped = any(
        len(eb.actions) == 0 and not eb.is_factory
        for eb in effects
    )
    coroutine_score = 0.0 if has_unmapped else 1.0

    score = (
        W_EFFECTS * effects_ratio
        + W_ACTIONS * actions_ratio
        + W_FORWARD * forward_match_ratio
        + W_COROUTINE * coroutine_score
    )
    score = round(max(0.0, min(1.0, score)), 4)

    return TranspileScore(
        card_id=card_id,
        score=score,
        effects_ratio=round(effects_ratio, 4),
        actions_ratio=round(actions_ratio, 4),
        forward_match_ratio=round(forward_match_ratio, 4),
        has_unmapped_coroutines=has_unmapped,
        below_threshold=score < threshold,
    )
