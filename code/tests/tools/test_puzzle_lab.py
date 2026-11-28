"""Comprehensive offline unit & integration tests for `tools.puzzle_lab` (Track 4)."""

from __future__ import annotations

import ast
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Callable, Mapping

import httpx
import pytest

from digimon_gym.agents.studio_puzzles import PuzzleResetWrapper, load_studio_puzzle
from digimon_gym.digimon_gym import DigimonEnv
from tools.puzzle_lab import (
    PUZZLE_SCHEMA,
    SEARCH_EXAM_SCHEMA,
    CardTextCatalog,
    PuzzleValidationError,
    resolve_repo_root,
    validate_studio_puzzle_doc,
)
from tools.puzzle_lab.cli import main as cli_main
from tools.puzzle_lab.llm_router import (
    CLOSED_LLM_ENV_VAR,
    DEFAULT_GEMINI_AUTHOR_MODEL,
    DEFAULT_GEMINI_SCANNER_MODEL,
    ChatMessage,
    GeminiDevProvider,
    LLMProvider,
    LLMResponse,
    LLMRouter,
    OpenAICompatProvider,
    Role,
    ToolCall,
    ToolSpec,
)
from tools.puzzle_lab.mcp_client import (
    McpRpcError,
    McpToolError,
    MockMcpTransport,
    StdioMcpClient,
)
from tools.puzzle_lab.mine_replays import find_candidate_moments, mine_replays
from tools.puzzle_lab.synthesize_archetype import (
    inspect_starved_embeddings,
    synthesize_archetype,
)

FILLER_DECK = ["ST1-01"] * 4 + ["ST1-03"] * 50


class ScriptedLLMProvider(LLMProvider):
    """Deterministic scripted `LLMProvider` for multi-turn workflow tests."""

    is_closed_weight = False

    def __init__(
        self,
        steps: list[Callable[[list[ChatMessage]], LLMResponse] | LLMResponse],
        *,
        model: str = "scripted-mock",
    ) -> None:
        self.model = model
        self._steps = list(steps)
        self.calls: list[dict[str, Any]] = []

    def complete(
        self,
        messages,
        *,
        tools=None,
        json_schema=None,
        schema_name="response",
        temperature=0.2,
        max_tokens=4096,
    ) -> LLMResponse:
        msg_list = list(messages)
        self.calls.append(
            {
                "messages": msg_list,
                "tools": list(tools or []),
                "json_schema": json_schema,
                "schema_name": schema_name,
            }
        )
        if not self._steps:
            raise AssertionError("ScriptedLLMProvider exhausted all scripted responses")
        nxt = self._steps.pop(0)
        return nxt(msg_list) if callable(nxt) else nxt


# ── 1. Closed-Weight Gate & GeminiDevProvider Tests ───────────────────────────


def test_gemini_dev_provider_raises_permission_error_by_default(monkeypatch: pytest.MonkeyPatch):
    monkeypatch.delenv(CLOSED_LLM_ENV_VAR, raising=False)

    with pytest.raises(PermissionError, match=CLOSED_LLM_ENV_VAR):
        GeminiDevProvider()

    with pytest.raises(PermissionError, match=CLOSED_LLM_ENV_VAR):
        LLMRouter.from_config(provider="gemini")

    # Even if env was set at init and later revoked before .complete(), .complete() re-checks!
    monkeypatch.setenv(CLOSED_LLM_ENV_VAR, "1")
    prov = GeminiDevProvider(api_key="fake-key")
    monkeypatch.setenv(CLOSED_LLM_ENV_VAR, "0")
    with pytest.raises(PermissionError, match=CLOSED_LLM_ENV_VAR):
        prov.complete([ChatMessage(role="user", content="hello")])


def test_gemini_dev_provider_hydrogen_scanner_neon_author_and_context_cache(
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
):
    monkeypatch.setenv(CLOSED_LLM_ENV_VAR, "1")
    monkeypatch.setenv("GEMINI_API_KEY", "test-gemini-dev-key")

    captured_requests: list[tuple[str, dict[str, str], dict[str, Any]]] = []

    def _handler(request: httpx.Request) -> httpx.Response:
        body = json.loads(request.content.decode("utf-8"))
        captured_requests.append((str(request.url), dict(request.headers), body))
        path = request.url.path

        if path.endswith("/cachedContents"):
            return httpx.Response(
                200,
                json={"name": "cachedContents/rules-pool-cache-001"},
            )
        if f"/models/{DEFAULT_GEMINI_SCANNER_MODEL}:generateContent" in path:
            return httpx.Response(
                200,
                json={
                    "modelVersion": DEFAULT_GEMINI_SCANNER_MODEL,
                    "candidates": [
                        {
                            "content": {
                                "role": "model",
                                "parts": [
                                    {
                                        "text": json.dumps(
                                            {"keep": True, "category": "dna_lethal"}
                                        )
                                    }
                                ],
                            }
                        }
                    ],
                },
            )
        if f"/models/{DEFAULT_GEMINI_AUTHOR_MODEL}:generateContent" in path:
            # First call emits a functionCall; second call after functionResponse emits final JSON
            contents = body.get("contents") or []
            has_fn_resp = any(
                "functionResponse" in part
                for c in contents
                for part in (c.get("parts") or [])
            )
            if "tools" in body and not has_fn_resp:
                return httpx.Response(
                    200,
                    json={
                        "modelVersion": DEFAULT_GEMINI_AUTHOR_MODEL,
                        "candidates": [
                            {
                                "content": {
                                    "role": "model",
                                    "parts": [
                                        {
                                            "functionCall": {
                                                "name": "inspect_stack",
                                                "args": {"game_id": "g1", "player": 1, "slot": 0},
                                            }
                                        }
                                    ],
                                }
                            }
                        ],
                    },
                )
            return httpx.Response(
                200,
                json={
                    "modelVersion": DEFAULT_GEMINI_AUTHOR_MODEL,
                    "candidates": [
                        {
                            "content": {
                                "role": "model",
                                "parts": [{"text": json.dumps({"solved": True})}],
                            }
                        }
                    ],
                },
            )
        return httpx.Response(404, json={"error": f"unexpected path {path}"})

    transport = httpx.MockTransport(_handler)
    scanner = GeminiDevProvider(model=DEFAULT_GEMINI_SCANNER_MODEL, transport=transport)
    author = GeminiDevProvider(model=DEFAULT_GEMINI_AUTHOR_MODEL, transport=transport)
    assert scanner.model == "gemini-4-hydrogen"
    assert author.model == "gemini-4-neon"

    # Pin RULES_CONTEXT + card pool text via cachedContents
    rules_md = tmp_path / "RULES_CONTEXT.md"
    rules_md.write_text("# Digimon Rules Summary\nBreeding phase precedes Main phase.\n", encoding="utf-8")
    cache_name = author.pin_rules_and_card_pool(
        rules_path=rules_md,
        card_pool_text="BT16-025 Paildramon: DNA Digivolve Blue Lv.4 + Green Lv.4",
    )
    assert cache_name == "cachedContents/rules-pool-cache-001"

    cache_url, cache_headers, cache_body = captured_requests[0]
    assert cache_url.endswith("/v1beta/cachedContents")
    assert cache_headers["x-goog-api-key"] == "test-gemini-dev-key"
    assert cache_body["model"] == "models/gemini-4-neon"
    assert "RULES_CONTEXT" in cache_body["contents"][0]["parts"][0]["text"]
    assert "BT16-025 Paildramon" in cache_body["contents"][0]["parts"][1]["text"]

    router = LLMRouter(scanner=scanner, author=author)
    scan_json = router.complete_structured(
        Role.SCANNER,
        [ChatMessage(role="user", content="triage moment")],
        json_schema={"type": "object", "properties": {"keep": {"type": "boolean"}}},
    )
    assert scan_json == {"keep": True, "category": "dna_lethal"}

    # Verify AUTHOR sends `cachedContent` without colliding `systemInstruction` and handles tool loop
    author.cached_content_name = None  # enable inline tool declarations for tool-loop test
    loop_res = router.run_tool_loop(
        Role.AUTHOR,
        [
            ChatMessage(role="system", content="Author instructions"),
            ChatMessage(role="user", content="Check stack 0"),
        ],
        tools=[
            ToolSpec(
                name="inspect_stack",
                description="Inspect stack",
                parameters={"type": "object", "properties": {"game_id": {"type": "string"}}, "additionalProperties": False},
            )
        ],
        tool_executor=lambda tc: {"ok": True, "top_card": {"card_id": "BT16-025"}},
        max_turns=4,
    )
    assert loop_res.turns_used == 2
    assert len(loop_res.tool_trace) == 1
    assert loop_res.tool_trace[0].name == "inspect_stack"
    assert loop_res.final_response.parsed_json == {"solved": True}

    # Now re-attach cachedContent and confirm `cachedContent` is attached & `systemInstruction` omitted
    author.cached_content_name = cache_name
    cached_resp = router.complete_structured(
        Role.AUTHOR,
        [
            ChatMessage(role="system", content="Extra turn instruction"),
            ChatMessage(role="user", content="Synthesize"),
        ],
        json_schema={"type": "object", "properties": {"solved": {"type": "boolean"}}},
    )
    assert cached_resp == {"solved": True}
    _, _, last_body = captured_requests[-1]
    assert last_body["cachedContent"] == "cachedContents/rules-pool-cache-001"
    assert "systemInstruction" not in last_body


