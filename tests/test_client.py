"""Tests for the multi-provider LLM client module."""

from __future__ import annotations

import json
from types import SimpleNamespace
from typing import Any
from unittest.mock import MagicMock, patch

import pytest
from pydantic import BaseModel

from digimon_gym.ai.client import (
    ANTHROPIC_MODEL_DEFAULTS,
    ANTHROPIC_PRICING,
    MODEL_DEFAULTS,
    MODEL_PRICING,
    AnthropicClient,
    LLMClient,
    OpenAIClient,
    OpenAIResponsesClient,
    StructuredRunResult,
    create_llm_client,
)


# ---------------------------------------------------------------------------
# Test schema model
# ---------------------------------------------------------------------------

class DummyOutput(BaseModel):
    score: int
    summary: str


class NestedOutput(BaseModel):
    items: list[str]
    metadata: dict[str, Any]


# ---------------------------------------------------------------------------
# Abstract interface
# ---------------------------------------------------------------------------

class TestLLMClientInterface:
    """Verify the abstract base class cannot be instantiated and defines the contract."""

    def test_cannot_instantiate_directly(self):
        with pytest.raises(TypeError):
            LLMClient()

    def test_openai_client_is_llm_client(self):
        client = OpenAIClient()
        assert isinstance(client, LLMClient)

    def test_anthropic_client_is_llm_client(self):
        client = AnthropicClient()
        assert isinstance(client, LLMClient)

    def test_interface_methods_exist(self):
        for method_name in ("run_structured", "resolve_model", "estimate_cost"):
            assert hasattr(LLMClient, method_name)


# ---------------------------------------------------------------------------
# Backward compat alias
# ---------------------------------------------------------------------------

class TestBackwardCompatAlias:
    def test_openai_responses_client_is_openai_client(self):
        assert OpenAIResponsesClient is OpenAIClient

    def test_alias_creates_correct_instance(self):
        client = OpenAIResponsesClient()
        assert isinstance(client, OpenAIClient)
        assert isinstance(client, LLMClient)


# ---------------------------------------------------------------------------
# OpenAI Client
# ---------------------------------------------------------------------------

