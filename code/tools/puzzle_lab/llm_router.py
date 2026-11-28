"""Multi-Provider LLM Router for `tools.puzzle_lab` (Dev-Only).

Defines logical tiers `Role.SCANNER` (fast candidate triage) and `Role.AUTHOR`
(deep card-text & trigger-chain reasoning) over:
- `OpenAICompatProvider`: Open-weight endpoints (local RTX 4070 Ollama /
  `llama.cpp` at `http://localhost:11434/v1`, e.g. `qwen3:8b` / `qwen3:14b`;
  RunPod `vLLM` / `SGLang`, e.g. `Qwen/Qwen3-32B`) via `httpx`.
- `GeminiDevProvider`: Closed-weight dev endpoints (`gemini-4-hydrogen` scanner,
  `gemini-4-neon` author) via Gemini REST `v1beta` + optional `cachedContents`.

Strict Dev-Only Gate:
Instantiating or calling any closed-weight provider raises `PermissionError`
unless `os.environ.get("DIGIMON_ALLOW_CLOSED_LLM") == "1"` or explicit
`allow_closed_models=True` is passed. Never reads API keys or opens network
connections at module import time.
"""

from __future__ import annotations

import copy
import json
import os
import re
import uuid
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Any, Callable, Mapping, Optional, Sequence

import httpx

CLOSED_LLM_ENV_VAR = "DIGIMON_ALLOW_CLOSED_LLM"

DEFAULT_OLLAMA_BASE_URL = "http://localhost:11434/v1"
DEFAULT_OLLAMA_SCANNER_MODEL = "qwen3:8b"
DEFAULT_OLLAMA_AUTHOR_MODEL = "qwen3:14b"

DEFAULT_VLLM_SCANNER_MODEL = "Qwen/Qwen3-32B"
DEFAULT_VLLM_AUTHOR_MODEL = "Qwen/Qwen3-32B"

DEFAULT_GEMINI_BASE_URL = "https://generativelanguage.googleapis.com/v1beta"
DEFAULT_GEMINI_SCANNER_MODEL = "gemini-4-hydrogen"
DEFAULT_GEMINI_AUTHOR_MODEL = "gemini-4-neon"

_THINK_BLOCK_RE = re.compile(r"<think>.*?</think>", flags=re.DOTALL | re.IGNORECASE)
_JSON_FENCE_RE = re.compile(r"^```(?:json)?\s*(.*?)\s*```$", flags=re.DOTALL | re.IGNORECASE)


class Role(str, Enum):
    """Logical LLM routing roles for Puzzle Lab workflows."""

    SCANNER = "scanner"
    AUTHOR = "author"


def assert_closed_llm_allowed(allow_closed_models: bool = False) -> None:
    """Enforce the strict dev-only gate before constructing or calling closed LLMs."""
    if allow_closed_models or os.environ.get(CLOSED_LLM_ENV_VAR) == "1":
        return
    raise PermissionError(
        f"Closed-weight LLM providers are strictly dev-only and blocked by default. "
        f"Set {CLOSED_LLM_ENV_VAR}=1 or pass --allow-closed-models (allow_closed_models=True)."
    )


@dataclass(frozen=True)
class ToolSpec:
    """Provider-neutral tool declaration (JSON Schema parameters)."""

    name: str
    description: str
    parameters: dict[str, Any] = field(
        default_factory=lambda: {"type": "object", "properties": {}}
    )


@dataclass(frozen=True)
class ToolCall:
    """Single assistant tool invocation."""

    id: str
    name: str
    arguments: dict[str, Any]


@dataclass
class ChatMessage:
    """Provider-neutral conversation turn."""

    role: str  # "system" | "user" | "assistant" | "tool"
    content: str = ""
    tool_calls: list[ToolCall] = field(default_factory=list)
    tool_call_id: Optional[str] = None
    name: Optional[str] = None


