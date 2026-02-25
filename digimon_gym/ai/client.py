"""OpenAI Responses API wrapper with structured JSON output support."""

from __future__ import annotations

import json
import math
import os
from copy import deepcopy
from dataclasses import dataclass
from typing import Any, Type

from pydantic import BaseModel
from digimon_gym.env import load_project_env


# Ensure .env values are available when MODEL_DEFAULTS / MODEL_PRICING are defined.
load_project_env()


MODEL_DEFAULTS = {
    "review_batch": os.getenv("OPENAI_MODEL_REVIEW", "gpt-4.1"),
    "qa_analysis": os.getenv("OPENAI_MODEL_QA", "gpt-4.1-mini"),
    "engine_audit": os.getenv("OPENAI_MODEL_ENGINE", "gpt-4.1-mini"),
    "script_autofix": os.getenv("OPENAI_MODEL_AUTOFIX", os.getenv("OPENAI_MODEL_REVIEW", "gpt-4.1")),
}

# USD per 1M tokens (rough planning defaults; override via env if needed).
MODEL_PRICING = {
    "gpt-4.1": (
        float(os.getenv("OPENAI_PRICE_GPT41_INPUT", "2.0")),
        float(os.getenv("OPENAI_PRICE_GPT41_OUTPUT", "8.0")),
    ),
    "gpt-4.1-mini": (
        float(os.getenv("OPENAI_PRICE_GPT41_MINI_INPUT", "0.4")),
        float(os.getenv("OPENAI_PRICE_GPT41_MINI_OUTPUT", "1.6")),
    ),
}

TASK_MAX_OUTPUT_TOKENS = {
    "review_batch": int(os.getenv("OPENAI_MAX_OUTPUT_TOKENS_REVIEW", "1200")),
    "qa_analysis": int(os.getenv("OPENAI_MAX_OUTPUT_TOKENS_QA", "800")),
    "engine_audit": int(os.getenv("OPENAI_MAX_OUTPUT_TOKENS_ENGINE", "800")),
    "script_autofix": int(os.getenv("OPENAI_MAX_OUTPUT_TOKENS_AUTOFIX", "6000")),
}


@dataclass
class StructuredRunResult:
    model_name: str
    output: dict[str, Any]
    input_tokens: int
    output_tokens: int


class OpenAIResponsesClient:
    def __init__(self) -> None:
        self.api_key = os.getenv("OPENAI_API_KEY", "")
        self._client = None

    def _get_client(self):
        if not self.api_key:
            raise RuntimeError("OPENAI_API_KEY is not configured")
        if self._client is None:
            from openai import OpenAI

            self._client = OpenAI(api_key=self.api_key)
        return self._client

    def resolve_model(self, task_type: str, override_model: str | None = None) -> str:
        if override_model:
            return override_model
        return MODEL_DEFAULTS.get(task_type, MODEL_DEFAULTS["review_batch"])

    def estimate_cost(self, task_type: str, payload: dict[str, Any], model_name: str | None = None) -> float:
        model = self.resolve_model(task_type, model_name)
        # review_batch dispatches one model call per card in dispatcher._run_review_batch.
        # Estimate as sum(single-card call estimates) to avoid underestimating cost.
        if task_type == "review_batch":
            cards = payload.get("cards", [])
            if isinstance(cards, list) and cards:
                total = 0.0
                for card in cards:
                    total += self._estimate_single_call_cost(model, {"cards": [card]})
                return total
        return self._estimate_single_call_cost(model, payload)

    @staticmethod
    def _estimate_single_call_cost(model: str, payload: dict[str, Any]) -> float:
        in_price, out_price = MODEL_PRICING.get(model, (1.0, 4.0))
        payload_size = len(json.dumps(payload, ensure_ascii=False))
        estimated_input_tokens = max(400, math.ceil(payload_size / 3))
        estimated_output_tokens = max(300, estimated_input_tokens // 2)
        return (
            (estimated_input_tokens / 1_000_000.0) * in_price
            + (estimated_output_tokens / 1_000_000.0) * out_price
        )

    def run_structured(
        self,
        *,
        task_type: str,
        system_prompt: str,
        user_prompt: str,
        schema_model: Type[BaseModel],
        model_name: str | None = None,
        max_output_tokens: int | None = None,
    ) -> StructuredRunResult:
        client = self._get_client()
        model = self.resolve_model(task_type, model_name)
        schema = self._to_openai_strict_json_schema(schema_model.model_json_schema())
        response_max_output_tokens = max_output_tokens or TASK_MAX_OUTPUT_TOKENS.get(task_type, 1200)

        response = client.responses.create(
            model=model,
            input=[
                {
                    "role": "system",
                    "content": [{"type": "input_text", "text": system_prompt}],
                },
                {
                    "role": "user",
                    "content": [{"type": "input_text", "text": user_prompt}],
                },
            ],
            text={
                "format": {
                    "type": "json_schema",
                    "name": schema_model.__name__,
                    "schema": schema,
                    "strict": True,
                }
            },
            max_output_tokens=response_max_output_tokens,
        )

        raw_text = getattr(response, "output_text", None)
        if not raw_text:
            raw_text = self._extract_text_fallback(response)
        if not raw_text:
            raise RuntimeError("No JSON output text returned by model")

        parsed = schema_model.model_validate_json(raw_text).model_dump()
        input_tokens, output_tokens = self._extract_usage(response)
        return StructuredRunResult(
            model_name=model,
            output=parsed,
            input_tokens=input_tokens,
            output_tokens=output_tokens,
        )

    @staticmethod
    def _extract_text_fallback(response: Any) -> str:
        output = getattr(response, "output", None)
        if not output:
            return ""
        try:
            for item in output:
                content = getattr(item, "content", None) or item.get("content", [])
                for block in content:
                    text = getattr(block, "text", None) or block.get("text")
                    if text:
                        return text
        except Exception:
            return ""
        return ""

    @staticmethod
    def _extract_usage(response: Any) -> tuple[int, int]:
        usage = getattr(response, "usage", None)
        if usage is None:
            return 0, 0
        if isinstance(usage, dict):
            return int(usage.get("input_tokens", 0)), int(usage.get("output_tokens", 0))
        return int(getattr(usage, "input_tokens", 0)), int(getattr(usage, "output_tokens", 0))

    @classmethod
    def _to_openai_strict_json_schema(cls, schema: dict[str, Any]) -> dict[str, Any]:
        strict = deepcopy(schema)

        def walk(node: Any) -> None:
            if not isinstance(node, dict):
                return

            props = node.get("properties")
            if isinstance(props, dict):
                # OpenAI strict schema expects all defined object properties to be required.
                node["required"] = list(props.keys())
                node["additionalProperties"] = False
                for child in props.values():
                    walk(child)

            items = node.get("items")
            if isinstance(items, dict):
                walk(items)
            elif isinstance(items, list):
                for item in items:
                    walk(item)

            for key in ("anyOf", "allOf", "oneOf"):
                variants = node.get(key)
                if isinstance(variants, list):
                    for variant in variants:
                        walk(variant)

            defs = node.get("$defs")
            if isinstance(defs, dict):
                for child in defs.values():
                    walk(child)

            # Defaults are unnecessary for strict response schemas and can conflict.
            node.pop("default", None)

        walk(strict)
        return strict
