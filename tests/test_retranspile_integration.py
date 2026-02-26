# tests/test_retranspile_integration.py
"""Integration test for the scoring + retranspile pipeline."""
import pytest
from tools.transpiler.scoring import score_card, TranspileScore
from tools.transpiler.models import EffectBlock
from tools.transpiler.validation import ValidationResult


class TestScoringIntegration:
    """Test scoring against real-ish transpiler data structures."""

    def test_score_real_effect_blocks(self):
        """Score a card with realistic EffectBlock data."""
        eb1 = EffectBlock()
        eb1.timing = "EffectTiming.OnPlay"
        eb1.actions = ["draw"]
        eb1.is_factory = False

        eb2 = EffectBlock()
        eb2.timing = "EffectTiming.WhenDigivolving"
        eb2.actions = ["gain_memory"]
        eb2.is_factory = False

        vr = ValidationResult()
        vr.card_id = "BT24-042"
        vr.forward_issues = []
        vr.reverse_issues = []
        vr.timing_issues = []

        meta = {"card_id": "BT24-042", "effect": "[On Play] Draw 1.\n[When Digivolving] Gain 1 memory."}

        result = score_card("BT24-042", [eb1, eb2], vr, meta)
        assert result.score >= 0.8
        assert result.below_threshold is False

    def test_score_with_missing_effects(self):
        """Card with 3 expected effects but only 1 extracted."""
        eb1 = EffectBlock()
        eb1.actions = ["draw"]
        eb1.is_factory = False

        vr = ValidationResult()
        vr.card_id = "BT24-042"
        vr.forward_issues = ["missing_reveal", "missing_delete"]
        vr.reverse_issues = []
        vr.timing_issues = []

        meta = {"card_id": "BT24-042", "effect": "[On Play] Draw 1.\n[When Digivolving] Reveal top 3.\n[On Deletion] Delete 1."}

        result = score_card("BT24-042", [eb1], vr, meta)
        assert result.score < 0.7
        assert result.below_threshold is True


class TestScoringToRetranspileFlow:
    """Test that low scores correctly trigger retranspile task creation."""

    def test_low_score_card_gets_retranspile_task(self):
        """Verify the data flow from scoring to task creation."""
        # Score a low-confidence card
        eb = EffectBlock()
        eb.actions = []
        eb.is_factory = False

        vr = ValidationResult()
        vr.card_id = "BT24-099"
        vr.forward_issues = ["missing_x", "missing_y", "missing_z"]
        vr.reverse_issues = []
        vr.timing_issues = []

        meta = {"card_id": "BT24-099", "effect": "Line1\nLine2\nLine3"}

        result = score_card("BT24-099", [eb], vr, meta, threshold=0.7)
        assert result.below_threshold is True
        assert result.score < 0.5

        # This score would trigger a llm_transpile task in the orchestrator
        # (actual DB integration tested in test_ai_pipeline.py)

    def test_high_score_card_skips_retranspile(self):
        """Cards above threshold should not trigger retranspilation."""
        eb1 = EffectBlock()
        eb1.actions = ["draw", "gain_memory"]
        eb1.is_factory = False

        eb2 = EffectBlock()
        eb2.actions = ["change_dp"]
        eb2.is_factory = False

        vr = ValidationResult()
        vr.card_id = "BT24-050"
        vr.forward_issues = []
        vr.reverse_issues = []
        vr.timing_issues = []

        meta = {"card_id": "BT24-050", "effect": "[On Play] Draw 1 and gain 1 memory.\n[When Digivolving] DP-1000."}

        result = score_card("BT24-050", [eb1, eb2], vr, meta, threshold=0.7)
        assert result.below_threshold is False
        assert result.score >= 0.7


class TestPatternLearnerIntegration:
    """Test pattern learner with realistic data."""

    def test_cluster_realistic_condition_guard_diffs(self):
        """Multiple cards needing the same condition guard should cluster."""
        import json
        from digimon_gym.ai.pattern_learner import cluster_autofix_diffs

        class MockAudit:
            def __init__(self, card_id, before, after):
                self.card_id = card_id
                self.applied_files_json = json.dumps([{
                    "path": f"scripts/{card_id.lower()}.py",
                    "before": before,
                    "after": after,
                }])
                self.status = "applied"

        audits = [
            MockAudit(
                f"BT24-{i:03d}",
                before=f"def check{i}(ctx):\n    return True",
                after=f"def check{i}(ctx):\n    # condition guard\n    if not card.owner.is_my_turn:\n        return False\n    return True",
            )
            for i in range(6)
        ]

        clusters = cluster_autofix_diffs(audits, min_cluster_size=3)
        assert len(clusters) >= 1
        biggest = clusters[0]
        assert biggest.count >= 3
        assert biggest.change_type == "condition_guard"
