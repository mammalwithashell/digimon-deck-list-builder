"""MCP server: tool definitions + dispatch for the stage→capture→assert→emit
loop, over the browser (`/debug`) and desktop (Tauri bridge) transports.

Protocol via the official `mcp` SDK (`@server.list_tools()` /
`@server.call_tool()`), mirroring `digimon-training-mcp`.
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

from . import fixtures as _fx
from . import spec_gen as _spec
from .transport import BrowserClient, DesktopClient

logger = logging.getLogger("digimon_scenario_mcp")


# ─── Context ─────────────────────────────────────────────────────────


@dataclass
class ServerContext:
    scenarios_dir: Optional[Path]
    e2e_dir: Optional[Path]
    browser_base_url: str
    desktop_base_url: Optional[str]


def _client(ctx: ServerContext, target: str):
    if target == "browser":
        return BrowserClient(ctx.browser_base_url)
    if target == "desktop":
        return DesktopClient(ctx.desktop_base_url)
    raise ValueError(f"target must be 'browser' or 'desktop', got {target!r}")


def _extract_pending(ui: Any) -> Any:
    if isinstance(ui, dict):
        if "pendingSelection" in ui:
            return ui["pendingSelection"]
        st = ui.get("state")
        if isinstance(st, dict):
            return st.get("pendingSelection") or st.get("pending_selection")
    return None


# ─── Tool schemas ────────────────────────────────────────────────────

_TARGET = {
    "type": "string",
    "enum": ["browser", "desktop"],
    "description": "browser = hosted-API /debug (multi-game, game_id); desktop = Tauri bridge (single live game).",
}

TOOL_DEFINITIONS: list[mcp_types.Tool] = [
    mcp_types.Tool(
        name="stage_scenario",
        description="Stage a board (from an inline fixture or a saved fixture name) without playing a game. Returns game_id (browser) + rendered state.",
        inputSchema={
            "type": "object",
            "properties": {
                "target": _TARGET,
                "fixture": {"type": "object", "description": "Inline scenario fixture."},
                "fixture_name": {"type": "string", "description": "Name of a saved fixture under qa/scenarios/."},
            },
            "required": ["target"],
            "additionalProperties": False,
        },
    ),
    mcp_types.Tool(
        name="read_state",
        description="Full-information internal state of a staged game.",
        inputSchema={
            "type": "object",
            "properties": {"target": _TARGET, "game_id": {"type": "string"}},
            "required": ["target"],
            "additionalProperties": False,
        },
    ),
    mcp_types.Tool(
        name="get_mask",
        description="Current legal-action mask for the staged game.",
        inputSchema={
            "type": "object",
            "properties": {"target": _TARGET, "game_id": {"type": "string"}},
            "required": ["target"],
            "additionalProperties": False,
        },
    ),
    mcp_types.Tool(
        name="get_pending_selection",
        description="The currently-installed pending selection (or null), pulled from the UI state.",
        inputSchema={
            "type": "object",
            "properties": {"target": _TARGET, "game_id": {"type": "string"}},
            "required": ["target"],
            "additionalProperties": False,
        },
    ),
    mcp_types.Tool(
        name="step",
        description="Apply one legal action to drive the staged game to a decision point.",
        inputSchema={
            "type": "object",
            "properties": {
                "target": _TARGET,
                "action": {"type": "integer", "minimum": 0},
                "game_id": {"type": "string"},
            },
            "required": ["target", "action"],
            "additionalProperties": False,
        },
    ),
    mcp_types.Tool(
        name="set_memory",
        description="Set the memory gauge on the staged game.",
        inputSchema={
            "type": "object",
            "properties": {
                "target": _TARGET,
                "memory": {"type": "integer", "minimum": -10, "maximum": 10},
                "game_id": {"type": "string"},
            },
            "required": ["target", "memory"],
            "additionalProperties": False,
        },
    ),
    mcp_types.Tool(
        name="inject_card",
        description="Inject one card into a zone (hand | deck_top | security_top | trash).",
        inputSchema={
            "type": "object",
            "properties": {
                "target": _TARGET,
                "player_id": {"type": "integer", "minimum": 1, "maximum": 2},
                "card_id": {"type": "string"},
                "zone": {"type": "string"},
                "game_id": {"type": "string"},
            },
            "required": ["target", "player_id", "card_id", "zone"],
            "additionalProperties": False,
        },
    ),
    mcp_types.Tool(
        name="capture_snapshot",
        description="Capture the current board as a scenario fixture; optionally save it under qa/scenarios/ as `save_as`.",
        inputSchema={
            "type": "object",
            "properties": {
                "target": _TARGET,
                "game_id": {"type": "string"},
                "save_as": {"type": "string", "description": "If set, save the captured fixture under this name."},
            },
            "required": ["target"],
            "additionalProperties": False,
        },
    ),
    mcp_types.Tool(
        name="evaluate",
        description="Evaluate engine assertions against a staged game (browser target only).",
        inputSchema={
            "type": "object",
            "properties": {
                "target": _TARGET,
                "game_id": {"type": "string"},
                "assertions": {"type": "array", "items": {"type": "object"}},
            },
            "required": ["target", "assertions"],
            "additionalProperties": False,
        },
    ),
    mcp_types.Tool(
        name="list_fixtures",
        description="List saved scenario fixtures under qa/scenarios/.",
        inputSchema={"type": "object", "properties": {}, "additionalProperties": False},
    ),
    mcp_types.Tool(
        name="load_fixture",
        description="Load a saved scenario fixture by name.",
        inputSchema={
            "type": "object",
            "properties": {"name": {"type": "string"}},
            "required": ["name"],
            "additionalProperties": False,
        },
    ),
    mcp_types.Tool(
        name="save_fixture",
        description="Save a scenario fixture under qa/scenarios/.",
        inputSchema={
            "type": "object",
            "properties": {"name": {"type": "string"}, "fixture": {"type": "object"}},
            "required": ["name", "fixture"],
            "additionalProperties": False,
        },
    ),
    mcp_types.Tool(
        name="add_assertion",
        description="Append an assertion to a fixture's 'engine' or 'ui' bucket (pure; returns the edited fixture).",
        inputSchema={
            "type": "object",
            "properties": {
                "fixture": {"type": "object"},
                "assertion": {"type": "object"},
                "bucket": {"type": "string", "enum": ["engine", "ui"]},
            },
            "required": ["fixture", "assertion"],
            "additionalProperties": False,
        },
    ),
    mcp_types.Tool(
        name="emit_playwright_spec",
        description="Save a fixture + scaffold a matching Playwright .spec.ts under code/frontend/e2e/.",
        inputSchema={
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "fixture": {"type": "object", "description": "Inline fixture; if omitted, the saved fixture `name` is loaded."},
            },
            "required": ["name"],
            "additionalProperties": False,
        },
    ),
]


# ─── Handlers ────────────────────────────────────────────────────────

ToolHandler = Callable[[ServerContext, Dict[str, Any]], Awaitable[Dict[str, Any]]]


def _resolve_fixture(ctx: ServerContext, args: Dict[str, Any]) -> Any:
    if isinstance(args.get("fixture"), dict):
        return args["fixture"]
    name = args.get("fixture_name")
    if isinstance(name, str) and name:
        loaded = _fx.load_fixture(ctx.scenarios_dir, name)
        if not loaded.get("ok"):
            return loaded
        return loaded["fixture"]
    return {"ok": False, "error": "provide 'fixture' (inline) or 'fixture_name'"}


async def _h_stage(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    fixture = _resolve_fixture(ctx, args)
    if isinstance(fixture, dict) and fixture.get("ok") is False:
        return fixture
    client = _client(ctx, args["target"])
    try:
        result = client.stage(fixture)
    finally:
        client.close()
    return {"ok": True, "target": args["target"], "game_id": result.get("game_id"), "result": result}


async def _h_read_state(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    client = _client(ctx, args["target"])
    try:
        return {"ok": True, "state": client.read_state(args.get("game_id"))}
    finally:
        client.close()


async def _h_get_mask(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    client = _client(ctx, args["target"])
    try:
        return {"ok": True, **client.get_mask(args.get("game_id"))}
    finally:
        client.close()


async def _h_get_pending(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    client = _client(ctx, args["target"])
    try:
        ui = client.ui_state(args.get("game_id"))
    finally:
        client.close()
    return {"ok": True, "pending": _extract_pending(ui)}


async def _h_step(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    client = _client(ctx, args["target"])
    try:
        return {"ok": True, "result": client.step(args.get("game_id"), int(args["action"]))
                if args["target"] == "browser" else client.step(int(args["action"]))}
    finally:
        client.close()


async def _h_set_memory(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    client = _client(ctx, args["target"])
    try:
        return {"ok": True, "result": client.set_memory(args.get("game_id"), int(args["memory"]))
                if args["target"] == "browser" else client.set_memory(int(args["memory"]))}
    finally:
        client.close()


async def _h_inject_card(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    client = _client(ctx, args["target"])
    pid, cid, zone = int(args["player_id"]), str(args["card_id"]), str(args["zone"])
    try:
        result = (
            client.inject_card(args.get("game_id"), pid, cid, zone)
            if args["target"] == "browser"
            else client.inject_card(pid, cid, zone)
        )
        return {"ok": True, "result": result}
    finally:
        client.close()


async def _h_capture(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    client = _client(ctx, args["target"])
    try:
        fixture = client.capture(args.get("game_id"))
    finally:
        client.close()
    out: Dict[str, Any] = {"ok": True, "fixture": fixture}
    save_as = args.get("save_as")
    if isinstance(save_as, str) and save_as:
        out["saved"] = _fx.save_fixture(ctx.scenarios_dir, save_as, fixture)
    return out


async def _h_evaluate(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    if args["target"] != "browser":
        return {"ok": False, "error": "evaluate is browser-only (the desktop bridge has no evaluator)"}
    client = _client(ctx, "browser")
    try:
        return {"ok": True, **client.evaluate(args.get("game_id"), args["assertions"])}
    finally:
        client.close()


async def _h_list_fixtures(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    return _fx.list_fixtures(ctx.scenarios_dir)


async def _h_load_fixture(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    return _fx.load_fixture(ctx.scenarios_dir, str(args["name"]))


async def _h_save_fixture(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    return _fx.save_fixture(ctx.scenarios_dir, str(args["name"]), args["fixture"])


async def _h_add_assertion(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    return _fx.add_assertion(args["fixture"], args["assertion"], args.get("bucket", "engine"))


async def _h_emit_spec(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    name = str(args["name"])
    fixture = args.get("fixture")
    if not isinstance(fixture, dict):
        loaded = _fx.load_fixture(ctx.scenarios_dir, name)
        if not loaded.get("ok"):
            return loaded
        fixture = loaded["fixture"]
    return _spec.generate_spec(fixture, name, ctx.scenarios_dir, ctx.e2e_dir)


TOOL_HANDLERS: Dict[str, ToolHandler] = {
    "stage_scenario": _h_stage,
    "read_state": _h_read_state,
    "get_mask": _h_get_mask,
    "get_pending_selection": _h_get_pending,
    "step": _h_step,
    "set_memory": _h_set_memory,
    "inject_card": _h_inject_card,
    "capture_snapshot": _h_capture,
    "evaluate": _h_evaluate,
    "list_fixtures": _h_list_fixtures,
    "load_fixture": _h_load_fixture,
    "save_fixture": _h_save_fixture,
    "add_assertion": _h_add_assertion,
    "emit_playwright_spec": _h_emit_spec,
}


# ─── Assembly ────────────────────────────────────────────────────────


def build_server(ctx: ServerContext) -> Server:
    server: Server = Server("digimon-scenario-mcp")

    @server.list_tools()
    async def _list_tools() -> list[mcp_types.Tool]:
        return TOOL_DEFINITIONS

    @server.call_tool()
    async def _call_tool(name: str, arguments: Dict[str, Any]) -> list[mcp_types.TextContent]:
        handler = TOOL_HANDLERS.get(name)
        if handler is None:
            payload: Dict[str, Any] = {"ok": False, "error": f"unknown tool '{name}'"}
        else:
            try:
                payload = await handler(ctx, arguments or {})
            except Exception as exc:  # noqa: BLE001 - surface as structured error
                logger.exception("tool '%s' raised", name)
                payload = {"ok": False, "error": f"{type(exc).__name__}: {exc}"}
        return [mcp_types.TextContent(type="text", text=json.dumps(payload, separators=(",", ":")))]

    return server


async def run_stdio(ctx: ServerContext) -> None:
    server = build_server(ctx)
    async with stdio_server() as (read_stream, write_stream):
        await server.run(read_stream, write_stream, server.create_initialization_options())