# ── 2. OpenAICompatProvider (Ollama + RunPod vLLM) Tests ──────────────────────


def test_openai_compat_provider_ollama_fallback_and_vllm_tool_calls():
    ollama_calls: list[dict[str, Any]] = []

    def _ollama_handler(request: httpx.Request) -> httpx.Response:
        assert str(request.url) == "http://localhost:11434/v1/chat/completions"
        body = json.loads(request.content.decode("utf-8"))
        ollama_calls.append(body)
        # Simulate local llama.cpp/Ollama rejecting `json_schema` with 400 and accepting `json_object`
        rf = body.get("response_format") or {}
        if rf.get("type") == "json_schema":
            return httpx.Response(400, json={"error": "json_schema unsupported on this runner"})
        assert rf.get("type") == "json_object"
        return httpx.Response(
            200,
            json={
                "model": "qwen3:8b",
                "choices": [
                    {
                        "message": {
                            "role": "assistant",
                            "content": "<think>Evaluating replay drop...</think>\n{\"keep\": true, \"score\": 0.9}",
                        }
                    }
                ],
            },
        )

    vllm_calls: list[dict[str, Any]] = []

    def _vllm_handler(request: httpx.Request) -> httpx.Response:
        assert str(request.url) == "https://pod99-8000.proxy.runpod.net/v1/chat/completions"
        body = json.loads(request.content.decode("utf-8"))
        vllm_calls.append(body)
        msgs = body["messages"]
        if not any(m.get("role") == "tool" for m in msgs):
            return httpx.Response(
                200,
                json={
                    "model": "Qwen/Qwen3-32B",
                    "choices": [
                        {
                            "message": {
                                "role": "assistant",
                                "content": "",
                                "tool_calls": [
                                    {
                                        "id": "call_score_1",
                                        "type": "function",
                                        "function": {
                                            "name": "score_line",
                                            "arguments": json.dumps({"game_id": "g42", "actions": [63, 114]}),
                                        },
                                    }
                                ],
                            }
                        }
                    ],
                },
            )
        return httpx.Response(
            200,
            json={
                "model": "Qwen/Qwen3-32B",
                "choices": [
                    {
                        "message": {
                            "role": "assistant",
                            "content": "```json\n{\"winner\": 1, \"line\": [63, 114]}\n```",
                        }
                    }
                ],
            },
        )

    scanner = OpenAICompatProvider.ollama(
        model="qwen3:8b",
        transport=httpx.MockTransport(_ollama_handler),
    )
    author = OpenAICompatProvider.vllm(
        base_url="https://pod99-8000.proxy.runpod.net/v1",
        model="Qwen/Qwen3-32B",
        api_key="rp_test_key",
        transport=httpx.MockTransport(_vllm_handler),
    )
    router = LLMRouter(scanner=scanner, author=author)

    res_scan = router.complete_structured(
        Role.SCANNER,
        [ChatMessage(role="user", content="Check candidate")],
        json_schema={"type": "object", "properties": {"keep": {"type": "boolean"}}},
    )
    assert res_scan == {"keep": True, "score": 0.9}
    assert len(ollama_calls) == 2  # initial json_schema + automatic json_object fallback

    loop = router.run_tool_loop(
        Role.AUTHOR,
        [ChatMessage(role="user", content="Find lethal line")],
        tools=[
            ToolSpec(
                name="score_line",
                description="Score line",
                parameters={"type": "object", "properties": {"game_id": {"type": "string"}}},
            )
        ],
        tool_executor=lambda tc: {"ok": True, "final_winner": 1},
        max_turns=4,
    )
    assert loop.turns_used == 2
    assert loop.tool_trace[0].arguments == {"game_id": "g42", "actions": [63, 114]}
    assert loop.final_response.parsed_json == {"winner": 1, "line": [63, 114]}


# ── 3. StdioMcpClient + MockMcpTransport Tests ────────────────────────────────


def test_stdio_mcp_client_handshake_envelope_and_errors():
    transport = MockMcpTransport(server_name="digimon-engine-mcp")
    transport.register_tool(
        "inspect_stack",
        lambda args: {
            "ok": True,
            "top_card": {"card_id": "BT16-025", "card_name": "Paildramon"},
            "slot": args.get("slot", 0),
        },
        description="Inspect stack slot",
    )
    transport.register_tool(
        "broken_tool",
        lambda args: {"ok": False, "error": "invalid game_id"},
    )

    with StdioMcpClient(transport) as client:
        assert transport.initialized is True
        assert client.server_info["name"] == "digimon-engine-mcp"
        specs = client.to_llm_tool_specs(allowed_names=["inspect_stack"])
        assert len(specs) == 1
        assert specs[0].name == "inspect_stack"

        out = client.call_tool("inspect_stack", {"game_id": "g1", "player": 1, "slot": 0})
        assert out["ok"] is True
        assert out["top_card"]["card_id"] == "BT16-025"

        # Default check_ok=False returns error dict so LLM tool loops can self-correct
        err_dict = client.call_tool("broken_tool", {})
        assert err_dict["ok"] is False

        # check_ok=True raises McpToolError
        with pytest.raises(McpToolError, match="invalid game_id"):
            client.call_tool("broken_tool", {}, check_ok=True)

        # Calling unregistered tool raises JSON-RPC -32602 McpRpcError
        with pytest.raises(McpRpcError):
            client.call_tool("nonexistent_tool", {})


# ── 4. Workflow 1: `find_candidate_moments` + `mine_replays` E2E Tests ────────


def _sample_staged_snapshot() -> dict[str, Any]:
    return {
        "decks": {"1": list(FILLER_DECK), "2": list(FILLER_DECK)},
        "seed": 7,
        "state": {"memory": 2, "phase": "Main", "turn": 3, "first_player": 1},
        "zones": {
            "1": {
                "breeding": None,
                "hand": ["BT16-025", "ST1-03"],
                "field": [
                    {"stack": ["BT12-021", "BT12-022"], "is_suspended": False, "turn_played": 1},
                    {"stack": ["BT12-050"], "is_suspended": False, "turn_played": 1},
                ],
                "security": ["ST1-03", "ST1-03"],
                "trash": [],
                "deck_top": ["ST1-03"] * 30,
                "egg_deck": ["ST1-01"] * 4,
            },
            "2": {
                "breeding": None,
                "hand": ["ST1-03"],
                "field": [
                    {"stack": ["EX13-024"], "is_suspended": False, "turn_played": 1},
                ],
                "security": ["ST1-03"],
                "trash": [],
                "deck_top": ["ST1-03"] * 30,
                "egg_deck": ["ST1-01"] * 4,
            },
        },
    }


