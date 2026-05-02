# tests/test_pattern_learner.py
import json
import pytest
from server.ai.pattern_learner import cluster_autofix_diffs, DiffCluster


def _make_audit_record(card_id: str, before: str, after: str):
    """Create a mock audit record with before/after script content."""
    class MockAudit:
        def __init__(self):
            self.card_id = card_id
            self.applied_files_json = json.dumps([{
                "path": f"digimon_gym/engine/data/scripts/generated/bt24/{card_id.lower().replace('-', '_')}.py",
                "before": before,
                "after": after,
            }])
            self.status = "applied"
    return MockAudit()


class TestClusterAutofixDiffs:
    def test_empty_input(self):
        result = cluster_autofix_diffs([])
        assert result == []

    def test_single_diff_below_threshold(self):
        """A single diff doesn't form a cluster (min_size=3)."""
        diffs = [_make_audit_record(
            card_id="BT24-001",
            before="def condition0(ctx):\n    return True",
            after="def condition0(ctx):\n    if not card.owner.is_my_turn:\n        return False\n    return True",
        )]
        result = cluster_autofix_diffs(diffs, min_cluster_size=3)
        assert result == []

    def test_clusters_similar_diffs(self):
        """Three diffs with the same change type should form one cluster."""
        diffs = [
            _make_audit_record(
                card_id=f"BT24-{i:03d}",
                before=f"def condition{i}(ctx):\n    return True",
                after=f"def condition{i}(ctx):\n    if not card.owner.is_my_turn:\n        return False\n    return True",
            )
            for i in range(5)
        ]
        result = cluster_autofix_diffs(diffs, min_cluster_size=3)
        assert len(result) >= 1
        assert result[0].count >= 3

    def test_cluster_has_required_fields(self):
        diffs = [
            _make_audit_record(
                card_id=f"BT24-{i:03d}",
                before="player.draw_cards(1)",
                after="player.draw_cards(2)",
            )
            for i in range(4)
        ]
        result = cluster_autofix_diffs(diffs, min_cluster_size=3)
        if result:
            c = result[0]
            assert isinstance(c, DiffCluster)
            assert hasattr(c, "description")
            assert hasattr(c, "change_type")
            assert hasattr(c, "card_ids")
            assert hasattr(c, "representative_diffs")
            assert hasattr(c, "count")


class TestCreateLearnRun:
    def test_create_learn_run_exists(self):
        from server.ai.pattern_learner import create_learn_run
        assert callable(create_learn_run)