@dataclass
class LLMResponse:
    """Normalized completion response from any `LLMProvider`."""

    content: str
    tool_calls: list[ToolCall] = field(default_factory=list)
    parsed_json: Optional[dict[str, Any]] = None
    model: str = ""
    usage: dict[str, Any] = field(default_factory=dict)
    raw: dict[str, Any] = field(default_factory=dict)


@dataclass
class ToolTraceEntry:
    turn: int
    tool_call_id: str
    name: str
    arguments: dict[str, Any]
    result: Any


@dataclass
class ToolLoopResult:
    final_response: LLMResponse
    messages: list[ChatMessage]
    tool_trace: list[ToolTraceEntry]
    turns_used: int


def extract_json_object(text: str) -> Optional[dict[str, Any]]:
    """Strip `<think>` reasoning tags / markdown fences and parse a JSON object."""
    if not text:
        return None
    cleaned = _THINK_BLOCK_RE.sub("", text).strip()
    fence = _JSON_FENCE_RE.match(cleaned)
    if fence:
        cleaned = fence.group(1).strip()
    try:
        parsed = json.loads(cleaned)
        if isinstance(parsed, dict):
            return parsed
    except Exception:
        pass
    # Fallback: locate outermost {...} object in case prose surrounds JSON.
    first_brace = cleaned.find("{")
    last_brace = cleaned.rfind("}")
    if first_brace != -1 and last_brace > first_brace:
        snippet = cleaned[first_brace : last_brace + 1]
        try:
            parsed = json.loads(snippet)
            if isinstance(parsed, dict):
                return parsed
        except Exception:
            return None
    return None


class LLMProvider(ABC):
    """Abstract base class for Puzzle Lab LLM backends."""

    is_closed_weight: bool = False
    model: str = ""

    @abstractmethod
    def complete(
        self,
        messages: Sequence[ChatMessage],
        *,
        tools: Optional[Sequence[ToolSpec]] = None,
        json_schema: Optional[dict[str, Any]] = None,
        schema_name: str = "response",
        temperature: float = 0.2,
        max_tokens: int = 4096,
    ) -> LLMResponse:
        raise NotImplementedError

    def close(self) -> None:
        """Release any underlying HTTP client resources."""