def test_find_candidate_moments_surfaces_value_drops_disagreements_and_stranded_cards(tmp_path: Path):
    snap = _sample_staged_snapshot()
    rec_path = tmp_path / "match_01.json"
    winprob_path = tmp_path / "match_01.winprob.json"
    eval_path = tmp_path / "checkpoint_1000_eval.json"

    recording = {
        "action_schema_version": 2,
        "initial_state": {"first_player_id": 1, "seed": 7},
        "actions": [
            # Ignored Mulligan row
            {"step": 0, "turn": 1, "phase": "Mulligan", "player_id": 1, "action_id": 0},
            # Decision 0: forced single action (played_prior = 1.0) -> should NOT trigger value_drop
            {
                "step": 1,
                "turn": 1,
                "phase": "Breeding",
                "player_id": 1,
                "action_id": 62,
                "snapshot": snap,
            },
            # Decision 1: P1 blunders in Main (value +0.40 for P1 -> next decision P2 has +0.30, i.e. -0.30 for P1 => delta -0.70!)
            # Also has stranded DNA card BT16-025 in hand while playing action 62 (Pass)
            {
                "step": 2,
                "turn": 3,
                "phase": "Main",
                "player_id": 1,
                "action_id": 62,
                "snapshot": snap,
                "legal_actions": [
                    {"action_id": 63, "card_id": "BT16-025", "label": "DNA Digivolve Paildramon"},
                    {"action_id": 62, "card_id": None, "label": "Pass"},
                ],
            },
            # Decision 2: P2 has policy vs search disagreement
            {
                "step": 3,
                "turn": 3,
                "phase": "Main",
                "player_id": 2,
                "action_id": 114,
                "policy_top_action_id": 114,
                "search_top_action_id": 62,
                "snapshot": snap,
            },
        ],
    }
    winprob = {
        "schema": "winprob-annotations-v1",
        "recording": str(rec_path),
        "decisions": [
            {"step": 1, "turn": 1, "phase": "Breeding", "player_id": 1, "action_id": 62, "value": 0.40, "played_prior": 1.0},
            {"step": 2, "turn": 3, "phase": "Main", "player_id": 1, "action_id": 62, "value": 0.40, "played_prior": 0.55},
            {"step": 3, "turn": 3, "phase": "Main", "player_id": 2, "action_id": 114, "value": 0.30, "played_prior": 0.70},
        ],
    }
    card_eval_report = {
        "schema_version": 1,
        "checkpoint_step": 1000,
        "routes": [
            {"player": 1, "card_id": "BT16-025", "route": "dna_digivolve", "opportunities": 6, "choices": 0}
        ],
        "cards": [],
        "effect_sequences": [],
    }

    rec_path.write_text(json.dumps(recording), encoding="utf-8")
    winprob_path.write_text(json.dumps(winprob), encoding="utf-8")
    eval_path.write_text(json.dumps(card_eval_report), encoding="utf-8")

    moments = find_candidate_moments(
        [tmp_path],
        card_eval_paths=[eval_path],
        value_drop_threshold=-0.25,
        min_stranded_opportunities=3,
    )
    kinds = {m.kind for m in moments}
    assert kinds == {"value_drop", "policy_search_disagreement", "stranded_card"}

    drop_m = next(m for m in moments if m.kind == "value_drop")
    assert drop_m.recorded_step == 2
    assert drop_m.value_delta == pytest.approx(-0.70)

    disagree_m = next(m for m in moments if m.kind == "policy_search_disagreement")
    assert disagree_m.policy_top_action_id == 114
    assert disagree_m.search_top_action_id == 62

    stranded_m = next(m for m in moments if m.kind == "stranded_card")
    assert "BT16-025" in stranded_m.focus_card_ids


def test_mine_replays_end_to_end_verifies_valid_and_rejects_unverified_puzzles(tmp_path: Path):
    recs_dir = tmp_path / "replays"
    recs_dir.mkdir()
    puzzles_dir = tmp_path / "puzzles"
    exams_dir = tmp_path / "search-exams"
    ckpt_zip = tmp_path / "step_5000.zip"
    ckpt_zip.write_bytes(b"PK-placeholder")

    # Create a recording with 3 candidate drops:
    # - Moment A: Scanner rejects as RNG false positive
    # - Moment B: Scanner keeps, Author submits puzzle whose `avoid` line unexpectedly solves -> REJECTED by verify_puzzle!
    # - Moment C: Scanner keeps, Author uses MCP tools (`load_puzzle`, `inspect_stack`, `score_line`, `plasticity_probe`, `verify_puzzle`, `save_puzzle`) -> ACCEPTED & saved!
    base_zones = {
        "1": {
            "breeding": None,
            "hand": ["ST1-03"],
            "field": [{"stack": ["ST1-03"], "is_suspended": False, "turn_played": 1}],
            "security": ["ST1-03"],
            "trash": [],
            "deck_top": ["ST1-03"] * 40,
            "egg_deck": ["ST1-01"] * 4,
        },
        "2": {
            "breeding": None,
            "hand": ["ST1-03"],
            "field": [],
            "security": [],
            "trash": [],
            "deck_top": ["ST1-03"] * 40,
            "egg_deck": ["ST1-01"] * 4,
        },
    }
    snap = {
        "decks": {"1": list(FILLER_DECK), "2": list(FILLER_DECK)},
        "seed": 11,
        "state": {"memory": 2, "phase": "Main", "turn": 3, "first_player": 1},
        "zones": base_zones,
    }
    rec_file = recs_dir / "game_a.json"
    rec_file.write_text(
        json.dumps(
            {
                "initial_state": {"first_player_id": 1, "seed": 11},
                "actions": [
                    {"step": 1, "turn": 2, "phase": "Main", "player_id": 1, "action_id": 62, "value": 0.50, "value_after": -0.40, "snapshot": snap},
                    {"step": 2, "turn": 3, "phase": "Main", "player_id": 1, "action_id": 62, "value": 0.45, "value_after": -0.35, "snapshot": snap},
                    {"step": 3, "turn": 4, "phase": "Main", "player_id": 1, "action_id": 62, "value": 0.40, "value_after": -0.30, "snapshot": snap},
                ],
            }
        ),
        encoding="utf-8",
    )

    engine_transport = MockMcpTransport(server_name="digimon-engine-mcp")
    engine_transport.register_tool(
        "load_puzzle",
        lambda args: {"ok": True, "game_id": "g-author-1", "seat": 1, "mode": args.get("mode", "author")},
    )
    engine_transport.register_tool(
        "inspect_position",
        lambda args: {"ok": True, "policy_value": -0.1, "chosen_action_id": 114},
    )
    engine_transport.register_tool(
        "inspect_stack",
        lambda args: {"ok": True, "top_card": {"card_id": "ST1-03", "card_name": "Agumon", "dp_effective": 2000}},
    )
    engine_transport.register_tool(
        "score_line",
        lambda args: {"ok": True, "root_mover": 1, "steps": [{"action_id": 114}], "final_winner": 1},
    )

    def _mock_verify_puzzle(args: dict[str, Any]) -> dict[str, Any]:
        pjson = args.get("puzzle_json") or {}
        if pjson.get("id") == "bad-unverified-puzzle":
            # Avoid line solved -> should fail verification!
            return {
                "ok": True,
                "target_results": [{"id": "t1", "passed": True, "solved": True}],
                "avoid_results": [{"id": "a1", "passed": False, "solved": True}],
                "expectation_results": [],
            }
        return {
            "ok": True,
            "target_results": [{"id": "lethal", "passed": True, "solved": True, "final_winner": 1}],
            "avoid_results": [{"id": "pass-blunder", "passed": True, "solved": False, "refuted": True}],
            "expectation_results": [{"passed": True}],
        }

    engine_transport.register_tool("verify_puzzle", _mock_verify_puzzle)

    training_transport = MockMcpTransport(server_name="digimon-training-mcp")
    training_transport.register_tool(
        "plasticity_probe",
        lambda args: {
            "ok": True,
            "pre": {"target_prob": 0.08, "margin": -1.4},
            "post": {"target_prob": 0.74, "margin": 1.9},
            "learnable": True,
            "diagnosis": "healthy gradient update",
        },
    )
    training_transport.register_tool(
        "position_saliency",
        lambda args: {"ok": True, "objective": "logit_margin", "base_value": 0.1, "top_features": []},
    )

    scanner_provider = ScriptedLLMProvider(
        [
            # Moment 1 (delta -0.90): Scanner rejects
            LLMResponse(
                content="",
                parsed_json={
                    "keep": False,
                    "category": "rng_security",
                    "reason": "Pure security flip variance",
                    "focus_cards": [],
                    "candidate_target_actions": [],
                    "candidate_avoid_actions": [],
                },
            ),
            # Moment 2 (delta -0.80): Scanner keeps (will fail verify_puzzle)
            LLMResponse(
                content="",
                parsed_json={
                    "keep": True,
                    "category": "missed_lethal",
                    "reason": "Missed direct swing",
                    "focus_cards": ["ST1-03"],
                    "candidate_target_actions": [114],
                    "candidate_avoid_actions": [62],
                },
            ),
            # Moment 3 (delta -0.70): Scanner keeps (will succeed)
            LLMResponse(
                content="",
                parsed_json={
                    "keep": True,
                    "category": "direct_attack_lethal",
                    "reason": "Direct attack wins immediately",
                    "focus_cards": ["ST1-03"],
                    "candidate_target_actions": [114],
                    "candidate_avoid_actions": [62],
                },
            ),
        ]
    )

    author_provider = ScriptedLLMProvider(
        [
            # Author run for Moment 2 -> saves a bad puzzle where avoid also solves
            LLMResponse(
                content="",
                tool_calls=[
                    ToolCall(
                        id="c_bad_save",
                        name="save_puzzle",
                        arguments={
                            "id": "bad-unverified-puzzle",
                            "title": "Bad Unverified Puzzle",
                            "description": "Avoid line accidentally wins",
                            "lines": [
                                {"id": "t1", "name": "target", "role": "target", "actions": [114]},
                                {"id": "a1", "name": "avoid", "role": "avoid", "actions": [0]},
                            ],
                        },
                    )
                ],
            ),
            # Author run for Moment 3 -> multi-step MCP tool inspection + save_puzzle
            LLMResponse(
                content="",
                tool_calls=[
                    ToolCall(
                        id="c_load",
                        name="load_puzzle",
                        arguments={"puzzle_json": {"id": "agumon-direct-lethal", "setup": {"kind": "staged", **snap}}},
                    ),
                    ToolCall(
                        id="c_stack",
                        name="inspect_stack",
                        arguments={"game_id": "g-author-1", "player": 1, "zone": "battle", "slot": 0},
                    ),
                    ToolCall(
                        id="c_score",
                        name="score_line",
                        arguments={"game_id": "g-author-1", "actions": [114]},
                    ),
                    ToolCall(
                        id="c_plast",
                        name="plasticity_probe",
                        arguments={"target_action_id": 114, "avoid_action_ids": [62]},
                    ),
                ],
            ),
            LLMResponse(
                content="",
                tool_calls=[
                    ToolCall(
                        id="c_good_save",
                        name="save_puzzle",
                        arguments={
                            "id": "agumon-direct-lethal",
                            "title": "Agumon Direct Attack Lethal",
                            "description": "Swing with unsuspended Agumon on 0-security opponent instead of passing.",
                            "tags": ["lethal", "replay-mined"],
                            "seat": 1,
                            "setup": {"kind": "staged", "state": snap["state"], "zones": base_zones, "prefix_actions": []},
                            "lines": [
                                {"id": "lethal", "name": "direct_lethal", "role": "target", "actions": [114]},
                                {"id": "pass-blunder", "name": "pass_turn", "role": "avoid", "actions": [62]},
                            ],
                            "rl_goal": {"mode": "playout_to_win", "max_turns": None, "max_actions": None, "win_assertions": []},
                        },
                    )
                ],
            ),
        ]
    )

    router = LLMRouter(scanner=scanner_provider, author=author_provider)
    with StdioMcpClient(engine_transport) as eng_mcp, StdioMcpClient(training_transport) as trn_mcp:
        report = mine_replays(
            [recs_dir],
            router=router,
            engine_mcp=eng_mcp,
            training_mcp=trn_mcp,
            checkpoint_path=ckpt_zip,
            puzzles_dir=puzzles_dir,
            search_exams_dir=exams_dir,
            folder="mined",
            require_plasticity=True,
            validate_with_engine=True,
        )

    assert report.candidates_found == 3
    assert report.scanner_kept == 2
    assert report.puzzles_written == 1

    # Unverified puzzle MUST NOT exist on disk
    assert not (puzzles_dir / "mined" / "bad-unverified-puzzle.json").exists()

    # Verified puzzle MUST exist, pass studio-puzzle-v1 validation, and load in StudioPuzzle + PuzzleResetWrapper!
    saved_path = puzzles_dir / "mined" / "agumon-direct-lethal.json"
    exam_path = exams_dir / "agumon-direct-lethal.json"
    assert saved_path.exists()
    assert exam_path.exists()

    saved_doc = json.loads(saved_path.read_text(encoding="utf-8"))
    assert saved_doc["schema"] == PUZZLE_SCHEMA
    validate_studio_puzzle_doc(saved_doc, validate_with_engine=True)

    exam_doc = json.loads(exam_path.read_text(encoding="utf-8"))
    assert exam_doc["schema"] == SEARCH_EXAM_SCHEMA

    loaded_puzzle = load_studio_puzzle(saved_path)
    assert loaded_puzzle.id == "agumon-direct-lethal"
    wrapped_env = PuzzleResetWrapper(DigimonEnv(), [loaded_puzzle], puzzle_prob=1.0, seed=0)
    obs, info = wrapped_env.reset(seed=0)
    assert obs.shape == wrapped_env.observation_space.shape
    assert info["studio_puzzle_id"] == "agumon-direct-lethal"
    mask = wrapped_env.action_mask()
    assert int(mask[114]) == 1  # Slot 0 direct attack is legal on the staged board!