class TestOpenAIClient:
    def test_resolve_model_default(self):
        client = OpenAIClient()
        assert client.resolve_model("review_batch") == MODEL_DEFAULTS["review_batch"]
        assert client.resolve_model("qa_analysis") == MODEL_DEFAULTS["qa_analysis"]

    def test_resolve_model_override(self):
        client = OpenAIClient()
        assert client.resolve_model("review_batch", override_model="custom-model") == "custom-model"

    def test_resolve_model_unknown_task_falls_back(self):
        client = OpenAIClient()
        result = client.resolve_model("unknown_task_type")
        assert result == MODEL_DEFAULTS["review_batch"]

    def test_estimate_cost_returns_float(self):
        client = OpenAIClient()
        cost = client.estimate_cost("qa_analysis", {"report_text": "test"})
        assert isinstance(cost, float)
        assert cost > 0

    def test_estimate_cost_review_batch_scales(self):
        client = OpenAIClient()
        one = client.estimate_cost("review_batch", {"cards": [{"card_id": "X"}]})
        three = client.estimate_cost("review_batch", {"cards": [{"card_id": "A"}, {"card_id": "B"}, {"card_id": "C"}]})
        assert three > one
        assert three == pytest.approx(one * 3, rel=0.05)

    def test_get_client_raises_without_api_key(self):
        client = OpenAIClient()
        client.api_key = ""
        with pytest.raises(RuntimeError, match="OPENAI_API_KEY"):
            client._get_client()

    def test_run_structured_openai(self):
        """Mock OpenAI responses.create and verify structured output parsing."""
        client = OpenAIClient()
        client.api_key = "test-key"

        output_json = json.dumps({"score": 42, "summary": "looks good"})
        mock_response = SimpleNamespace(
            output_text=output_json,
            usage=SimpleNamespace(input_tokens=100, output_tokens=50),
        )

        mock_openai = MagicMock()
        mock_openai.responses.create.return_value = mock_response
        client._client = mock_openai

        result = client.run_structured(
            task_type="review_batch",
            system_prompt="You are a reviewer.",
            user_prompt="Review this.",
            schema_model=DummyOutput,
        )

        assert isinstance(result, StructuredRunResult)
        assert result.output == {"score": 42, "summary": "looks good"}
        assert result.input_tokens == 100
        assert result.output_tokens == 50
        assert result.model_name == MODEL_DEFAULTS["review_batch"]

        # Verify the API was called correctly
        mock_openai.responses.create.assert_called_once()
        call_kwargs = mock_openai.responses.create.call_args
        assert call_kwargs.kwargs["model"] == MODEL_DEFAULTS["review_batch"]

    def test_run_structured_fallback_text_extraction(self):
        """Verify fallback text extraction when output_text is None."""
        client = OpenAIClient()
        client.api_key = "test-key"

        output_json = json.dumps({"score": 7, "summary": "fallback path"})
        mock_response = SimpleNamespace(
            output_text=None,
            output=[
                SimpleNamespace(
                    content=[SimpleNamespace(text=output_json)]
                )
            ],
            usage=SimpleNamespace(input_tokens=80, output_tokens=40),
        )

        mock_openai = MagicMock()
        mock_openai.responses.create.return_value = mock_response
        client._client = mock_openai

        result = client.run_structured(
            task_type="qa_analysis",
            system_prompt="sys",
            user_prompt="usr",
            schema_model=DummyOutput,
        )

        assert result.output == {"score": 7, "summary": "fallback path"}

    def test_run_structured_raises_on_no_text(self):
        """When no text is extractable, RuntimeError should be raised."""
        client = OpenAIClient()
        client.api_key = "test-key"

        mock_response = SimpleNamespace(
            output_text=None,
            output=[],
            usage=SimpleNamespace(input_tokens=0, output_tokens=0),
        )

        mock_openai = MagicMock()
        mock_openai.responses.create.return_value = mock_response
        client._client = mock_openai

        with pytest.raises(RuntimeError, match="No JSON output text"):
            client.run_structured(
                task_type="review_batch",
                system_prompt="sys",
                user_prompt="usr",
                schema_model=DummyOutput,
            )

    def test_extract_usage_dict(self):
        response = SimpleNamespace(usage={"input_tokens": 55, "output_tokens": 22})
        assert OpenAIClient._extract_usage(response) == (55, 22)

    def test_extract_usage_none(self):
        response = SimpleNamespace(usage=None)
        assert OpenAIClient._extract_usage(response) == (0, 0)

    def test_to_openai_strict_json_schema(self):
        raw = DummyOutput.model_json_schema()
        strict = OpenAIClient._to_openai_strict_json_schema(raw)
        assert set(strict.get("required", [])) == {"score", "summary"}
        assert strict.get("additionalProperties") is False

    def test_custom_max_output_tokens(self):
        """Verify max_output_tokens override is passed through."""
        client = OpenAIClient()
        client.api_key = "test-key"

        output_json = json.dumps({"score": 1, "summary": "x"})
        mock_response = SimpleNamespace(
            output_text=output_json,
            usage=SimpleNamespace(input_tokens=10, output_tokens=5),
        )

        mock_openai = MagicMock()
        mock_openai.responses.create.return_value = mock_response
        client._client = mock_openai

        client.run_structured(
            task_type="review_batch",
            system_prompt="sys",
            user_prompt="usr",
            schema_model=DummyOutput,
            max_output_tokens=9999,
        )

        call_kwargs = mock_openai.responses.create.call_args.kwargs
        assert call_kwargs["max_output_tokens"] == 9999


# ---------------------------------------------------------------------------
# Anthropic Client
# ---------------------------------------------------------------------------

