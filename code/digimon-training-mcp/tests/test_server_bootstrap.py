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


def test_tool_definitions_has_seven_entries():
    names = [tool.name for tool in TOOL_DEFINITIONS]
    assert sorted(names) == sorted([
        "list_runs",
        "run_summary",
        "run_metric",
        "run_tags",
        "run_recordings",
        "run_checkpoints",
        "run_deck_pool",
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
async def test_each_tool_returns_not_implemented_placeholder(ctx: ServerContext):
    """Confirm every registered tool round-trips end-to-end through the handler
    layer and returns the structured placeholder. Phase 5 swaps the handlers
    for real implementations; this test guards the wiring."""
    for tool_name, handler in TOOL_HANDLERS.items():
        result = await handler(ctx, {})
        assert isinstance(result, dict), f"{tool_name} returned {type(result).__name__}"
        assert result.get("ok") is False
        assert "not implemented" in result.get("error", "").lower()


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
