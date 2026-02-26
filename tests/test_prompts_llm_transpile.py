# tests/test_prompts_llm_transpile.py
from digimon_gym.ai.prompts import build_llm_transpile_messages


class TestBuildLLMTranspileMessages:
    def test_returns_system_and_user(self):
        system, user = build_llm_transpile_messages(
            card_id="BT24-042",
            card_text="[On Play] Draw 2 cards.",
            cs_source="public class BT24_042 : CEntity_Effect { ... }",
            regex_output="class BT24_042(CardScript): ...",
            regex_score=0.45,
            context_chunks=[],
            pinned_engine_chunks=None,
            few_shot_examples=[],
        )
        assert isinstance(system, str)
        assert isinstance(user, str)
        assert len(system) > 50
        assert "BT24-042" in user

    def test_includes_cs_source_in_user(self):
        system, user = build_llm_transpile_messages(
            card_id="BT24-042",
            card_text="[On Play] Draw 2.",
            cs_source="public class BT24_042 : CEntity_Effect { MARKER }",
            regex_output="class BT24_042(CardScript): ...",
            regex_score=0.45,
            context_chunks=[],
            pinned_engine_chunks=None,
            few_shot_examples=[],
        )
        assert "MARKER" in user

    def test_includes_few_shot_examples(self):
        examples = [{"card_id": "BT24-001", "script": "class BT24_001(CardScript): pass"}]
        system, user = build_llm_transpile_messages(
            card_id="BT24-042",
            card_text="[On Play] Draw 2.",
            cs_source="...",
            regex_output="...",
            regex_score=0.45,
            context_chunks=[],
            pinned_engine_chunks=None,
            few_shot_examples=examples,
        )
        assert "BT24-001" in user

    def test_includes_regex_score(self):
        _, user = build_llm_transpile_messages(
            card_id="BT24-042",
            card_text="effect",
            cs_source="...",
            regex_output="...",
            regex_score=0.45,
            context_chunks=[],
            pinned_engine_chunks=None,
            few_shot_examples=[],
        )
        assert "0.45" in user