class TestAnthropicClient:
    def test_resolve_model_default(self):
        client = AnthropicClient()
        assert client.resolve_model("review_batch") == ANTHROPIC_MODEL_DEFAULTS["review_batch"]
        assert client.resolve_model("qa_analysis") == ANTHROPIC_MODEL_DEFAULTS["qa_analysis"]

    def test_resolve_model_override(self):
        client = AnthropicClient()
        assert client.resolve_model("review_batch", override_model="claude-opus-4-6") == "claude-opus-4-6"

    def test_resolve_model_unknown_task_falls_back(self):
        client = AnthropicClient()
        result = client.resolve_model("unknown_task_type")
        assert result == ANTHROPIC_MODEL_DEFAULTS["review_batch"]

    def test_estimate_cost_returns_float(self):
        client = AnthropicClient()
        cost = client.estimate_cost("qa_analysis", {"report_text": "test"})
        assert isinstance(cost, float)
        assert cost > 0

    def test_estimate_cost_review_batch_scales(self):
        client = AnthropicClient()
        one = client.estimate_cost("review_batch", {"cards": [{"card_id": "X"}]})
        three = client.estimate_cost("review_batch", {"cards": [{"card_id": "A"}, {"card_id": "B"}, {"card_id": "C"}]})
        assert three > one
        assert three == pytest.approx(one * 3, rel=0.05)

    def test_anthropic_cost_higher_than_openai_mini(self):
        """Anthropic default pricing should be higher than gpt-4.1-mini."""
        oai = OpenAIClient()
        ant = AnthropicClient()
        payload = {"report_text": "test content for cost estimation"}
        oai_cost = oai.estimate_cost("qa_analysis", payload)
        ant_cost = ant.estimate_cost("qa_analysis", payload)
        # Anthropic sonnet ($3/$15) vs gpt-4.1-mini ($0.4/$1.6)
        assert ant_cost > oai_cost

    def test_get_client_raises_without_api_key(self):
        client = AnthropicClient()
        client.api_key = ""
        with pytest.raises(RuntimeError, match="ANTHROPIC_API_KEY"):
            client._get_client()

    def test_run_structured_anthropic(self):
        """Mock Anthropic messages.create and verify tool_use extraction."""
        client = AnthropicClient()
        client.api_key = "test-key"

        tool_use_block = SimpleNamespace(
            type="tool_use",
            name="DummyOutput",
            input={"score": 99, "summary": "excellent"},
        )
        mock_response = SimpleNamespace(
            content=[tool_use_block],
            usage=SimpleNamespace(input_tokens=200, output_tokens=80),
        )

        mock_anthropic = MagicMock()
        mock_anthropic.messages.create.return_value = mock_response
        client._client = mock_anthropic

        result = client.run_structured(
            task_type="review_batch",
            system_prompt="You are a reviewer.",
            user_prompt="Review this card.",
            schema_model=DummyOutput,
        )

        assert isinstance(result, StructuredRunResult)
        assert result.output == {"score": 99, "summary": "excellent"}
        assert result.input_tokens == 200
        assert result.output_tokens == 80
        assert result.model_name == ANTHROPIC_MODEL_DEFAULTS["review_batch"]

        # Verify the API was called correctly
        mock_anthropic.messages.create.assert_called_once()
        call_kwargs = mock_anthropic.messages.create.call_args.kwargs
        assert call_kwargs["model"] == ANTHROPIC_MODEL_DEFAULTS["review_batch"]
        assert call_kwargs["system"] == "You are a reviewer."
        assert call_kwargs["messages"] == [{"role": "user", "content": "Review this card."}]

        # Verify tool definition
        tools = call_kwargs["tools"]
        assert len(tools) == 1
        assert tools[0]["name"] == "DummyOutput"
        assert tools[0]["description"] == "Output structured data"
        assert "properties" in tools[0]["input_schema"]

        # Verify tool_choice forces the schema tool
        assert call_kwargs["tool_choice"] == {"type": "tool", "name": "DummyOutput"}

    def test_run_structured_raises_when_no_tool_use_block(self):
        """When no tool_use block is found, RuntimeError should be raised."""
        client = AnthropicClient()
        client.api_key = "test-key"

        text_block = SimpleNamespace(type="text", text="I could not do that.")
        mock_response = SimpleNamespace(
            content=[text_block],
            usage=SimpleNamespace(input_tokens=50, output_tokens=20),
        )

        mock_anthropic = MagicMock()
        mock_anthropic.messages.create.return_value = mock_response
        client._client = mock_anthropic

        with pytest.raises(RuntimeError, match="No tool_use content block"):
            client.run_structured(
                task_type="review_batch",
                system_prompt="sys",
                user_prompt="usr",
                schema_model=DummyOutput,
            )

    def test_run_structured_with_model_override(self):
        """Verify model_name override is passed through."""
        client = AnthropicClient()
        client.api_key = "test-key"

        tool_use_block = SimpleNamespace(
            type="tool_use",
            name="DummyOutput",
            input={"score": 1, "summary": "ok"},
        )
        mock_response = SimpleNamespace(
            content=[tool_use_block],
            usage=SimpleNamespace(input_tokens=10, output_tokens=5),
        )

        mock_anthropic = MagicMock()
        mock_anthropic.messages.create.return_value = mock_response
        client._client = mock_anthropic

        result = client.run_structured(
            task_type="review_batch",
            system_prompt="sys",
            user_prompt="usr",
            schema_model=DummyOutput,
            model_name="claude-opus-4-6",
        )

        assert result.model_name == "claude-opus-4-6"
        call_kwargs = mock_anthropic.messages.create.call_args.kwargs
        assert call_kwargs["model"] == "claude-opus-4-6"

    def test_run_structured_custom_max_tokens(self):
        """Verify max_output_tokens override is passed through."""
        client = AnthropicClient()
        client.api_key = "test-key"

        tool_use_block = SimpleNamespace(
            type="tool_use",
            name="DummyOutput",
            input={"score": 1, "summary": "x"},
        )
        mock_response = SimpleNamespace(
            content=[tool_use_block],
            usage=SimpleNamespace(input_tokens=10, output_tokens=5),
        )

        mock_anthropic = MagicMock()
        mock_anthropic.messages.create.return_value = mock_response
        client._client = mock_anthropic

        client.run_structured(
            task_type="review_batch",
            system_prompt="sys",
            user_prompt="usr",
            schema_model=DummyOutput,
            max_output_tokens=4096,
        )

        call_kwargs = mock_anthropic.messages.create.call_args.kwargs
        assert call_kwargs["max_tokens"] == 4096

    def test_extract_usage_dict(self):
        response = SimpleNamespace(usage={"input_tokens": 300, "output_tokens": 120})
        assert AnthropicClient._extract_usage(response) == (300, 120)

    def test_extract_usage_none(self):
        response = SimpleNamespace(usage=None)
        assert AnthropicClient._extract_usage(response) == (0, 0)

    def test_extract_tool_input_finds_correct_block(self):
        """When multiple content blocks exist, find the matching tool_use."""
        text_block = SimpleNamespace(type="text", text="thinking...")
        tool_block = SimpleNamespace(
            type="tool_use",
            name="DummyOutput",
            input={"score": 5, "summary": "found"},
        )
        wrong_tool = SimpleNamespace(
            type="tool_use",
            name="OtherTool",
            input={"x": 1},
        )
        response = SimpleNamespace(content=[text_block, wrong_tool, tool_block])
        result = AnthropicClient._extract_tool_input(response, "DummyOutput")
        assert result == {"score": 5, "summary": "found"}

    def test_extract_tool_input_returns_none_when_missing(self):
        response = SimpleNamespace(content=[])
        assert AnthropicClient._extract_tool_input(response, "DummyOutput") is None

    def test_extract_tool_input_returns_none_when_no_content(self):
        response = SimpleNamespace(content=None)
        assert AnthropicClient._extract_tool_input(response, "DummyOutput") is None

    def test_schema_passed_as_raw_json_schema(self):
        """Anthropic should receive the raw Pydantic JSON schema (no strict transforms)."""
        client = AnthropicClient()
        client.api_key = "test-key"

        tool_use_block = SimpleNamespace(
            type="tool_use",
            name="DummyOutput",
            input={"score": 1, "summary": "test"},
        )
        mock_response = SimpleNamespace(
            content=[tool_use_block],
            usage=SimpleNamespace(input_tokens=10, output_tokens=5),
        )

        mock_anthropic = MagicMock()
        mock_anthropic.messages.create.return_value = mock_response
        client._client = mock_anthropic

        client.run_structured(
            task_type="review_batch",
            system_prompt="sys",
            user_prompt="usr",
            schema_model=DummyOutput,
        )

        call_kwargs = mock_anthropic.messages.create.call_args.kwargs
        input_schema = call_kwargs["tools"][0]["input_schema"]
        # Raw Pydantic schema does NOT have additionalProperties=False forced
        # (unless the model itself declares it). DummyOutput is a simple BaseModel
        # so this checks the schema is the raw version.
        expected = DummyOutput.model_json_schema()
        assert input_schema == expected


