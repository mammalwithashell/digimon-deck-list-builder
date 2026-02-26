# tests/test_contracts.py
from digimon_gym.ai.contracts import LLMTranspileOutput


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