# ── 5. Workflow 2: `synthesize_archetype` E2E Tests ───────────────────────────


def test_synthesize_archetype_combos_interrupt_traps_and_starved_embeddings(tmp_path: Path):
    puzzles_dir = tmp_path / "puzzles"
    exams_dir = tmp_path / "exams"
    ckpt_zip = tmp_path / "champion.zip"
    ckpt_zip.write_bytes(b"PK-placeholder")

    # Build a self-contained mini deck_library.json using real implemented cards
    mini_library = tmp_path / "deck_library.json"
    imperial_deck = ["BT12-002"] * 4 + ["BT12-021"] * 15 + ["BT12-022"] * 15 + ["BT12-050"] * 10 + ["BT16-025"] * 10
    mini_library.write_text(
        json.dumps(
            {
                "version": 1,
                "archetypes": {
                    "Imperialdramon": {
                        "archetype_name": "Imperialdramon",
                        "decklists": [
                            {
                                "deck_id": "imp-digilab-1",
                                "source": "digilab",
                                "is_top_cut": True,
                                "decklist": json.dumps(imperial_deck),
                            }
                        ],
                    }
                },
            }
        ),
        encoding="utf-8",
    )

    training_transport = MockMcpTransport(server_name="digimon-training-mcp")
    training_transport.register_tool(
        "card_embedding_neighbors",
        lambda args: {
            "ok": True,
            "cards": [
                {
                    "card_id": "BT16-025",
                    "card_name": "Paildramon",
                    "l2_norm": 0.12,  # Starved norm!
                    "neighbors": [{"card_id": "ST1-03", "cosine_similarity": 0.18}],
                },
                {
                    "card_id": "BT12-022",
                    "card_name": "ExVeemon",
                    "l2_norm": 1.45,
                    "neighbors": [{"card_id": "BT12-050", "cosine_similarity": 0.82}],
                },
            ],
        },
    )
    training_transport.register_tool(
        "plasticity_probe",
        lambda args: {"ok": True, "learnable": True, "diagnosis": "ok"},
    )

    engine_transport = MockMcpTransport(server_name="digimon-engine-mcp")
    engine_transport.register_tool(
        "load_puzzle",
        lambda args: {"ok": True, "game_id": "g-synth", "seat": 1, "mode": "author"},
    )
    engine_transport.register_tool(
        "score_line",
        lambda args: {
            "ok": True,
            "root_mover": 1,
            "steps": [{"action_id": a} for a in args.get("actions", [])],
            "final_winner": 1,
        },
    )

    def _verify_synth(args: dict[str, Any]) -> dict[str, Any]:
        pjson = args.get("puzzle_json") or {}
        if pjson.get("id") == "broken-blueprint":
            return {
                "ok": True,
                "target_results": [{"id": "t", "passed": False, "error": "illegal action 999"}],
                "avoid_results": [],
                "expectation_results": [],
            }
        return {
            "ok": True,
            "target_results": [{"id": "target", "passed": True, "solved": True, "final_winner": 1}],
            "avoid_results": [{"id": "avoid", "passed": True, "solved": False, "refuted": True}],
            "expectation_results": [],
        }

    engine_transport.register_tool("verify_puzzle", _verify_synth)

    def _author_blueprints(messages: list[ChatMessage]) -> LLMResponse:
        prompt_data = json.loads(messages[-1].content)
        # Verify starved card IDs and EX12-034 / EX12-071 interrupt trap reference cards were injected!
        assert "BT16-025" in prompt_data["starved_card_ids"]
        trap_ids = {c["card_id"] for c in prompt_data["interrupt_trap_reference_cards"]}
        assert {"EX12-034", "EX12-071"}.issubset(trap_ids)

        blueprints = [
            # 1. Offensive DNA Paildramon lethal
            {
                "id": "imperialdramon-dna-paildramon-lethal",
                "title": "Imperialdramon DNA Paildramon Jamming Lethal",
                "description": "DNA digivolve ExVeemon + Stingmon into BT16-025 Paildramon for 0 memory and swing.",
                "tags": ["dna", "lethal"],
                "seat": 1,
                "setup": {
                    "kind": "staged",
                    "state": {"memory": 2, "phase": "Main", "turn": 3, "first_player": 1},
                    "zones": {
                        "1": {
                            "hand": ["BT16-025"],
                            "field": [
                                {"stack": ["BT12-021", "BT12-022"], "is_suspended": False, "turn_played": 1},
                                {"stack": ["BT12-050"], "is_suspended": False, "turn_played": 1},
                            ],
                            "security": ["BT12-021"],
                            "trash": [],
                        },
                        "2": {
                            "hand": [],
                            "field": [],
                            "security": [],
                            "trash": [],
                        },
                    },
                },
                "lines": [
                    {"id": "dna-lethal", "name": "dna_paildramon_lethal", "role": "target", "actions": [63, 0, 1, 114]},
                    {"id": "hard-evo-pass", "name": "normal_digivolve_pass", "role": "avoid", "actions": [400]},
                ],
                "rl_goal": {"mode": "playout_to_win"},
            },
            # 2. Defensive [All Turns] interrupt trap (EX12-034 Erlangmon + EX12-071 Saneiketsu Invitation)
            {
                "id": "imperialdramon-vs-erlangmon-saneiketsu-trap",
                "title": "Play Around EX12-034 Erlangmon + EX12-071 Saneiketsu Invitation Trap",
                "description": "Avoid triggering Erlangmon's WhenWouldLeave + Saneiketsu Delay Lv.6 counter.",
                "tags": ["all-turns", "trap"],
                "seat": 1,
                "setup": {
                    "kind": "staged",
                    "state": {"memory": 3, "phase": "Main", "turn": 4, "first_player": 1},
                    "zones": {
                        "1": {
                            "hand": ["BT16-025"],
                            "field": [
                                {"stack": ["BT12-021", "BT12-022"], "is_suspended": False, "turn_played": 1},
                                {"stack": ["BT12-050"], "is_suspended": False, "turn_played": 1},
                            ],
                            "security": ["BT12-021", "BT12-021"],
                            "trash": [],
                        },
                        "2": {
                            "hand": ["EX12-034"],
                            "field": [
                                {"stack": ["EX12-034"], "is_suspended": False, "turn_played": 2},
                                {"stack": ["EX12-071"], "is_suspended": False, "turn_played": 2},
                            ],
                            "security": ["ST1-03"],
                            "trash": [],
                        },
                    },
                },
                "lines": [
                    {"id": "safe-dna", "name": "safe_dna_sequence", "role": "target", "actions": [63, 0, 1]},
                    {"id": "walk-into-trap", "name": "naive_removal_into_erlangmon", "role": "avoid", "actions": [100]},
                ],
                "refutation_triggers": [
                    {
                        "source_card_id": "EX12-034",
                        "phase": None,
                        "accept_optional": True,
                        "prefer_label_contains": "Play",
                    },
                    {
                        "source_card_id": "EX12-071",
                        "phase": None,
                        "accept_optional": True,
                        "prefer_label_contains": None,
                    },
                ],
                "rl_goal": {
                    "mode": "bounded_horizon",
                    "max_turns": 1,
                    "max_actions": 6,
                    "win_assertions": [
                        {"kind": "stack_top", "player": 1, "field_index": 0, "card_id": "BT16-025"}
                    ],
                },
            },
            # 3. Broken / unverified blueprint -> must be rejected!
            {
                "id": "broken-blueprint",
                "title": "Broken Illegal Action Blueprint",
                "description": "Contains illegal action 999",
                "seat": 1,
                "setup": {
                    "kind": "staged",
                    "state": {"memory": 1, "phase": "Main", "turn": 2, "first_player": 1},
                    "zones": {"1": {"hand": ["BT16-025"]}, "2": {"hand": []}},
                },
                "lines": [
                    {"id": "t", "name": "bad_target", "role": "target", "actions": [999]},
                ],
            },
        ]
        return LLMResponse(content="", parsed_json={"blueprints": blueprints})

    router = LLMRouter(
        scanner=ScriptedLLMProvider([]),
        author=ScriptedLLMProvider([_author_blueprints]),
    )

    with StdioMcpClient(engine_transport) as eng_mcp, StdioMcpClient(training_transport) as trn_mcp:
        report = synthesize_archetype(
            "Imperialdramon",
            router=router,
            engine_mcp=eng_mcp,
            training_mcp=trn_mcp,
            checkpoint_zip=ckpt_zip,
            deck_library_path=mini_library,
            puzzles_dir=puzzles_dir,
            search_exams_dir=exams_dir,
            require_admissible=False,
            validate_with_engine=True,
        )

    assert report.blueprints_proposed == 3
    assert report.puzzles_written == 2
    assert report.starved_embeddings is not None
    assert "BT16-025" in report.starved_embeddings.starved_card_ids

    # Broken blueprint rejected and NOT written
    assert not (puzzles_dir / "imperialdramon" / "broken-blueprint.json").exists()

    # Both verified puzzles written, valid studio-puzzle-v1, and loadable by load_studio_puzzle!
    p_lethal = puzzles_dir / "imperialdramon" / "imperialdramon-dna-paildramon-lethal.json"
    p_trap = puzzles_dir / "imperialdramon" / "imperialdramon-vs-erlangmon-saneiketsu-trap.json"
    assert p_lethal.exists()
    assert p_trap.exists()

    trap_doc = json.loads(p_trap.read_text(encoding="utf-8"))
    assert trap_doc["schema"] == PUZZLE_SCHEMA
    assert len(trap_doc["refutation_triggers"]) == 2
    assert trap_doc["refutation_triggers"][0]["source_card_id"] == "EX12-034"
    assert trap_doc["refutation_triggers"][1]["source_card_id"] == "EX12-071"
    validate_studio_puzzle_doc(trap_doc, validate_with_engine=True)

    # Exported search-exam carries refutation_triggers too
    trap_exam = json.loads((exams_dir / "imperialdramon-vs-erlangmon-saneiketsu-trap.json").read_text(encoding="utf-8"))
    assert trap_exam["schema"] == SEARCH_EXAM_SCHEMA
    assert len(trap_exam["refutation_triggers"]) == 2

    sp_lethal = load_studio_puzzle(p_lethal)
    sp_trap = load_studio_puzzle(p_trap)
    assert sp_lethal.id == "imperialdramon-dna-paildramon-lethal"
    assert sp_trap.rl_goal["mode"] == "bounded_horizon"


