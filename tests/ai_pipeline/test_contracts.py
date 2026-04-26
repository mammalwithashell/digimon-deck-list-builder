# tests/test_contracts.py
from server.ai.contracts import LLMTranspileOutput


class TestLLMTranspileOutput:
    def test_valid_output(self):
        out = LLMTranspileOutput(
            script_content="class BT24_001(CardScript): ...",
            effects_implemented=["OnPlay Draw 1"],
            effects_skipped=[],
            engine_gaps=[],
            reasoning="All effects mapped.",
        )
        assert out.script_content.startswith("class")

    def test_rejects_extra_fields(self):
        import pytest
        with pytest.raises(Exception):
            LLMTranspileOutput(
                script_content="...",
                effects_implemented=[],
                effects_skipped=[],
                engine_gaps=[],
                reasoning="ok",
                rogue_field="bad",
            )


from server.ai.contracts import TranspilerLearnOutput, TranspilerPatchSuggestion


class TestTranspilerLearnOutput:
    def test_valid_output(self):
        patch = TranspilerPatchSuggestion(
            target_file="tools/transpiler/extractors.py",
            description="Add is_my_turn guard extraction",
            before_snippet="# existing code",
            after_snippet="# new code",
            cards_affected=["BT24-001", "BT24-002"],
        )
        out = TranspilerLearnOutput(
            cluster_summary="5 cards needed is_my_turn guards",
            patches=[patch],
            estimated_cards_fixed=5,
            confidence="medium",
        )
        assert len(out.patches) == 1
        assert out.confidence == "medium"

    def test_rejects_invalid_confidence(self):
        import pytest
        with pytest.raises(Exception):
            TranspilerLearnOutput(
                cluster_summary="...",
                patches=[],
                estimated_cards_fixed=0,
                confidence="maybe",
            )