class OpenAICompatProvider(LLMProvider):
    """Open-weight OpenAI-compatible `/chat/completions` provider (`Ollama`, `llama.cpp`, `vLLM`, `SGLang`)."""

    is_closed_weight = False

    def __init__(
        self,
        *,
        model: str = DEFAULT_OLLAMA_SCANNER_MODEL,
        base_url: str = DEFAULT_OLLAMA_BASE_URL,
        api_key: Optional[str] = None,
        api_key_env: str = "OPENAI_COMPAT_API_KEY",
        timeout: float = 120.0,
        client: Optional[httpx.Client] = None,
        transport: Optional[httpx.BaseTransport] = None,
        extra_body: Optional[dict[str, Any]] = None,
    ) -> None:
        self.model = model
        self.base_url = base_url.rstrip("/")
        self._explicit_api_key = api_key
        self._api_key_env = api_key_env
        self._timeout = timeout
        self._client = client
        self._owns_client = client is None
        self._transport = transport
        self._extra_body = dict(extra_body or {})

    @classmethod
    def ollama(
        cls,
        *,
        model: str = DEFAULT_OLLAMA_SCANNER_MODEL,
        base_url: str = DEFAULT_OLLAMA_BASE_URL,
        timeout: float = 120.0,
        client: Optional[httpx.Client] = None,
        transport: Optional[httpx.BaseTransport] = None,
    ) -> "OpenAICompatProvider":
        return cls(
            model=model,
            base_url=base_url,
            api_key="ollama",
            timeout=timeout,
            client=client,
            transport=transport,
        )

    @classmethod
    def vllm(
        cls,
        *,
        base_url: str,
        model: str = DEFAULT_VLLM_AUTHOR_MODEL,
        api_key: Optional[str] = None,
        timeout: float = 180.0,
        client: Optional[httpx.Client] = None,
        transport: Optional[httpx.BaseTransport] = None,
    ) -> "OpenAICompatProvider":
        return cls(
            model=model,
            base_url=base_url,
            api_key=api_key,
            api_key_env="RUNPOD_API_KEY",
            timeout=timeout,
            client=client,
            transport=transport,
        )

    def _get_client(self) -> httpx.Client:
        if self._client is None:
            self._client = httpx.Client(timeout=self._timeout, transport=self._transport)
        return self._client

    def _resolve_api_key(self) -> str:
        if self._explicit_api_key is not None:
            return self._explicit_api_key
        return (
            os.environ.get(self._api_key_env)
            or os.environ.get("VLLM_API_KEY")
            or os.environ.get("OPENAI_API_KEY")
            or "EMPTY"
        )

    def close(self) -> None:
        if self._owns_client and self._client is not None:
            self._client.close()
            self._client = None

    @staticmethod
    def _serialize_messages(messages: Sequence[ChatMessage]) -> list[dict[str, Any]]:
        wire: list[dict[str, Any]] = []
        for m in messages:
            entry: dict[str, Any] = {"role": m.role, "content": m.content or ""}
            if m.role == "assistant" and m.tool_calls:
                entry["tool_calls"] = [
                    {
                        "id": tc.id,
                        "type": "function",
                        "function": {
                            "name": tc.name,
                            "arguments": json.dumps(tc.arguments),
                        },
                    }
                    for tc in m.tool_calls
                ]
            if m.role == "tool":
                entry["tool_call_id"] = m.tool_call_id or "call_0"
                if m.name:
                    entry["name"] = m.name
            wire.append(entry)
        return wire

    def complete(
        self,
        messages: Sequence[ChatMessage],
        *,
        tools: Optional[Sequence[ToolSpec]] = None,
        json_schema: Optional[dict[str, Any]] = None,
        schema_name: str = "response",
        temperature: float = 0.2,
        max_tokens: int = 4096,
    ) -> LLMResponse:
        client = self._get_client()
        url = f"{self.base_url}/chat/completions"
        headers = {
            "Content-Type": "application/json",
            "Authorization": f"Bearer {self._resolve_api_key()}",
        }

        payload: dict[str, Any] = {
            "model": self.model,
            "messages": self._serialize_messages(messages),
            "temperature": temperature,
            "max_tokens": max_tokens,
            **self._extra_body,
        }

        if tools:
            payload["tools"] = [
                {
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    },
                }
                for t in tools
            ]
            payload["tool_choice"] = "auto"

        if json_schema is not None and not tools:
            payload["response_format"] = {
                "type": "json_schema",
                "json_schema": {
                    "name": schema_name,
                    "strict": True,
                    "schema": json_schema,
                },
            }

        resp = client.post(url, headers=headers, json=payload)

        # Automatic fallback to `{"type": "json_object"}` for Ollama / llama.cpp builds
        # that reject strict `json_schema` response_format with 400 / 422.
        if resp.status_code in (400, 422) and json_schema is not None and not tools:
            fallback_payload = copy.deepcopy(payload)
            fallback_payload["response_format"] = {"type": "json_object"}
            schema_hint = (
                f"\n\nRespond ONLY with a valid JSON object conforming to schema:\n"
                f"{json.dumps(json_schema)}"
            )
            if fallback_payload["messages"]:
                fallback_payload["messages"][-1]["content"] = (
                    str(fallback_payload["messages"][-1].get("content") or "") + schema_hint
                )
            resp = client.post(url, headers=headers, json=fallback_payload)

        resp.raise_for_status()
        data = resp.json()

        choice = (data.get("choices") or [{}])[0]
        msg = choice.get("message") or {}
        content = str(msg.get("content") or "")

        tool_calls: list[ToolCall] = []
        for idx, raw_tc in enumerate(msg.get("tool_calls") or []):
            fn = raw_tc.get("function") or {}
            raw_args = fn.get("arguments", "{}")
            if isinstance(raw_args, Mapping):
                args_dict = dict(raw_args)
            elif isinstance(raw_args, str):
                try:
                    loaded = json.loads(raw_args) if raw_args.strip() else {}
                    args_dict = loaded if isinstance(loaded, dict) else {"value": loaded}
                except Exception:
                    args_dict = extract_json_object(raw_args) or {}
            else:
                args_dict = {}
            tool_calls.append(
                ToolCall(
                    id=str(raw_tc.get("id") or f"call_{idx}_{uuid.uuid4().hex[:8]}"),
                    name=str(fn.get("name") or ""),
                    arguments=args_dict,
                )
            )

        parsed_json = extract_json_object(content) if (json_schema is not None or not tool_calls) else None
        return LLMResponse(
            content=content,
            tool_calls=tool_calls,
            parsed_json=parsed_json,
            model=str(data.get("model") or self.model),
            usage=dict(data.get("usage") or {}),
            raw=data,
        )