# ── 6. CLI Entrypoint Tests (`python -m tools.puzzle_lab`) ────────────────────


def test_cli_check_providers_and_closed_gate(monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]):
    monkeypatch.delenv(CLOSED_LLM_ENV_VAR, raising=False)

    # Default open-weight Ollama check succeeds with rc=0
    rc_open = cli_main(["check-providers"])
    assert rc_open == 0
    out_open = json.loads(capsys.readouterr().out)
    assert out_open["ok"] is True
    assert out_open["scanner"]["model"] == "qwen3:8b"
    assert out_open["author"]["model"] == "qwen3:14b"
    assert out_open["author"]["is_closed_weight"] is False

    # Closed Gemini without flag/env fails with rc=2
    rc_blocked = cli_main(["check-providers", "--provider", "gemini"])
    assert rc_blocked == 2
    err_blocked = capsys.readouterr().err
    assert "PermissionError" in err_blocked

    # Closed Gemini with --allow-closed-models succeeds with rc=0
    rc_allowed = cli_main(["check-providers", "--provider", "gemini", "--allow-closed-models"])
    assert rc_allowed == 0
    out_allowed = json.loads(capsys.readouterr().out)
    assert out_allowed["scanner"]["model"] == "gemini-4-hydrogen"
    assert out_allowed["author"]["model"] == "gemini-4-neon"
    assert out_allowed["author"]["is_closed_weight"] is True


def test_python_dash_m_tools_puzzle_lab_entrypoint():
    repo_root = resolve_repo_root()
    proc = subprocess.run(
        [sys.executable, "-m", "tools.puzzle_lab", "check-providers"],
        cwd=str(repo_root),
        env={"PYTHONPATH": str(repo_root / "code")},
        capture_output=True,
        text=True,
        check=False,
    )
    assert proc.returncode == 0, proc.stderr
    payload = json.loads(proc.stdout)
    assert payload["ok"] is True


# ── 7. Production Import-Isolation Test ───────────────────────────────────────


