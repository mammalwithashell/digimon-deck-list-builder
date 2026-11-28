"""MCP server bootstrap and tool registration.

Phase 4 skeleton: the tools are declared with their JSON schemas so
``tools/list`` round-trips end-to-end, but the handlers return a structured
``{"ok": false, "error": "not implemented yet"}`` placeholder. Phase 5 wires
in the real handlers from ``runs``, ``summary``, ``tb_metrics``, ``recordings``,
``checkpoints``, ``deck_pool``, ``per_game`` modules.

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

from .anchored import run_anchored_evals as _run_anchored_evals
from .checkpoints import run_checkpoints as _run_checkpoints
from .deck_pool import run_deck_pool as _run_deck_pool
from .eval_artifacts import (
    champion_standings as _champion_standings,
    run_elo_ladder as _run_elo_ladder,
    run_exploitability as _run_exploitability,
)
from .panic_families import load_panic_families
from .paths import resolve_model_run_dir
from .per_game import run_per_game_evals as _run_per_game_evals
from .recordings import run_recordings as _run_recordings
from .runs import list_runs as _list_runs
from .summary import run_summary as _run_summary
from .tb_metrics import run_metric as _run_metric, run_tags as _run_tags
from .whitebox import (
    card_embedding_neighbors as _card_embedding_neighbors,
    export_checkpoint_onnx as _export_checkpoint_onnx,
    plasticity_probe as _plasticity_probe,
    position_saliency as _position_saliency,
)


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
    mcp_types.Tool(
        name="run_per_game_evals",
        description=(
            "Per-game rows from eval_game_log.jsonl with optional filter / limit. "
            "Surfaces individual game outcomes (digivolve counts, archetypes, "
            "result, episode_length, recording_path) that the per-window means "
            "in evals.jsonl hide. Sorted by (step, eval_window_idx, game_idx)."
        ),
        inputSchema={
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "filter": {
                    "type": "object",
                    "properties": {
                        "agent_archetype": {"type": "string"},
                        "opponent_archetype": {"type": "string"},
                        "result": {"type": "string", "enum": ["win", "loss", "draw"]},
                        "digivolves_agent_min": {"type": "integer", "minimum": 0},
                        "dna_digivolves_agent_min": {"type": "integer", "minimum": 0},
                        "step_min": {"type": "integer", "minimum": 0},
                        "step_max": {"type": "integer", "minimum": 0},
                    },
                    "additionalProperties": False,
                },
                "limit": {"type": "integer", "minimum": 1},
            },
            "required": ["name"],
            "additionalProperties": False,
        },
    ),
    mcp_types.Tool(
        name="run_anchored_evals",
        description=(
            "Return in-training anchored panel rows (anchored_evals.jsonl) for a run, "
            "most recent first — per-anchor seat-balanced win rates vs greedy + frozen "
            "champions. The trustworthy in-run progress signal (CLAUDE.md rule 30); "
            "written by AnchoredEvalCallback when anchored_eval_freq > 0."
        ),
        inputSchema={
            "type": "object",
            "properties": {
                "name": {"type": "string", "description": "Run name (directory under --runs-dir)."},
                "limit": {"type": "integer", "minimum": 1},
            },
            "required": ["name"],
            "additionalProperties": False,
        },
    ),
    mcp_types.Tool(
        name="champion_standings",
        description=(
            "List the registered evaluation champions (frozen model snapshots used "
            "as fixed anchors) from models/champions/registry.json — name, provenance, "
            "observation profile, tensor-layout hash."
        ),
        inputSchema={"type": "object", "properties": {}, "additionalProperties": False},
    ),
    mcp_types.Tool(
        name="run_elo_ladder",
        description=(
            "Return the persisted Elo ladder for a run (<models>/<name>/elo_ladder.json): "
            "greedy-anchored ratings + standard errors, the pairwise matchup matrix, and "
            "any forgetting/cycling upsets. Produced by code/tools/elo_ladder_cli.py."
        ),
        inputSchema=_NAME_SCHEMA,
    ),
    mcp_types.Tool(
        name="run_exploitability",
        description=(
            "Return the persisted approximate-exploitability result for a run "
            "(<models>/<name>/exploitability.json) — the exploiter's peak win rate vs the "
            "frozen target, labeled as a budget-bound LOWER bound, not a robustness certificate."
        ),
        inputSchema=_NAME_SCHEMA,
    ),
    mcp_types.Tool(
        name="card_embedding_neighbors",
        description=(
            "Read-only white-box inspection of a checkpoint's CardEmbeddingExtractor "
            "embedding table: L2 norms, top-k nearest neighbor cards by cosine similarity "
            "(enriched with card_name, level, colors, traits), and pairwise cosines."
        ),
        inputSchema={
            "type": "object",
            "properties": {
                "checkpoint_path": {
                    "type": "string",
                    "description": "Path to SB3 .zip checkpoint (absolute, or relative to models_dir / repo_root).",
                },
                "card_ids": {
                    "type": "array",
                    "items": {"type": "string"},
                    "minItems": 1,
                    "description": "Card IDs to inspect (e.g. ['BT16-025', 'BT12-022']).",
                },
                "top_k": {
                    "type": "integer",
                    "minimum": 1,
                    "default": 10,
                    "description": "Number of nearest neighbors to return per card.",
                },
            },
            "required": ["checkpoint_path", "card_ids"],
            "additionalProperties": False,
        },
    ),
    mcp_types.Tool(
        name="position_saliency",
        description=(
            "Compute exact PyTorch gradient saliency (|grad * obs| and |grad|) on a "
            "puzzle or staged scenario position for an MLP or LSTM .zip checkpoint, "
            "mapped onto observation layout sections and card-ID slots."
        ),
        inputSchema={
            "type": "object",
            "properties": {
                "checkpoint_path": {
                    "type": "string",
                    "description": "Path to SB3 .zip checkpoint (absolute, or relative to models_dir / repo_root).",
                },
                "puzzle_path": {
                    "type": "string",
                    "description": "Optional path to a studio-puzzle-v1 or search-exam-v1 JSON file.",
                },
                "scenario_json": {
                    "oneOf": [
                        {"type": "string"},
                        {"type": "object"},
                    ],
                    "description": "Optional inline puzzle/scenario JSON string or object.",
                },
                "target_action_id": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Optional target action ID for target_logit or logit_margin objective.",
                },
                "avoid_action_id": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Optional rival action ID contrasted against target_action_id (logit_margin).",
                },
                "top_k": {
                    "type": "integer",
                    "minimum": 1,
                    "default": 15,
                    "description": "Number of highest-saliency individual features to return.",
                },
            },
            "required": ["checkpoint_path"],
            "additionalProperties": False,
        },
    ),
    mcp_types.Tool(
        name="plasticity_probe",
        description=(
            "In-memory read-only plasticity / tensor-blindness probe: loads a fresh CPU copy "
            "of the .zip checkpoint, runs N Adam steps toward target_action_id (+1.0 value "
            "and optional margin hinge vs avoid_action_ids), reports pre/post metrics, "
            "learnable flag, and diagnosis, then discards the copy without touching disk."
        ),
        inputSchema={
            "type": "object",
            "properties": {
                "checkpoint_path": {
                    "type": "string",
                    "description": "Path to SB3 .zip checkpoint (absolute, or relative to models_dir / repo_root).",
                },
                "puzzle_path": {
                    "type": "string",
                    "description": "Optional path to a studio-puzzle-v1 or search-exam-v1 JSON file.",
                },
                "scenario_json": {
                    "oneOf": [
                        {"type": "string"},
                        {"type": "object"},
                    ],
                    "description": "Optional inline puzzle/scenario JSON string or object.",
                },
                "target_action_id": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Legal target action ID to optimize toward.",
                },
                "avoid_action_ids": {
                    "type": "array",
                    "items": {"type": "integer", "minimum": 0},
                    "description": "Optional rival action IDs for margin hinge penalty.",
                },
                "steps": {
                    "type": "integer",
                    "minimum": 1,
                    "default": 10,
                    "description": "Number of in-memory Adam update steps.",
                },
                "lr": {
                    "type": "number",
                    "exclusiveMinimum": 0,
                    "default": 0.0001,
                    "description": "Adam learning rate for the probe.",
                },
            },
            "required": ["checkpoint_path", "target_action_id"],
            "additionalProperties": False,
        },
    ),
    mcp_types.Tool(
        name="export_checkpoint_onnx",
        description=(
            "Export an MLP or LSTM SB3 .zip checkpoint into a sha256-keyed ONNX cache "
            "directory (never writing next to checkpoint_path), returning onnx_path, "
            "value_onnx_path (if LSTM), meta_path, model_type, and sha256."
        ),
        inputSchema={
            "type": "object",
            "properties": {
                "checkpoint_path": {
                    "type": "string",
                    "description": "Path to SB3 .zip checkpoint (absolute, or relative to models_dir / repo_root).",
                },
                "cache_dir": {
                    "type": "string",
                    "description": "Optional cache root directory (defaults to <tempdir>/digimon_onnx_cache/<sha256>).",
                },
            },
            "required": ["checkpoint_path"],
            "additionalProperties": False,
        },
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


def _resolve_checkpoint_arg(ctx: ServerContext, raw_path: str) -> Path:
    p = Path(raw_path).expanduser()
    if p.is_absolute():
        return p
    if ctx.models_dir is not None:
        cand = ctx.models_dir / p
        if cand.is_file():
            return cand
        if len(p.parts) >= 2:
            resolved_run = resolve_model_run_dir(ctx.models_dir, p.parts[0])
            if resolved_run is not None:
                nested = resolved_run[0] / Path(*p.parts[1:])
                if nested.is_file():
                    return nested
    if ctx.repo_root is not None:
        cand = ctx.repo_root / p
        if cand.is_file():
            return cand
    if p.is_file():
        return p.resolve()
    if ctx.models_dir is not None:
        return ctx.models_dir / p
    if ctx.repo_root is not None:
        return ctx.repo_root / p
    return p


def _resolve_puzzle_arg(ctx: ServerContext, raw_path: Optional[str]) -> Optional[Path]:
    if raw_path is None or not str(raw_path).strip():
        return None
    p = Path(str(raw_path)).expanduser()
    if p.is_absolute():
        return p
    if ctx.repo_root is not None:
        for base in (
            ctx.repo_root,
            ctx.repo_root / "qa" / "puzzles",
            ctx.repo_root / "qa" / "search-exams",
        ):
            cand = base / p
            if cand.is_file():
                return cand
    if p.is_file():
        return p.resolve()
    if ctx.repo_root is not None:
        return ctx.repo_root / p
    return p


async def _h_list_runs(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    return _list_runs(ctx.runs_dir, ctx.models_dir)


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


async def _h_run_per_game_evals(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    name = args.get("name")
    if not isinstance(name, str) or not name:
        return {"ok": False, "error": "missing or invalid 'name' argument"}
    filt = args.get("filter") or {}
    if not isinstance(filt, dict):
        return {"ok": False, "error": "'filter' must be an object"}
    limit = args.get("limit")
    limit_int = int(limit) if limit is not None else None
    return _run_per_game_evals(ctx.models_dir, name, filt, limit_int)


async def _h_run_anchored_evals(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    name = args.get("name")
    if not isinstance(name, str) or not name:
        return {"ok": False, "error": "missing or invalid 'name' argument"}
    run_dir = _resolve_run_dir(ctx, name)
    limit = args.get("limit")
    limit_int = int(limit) if limit is not None else None
    return _run_anchored_evals(run_dir, name, limit_int)


async def _h_champion_standings(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    return _champion_standings(ctx.models_dir)


async def _h_run_elo_ladder(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    name = args.get("name")
    if not isinstance(name, str) or not name:
        return {"ok": False, "error": "missing or invalid 'name' argument"}
    return _run_elo_ladder(ctx.models_dir, name)


async def _h_run_exploitability(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    name = args.get("name")
    if not isinstance(name, str) or not name:
        return {"ok": False, "error": "missing or invalid 'name' argument"}
    return _run_exploitability(ctx.models_dir, name)


async def _h_card_embedding_neighbors(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    ckpt = args.get("checkpoint_path")
    card_ids = args.get("card_ids")
    if not isinstance(ckpt, str) or not ckpt.strip():
        return {"ok": False, "error": "missing or invalid 'checkpoint_path' argument"}
    if not isinstance(card_ids, list) or not card_ids:
        return {"ok": False, "error": "missing or invalid 'card_ids' argument"}
    top_k = int(args.get("top_k", 10))
    return _card_embedding_neighbors(
        _resolve_checkpoint_arg(ctx, ckpt),
        card_ids,
        top_k=top_k,
    )


async def _h_position_saliency(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    ckpt = args.get("checkpoint_path")
    if not isinstance(ckpt, str) or not ckpt.strip():
        return {"ok": False, "error": "missing or invalid 'checkpoint_path' argument"}
    puzzle_raw = args.get("puzzle_path")
    if puzzle_raw is not None and not isinstance(puzzle_raw, str):
        return {"ok": False, "error": "'puzzle_path' must be a string"}
    scenario_json = args.get("scenario_json")
    target_action_id = args.get("target_action_id")
    avoid_action_id = args.get("avoid_action_id")
    top_k = int(args.get("top_k", 15))
    return _position_saliency(
        _resolve_checkpoint_arg(ctx, ckpt),
        puzzle_path=_resolve_puzzle_arg(ctx, puzzle_raw),
        scenario_json=scenario_json,
        target_action_id=int(target_action_id) if target_action_id is not None else None,
        avoid_action_id=int(avoid_action_id) if avoid_action_id is not None else None,
        top_k=top_k,
    )


async def _h_plasticity_probe(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    ckpt = args.get("checkpoint_path")
    if not isinstance(ckpt, str) or not ckpt.strip():
        return {"ok": False, "error": "missing or invalid 'checkpoint_path' argument"}
    target_action_id = args.get("target_action_id")
    if target_action_id is None:
        return {"ok": False, "error": "missing 'target_action_id' argument"}
    puzzle_raw = args.get("puzzle_path")
    if puzzle_raw is not None and not isinstance(puzzle_raw, str):
        return {"ok": False, "error": "'puzzle_path' must be a string"}
    scenario_json = args.get("scenario_json")
    avoid_raw = args.get("avoid_action_ids")
    if avoid_raw is not None and not isinstance(avoid_raw, list):
        return {"ok": False, "error": "'avoid_action_ids' must be an array of integers"}
    avoid_ids = [int(a) for a in avoid_raw] if avoid_raw is not None else None
    steps = int(args.get("steps", 10))
    lr = float(args.get("lr", 1e-4))
    return _plasticity_probe(
        _resolve_checkpoint_arg(ctx, ckpt),
        puzzle_path=_resolve_puzzle_arg(ctx, puzzle_raw),
        scenario_json=scenario_json,
        target_action_id=int(target_action_id),
        avoid_action_ids=avoid_ids,
        steps=steps,
        lr=lr,
    )


async def _h_export_checkpoint_onnx(ctx: ServerContext, args: Dict[str, Any]) -> Dict[str, Any]:
    ckpt = args.get("checkpoint_path")
    if not isinstance(ckpt, str) or not ckpt.strip():
        return {"ok": False, "error": "missing or invalid 'checkpoint_path' argument"}
    cache_dir = args.get("cache_dir")
    if cache_dir is not None and not isinstance(cache_dir, str):
        return {"ok": False, "error": "'cache_dir' must be a string"}
    return _export_checkpoint_onnx(
        _resolve_checkpoint_arg(ctx, ckpt),
        cache_dir=cache_dir,
    )


TOOL_HANDLERS: Dict[str, ToolHandler] = {
    "list_runs": _h_list_runs,
    "run_summary": _h_run_summary,
    "run_metric": _h_run_metric,
    "run_tags": _h_run_tags,
    "run_recordings": _h_run_recordings,
    "run_checkpoints": _h_run_checkpoints,
    "run_deck_pool": _h_run_deck_pool,
    "run_per_game_evals": _h_run_per_game_evals,
    "run_anchored_evals": _h_run_anchored_evals,
    "champion_standings": _h_champion_standings,
    "run_elo_ladder": _h_run_elo_ladder,
    "run_exploitability": _h_run_exploitability,
    "card_embedding_neighbors": _h_card_embedding_neighbors,
    "position_saliency": _h_position_saliency,
    "plasticity_probe": _h_plasticity_probe,
    "export_checkpoint_onnx": _h_export_checkpoint_onnx,
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
