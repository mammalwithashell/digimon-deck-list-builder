"""Server-bootstrap tests.

These exercise the MCP protocol surface in-process without spawning a subprocess
— faster than integration tests, and sufficient for Phase 4's "skeleton works"
contract: tools/list returns the seven tools with their schemas, and tools/call
on each one returns a structured 'not implemented yet' placeholder.

Phase 7 adds proper subprocess-based integration tests that round-trip over
stdio.
"""

from __future__ import annotations

import json
from typing import Any, Dict

import pytest
from mcp.server.lowlevel.server import Server

from digimon_training_mcp.server import (
    TOOL_DEFINITIONS,
    TOOL_HANDLERS,
    ServerContext,
    build_server,
)


@pytest.fixture
def ctx() -> ServerContext:
    return ServerContext(runs_dir=None, models_dir=None, repo_root=None)


def test_tool_definitions_has_expected_entries():
    names = [tool.name for tool in TOOL_DEFINITIONS]
    assert sorted(names) == sorted([
        "list_runs",
        "run_summary",
        "run_metric",
        "run_tags",
        "run_recordings",
        "run_checkpoints",
        "run_deck_pool",
        "run_per_game_evals",
        "run_anchored_evals",
        "champion_standings",
        "run_elo_ladder",
        "run_exploitability",
    ])


def test_every_tool_has_a_handler():
    for tool in TOOL_DEFINITIONS:
        assert tool.name in TOOL_HANDLERS, f"tool '{tool.name}' has no handler entry"


def test_tool_schemas_are_well_formed():
    for tool in TOOL_DEFINITIONS:
        schema = tool.inputSchema
        assert isinstance(schema, dict)
        assert schema.get("type") == "object"
        assert "properties" in schema
        assert schema.get("additionalProperties") is False, (
            f"tool '{tool.name}' must set additionalProperties=False to reject typos"
        )


def test_build_server_returns_mcp_server(ctx: ServerContext):
    server = build_server(ctx)
    assert isinstance(server, Server)
    assert server.name == "digimon-training-mcp"


@pytest.mark.asyncio
async def test_each_tool_handler_returns_structured_dict(ctx: ServerContext):
    """Every registered tool returns a dict with an explicit ok flag.

    With a ServerContext pointing nowhere, each handler should error gracefully
    rather than raise — confirms the "no run_dir / models_dir found" error path
    is wired in every handler.
    """
    args_by_tool: Dict[str, Dict[str, Any]] = {
        "list_runs": {},
        "run_summary": {"name": "missing"},
        "run_metric": {"name": "missing", "tag": "pilot/win_rate"},
        "run_tags": {"name": "missing"},
        "run_recordings": {"name": "missing"},
        "run_checkpoints": {"name": "missing"},
        "run_deck_pool": {"name": "missing"},
        "run_per_game_evals": {"name": "missing"},
        "run_anchored_evals": {"name": "missing"},
        "champion_standings": {},
        "run_elo_ladder": {"name": "missing"},
        "run_exploitability": {"name": "missing"},
    }
    # run_per_game_evals returns ok=True with empty rows for missing runs
    # (consistent with the spec's "empty list, not error" scenario).
    ok_true_tools = {"run_per_game_evals"}
    for tool_name, handler in TOOL_HANDLERS.items():
        result = await handler(ctx, args_by_tool[tool_name])
        assert isinstance(result, dict), f"{tool_name} returned {type(result).__name__}"
        assert "ok" in result, f"{tool_name} missing 'ok' key: {result}"
        if tool_name in ok_true_tools:
            assert result["ok"] is True, f"{tool_name} expected ok=True for missing run"
        else:
            assert result["ok"] is False
            assert "error" in result and result["error"]


def test_run_metric_schema_accepts_string_or_array_tag():
    metric_tool = next(t for t in TOOL_DEFINITIONS if t.name == "run_metric")
    tag_schema = metric_tool.inputSchema["properties"]["tag"]
    assert "oneOf" in tag_schema
    variants = tag_schema["oneOf"]
    assert any(v.get("type") == "string" for v in variants)
    assert any(v.get("type") == "array" for v in variants)


def test_run_recordings_filter_enum():
    rec_tool = next(t for t in TOOL_DEFINITIONS if t.name == "run_recordings")
    filter_schema = rec_tool.inputSchema["properties"]["filter"]
    assert set(filter_schema["enum"]) == {"crash", "draw", "all"}