def test_production_surfaces_never_reference_puzzle_lab_or_closed_llm_gate():
    """AST-scan `code/server/`, `code/digimon_gym/`, and `code/src-tauri/` to assert
    zero references to `puzzle_lab` or `DIGIMON_ALLOW_CLOSED_LLM`."""
    repo_root = resolve_repo_root()
    forbidden_tokens = ("puzzle_lab", "DIGIMON_ALLOW_CLOSED_LLM")
    prod_dirs = [
        repo_root / "code" / "server",
        repo_root / "code" / "digimon_gym",
        repo_root / "code" / "src-tauri",
    ]

    violations: list[str] = []

    for prod_dir in prod_dirs:
        if not prod_dir.exists():
            continue

        # 1. AST-scan all Python files
        for py_file in sorted(prod_dir.rglob("*.py")):
            if "__pycache__" in py_file.parts:
                continue
            source = py_file.read_text(encoding="utf-8")
            tree = ast.parse(source, filename=str(py_file))
            for node in ast.walk(tree):
                if isinstance(node, ast.Import):
                    for alias in node.names:
                        if any(tok in alias.name for tok in forbidden_tokens):
                            violations.append(f"{py_file}:{node.lineno} import {alias.name}")
                elif isinstance(node, ast.ImportFrom):
                    mod = node.module or ""
                    if any(tok in mod for tok in forbidden_tokens):
                        violations.append(f"{py_file}:{node.lineno} from {mod} import ...")
                    for alias in node.names:
                        if any(tok in alias.name for tok in forbidden_tokens):
                            violations.append(f"{py_file}:{node.lineno} import {alias.name}")
                elif isinstance(node, ast.Name):
                    if any(tok in node.id for tok in forbidden_tokens):
                        violations.append(f"{py_file}:{node.lineno} name {node.id}")
                elif isinstance(node, ast.Attribute):
                    if any(tok in node.attr for tok in forbidden_tokens):
                        violations.append(f"{py_file}:{node.lineno} attr {node.attr}")
                elif isinstance(node, ast.Constant) and isinstance(node.value, str):
                    if any(tok in node.value for tok in forbidden_tokens):
                        violations.append(f"{py_file}:{node.lineno} string {node.value!r}")

        # 2. Also scan Rust/TS/config files in `code/src-tauri/` (excluding build `target/`)
        if prod_dir.name == "src-tauri":
            for src_file in sorted(prod_dir.rglob("*")):
                if not src_file.is_file() or "target" in src_file.parts:
                    continue
                if src_file.suffix in (".rs", ".ts", ".tsx", ".js", ".json", ".toml"):
                    text = src_file.read_text(encoding="utf-8", errors="ignore")
                    for tok in forbidden_tokens:
                        if tok in text:
                            violations.append(f"{src_file}: contains {tok!r}")

    assert violations == [], f"Production boundary violated:\n" + "\n".join(violations)


# ── 8. Real Engine Native Recording, Admissibility, Plasticity & Security ─────


def test_native_rust_headless_recording_snapshot_extraction_in_find_candidate_moments(tmp_path: Path):
    import digimon_engine as de

    hg = de.RustHeadlessGame(list(FILLER_DECK), list(FILLER_DECK), record_actions=True, seed=99)
    hg.skip_mulligan()
    hg.step(62)  # Breeding pass -> enters Main phase
    hg.step(62)  # Main pass -> hands turn to Player 2
    hg.step(62)  # Player 2 Breeding pass

    raw_rec = hg.get_recording()
    rec_obj = json.loads(raw_rec) if isinstance(raw_rec, str) else raw_rec
    rec_file = tmp_path / "native_game.json"
    rec_file.write_text(json.dumps(rec_obj), encoding="utf-8")

    # Attach a .winprob.json sidecar marking step index 1 (Main phase pass) as a -0.45 blunder
    winprob_file = tmp_path / "native_game.winprob.json"
    winprob_file.write_text(
        json.dumps(
            {
                "schema": "winprob-annotations-v1",
                "recording": str(rec_file),
                "decisions": [
                    {"step": 1, "turn": 1, "phase": "Breeding", "player_id": 1, "action_id": 62, "value": 0.20, "played_prior": 1.0},
                    {"step": 2, "turn": 1, "phase": "Main", "player_id": 1, "action_id": 62, "value": 0.25, "played_prior": 0.30},
                    {"step": 3, "turn": 2, "phase": "Breeding", "player_id": 2, "action_id": 62, "value": 0.25, "played_prior": 0.50},
                ],
            }
        ),
        encoding="utf-8",
    )

    moments = find_candidate_moments([rec_file], value_drop_threshold=-0.25)
    assert len(moments) == 1
    m = moments[0]
    assert m.kind == "value_drop"
    assert m.phase == "Main"
    assert m.value_delta == pytest.approx(-0.50)  # +0.25 -> -0.25 from P1's perspective
    assert m.scenario["state"]["phase"] == "Main"
    assert len(m.legal_actions) > 0
    assert any(int(a["action_id"]) == 62 for a in m.legal_actions)


def test_real_deck_library_and_card_text_catalog_metadata():
    from tools.puzzle_lab.synthesize_archetype import load_archetype_context

    ctx = load_archetype_context("Adventure", require_admissible=True)
    assert ctx.archetype_name == "Adventure"
    assert len(ctx.deck_cards) >= 50
    assert len(ctx.distinct_cards) >= 10

    catalog = CardTextCatalog()
    erlang = catalog.get("EX12-034")
    saneiketsu = catalog.get("EX12-071")
    paildramon = catalog.get("BT16-025")
    assert erlang is not None and erlang.effect_text
    assert saneiketsu is not None and saneiketsu.effect_text
    assert paildramon is not None
    assert paildramon.traits
    assert paildramon.evo_costs


def test_plasticity_probe_rejection_blocks_tensor_blind_puzzle(tmp_path: Path):
    puzzles_dir = tmp_path / "puzzles"
    ckpt_zip = tmp_path / "step_1000.zip"
    ckpt_zip.write_bytes(b"PK-placeholder")

    mini_lib = tmp_path / "deck_library.json"
    mini_lib.write_text(
        json.dumps(
            {
                "version": 1,
                "archetypes": {
                    "StarterRed": {
                        "archetype_name": "StarterRed",
                        "decklists": [{"deck_id": "sr-1", "source": "digilab", "decklist": json.dumps(FILLER_DECK)}],
                    }
                },
            }
        ),
        encoding="utf-8",
    )

    eng_transport = MockMcpTransport(server_name="digimon-engine-mcp")
    eng_transport.register_tool("load_puzzle", lambda args: {"ok": True, "game_id": "g1"})
    eng_transport.register_tool("score_line", lambda args: {"ok": True, "final_winner": 1})
    eng_transport.register_tool(
        "verify_puzzle",
        lambda args: {
            "ok": True,
            "target_results": [{"id": "t", "passed": True, "solved": True}],
            "avoid_results": [{"id": "a", "passed": True, "solved": False}],
            "expectation_results": [],
        },
    )

    trn_transport = MockMcpTransport(server_name="digimon-training-mcp")
    trn_transport.register_tool("card_embedding_neighbors", lambda args: {"ok": True, "cards": []})
    trn_transport.register_tool(
        "plasticity_probe",
        lambda args: {
            "ok": True,
            "learnable": False,
            "diagnosis": "zero input gradient (tensor-blind modifier)",
        },
    )

    blueprint = {
        "id": "tensor-blind-puzzle",
        "title": "Tensor Blind Puzzle",
        "description": "Should be blocked by require_plasticity=True",
        "seat": 1,
        "setup": {
            "kind": "staged",
            "state": {"memory": 2, "phase": "Main", "turn": 2, "first_player": 1},
            "zones": {"1": {"hand": ["ST1-03"]}, "2": {"hand": []}},
        },
        "lines": [
            {"id": "t", "name": "target", "role": "target", "actions": [0]},
            {"id": "a", "name": "avoid", "role": "avoid", "actions": [62]},
        ],
    }

    router = LLMRouter(
        scanner=ScriptedLLMProvider([]),
        author=ScriptedLLMProvider([LLMResponse(content="", parsed_json={"blueprints": [blueprint]})]),
    )

    with StdioMcpClient(eng_transport) as eng_mcp, StdioMcpClient(trn_transport) as trn_mcp:
        report = synthesize_archetype(
            "StarterRed",
            router=router,
            engine_mcp=eng_mcp,
            training_mcp=trn_mcp,
            checkpoint_zip=ckpt_zip,
            deck_library_path=mini_lib,
            puzzles_dir=puzzles_dir,
            require_admissible=False,
            require_plasticity=True,
        )

    assert report.puzzles_written == 0
    assert "tensor-blind" in report.outcomes[0].rejection_reason
    assert not (puzzles_dir / "starterred" / "tensor-blind-puzzle.json").exists()


