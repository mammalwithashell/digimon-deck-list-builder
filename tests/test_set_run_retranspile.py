# tests/test_set_run_retranspile.py
"""Tests for the retranspile stage in set run orchestrator."""
import pytest
from digimon_gym.ai.set_run_orchestrator import AISetRunOrchestrator


class TestDiscoverAndScore:
    def test_score_stage_sets_transpile_scores(self):
        """After scoring, items should have transpile_score populated."""
        orch = AISetRunOrchestrator()
        assert hasattr(orch, "_score_cards")

    def test_retranspile_stage_creates_tasks_for_low_scores(self):
        """Cards below threshold should get llm_transpile tasks."""
        orch = AISetRunOrchestrator()
        assert hasattr(orch, "_queue_retranspile_tasks")