def _clean_schema_for_gemini(schema: Mapping[str, Any]) -> dict[str, Any]:
    """Strip JSON Schema keywords (`additionalProperties`, `$schema`) rejected by some Gemini tool declarations."""
    cleaned: dict[str, Any] = {}
    for k, v in schema.items():
        if k in ("$schema", "additionalProperties"):
            continue
        if isinstance(v, Mapping):
            cleaned[k] = _clean_schema_for_gemini(v)
        elif isinstance(v, list):
            cleaned[k] = [_clean_schema_for_gemini(i) if isinstance(i, Mapping) else i for i in v]
        else:
            cleaned[k] = v
    return cleaned


class GeminiDevProvider(LLMProvider):
    """Closed-weight Gemini REST `v1beta` provider (`gemini-4-hydrogen` / `gemini-4-neon`).

    Strictly gated behind `DIGIMON_ALLOW_CLOSED_LLM=1` or `allow_closed_models=True`.
    Supports `cachedContents` to pin `docs/RULES_CONTEXT.md` + card pool text once.
    """

    is_closed_weight = True

    def __init__(
        self,
        *,
        model: str = DEFAULT_GEMINI_AUTHOR_MODEL,
        allow_closed_models: bool = False,
        api_key: Optional[str] = None,
        base_url: str = DEFAULT_GEMINI_BASE_URL,
        cached_content_name: Optional[str] = None,
        timeout: float = 120.0,
        client: Optional[httpx.Client] = None,
        transport: Optional[httpx.BaseTransport] = None,
    ) -> None:
        assert_closed_llm_allowed(allow_closed_models)
        self._allow_closed_models = bool(allow_closed_models)
        self.model = model.removeprefix("models/")
        self.base_url = base_url.rstrip("/")
        self._explicit_api_key = api_key
        self.cached_content_name = cached_content_name
        self._timeout = timeout
        self._client = client
        self._owns_client = client is None
        self._transport = transport

    def _get_client(self) -> httpx.Client:
        if self._client is None:
            self._client = httpx.Client(timeout=self._timeout, transport=self._transport)
        return self._client

    def _resolve_api_key(self) -> str:
        key = (
            self._explicit_api_key
            or os.environ.get("GEMINI_API_KEY")
            or os.environ.get("GOOGLE_API_KEY")
        )
        if not key:
            raise RuntimeError(
                "GeminiDevProvider requires GEMINI_API_KEY or GOOGLE_API_KEY (or explicit api_key=...)."
            )
        return key

    def close(self) -> None:
        if self._owns_client and self._client is not None:
            self._client.close()
            self._client = None

    def create_context_cache(
        self,
        *,
        system_instruction: str,
        context_blocks: Sequence[str],
        display_name: str = "digimon-puzzle-lab-context",
        ttl_seconds: int = 3600,
        tools: Optional[Sequence[ToolSpec]] = None,
    ) -> str:
        """Create a Gemini `cachedContents` resource pinning rules & card texts once."""
        assert_closed_llm_allowed(self._allow_closed_models)
        client = self._get_client()
        headers = {
            "Content-Type": "application/json",
            "x-goog-api-key": self._resolve_api_key(),
        }
        payload: dict[str, Any] = {
            "model": f"models/{self.model}",
            "displayName": display_name,
            "contents": [
                {
                    "role": "user",
                    "parts": [{"text": str(block)} for block in context_blocks if str(block)],
                }
            ],
            "ttl": f"{int(ttl_seconds)}s",
        }
        if system_instruction:
            payload["systemInstruction"] = {"parts": [{"text": system_instruction}]}
        if tools:
            payload["tools"] = [
                {
                    "functionDeclarations": [
                        {
                            "name": t.name,
                            "description": t.description,
                            "parameters": _clean_schema_for_gemini(t.parameters),
                        }
                        for t in tools
                    ]
                }
            ]

        resp = client.post(f"{self.base_url}/cachedContents", headers=headers, json=payload)
        resp.raise_for_status()
        data = resp.json()
        cache_name = str(data.get("name") or "")
        if not cache_name:
            raise RuntimeError(f"Gemini cachedContents response missing 'name': {data!r}")
        self.cached_content_name = cache_name
        return cache_name

    def pin_rules_and_card_pool(
        self,
        *,
        rules_path: Optional[Path] = None,
        card_pool_text: str = "",
        system_instruction: str = "You are a Digimon TCG rules and tactical puzzle specialist.",
        ttl_seconds: int = 3600,
    ) -> str:
        """Convenience helper to pin `docs/RULES_CONTEXT.md` + card pool text via `cachedContents`."""
        blocks: list[str] = []
        if rules_path is not None and rules_path.exists():
            blocks.append(f"# RULES_CONTEXT\n\n{rules_path.read_text(encoding='utf-8')}")
        if card_pool_text:
            blocks.append(f"# CARD_POOL\n\n{card_pool_text}")
        if not blocks:
            blocks.append("# DIGIMON_PUZZLE_LAB_CONTEXT")
        return self.create_context_cache(
            system_instruction=system_instruction,
            context_blocks=blocks,
            ttl_seconds=ttl_seconds,
        )

    @staticmethod
    def _serialize_contents(
        messages: Sequence[ChatMessage],
        *,
        using_cached_content: bool,
    ) -> tuple[Optional[str], list[dict[str, Any]]]:
        sys_parts: list[str] = []
        contents: list[dict[str, Any]] = []

        idx = 0
        n = len(messages)
        while idx < n:
            m = messages[idx]
            if m.role == "system":
                if m.content:
                    sys_parts.append(m.content)
                idx += 1
                continue

            if m.role == "user":
                contents.append({"role": "user", "parts": [{"text": m.content or ""}]})
                idx += 1
                continue

            if m.role == "assistant":
                parts: list[dict[str, Any]] = []
                if m.content:
                    parts.append({"text": m.content})
                for tc in m.tool_calls:
                    parts.append({"functionCall": {"name": tc.name, "args": tc.arguments}})
                if not parts:
                    parts.append({"text": ""})
                contents.append({"role": "model", "parts": parts})
                idx += 1
                continue

            if m.role == "tool":
                # Group consecutive tool responses into a single Gemini user functionResponse turn.
                fn_parts: list[dict[str, Any]] = []
                while idx < n and messages[idx].role == "tool":
                    tm = messages[idx]
                    parsed_resp: dict[str, Any]
                    try:
                        loaded = json.loads(tm.content) if tm.content else {}
                        parsed_resp = loaded if isinstance(loaded, dict) else {"result": loaded}
                    except Exception:
                        parsed_resp = {"result": tm.content}
                    fn_parts.append(
                        {
                            "functionResponse": {
                                "name": tm.name or "tool",
                                "response": parsed_resp,
                            }
                        }
                    )
                    idx += 1
                contents.append({"role": "user", "parts": fn_parts})
                continue

            idx += 1

        system_text = "\n\n".join(sys_parts).strip() or None
        if using_cached_content and system_text:
            # Gemini v1beta forbids sending `systemInstruction` alongside `cachedContent`;
            # prepend turn-local instructions to the first user message instead.
            prefix = f"[Instructions]\n{system_text}\n\n"
            if contents and contents[0]["role"] == "user" and contents[0]["parts"] and "text" in contents[0]["parts"][0]:
                contents[0]["parts"][0]["text"] = prefix + contents[0]["parts"][0]["text"]
            else:
                contents.insert(0, {"role": "user", "parts": [{"text": prefix}]})
            system_text = None

        if not contents:
            contents.append({"role": "user", "parts": [{"text": ""}]})
        return system_text, contents

    def complete(
        self,
        messages: Sequence[ChatMessage],
        *,
        tools: Optional[Sequence[ToolSpec]] = None,
        json_schema: Optional[dict[str, Any]] = None,
        schema_name: str = "response",
        temperature: float = 0.2,
        max_tokens: int = 4096,
    ) -> LLMResponse:
        assert_closed_llm_allowed(self._allow_closed_models)
        client = self._get_client()
        headers = {
            "Content-Type": "application/json",
            "x-goog-api-key": self._resolve_api_key(),
        }

        using_cache = bool(self.cached_content_name)
        system_text, contents = self._serialize_contents(
            messages, using_cached_content=using_cache
        )

        gen_config: dict[str, Any] = {
            "temperature": temperature,
            "maxOutputTokens": max_tokens,
        }
        if json_schema is not None and not tools:
            gen_config["responseMimeType"] = "application/json"
            gen_config["responseJsonSchema"] = json_schema

        payload: dict[str, Any] = {
            "contents": contents,
            "generationConfig": gen_config,
        }
        if using_cache:
            payload["cachedContent"] = self.cached_content_name
        elif system_text:
            payload["systemInstruction"] = {"parts": [{"text": system_text}]}

        if tools and not using_cache:
            payload["tools"] = [
                {
                    "functionDeclarations": [
                        {
                            "name": t.name,
                            "description": t.description,
                            "parameters": _clean_schema_for_gemini(t.parameters),
                        }
                        for t in tools
                    ]
                }
            ]

        url = f"{self.base_url}/models/{self.model}:generateContent"
        resp = client.post(url, headers=headers, json=payload)
        resp.raise_for_status()
        data = resp.json()

        candidates = data.get("candidates") or []
        parts = ((candidates[0].get("content") or {}).get("parts") or []) if candidates else []

        text_chunks: list[str] = []
        tool_calls: list[ToolCall] = []
        for idx, part in enumerate(parts):
            if "text" in part and part["text"] is not None:
                text_chunks.append(str(part["text"]))
            if "functionCall" in part and isinstance(part["functionCall"], Mapping):
                fc = part["functionCall"]
                raw_args = fc.get("args") or {}
                args_dict = dict(raw_args) if isinstance(raw_args, Mapping) else {}
                tool_calls.append(
                    ToolCall(
                        id=str(fc.get("id") or f"gemini_call_{idx}_{uuid.uuid4().hex[:8]}"),
                        name=str(fc.get("name") or ""),
                        arguments=args_dict,
                    )
                )

        content = "".join(text_chunks)
        parsed_json = extract_json_object(content) if (json_schema is not None or not tool_calls) else None
        return LLMResponse(
            content=content,
            tool_calls=tool_calls,
            parsed_json=parsed_json,
            model=str(data.get("modelVersion") or self.model),
            usage=dict(data.get("usageMetadata") or {}),
            raw=data,
        )