def test_synthesize_archetype_refinement_loop_repairs_blueprint(tmp_path: Path):
    puzzles_dir = tmp_path / "puzzles"
    mini_lib = tmp_path / "deck_library.json"
    mini_lib.write_text(
        json.dumps(
            {
                "version": 1,
                "archetypes": {
                    "StarterRed": {
                        "archetype_name": "StarterRed",
                        "decklists": [{"deck_id": "sr-1", "source": "digilab", "decklist": json.dumps(FILLER_DECK)}],
                    }
                },
            }
        ),
        encoding="utf-8",
    )

    eng_transport = MockMcpTransport(server_name="digimon-engine-mcp")
    eng_transport.register_tool("load_puzzle", lambda args: {"ok": True, "game_id": "g-rep"})
    eng_transport.register_tool("score_line", lambda args: {"ok": True, "final_winner": 1})

    def _verify(args: dict[str, Any]) -> dict[str, Any]:
        pjson = args.get("puzzle_json") or {}
        target_actions = pjson["lines"][0]["actions"]
        if target_actions == [999]:
            return {"ok": True, "target_results": [{"id": "t", "passed": False, "error": "illegal 999"}], "avoid_results": []}
        return {"ok": True, "target_results": [{"id": "t", "passed": True, "solved": True}], "avoid_results": []}

    eng_transport.register_tool("verify_puzzle", _verify)

    initial_bp = {
        "id": "repaired-lethal",
        "title": "Repaired Lethal",
        "description": "Initially proposes action 999, repaired to 114 via tool loop",
        "seat": 1,
        "setup": {
            "kind": "staged",
            "state": {"memory": 1, "phase": "Main", "turn": 2, "first_player": 1},
            "zones": {
                "1": {"field": [{"stack": ["ST1-03"], "is_suspended": False, "turn_played": 1}], "hand": []},
                "2": {"field": [], "hand": [], "security": []},
            },
        },
        "lines": [{"id": "t", "name": "swing", "role": "target", "actions": [999]}],
    }

    router = LLMRouter(
        scanner=ScriptedLLMProvider([]),
        author=ScriptedLLMProvider(
            [
                LLMResponse(content="", parsed_json={"blueprints": [initial_bp]}),
                LLMResponse(
                    content="",
                    tool_calls=[
                        ToolCall(
                            id="c_fix",
                            name="save_puzzle",
                            arguments={
                                "id": "repaired-lethal",
                                "title": "Repaired Lethal",
                                "lines": [
                                    {"id": "t", "name": "swing", "role": "target", "actions": [114]},
                                    {"id": "a", "name": "pass", "role": "avoid", "actions": [62]},
                                ],
                            },
                        )
                    ],
                ),
            ]
        ),
    )

    with StdioMcpClient(eng_transport) as eng_mcp:
        report = synthesize_archetype(
            "StarterRed",
            router=router,
            engine_mcp=eng_mcp,
            deck_library_path=mini_lib,
            puzzles_dir=puzzles_dir,
            require_admissible=False,
            refinement_turns=2,
            validate_with_engine=True,
        )

    assert report.puzzles_written == 1
    assert report.outcomes[0].refined_turns == 1
    assert (puzzles_dir / "starterred" / "repaired-lethal.json").exists()


def test_schema_and_path_traversal_guards(tmp_path: Path):
    from tools.puzzle_lab import normalize_studio_puzzle_doc, write_studio_puzzle

    valid_doc = normalize_studio_puzzle_doc(
        {
            "id": "valid-slug",
            "title": "Valid",
            "lines": [{"id": "t", "name": "target", "role": "target", "actions": [62]}],
        }
    )
    validate_studio_puzzle_doc(valid_doc)

    # Path traversal in id or folder must raise PuzzleValidationError
    bad_id = dict(valid_doc, id="../escape")
    with pytest.raises(PuzzleValidationError):
        write_studio_puzzle(bad_id, tmp_path)

    bad_folder = dict(valid_doc, folder="../../outside")
    with pytest.raises(PuzzleValidationError):
        write_studio_puzzle(bad_folder, tmp_path)

    # Empty target lines must fail validation
    no_target = dict(valid_doc, lines=[{"id": "a", "name": "avoid", "role": "avoid", "actions": [62]}])
    with pytest.raises(PuzzleValidationError, match="target"):
        validate_studio_puzzle_doc(no_target)


# ── 9. Boss-Board Template × Challenger Counter-Matrix Tests ─────────────────


def test_boss_board_template_preset_and_knob_expansion():
    from tools.puzzle_lab.boss_matrix import load_boss_template, validate_challenger_zones_against_deck

    tmpl = load_boss_template("ex12-erlangmon-invitation")
    assert tmpl.id == "ex12-erlangmon-invitation"
    field_tops = [perm["stack"][-1] for perm in tmpl.defender_zones["field"]]
    assert field_tops == ["EX12-034", "TOKEN_KOTENKEN", "EX12-071"]
    assert "EX12-029" in tmpl.defender_zones["field"][0]["stack"]  # Lv.5 [SW] in Erlangmon sources
    assert {"EX12-029", "EX12-057"}.issubset(set(tmpl.defender_zones["hand"]))
    assert [r["source_card_id"] for r in tmpl.refutation_triggers] == ["EX12-034", "EX12-071"]

    z_sec1 = tmpl.materialize_defender_zones(1)
    z_sec2 = tmpl.materialize_defender_zones(2)
    assert len(z_sec1["security"]) == 1
    assert len(z_sec2["security"]) == 2
    assert "security" not in tmpl.defender_zones  # base template not mutated

    # Deck purity guard rejects out-of-deck cards & excess copies
    sample_deck = ["BT12-002"] * 4 + ["BT16-025"] * 2 + ["ST1-03"] * 48
    validate_challenger_zones_against_deck(
        {"hand": ["BT16-025"], "field": [{"stack": ["ST1-03", "BT16-025"]}]},
        sample_deck,
    )
    with pytest.raises(PuzzleValidationError, match="not in Challenger's decklist"):
        validate_challenger_zones_against_deck({"hand": ["EX13-045"]}, sample_deck)
    with pytest.raises(PuzzleValidationError, match="only contains 2x"):
        validate_challenger_zones_against_deck({"hand": ["BT16-025", "BT16-025", "BT16-025"]}, sample_deck)