# ---------------------------------------------------------------------------
# Factory function
# ---------------------------------------------------------------------------

class TestCreateLLMClient:
    def test_default_is_openai(self):
        with patch.dict("os.environ", {}, clear=False):
            # Ensure AI_PROVIDER is not set
            env = {k: v for k, v in __import__("os").environ.items() if k != "AI_PROVIDER"}
            with patch.dict("os.environ", env, clear=True):
                client = create_llm_client()
                assert isinstance(client, OpenAIClient)

    def test_explicit_openai(self):
        client = create_llm_client("openai")
        assert isinstance(client, OpenAIClient)

    def test_explicit_anthropic(self):
        client = create_llm_client("anthropic")
        assert isinstance(client, AnthropicClient)

    def test_env_var_anthropic(self):
        with patch.dict("os.environ", {"AI_PROVIDER": "anthropic"}):
            client = create_llm_client()
            assert isinstance(client, AnthropicClient)

    def test_env_var_openai(self):
        with patch.dict("os.environ", {"AI_PROVIDER": "openai"}):
            client = create_llm_client()
            assert isinstance(client, OpenAIClient)

    def test_explicit_provider_overrides_env(self):
        with patch.dict("os.environ", {"AI_PROVIDER": "anthropic"}):
            client = create_llm_client("openai")
            assert isinstance(client, OpenAIClient)

    def test_unknown_provider_defaults_to_openai(self):
        client = create_llm_client("gcp-vertex")
        assert isinstance(client, OpenAIClient)


