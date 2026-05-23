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

from .checkpoints import run_checkpoints as _run_checkpoints
from .deck_pool import run_deck_pool as _run_deck_pool
from .panic_families import load_panic_families
from .recordings import run_recordings as _run_recordings
from .runs import list_runs as _list_runs
from .summary import run_summary as _run_summary
from .tb_metrics import run_metric as _run_metric, run_tags as _run_tags


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


def _require_runs_dir(ctx: ServerContext) -> Optional[Dict[str, Any]]:
    if ctx.runs_dir is None:
        return {"ok": False, "error": "runs_dir not resolved — pass --runs-dir or run from a directory whose ancestor contains ./runs"}
    return None


def _resolve_run_dir(ctx: ServerContext, name: str) -> Optional[Path]:
    if ctx.runs_dir is None:
        return None
    candidate = ctx.runs_dir / name
    return candidate if candidate.is_dir() else None


async def _h_list_runs(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    return _list_runs(ctx.runs_dir)


async def _h_run_summary(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    name = args.get("name")
    if not isinstance(name, str) or not name:
        return {"ok": False, "error": "missing or invalid 'name' argument"}
    run_dir = _resolve_run_dir(ctx, name)
    if run_dir is None:
        return {"ok": False, "error": f"run '{name}' not found under runs_dir"}
    tail_evals = int(args.get("tail_evals", 10))
    families = load_panic_families(ctx.repo_root)
    return _run_summary(run_dir, tail_evals, families)


async def _h_run_metric(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    name = args.get("name")
    tag = args.get("tag")
    since_step = args.get("since_step")
    if not isinstance(name, str) or not name:
        return {"ok": False, "error": "missing or invalid 'name' argument"}
    if tag is None or (not isinstance(tag, (str, list))):
        return {"ok": False, "error": "missing or invalid 'tag' argument (string or array)"}
    run_dir = _resolve_run_dir(ctx, name)
    if run_dir is None:
        return {"ok": False, "error": f"run '{name}' not found under runs_dir"}
    since = int(since_step) if since_step is not None else None
    return _run_metric(run_dir, tag, since)


async def _h_run_tags(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    name = args.get("name")
    if not isinstance(name, str) or not name:
        return {"ok": False, "error": "missing or invalid 'name' argument"}
    run_dir = _resolve_run_dir(ctx, name)
    if run_dir is None:
        return {"ok": False, "error": f"run '{name}' not found under runs_dir"}
    return _run_tags(run_dir)


async def _h_run_recordings(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    name = args.get("name")
    if not isinstance(name, str) or not name:
        return {"ok": False, "error": "missing or invalid 'name' argument"}
    filter_mode = str(args.get("filter", "all"))
    limit = args.get("limit")
    limit_int = int(limit) if limit is not None else None
    return _run_recordings(ctx.models_dir, name, filter_mode, limit_int)


async def _h_run_checkpoints(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    name = args.get("name")
    if not isinstance(name, str) or not name:
        return {"ok": False, "error": "missing or invalid 'name' argument"}
    return _run_checkpoints(ctx.models_dir, name)


async def _h_run_deck_pool(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    name = args.get("name")
    if not isinstance(name, str) or not name:
        return {"ok": False, "error": "missing or invalid 'name' argument"}
    return _run_deck_pool(ctx.models_dir, name)


TOOL_HANDLERS: Dict[str, ToolHandler] = {
    "list_runs": _h_list_runs,
    "run_summary": _h_run_summary,
    "run_metric": _h_run_metric,
    "run_tags": _h_run_tags,
    "run_recordings": _h_run_recordings,
    "run_checkpoints": _h_run_checkpoints,
    "run_deck_pool": _h_run_deck_pool,
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