def test_compose_boss_matrix_multi_archetype_retry_on_refutation_and_summary(tmp_path: Path):
    from digimon_gym.agents.studio_puzzles import load_studio_puzzles_dir
    from tools.puzzle_lab.boss_matrix import compose_boss_matrix
    from tools.puzzle_lab.cli import build_parser

    # Verify CLI parser parses compose-boss-matrix cleanly
    parsed = build_parser().parse_args(
        [
            "compose-boss-matrix",
            "--boss-template",
            "ex12-erlangmon-invitation",
            "--archetypes",
            "Imperialdramon,Examon",
            "--security-counts",
            "1,2",
            "--memories",
            "2,4",
            "--max-retries",
            "3",
        ]
    )
    assert parsed.subcommand == "compose-boss-matrix"
    assert parsed.boss_template == "ex12-erlangmon-invitation"
    assert parsed.security_counts == "1,2"
    assert parsed.memories == "2,4"

    puzzles_dir = tmp_path / "qa" / "puzzles"
    exams_dir = tmp_path / "qa" / "search-exams"
    summary_path = tmp_path / "qa" / "qa-reports" / "erlangmon-matrix.json"

    imperial_deck = (
        ["BT12-002"] * 4
        + ["BT12-021"] * 14
        + ["BT12-022"] * 12
        + ["BT12-050"] * 12
        + ["BT16-025"] * 12
    )
    examon_deck = (
        ["EX13-005"] * 4
        + ["EX13-008"] * 14
        + ["EX13-018"] * 12
        + ["EX13-021"] * 12
        + ["EX13-024"] * 12
    )
    snapshot_path = tmp_path / "train_snapshot.json"
    snapshot_path.write_text(
        json.dumps(
            {
                "schema_version": 1,
                "eligible_archetypes": ["Imperialdramon", "Examon"],
                "decks": [
                    {"archetype": "Imperialdramon", "deck_id": "imp-snap-1", "source": "digilab", "card_ids": imperial_deck},
                    {"archetype": "Examon", "deck_id": "exa-snap-1", "source": "digilab", "card_ids": examon_deck},
                ],
            }
        ),
        encoding="utf-8",
    )

    eng_transport = MockMcpTransport(server_name="digimon-engine-mcp")
    eng_transport.register_tool("load_model", lambda args: {"ok": True, "model_id": args["model_id"]})
    eng_transport.register_tool("load_puzzle", lambda args: {"ok": True, "game_id": "g-matrix", "seat": 1})
    eng_transport.register_tool("close_game", lambda args: {"ok": True})
    eng_transport.register_tool(
        "score_line",
        lambda args: {
            "ok": True,
            "root_mover": 1,
            "steps": [{"action_id": a, "auto_adversary": False} for a in args.get("actions", [])],
            "final_winner": None if 901 in (args.get("actions") or []) else 1,
        },
    )

    def _verify_matrix_puzzle(args: dict[str, Any]) -> dict[str, Any]:
        pjson = args["puzzle_json"]
        target_actions = pjson["lines"][0]["actions"]
        # Action 901 simulates reckless removal refuted by EX12-034 Erlangmon's WhenWouldLeave + bounce!
        if 901 in target_actions:
            return {
                "ok": True,
                "target_results": [
                    {
                        "id": pjson["lines"][0]["id"],
                        "passed": False,
                        "solved": False,
                        "error": "Refuted by EX12-034 WhenWouldLeave -> Play EX12-029 -> EX12-071 Delay -> EX12-057 bounce",
                    }
                ],
                "avoid_results": [{"id": "avoid", "passed": True, "solved": False, "refuted": True}],
                "expectation_results": [],
            }
        return {
            "ok": True,
            "target_results": [{"id": pjson["lines"][0]["id"], "passed": True, "solved": True, "final_winner": 1}],
            "avoid_results": [{"id": "avoid", "passed": True, "solved": False, "refuted": True}],
            "expectation_results": [{"passed": True}],
        }

    eng_transport.register_tool("verify_puzzle", _verify_matrix_puzzle)
    eng_transport.register_tool(
        "inspect_position",
        lambda args: {
            "ok": True,
            "chosen_action_id": 63 if args.get("model_id") == "champion-v2" else 100,
            "policy_value": 0.42 if args.get("model_id") == "champion-v2" else -0.31,
            "search_value": 0.68 if args.get("model_id") == "champion-v2" else -0.55,
            "expectation_results": [{"passed": args.get("model_id") == "champion-v2"}],
        },
    )

    # Dynamic Role.AUTHOR responder covering all 2 archetypes × 2 security_counts × 2 memories = 8 cells,
    # including a forced Attempt-1 Erlangmon refutation on `(Imperialdramon, sec=2, mem=2)` that recovers on Attempt 2!
    refutation_feedback_seen = False

    def _dynamic_matrix_author(messages: list[ChatMessage]) -> LLMResponse:
        nonlocal refutation_feedback_seen
        initial_prompt = json.loads(messages[1].content)
        arch = initial_prompt["challenger"]["archetype"]
        sec = int(initial_prompt["knobs"]["defender_security_count"])
        mem = int(initial_prompt["knobs"]["challenger_starting_memory"])
        is_retry = len(messages) > 2

        if arch == "Imperialdramon":
            zones = {
                "hand": ["BT16-025"],
                "field": [
                    {"stack": ["BT12-021", "BT12-022"], "is_suspended": False, "turn_played": 1},
                    {"stack": ["BT12-050"], "is_suspended": False, "turn_played": 1},
                ],
                "security": ["BT12-021", "BT12-021"],
                "trash": [],
            }
            # On (Imperialdramon, sec=2, mem=2) Attempt 1: propose reckless removal [901] that gets refuted by Erlangmon!
            if sec == 2 and mem == 2 and not is_retry:
                return LLMResponse(
                    content="",
                    parsed_json={
                        "answer_mode": "lethal_answer",
                        "title": "Reckless Attempt (Will Be Refuted by Erlangmon)",
                        "description": "Tries naive bounce into EX12-034 WhenWouldLeave.",
                        "challenger_zones": zones,
                        "lines": [
                            {"id": "reckless", "name": "reckless_line", "role": "target", "actions": [901]},
                            {"id": "avoid", "name": "swing_into_kotenken", "role": "avoid", "actions": [100]},
                        ],
                    },
                )
            if sec == 2 and mem == 2 and is_retry:
                last_feedback = json.loads(messages[-1].content)
                assert "EX12-034" in json.dumps(last_feedback["verify_puzzle_result"])
                refutation_feedback_seen = True
        else:
            zones = {
                "hand": ["EX13-024"],
                "field": [
                    {"stack": ["EX13-008", "EX13-018", "EX13-021"], "is_suspended": False, "turn_played": 1},
                ],
                "security": ["EX13-008", "EX13-008"],
                "trash": [],
            }

        if sec == 1:
            return LLMResponse(
                content="",
                parsed_json={
                    "answer_mode": "lethal_answer",
                    "title": f"{arch} Lethal Bypass vs Erlangmon (sec={sec}, mem={mem})",
                    "description": "Sequences around Kotenken Blocker and Erlangmon for clean lethal.",
                    "challenger_zones": zones,
                    "lines": [
                        {"id": "target-lethal", "name": "lethal_bypass", "role": "target", "actions": [63, 114]},
                        {"id": "avoid", "name": "trigger_erlangmon_trap", "role": "avoid", "actions": [100]},
                    ],
                    "rl_goal": {"mode": "playout_to_win", "max_turns": None, "max_actions": None, "win_assertions": []},
                },
            )
        return LLMResponse(
            content="",
            parsed_json={
                "answer_mode": "safe_setup_or_bypass",
                "title": f"{arch} Safe Setup vs Erlangmon at 2 Security (mem={mem})",
                "description": "At 2 security, reckless removal punts into Erlangmon + Saneiketsu Invitation; set up safely.",
                "challenger_zones": zones,
                "lines": [
                    {"id": "target-safe", "name": "safe_setup", "role": "target", "actions": [63]},
                    {"id": "avoid", "name": "reckless_removal_punt", "role": "avoid", "actions": [100]},
                ],
                "rl_goal": {
                    "mode": "bounded_horizon",
                    "max_turns": 1,
                    "max_actions": 4,
                    "win_assertions": [{"kind": "zone_count", "player": 1, "zone": "field", "value": 1}],
                },
            },
        )

    # Allow 9 AUTHOR calls (8 cells + 1 retry on Imperialdramon sec=2 mem=2)
    router = LLMRouter(
        scanner=ScriptedLLMProvider([]),
        author=ScriptedLLMProvider([_dynamic_matrix_author] * 9),
    )

    with StdioMcpClient(eng_transport) as eng_mcp:
        report = compose_boss_matrix(
            "ex12-erlangmon-invitation",
            router=router,
            engine_mcp=eng_mcp,
            archetypes=["Imperialdramon", "Examon"],
            training_pool_snapshot=snapshot_path,
            security_counts=[1, 2],
            memories=[2, 4],
            puzzles_dir=puzzles_dir,
            search_exams_dir=exams_dir,
            summary_out_path=summary_path,
            eval_models=["champion-v2.onnx", "legacy-v1.onnx"],
            max_retries=3,
            validate_with_engine=True,
        )

    assert refutation_feedback_seen is True
    assert report.cells_total == 8
    assert report.puzzles_written == 8
    assert summary_path.exists()

    summary_doc = json.loads(summary_path.read_text(encoding="utf-8"))
    assert summary_doc["schema"] == "boss-counter-matrix-v1"
    assert summary_doc["puzzles_written"] == 8

    # Check the retried cell specifically
    retried_cell = next(
        c for c in report.cells
        if c.challenger_archetype == "Imperialdramon" and c.security_count == 2 and c.memory == 2
    )
    assert retried_cell.accepted is True
    assert retried_cell.attempts_used == 2
    assert retried_cell.answer_mode == "safe_setup_or_bypass"
    assert len(retried_cell.model_evals) == 2
    assert retried_cell.model_evals[0].chose_target is True
    assert retried_cell.model_evals[1].chose_avoid is True

    # Confirm exact file naming convention & security_count/memory staging on disk
    p_sec1 = puzzles_dir / "boss-matrix" / "ex12-erlangmon-invitation" / "ex12-erlangmon-invitation-sec1-mem2-vs-imperialdramon.json"
    p_sec2 = puzzles_dir / "boss-matrix" / "ex12-erlangmon-invitation" / "ex12-erlangmon-invitation-sec2-mem2-vs-imperialdramon.json"
    assert p_sec1.exists()
    assert p_sec2.exists()

    doc_sec1 = json.loads(p_sec1.read_text(encoding="utf-8"))
    doc_sec2 = json.loads(p_sec2.read_text(encoding="utf-8"))
    assert len(doc_sec1["setup"]["zones"]["2"]["security"]) == 1
    assert len(doc_sec2["setup"]["zones"]["2"]["security"]) == 2
    assert doc_sec1["rl_goal"]["mode"] == "playout_to_win"
    assert doc_sec2["rl_goal"]["mode"] == "bounded_horizon"

    # Every generated puzzle in the matrix folder loads cleanly via load_studio_puzzles_dir!
    loaded_all = load_studio_puzzles_dir(puzzles_dir)
    assert len(loaded_all) == 8