# ---------------------------------------------------------------------------
# Dispatcher integration
# ---------------------------------------------------------------------------

class TestDispatcherUsesFactory:
    """Verify the dispatcher defaults to the factory function."""

    def test_dispatcher_init_creates_llm_client(self):
        """TaskDispatcher should use create_llm_client() when no client is provided."""
        with patch("digimon_gym.ai.dispatcher.create_llm_client") as mock_factory:
            mock_client = MagicMock(spec=LLMClient)
            mock_factory.return_value = mock_client

            from digimon_gym.ai.dispatcher import TaskDispatcher

            dispatcher = TaskDispatcher()
            mock_factory.assert_called_once()
            assert dispatcher.client is mock_client

    def test_dispatcher_accepts_custom_client(self):
        """TaskDispatcher should accept a custom client without calling the factory."""
        custom_client = MagicMock(spec=LLMClient)

        from digimon_gym.ai.dispatcher import TaskDispatcher

        dispatcher = TaskDispatcher(client=custom_client)
        assert dispatcher.client is custom_client

    def test_dispatcher_still_imports_openai_responses_client(self):
        """Backward compat: OpenAIResponsesClient should still be importable from dispatcher's imports."""
        from digimon_gym.ai.dispatcher import OpenAIResponsesClient as ImportedAlias

        assert ImportedAlias is OpenAIClient


# ---------------------------------------------------------------------------
# StructuredRunResult
# ---------------------------------------------------------------------------

class TestStructuredRunResult:
    def test_dataclass_fields(self):
        result = StructuredRunResult(
            model_name="test-model",
            output={"key": "value"},
            input_tokens=150,
            output_tokens=75,
        )
        assert result.model_name == "test-model"
        assert result.output == {"key": "value"}
        assert result.input_tokens == 150
        assert result.output_tokens == 75


# ---------------------------------------------------------------------------
# Pricing constants
# ---------------------------------------------------------------------------

class TestPricingConstants:
    def test_openai_pricing_has_expected_models(self):
        assert "gpt-4.1" in MODEL_PRICING
        assert "gpt-4.1-mini" in MODEL_PRICING

    def test_anthropic_pricing_has_expected_models(self):
        assert "claude-sonnet-4-6" in ANTHROPIC_PRICING
        assert "claude-opus-4-6" in ANTHROPIC_PRICING
        assert "claude-haiku-4-5" in ANTHROPIC_PRICING

    def test_pricing_tuples_are_positive(self):
        for model, (inp, out) in MODEL_PRICING.items():
            assert inp > 0, f"{model} input price should be positive"
            assert out > 0, f"{model} output price should be positive"

        for model, (inp, out) in ANTHROPIC_PRICING.items():
            assert inp > 0, f"{model} input price should be positive"
            assert out > 0, f"{model} output price should be positive"

    def test_anthropic_defaults_cover_all_task_types(self):
        expected_tasks = {"review_batch", "qa_analysis", "engine_audit", "script_autofix"}
        assert set(ANTHROPIC_MODEL_DEFAULTS.keys()) == expected_tasks

    def test_openai_defaults_cover_all_task_types(self):
        expected_tasks = {"review_batch", "qa_analysis", "engine_audit", "script_autofix"}
        assert set(MODEL_DEFAULTS.keys()) == expected_tasks