class LLMRouter:
    """Two-tier router mapping `Role.SCANNER` and `Role.AUTHOR` to `LLMProvider`s."""

    def __init__(
        self,
        *,
        scanner: LLMProvider,
        author: LLMProvider,
    ) -> None:
        self._providers: dict[Role, LLMProvider] = {
            Role.SCANNER: scanner,
            Role.AUTHOR: author,
        }

    def provider_for(self, role: Role) -> LLMProvider:
        return self._providers[role]

    def close(self) -> None:
        seen: set[int] = set()
        for p in self._providers.values():
            if id(p) not in seen:
                seen.add(id(p))
                p.close()

    def __enter__(self) -> "LLMRouter":
        return self

    def __exit__(self, exc_type: Any, exc: Any, tb: Any) -> None:
        self.close()

    def complete(
        self,
        role: Role,
        messages: Sequence[ChatMessage],
        *,
        tools: Optional[Sequence[ToolSpec]] = None,
        json_schema: Optional[dict[str, Any]] = None,
        schema_name: str = "response",
        temperature: float = 0.2,
        max_tokens: int = 4096,
    ) -> LLMResponse:
        return self._providers[role].complete(
            messages,
            tools=tools,
            json_schema=json_schema,
            schema_name=schema_name,
            temperature=temperature,
            max_tokens=max_tokens,
        )

    def complete_structured(
        self,
        role: Role,
        messages: Sequence[ChatMessage],
        *,
        json_schema: dict[str, Any],
        schema_name: str = "response",
        temperature: float = 0.1,
        max_tokens: int = 4096,
    ) -> dict[str, Any]:
        resp = self.complete(
            role,
            messages,
            json_schema=json_schema,
            schema_name=schema_name,
            temperature=temperature,
            max_tokens=max_tokens,
        )
        if resp.parsed_json is None:
            raise ValueError(
                f"{role.value} provider ({resp.model}) did not return valid JSON: {resp.content[:300]!r}"
            )
        return resp.parsed_json

    def run_tool_loop(
        self,
        role: Role,
        messages: Sequence[ChatMessage],
        *,
        tools: Sequence[ToolSpec],
        tool_executor: Callable[[ToolCall], Any],
        max_turns: int = 10,
        final_json_schema: Optional[dict[str, Any]] = None,
        schema_name: str = "final_output",
        temperature: float = 0.2,
        max_tokens: int = 4096,
        stop_condition: Optional[Callable[[ToolCall, Any], bool]] = None,
    ) -> ToolLoopResult:
        """Execute a bounded multi-turn tool-calling loop on `role`, returning final response + trace."""
        history = list(messages)
        trace: list[ToolTraceEntry] = []
        last_resp: Optional[LLMResponse] = None
        turns = 0
        stopped_by_condition = False

        for turn_idx in range(1, max_turns + 1):
            turns = turn_idx
            resp = self.complete(
                role,
                history,
                tools=tools,
                temperature=temperature,
                max_tokens=max_tokens,
            )
            last_resp = resp
            history.append(
                ChatMessage(
                    role="assistant",
                    content=resp.content,
                    tool_calls=list(resp.tool_calls),
                )
            )

            if not resp.tool_calls:
                break

            for tc in resp.tool_calls:
                try:
                    raw_out = tool_executor(tc)
                except Exception as exc:
                    raw_out = {"ok": False, "error": str(exc)}
                trace.append(
                    ToolTraceEntry(
                        turn=turn_idx,
                        tool_call_id=tc.id,
                        name=tc.name,
                        arguments=tc.arguments,
                        result=raw_out,
                    )
                )
                serialized = (
                    raw_out
                    if isinstance(raw_out, str)
                    else json.dumps(raw_out, ensure_ascii=False)
                )
                history.append(
                    ChatMessage(
                        role="tool",
                        tool_call_id=tc.id,
                        name=tc.name,
                        content=serialized,
                    )
                )
                if stop_condition is not None and stop_condition(tc, raw_out):
                    stopped_by_condition = True
            if stopped_by_condition:
                break

        assert last_resp is not None
        if not stopped_by_condition and final_json_schema is not None and last_resp.parsed_json is None:
            history.append(
                ChatMessage(
                    role="user",
                    content=(
                        "Tool budget reached or investigation complete. "
                        "Emit your final structured JSON output now."
                    ),
                )
            )
            last_resp = self.complete(
                role,
                history,
                json_schema=final_json_schema,
                schema_name=schema_name,
                temperature=temperature,
                max_tokens=max_tokens,
            )
            history.append(ChatMessage(role="assistant", content=last_resp.content))

        return ToolLoopResult(
            final_response=last_resp,
            messages=history,
            tool_trace=trace,
            turns_used=turns,
        )

    @classmethod
    def from_config(
        cls,
        *,
        provider: str = "ollama",
        scanner_provider: Optional[str] = None,
        author_provider: Optional[str] = None,
        scanner_model: Optional[str] = None,
        author_model: Optional[str] = None,
        base_url: Optional[str] = None,
        scanner_base_url: Optional[str] = None,
        author_base_url: Optional[str] = None,
        api_key: Optional[str] = None,
        allow_closed_models: bool = False,
        cached_content_name: Optional[str] = None,
        timeout: float = 120.0,
    ) -> "LLMRouter":
        """Construct a router from CLI/preset strings (`ollama`, `vllm`, `gemini`)."""

        def _make_one(role: Role, kind: str, model_override: Optional[str], url_override: Optional[str]) -> LLMProvider:
            norm = kind.strip().lower()
            if norm in ("ollama", "llamacpp", "llama.cpp"):
                default_model = (
                    DEFAULT_OLLAMA_SCANNER_MODEL if role == Role.SCANNER else DEFAULT_OLLAMA_AUTHOR_MODEL
                )
                return OpenAICompatProvider.ollama(
                    model=model_override or default_model,
                    base_url=url_override or base_url or DEFAULT_OLLAMA_BASE_URL,
                    timeout=timeout,
                )
            if norm in ("vllm", "sglang", "openai", "openai-compat"):
                default_model = (
                    DEFAULT_VLLM_SCANNER_MODEL if role == Role.SCANNER else DEFAULT_VLLM_AUTHOR_MODEL
                )
                resolved_url = (
                    url_override
                    or base_url
                    or os.environ.get("PUZZLE_LAB_BASE_URL")
                    or DEFAULT_OLLAMA_BASE_URL
                )
                return OpenAICompatProvider.vllm(
                    base_url=resolved_url,
                    model=model_override or default_model,
                    api_key=api_key,
                    timeout=timeout,
                )
            if norm in ("gemini", "google", "closed"):
                default_model = (
                    DEFAULT_GEMINI_SCANNER_MODEL if role == Role.SCANNER else DEFAULT_GEMINI_AUTHOR_MODEL
                )
                return GeminiDevProvider(
                    model=model_override or default_model,
                    allow_closed_models=allow_closed_models,
                    api_key=api_key,
                    base_url=url_override or base_url or DEFAULT_GEMINI_BASE_URL,
                    cached_content_name=cached_content_name if role == Role.AUTHOR else None,
                    timeout=timeout,
                )
            raise ValueError(
                f"Unknown LLM provider {kind!r}; expected 'ollama', 'vllm', 'openai-compat', or 'gemini'."
            )

        sp = scanner_provider or provider
        ap = author_provider or provider
        return cls(
            scanner=_make_one(Role.SCANNER, sp, scanner_model, scanner_base_url),
            author=_make_one(Role.AUTHOR, ap, author_model, author_base_url),
        )
