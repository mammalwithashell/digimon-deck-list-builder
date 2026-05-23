"""MCP server bootstrap and tool registration.

Phase 4 skeleton: the seven tools are declared with their JSON schemas so
``tools/list`` round-trips end-to-end, but the handlers return a structured
``{"ok": false, "error": "not implemented yet"}`` placeholder. Phase 5 wires
in the real handlers from ``runs``, ``summary``, ``tb_metrics``, ``recordings``,
``checkpoints``, ``deck_pool`` modules.

Protocol implementation uses the official ``mcp`` Python SDK (decorator-driven
``@server.list_tools()`` / ``@server.call_tool()``) — see design.md §Decision 6
for the trade-off vs hand-rolled JSON-RPC.
"""

from __future__ import annotations

import json
import logging
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Awaitable, Callable, Dict, Optional

import mcp.types as mcp_types
from mcp.server import Server
from mcp.server.stdio import stdio_server


logger = logging.getLogger("digimon_training_mcp")


# ─── Server context ──────────────────────────────────────────────────


@dataclass
class ServerContext:
    """Resolved paths the tool handlers operate on. Constructed at startup
    from CLI flags + ancestor-walk fallback; passed into each tool handler."""

    runs_dir: Optional[Path]
    models_dir: Optional[Path]
    repo_root: Optional[Path]


# ─── Tool schemas (declared once; reused by tools/list) ──────────────


_NAME_SCHEMA = {
    "type": "object",
    "properties": {"name": {"type": "string", "description": "Run name (directory under --runs-dir)."}},
    "required": ["name"],
    "additionalProperties": False,
}


TOOL_DEFINITIONS: list[mcp_types.Tool] = [
    mcp_types.Tool(
        name="list_runs",
        description="List all training runs under --runs-dir with active-status + latest step/win-rate.",
        inputSchema={"type": "object", "properties": {}, "additionalProperties": False},
    ),
    mcp_types.Tool(
        name="run_summary",
        description=(
            "Return header block, recent eval rows, panic counts by family, and "
            "the last ~50 lines of console.log for a run."
        ),
        inputSchema={
            "type": "object",
            "properties": {
                "name": {"type": "string", "description": "Run name (directory under --runs-dir)."},
                "tail_evals": {"type": "integer", "minimum": 1, "default": 10},
            },
            "required": ["name"],
            "additionalProperties": False,
        },
    ),
    mcp_types.Tool(
        name="run_metric",
        description=(
            "Return a TensorBoard scalar time-series for a run. `tag` may be a string "
            "(returns a list of {step, wall_time, value}) or an array (returns a dict "
            "keyed by tag). `since_step` filters server-side."
        ),
        inputSchema={
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "tag": {
                    "oneOf": [
                        {"type": "string"},
                        {"type": "array", "items": {"type": "string"}, "minItems": 1},
                    ]
                },
                "since_step": {"type": "integer", "minimum": 0},
            },
            "required": ["name", "tag"],
            "additionalProperties": False,
        },
    ),
    mcp_types.Tool(
        name="run_tags",
        description="List all scalar TensorBoard tags present in the run's event files.",
        inputSchema=_NAME_SCHEMA,
    ),
    mcp_types.Tool(
        name="run_recordings",
        description=(
            "Inventory recordings for a run. `filter` is one of 'crash' (reason=crash), "
            "'draw' (result=draw and reason!=crash), or 'all'. `limit` truncates after the filter. "
            "Each entry's `path` is consumable by digimon-engine-mcp's load_recording."
        ),
        inputSchema={
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "filter": {"type": "string", "enum": ["crash", "draw", "all"], "default": "all"},
                "limit": {"type": "integer", "minimum": 1},
            },
            "required": ["name"],
            "additionalProperties": False,
        },
    ),
    mcp_types.Tool(
        name="run_checkpoints",
        description="Inventory model checkpoints (step_NNNNNNNNN.zip files) for a run.",
        inputSchema=_NAME_SCHEMA,
    ),
    mcp_types.Tool(
        name="run_deck_pool",
        description="Return the parsed contents of deck_pool_snapshot.json for a run.",
        inputSchema=_NAME_SCHEMA,
    ),
]


# ─── Tool dispatch ───────────────────────────────────────────────────


ToolHandler = Callable[[ServerContext, Dict[str, Any]], Awaitable[Dict[str, Any]]]


async def _not_implemented(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    return {"ok": False, "error": "tool not implemented yet — Phase 5 wires the handler"}


TOOL_HANDLERS: Dict[str, ToolHandler] = {
    tool.name: _not_implemented for tool in TOOL_DEFINITIONS
}


# ─── Server assembly ─────────────────────────────────────────────────


def build_server(ctx: ServerContext) -> Server:
    """Construct an MCP ``Server`` instance with the seven tools registered."""
    server: Server = Server("digimon-training-mcp")

    @server.list_tools()
    async def _list_tools() -> list[mcp_types.Tool]:
        return TOOL_DEFINITIONS

    @server.call_tool()
    async def _call_tool(name: str, arguments: Dict[str, Any]) -> list[mcp_types.TextContent]:
        handler = TOOL_HANDLERS.get(name)
        if handler is None:
            payload = {"ok": False, "error": f"unknown tool '{name}'"}
        else:
            try:
                payload = await handler(ctx, arguments or {})
            except Exception as exc:
                logger.exception("tool '%s' raised", name)
                payload = {"ok": False, "error": f"{type(exc).__name__}: {exc}"}
        return [mcp_types.TextContent(type="text", text=json.dumps(payload, separators=(",", ":")))]

    return server


async def run_stdio(ctx: ServerContext) -> None:
    """Bootstrap the MCP server over stdio. Blocks until stdin closes."""
    server = build_server(ctx)
    async with stdio_server() as (read_stream, write_stream):
        await server.run(read_stream, write_stream, server.create_initialization_options())
