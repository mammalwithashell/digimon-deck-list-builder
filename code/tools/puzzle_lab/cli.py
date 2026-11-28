"""CLI Entrypoint for `tools.puzzle_lab` (`mine-replays`, `synthesize-archetype`, `check-providers`)."""

from __future__ import annotations

import argparse
import json
import sys
from contextlib import ExitStack
from pathlib import Path
from typing import Any, Optional, Sequence

from . import resolve_repo_root
from .llm_router import (
    ChatMessage,
    GeminiDevProvider,
    LLMRouter,
    OpenAICompatProvider,
    Role,
)
from .auto_curriculum import run_auto_curriculum
from .boss_matrix import compose_boss_matrix
from .mcp_client import StdioMcpClient
from .mine_replays import mine_replays
from .play_in import compile_play_in_with_fallback
from .synthesize_archetype import synthesize_archetype
from .tensor_fuzzer import audit_card_set_fidelity, run_tensor_completeness_fuzz


def _add_llm_args(parser: argparse.ArgumentParser) -> None:
    grp = parser.add_argument_group("LLM Router Options")
    grp.add_argument(
        "--provider",
        default="ollama",
        choices=["ollama", "vllm", "openai-compat", "gemini"],
        help="Default LLM provider preset for both SCANNER and AUTHOR (default: ollama).",
    )
    grp.add_argument("--scanner-provider", default=None, help="Override provider for Role.SCANNER.")
    grp.add_argument("--author-provider", default=None, help="Override provider for Role.AUTHOR.")
    grp.add_argument("--scanner-model", default=None, help="Override model ID for Role.SCANNER.")
    grp.add_argument("--author-model", default=None, help="Override model ID for Role.AUTHOR.")
    grp.add_argument("--base-url", default=None, help="Override base URL for OpenAI-compatible or Gemini API.")
    grp.add_argument("--scanner-base-url", default=None, help="Override base URL for Role.SCANNER.")
    grp.add_argument("--author-base-url", default=None, help="Override base URL for Role.AUTHOR.")
    grp.add_argument(
        "--allow-closed-models",
        action="store_true",
        help="Explicitly authorize dev-only closed-weight LLM endpoints (or set DIGIMON_ALLOW_CLOSED_LLM=1).",
    )
    grp.add_argument(
        "--pin-rules-cache",
        action="store_true",
        help="When AUTHOR uses GeminiDevProvider, pin docs/RULES_CONTEXT.md via cachedContents.",
    )
    grp.add_argument("--timeout", type=float, default=120.0, help="HTTP request timeout in seconds.")


def _build_router_from_args(args: argparse.Namespace) -> LLMRouter:
    router = LLMRouter.from_config(
        provider=args.provider,
        scanner_provider=args.scanner_provider,
        author_provider=args.author_provider,
        scanner_model=args.scanner_model,
        author_model=args.author_model,
        base_url=args.base_url,
        scanner_base_url=args.scanner_base_url,
        author_base_url=args.author_base_url,
        allow_closed_models=bool(args.allow_closed_models),
        timeout=float(args.timeout),
    )
    if getattr(args, "pin_rules_cache", False):
        author_prov = router.provider_for(Role.AUTHOR)
        if isinstance(author_prov, GeminiDevProvider):
            rules_path = resolve_repo_root() / "docs" / "RULES_CONTEXT.md"
            author_prov.pin_rules_and_card_pool(rules_path=rules_path)
    return router


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="python -m tools.puzzle_lab",
        description="Dev-Only Multi-Provider LLM Tactical Puzzle Lab CLI.",
    )
    sub = parser.add_subparsers(dest="subcommand", required=True)

    # 1. check-providers
    p_check = sub.add_parser(
        "check-providers",
        help="Validate SCANNER/AUTHOR provider configuration, closed-model gating, and optional live ping.",
    )
    _add_llm_args(p_check)
    p_check.add_argument(
        "--ping",
        action="store_true",
        help="Send a minimal structured JSON health check to both SCANNER and AUTHOR endpoints.",
    )

    # 2. mine-replays
    p_mine = sub.add_parser(
        "mine-replays",
        help="Mine native recordings + winprob sidecars + CardEvalCollector JSONs into verified Studio puzzles.",
    )
    p_mine.add_argument(
        "recordings",
        nargs="+",
        type=Path,
        help="Recording JSON files or directories to scan.",
    )
    p_mine.add_argument(
        "--winprob",
        nargs="*",
        type=Path,
        default=None,
        help="Optional explicit .winprob.json sidecar files or directories.",
    )
    p_mine.add_argument(
        "--card-eval",
        nargs="*",
        type=Path,
        default=None,
        help="Optional CardEvalCollector report JSON files or directories.",
    )
    p_mine.add_argument(
        "--checkpoint-zip",
        type=Path,
        default=None,
        help="Optional SB3 .zip checkpoint for digimon-training-mcp saliency / plasticity_probe.",
    )
    p_mine.add_argument(
        "--onnx-model",
        type=Path,
        default=None,
        help="Optional .onnx model path to load into digimon-engine-mcp.",
    )
    p_mine.add_argument("--puzzles-dir", type=Path, default=None, help="Output directory (default: qa/puzzles).")
    p_mine.add_argument(
        "--export-search-exams",
        action="store_true",
        help="Also export verified puzzles to qa/search-exams/ (or --search-exams-dir).",
    )
    p_mine.add_argument("--search-exams-dir", type=Path, default=None)
    p_mine.add_argument("--folder", default="", help="Optional subfolder under qa/puzzles/.")
    p_mine.add_argument("--value-drop-threshold", type=float, default=-0.25)
    p_mine.add_argument("--min-stranded-opportunities", type=int, default=3)
    p_mine.add_argument("--max-candidates", type=int, default=15)
    p_mine.add_argument("--max-puzzles", type=int, default=5)
    p_mine.add_argument("--max-tool-turns", type=int, default=10)
    p_mine.add_argument("--adversary-sims", type=int, default=64)
    p_mine.add_argument("--seeds", type=int, default=4)
    p_mine.add_argument("--require-plasticity", action="store_true")
    _add_llm_args(p_mine)

    # 3. synthesize-archetype
    p_synth = sub.add_parser(
        "synthesize-archetype",
        help="Synthesize offensive combos & defensive [All Turns] interrupt traps for an archetype.",
    )
    p_synth.add_argument("--archetype", required=True, help="Target archetype name in data/deck_library.json.")
    p_synth.add_argument("--opponent-archetype", default=None, help="Optional opponent archetype name.")
    p_synth.add_argument("--deck-library", type=Path, default=None, help="Override path to deck_library.json.")
    p_synth.add_argument(
        "--checkpoint-zip",
        type=Path,
        default=None,
        help="Optional SB3 .zip checkpoint for card_embedding_neighbors & plasticity_probe.",
    )
    p_synth.add_argument(
        "--onnx-model",
        type=Path,
        default=None,
        help="Optional .onnx model path to load into digimon-engine-mcp.",
    )
    p_synth.add_argument("--count", type=int, default=4, help="Number of puzzle blueprints to request.")
    p_synth.add_argument("--puzzles-dir", type=Path, default=None, help="Output directory (default: qa/puzzles).")
    p_synth.add_argument(
        "--export-search-exams",
        action="store_true",
        help="Also export verified puzzles to qa/search-exams/ (or --search-exams-dir).",
    )
    p_synth.add_argument("--search-exams-dir", type=Path, default=None)
    p_synth.add_argument("--folder", default=None, help="Optional subfolder under qa/puzzles/.")
    p_synth.add_argument("--no-interrupt-traps", action="store_true")
    p_synth.add_argument("--allow-unadmitted", action="store_true")
    p_synth.add_argument("--require-plasticity", action="store_true")
    p_synth.add_argument("--adversary-sims", type=int, default=64)
    p_synth.add_argument("--seeds", type=int, default=4)
    p_synth.add_argument("--refinement-turns", type=int, default=0)
    _add_llm_args(p_synth)

    # 4. compose-boss-matrix
    p_matrix = sub.add_parser(
        "compose-boss-matrix",
        help="Sweep a Defender Boss-Board Template across security/memory knobs and Challenger archetypes.",
    )
    p_matrix.add_argument(
        "--boss-template",
        default="ex12-erlangmon-invitation",
        help="Built-in boss preset ID (e.g. ex12-erlangmon-invitation) or path to template JSON.",
    )
    p_matrix.add_argument(
        "--archetypes",
        default=None,
        help="Comma-separated Challenger archetype names (or 'all' with --training-pool-snapshot).",
    )
    p_matrix.add_argument(
        "--training-pool-snapshot",
        type=Path,
        default=None,
        help="Optional train_snapshot.json path providing Challenger decks & eligible_archetypes.",
    )
    p_matrix.add_argument("--deck-library", type=Path, default=None, help="Override path to deck_library.json.")
    p_matrix.add_argument(
        "--security-counts",
        default="1,2",
        help="Comma-separated Defender security counts to sweep (default: 1,2).",
    )
    p_matrix.add_argument(
        "--memories",
        default="2,4",
        help="Comma-separated Challenger starting memory values to sweep (default: 2,4).",
    )
    p_matrix.add_argument(
        "--eval-models",
        default=None,
        help="Optional comma-separated ONNX paths or model IDs to grade via inspect_position.",
    )
    p_matrix.add_argument("--eval-simulations", type=int, default=64)
    p_matrix.add_argument("--max-retries", type=int, default=3)
    p_matrix.add_argument("--puzzles-dir", type=Path, default=None, help="Output directory (default: qa/puzzles).")
    p_matrix.add_argument("--folder", default=None, help="Subfolder under qa/puzzles/ (default: boss-matrix/<boss_id>).")
    p_matrix.add_argument("--summary-out", type=Path, default=None, help="Explicit output path for matrix summary JSON.")
    p_matrix.add_argument(
        "--export-search-exams",
        action="store_true",
        help="Also export verified puzzles to qa/search-exams/ (or --search-exams-dir).",
    )
    p_matrix.add_argument("--search-exams-dir", type=Path, default=None)
    p_matrix.add_argument("--allow-unadmitted", action="store_true")
    p_matrix.add_argument("--adversary-sims", type=int, default=64)
    p_matrix.add_argument("--seeds", type=int, default=4)
    _add_llm_args(p_matrix)

    # 5. compile-play-in
    p_playin = sub.add_parser(
        "compile-play-in",
        help="Compile a staged studio-puzzle-v1 into a legal Turn-1 replay_prefix (+ optional DCGO YAML).",
    )
    p_playin.add_argument("puzzle", type=Path, help="Input studio-puzzle-v1 JSON path.")
    p_playin.add_argument("--beam-width", type=int, default=32)
    p_playin.add_argument("--max-turns", type=int, default=5)
    p_playin.add_argument("--strict-zones", action="store_true")
    p_playin.add_argument("--max-llm-attempts", type=int, default=3)
    p_playin.add_argument("--max-tool-turns", type=int, default=6)
    p_playin.add_argument("--no-llm-fallback", action="store_true")
    p_playin.add_argument("--out-puzzle", type=Path, default=None, help="Output path for upgraded replay_prefix puzzle.")
    p_playin.add_argument("--out-dcgo-yaml", type=Path, default=None, help="Output path for symbolic DCGO scenario YAML.")
    p_playin.add_argument("--rest-deck-p0", default="custom")
    p_playin.add_argument("--rest-deck-p1", default="custom")
    p_playin.add_argument("--adversary-sims", type=int, default=64)
    p_playin.add_argument("--seeds", type=int, default=4)
    _add_llm_args(p_playin)

    # 6. fuzz-tensor
    p_fuzz = sub.add_parser(
        "fuzz-tensor",
        help="Run zero-LLM deterministic observation-tensor completeness differential fuzzer.",
    )
    p_fuzz.add_argument("--profile", default="standard_lite_v2", help="Observation profile (default: standard_lite_v2).")
    p_fuzz.add_argument("--families", default=None, help="Optional comma-separated rule family names.")
    p_fuzz.add_argument("--viewer", type=int, default=1, choices=[1, 2], help="Observer seat (1 or 2).")
    p_fuzz.add_argument("--checkpoint-zip", type=Path, default=None, help="Optional SB3 .zip checkpoint for plasticity_probe.")
    p_fuzz.add_argument("--plasticity-steps", type=int, default=10)
    p_fuzz.add_argument("--plasticity-lr", type=float, default=1e-3)
    p_fuzz.add_argument("--json-out", type=Path, default=None, help="Optional JSON report path.")
    p_fuzz.add_argument("--markdown-out", type=Path, default=None, help="Optional Markdown report path.")
    p_fuzz.add_argument("--fail-on-collision", action="store_true", help="Exit 1 if any family has l1_diff == 0.0.")

    # 7. audit-set
    p_audit = sub.add_parser(
        "audit-set",
        help="Run read-only LLM card-fidelity audit over implemented cards in a set.",
    )
    p_audit.add_argument("--set", dest="card_set", default=None, help="Card set prefix (e.g. EX12, BT24, ST23).")
    p_audit.add_argument("--card-ids", default=None, help="Optional comma-separated explicit card IDs.")
    p_audit.add_argument("--max-cards", type=int, default=25)
    p_audit.add_argument("--max-tool-turns", type=int, default=6)
    p_audit.add_argument("--cards-json", type=Path, default=None)
    p_audit.add_argument("--json-out", type=Path, default=None, help="Optional JSON audit report path.")
    p_audit.add_argument("--markdown-out", type=Path, default=None, help="Optional Markdown audit report path.")
    _add_llm_args(p_audit)

    # 8. auto-curriculum
    p_curr = sub.add_parser(
        "auto-curriculum",
        help="Air-gapped sidecar ingesting eval summaries, dual-gating drills, and emitting selfplay-shards-v1.",
    )
    p_curr.add_argument(
        "--eval-input",
        type=Path,
        required=True,
        help="Path to anchored_eval JSON report or Specialist League gates/gen<g> directory.",
    )
    p_curr.add_argument(
        "--out-dir",
        type=Path,
        required=True,
        help="Output bundle directory (verified_puzzles/, extra_shards/, curriculum_manifest.json).",
    )
    p_curr.add_argument("--recordings", nargs="*", type=Path, default=None, help="Optional recordings files or directories.")
    p_curr.add_argument(
        "--candidate-puzzles",
        nargs="*",
        type=Path,
        default=None,
        help="Optional pre-authored studio-puzzle-v1 files or directories to dual-gate.",
    )
    p_curr.add_argument("--checkpoint-zip", type=Path, default=None, help="Optional SB3 .zip checkpoint for plasticity_probe.")
    p_curr.add_argument("--onnx-model", type=Path, default=None)
    p_curr.add_argument("--threshold", type=float, default=0.45, help="Weak matchup win-rate threshold (default: 0.45).")
    p_curr.add_argument("--bottom-k", type=int, default=3, help="Number of worst matchups to target (default: 3).")
    p_curr.add_argument("--min-games", type=int, default=1)
    p_curr.add_argument("--no-synthesis", action="store_true", help="Disable LLM archetype/boss synthesis.")
    p_curr.add_argument("--no-llm", action="store_true", help="Run purely over --candidate-puzzles without LLM calls.")
    p_curr.add_argument("--boss-template", default=None)
    p_curr.add_argument("--deck-library", type=Path, default=None)
    p_curr.add_argument("--permutations-per-puzzle", type=int, default=16)
    p_curr.add_argument("--observation-profile", default="standard_lite_v2")
    p_curr.add_argument("--seed", type=int, default=0)
    p_curr.add_argument("--adversary-sims", type=int, default=64)
    p_curr.add_argument("--seeds", type=int, default=4)
    p_curr.add_argument("--plasticity-steps", type=int, default=10)
    p_curr.add_argument("--plasticity-lr", type=float, default=1e-3)
    p_curr.add_argument("--allow-unadmitted", action="store_true")
    _add_llm_args(p_curr)

    return parser


def _describe_provider(prov: Any) -> dict[str, Any]:
    info: dict[str, Any] = {
        "class": type(prov).__name__,
        "model": getattr(prov, "model", ""),
        "is_closed_weight": bool(getattr(prov, "is_closed_weight", False)),
    }
    if isinstance(prov, OpenAICompatProvider):
        info["base_url"] = prov.base_url
    elif isinstance(prov, GeminiDevProvider):
        info["base_url"] = prov.base_url
        info["cached_content_name"] = prov.cached_content_name
    return info


def _cmd_check_providers(args: argparse.Namespace) -> int:
    with _build_router_from_args(args) as router:
        summary: dict[str, Any] = {
            "ok": True,
            "scanner": _describe_provider(router.provider_for(Role.SCANNER)),
            "author": _describe_provider(router.provider_for(Role.AUTHOR)),
        }
        if args.ping:
            ping_schema = {
                "type": "object",
                "properties": {"ready": {"type": "boolean"}},
                "required": ["ready"],
                "additionalProperties": False,
            }
            for role in (Role.SCANNER, Role.AUTHOR):
                res = router.complete_structured(
                    role,
                    [ChatMessage(role="user", content='Return {"ready": true}.')],
                    json_schema=ping_schema,
                    schema_name="ping_response",
                    max_tokens=64,
                )
                summary[f"{role.value}_ping"] = res
        print(json.dumps(summary, indent=2))
    return 0


def _open_mcp_sessions(
    stack: ExitStack,
    *,
    onnx_model: Optional[Path],
    checkpoint_zip: Optional[Path],
) -> tuple[StdioMcpClient, Optional[StdioMcpClient], Optional[str]]:
    root = resolve_repo_root()
    engine_mcp = stack.enter_context(StdioMcpClient.for_engine_mcp(repo_root=root))
    training_mcp: Optional[StdioMcpClient] = None
    if checkpoint_zip is not None:
        training_mcp = stack.enter_context(StdioMcpClient.for_training_mcp(repo_root=root))

    resolved_onnx = onnx_model
    if resolved_onnx is None and checkpoint_zip is not None and training_mcp is not None:
        if training_mcp.has_tool("export_checkpoint_onnx"):
            exp = training_mcp.call_tool(
                "export_checkpoint_onnx",
                {"checkpoint_path": str(checkpoint_zip)},
            )
            if exp.get("ok") and exp.get("onnx_path"):
                resolved_onnx = Path(str(exp["onnx_path"]))

    model_id: Optional[str] = None
    if resolved_onnx is not None and engine_mcp.has_tool("load_model"):
        loaded = engine_mcp.call_tool(
            "load_model",
            {"model_id": "active-pilot", "onnx_path": str(resolved_onnx)},
        )
        if loaded.get("ok"):
            model_id = str(loaded.get("model_id") or "active-pilot")

    return engine_mcp, training_mcp, model_id


def _cmd_mine_replays(args: argparse.Namespace) -> int:
    root = resolve_repo_root()
    exams_dir = args.search_exams_dir or ((root / "qa" / "search-exams") if args.export_search_exams else None)

    with ExitStack() as stack:
        router = stack.enter_context(_build_router_from_args(args))
        engine_mcp, training_mcp, model_id = _open_mcp_sessions(
            stack,
            onnx_model=args.onnx_model,
            checkpoint_zip=args.checkpoint_zip,
        )
        report = mine_replays(
            args.recordings,
            router=router,
            engine_mcp=engine_mcp,
            training_mcp=training_mcp,
            winprob_paths=args.winprob,
            card_eval_paths=args.card_eval,
            checkpoint_path=args.checkpoint_zip,
            model_id=model_id,
            puzzles_dir=args.puzzles_dir,
            search_exams_dir=exams_dir,
            folder=args.folder,
            value_drop_threshold=args.value_drop_threshold,
            min_stranded_opportunities=args.min_stranded_opportunities,
            max_candidates=args.max_candidates,
            max_puzzles=args.max_puzzles,
            max_tool_turns=args.max_tool_turns,
            adversary_sims=args.adversary_sims,
            verify_seeds=args.seeds,
            require_plasticity=bool(args.require_plasticity),
            validate_with_engine=True,
        )

    out = {
        "candidates_found": report.candidates_found,
        "scanner_kept": report.scanner_kept,
        "puzzles_written": report.puzzles_written,
        "puzzles": [
            {
                "id": o.puzzle_id,
                "accepted": o.accepted,
                "path": str(o.puzzle_path) if o.puzzle_path else None,
                "rejection_reason": o.rejection_reason or None,
            }
            for o in report.outcomes
        ],
    }
    print(json.dumps(out, indent=2))
    return 0


def _cmd_synthesize_archetype(args: argparse.Namespace) -> int:
    root = resolve_repo_root()
    exams_dir = args.search_exams_dir or ((root / "qa" / "search-exams") if args.export_search_exams else None)

    with ExitStack() as stack:
        router = stack.enter_context(_build_router_from_args(args))
        engine_mcp, training_mcp, model_id = _open_mcp_sessions(
            stack,
            onnx_model=args.onnx_model,
            checkpoint_zip=args.checkpoint_zip,
        )
        report = synthesize_archetype(
            args.archetype,
            router=router,
            engine_mcp=engine_mcp,
            training_mcp=training_mcp,
            opponent_archetype=args.opponent_archetype,
            checkpoint_zip=args.checkpoint_zip,
            model_id=model_id,
            puzzles_dir=args.puzzles_dir,
            search_exams_dir=exams_dir,
            deck_library_path=args.deck_library,
            folder=args.folder,
            count=args.count,
            include_interrupt_traps=not bool(args.no_interrupt_traps),
            require_admissible=not bool(args.allow_unadmitted),
            require_plasticity=bool(args.require_plasticity),
            validate_with_engine=True,
            adversary_sims=args.adversary_sims,
            verify_seeds=args.seeds,
            refinement_turns=args.refinement_turns,
        )

    out = {
        "archetype": report.archetype_name,
        "deck_id": report.deck_id,
        "blueprints_proposed": report.blueprints_proposed,
        "puzzles_written": report.puzzles_written,
        "starved_card_ids": (
            report.starved_embeddings.starved_card_ids if report.starved_embeddings else []
        ),
        "puzzles": [
            {
                "id": o.puzzle_id,
                "category": o.category,
                "accepted": o.accepted,
                "path": str(o.puzzle_path) if o.puzzle_path else None,
                "rejection_reason": o.rejection_reason or None,
            }
            for o in report.outcomes
        ],
    }
    print(json.dumps(out, indent=2))
    return 0


def _parse_csv_ints(raw: str) -> list[int]:
    return [int(tok.strip()) for tok in str(raw or "").split(",") if tok.strip()]


def _parse_csv_strs(raw: Optional[str]) -> Optional[list[str]]:
    if raw is None:
        return None
    items = [tok.strip() for tok in str(raw).split(",") if tok.strip()]
    return items or None


def _cmd_compose_boss_matrix(args: argparse.Namespace) -> int:
    root = resolve_repo_root()
    exams_dir = args.search_exams_dir or ((root / "qa" / "search-exams") if args.export_search_exams else None)
    sec_counts = _parse_csv_ints(args.security_counts)
    memories = _parse_csv_ints(args.memories)
    archetypes = _parse_csv_strs(args.archetypes)
    eval_models = _parse_csv_strs(args.eval_models)

    with ExitStack() as stack:
        router = stack.enter_context(_build_router_from_args(args))
        engine_mcp, _, _ = _open_mcp_sessions(stack, onnx_model=None, checkpoint_zip=None)
        report = compose_boss_matrix(
            args.boss_template,
            router=router,
            engine_mcp=engine_mcp,
            archetypes=archetypes,
            training_pool_snapshot=args.training_pool_snapshot,
            security_counts=sec_counts,
            memories=memories,
            puzzles_dir=args.puzzles_dir,
            search_exams_dir=exams_dir,
            matrix_folder=args.folder,
            summary_out_path=args.summary_out,
            deck_library_path=args.deck_library,
            eval_models=eval_models,
            eval_simulations=args.eval_simulations,
            max_retries=args.max_retries,
            adversary_sims=args.adversary_sims,
            verify_seeds=args.seeds,
            require_admissible=not bool(args.allow_unadmitted),
            validate_with_engine=True,
        )

    out = report.to_summary_dict()
    out["summary_path"] = str(report.summary_path) if report.summary_path else None
    print(json.dumps(out, indent=2))
    return 0


def _cmd_compile_play_in(args: argparse.Namespace) -> int:
    with ExitStack() as stack:
        router = (
            None
            if bool(args.no_llm_fallback)
            else stack.enter_context(_build_router_from_args(args))
        )
        engine_mcp, _, _ = _open_mcp_sessions(stack, onnx_model=None, checkpoint_zip=None)
        res = compile_play_in_with_fallback(
            args.puzzle,
            engine_mcp=engine_mcp,
            router=router,
            beam_width=args.beam_width,
            max_turns=args.max_turns,
            strict_zones=bool(args.strict_zones),
            max_llm_attempts=args.max_llm_attempts,
            max_tool_turns=args.max_tool_turns,
            out_puzzle_path=args.out_puzzle,
            out_dcgo_yaml_path=args.out_dcgo_yaml,
            rest_deck_p0=args.rest_deck_p0,
            rest_deck_p1=args.rest_deck_p1,
            adversary_sims=args.adversary_sims,
            verify_seeds=args.seeds,
        )
    print(json.dumps(res.to_dict(), indent=2))
    return 0 if res.matched else 1


def _cmd_fuzz_tensor(args: argparse.Namespace) -> int:
    families = _parse_csv_strs(args.families)
    report = run_tensor_completeness_fuzz(
        profile=args.profile,
        families=families,
        viewer=args.viewer,
        checkpoint_zip=args.checkpoint_zip,
        plasticity_steps=args.plasticity_steps,
        plasticity_lr=args.plasticity_lr,
        json_out_path=args.json_out,
        markdown_out_path=args.markdown_out,
    )
    print(json.dumps(report.to_dict(), indent=2))
    if args.fail_on_collision and report.collisions:
        return 1
    return 0


def _cmd_audit_set(args: argparse.Namespace) -> int:
    card_ids = _parse_csv_strs(args.card_ids)
    with ExitStack() as stack:
        router = stack.enter_context(_build_router_from_args(args))
        engine_mcp, _, _ = _open_mcp_sessions(stack, onnx_model=None, checkpoint_zip=None)
        report = audit_card_set_fidelity(
            router=router,
            engine_mcp=engine_mcp,
            card_set=args.card_set,
            card_ids=card_ids,
            max_cards=args.max_cards,
            max_tool_turns=args.max_tool_turns,
            cards_json_path=args.cards_json,
            json_out_path=args.json_out,
            markdown_out_path=args.markdown_out,
        )
    print(json.dumps(report.to_dict(), indent=2))
    return 0


def _cmd_auto_curriculum(args: argparse.Namespace) -> int:
    with ExitStack() as stack:
        router = (
            None
            if bool(args.no_llm)
            else stack.enter_context(_build_router_from_args(args))
        )
        engine_mcp, training_mcp, model_id = _open_mcp_sessions(
            stack,
            onnx_model=args.onnx_model,
            checkpoint_zip=args.checkpoint_zip,
        )
        report = run_auto_curriculum(
            args.eval_input,
            out_dir=args.out_dir,
            engine_mcp=engine_mcp,
            training_mcp=training_mcp,
            router=router,
            recordings_paths=args.recordings,
            candidate_puzzles=args.candidate_puzzles,
            checkpoint_zip=args.checkpoint_zip,
            model_id=model_id,
            win_rate_threshold=args.threshold,
            bottom_k=args.bottom_k,
            min_games=args.min_games,
            enable_synthesis=not bool(args.no_synthesis),
            boss_template=args.boss_template,
            deck_library_path=args.deck_library,
            permutations_per_puzzle=args.permutations_per_puzzle,
            observation_profile=args.observation_profile,
            seed=args.seed,
            adversary_sims=args.adversary_sims,
            verify_seeds=args.seeds,
            plasticity_steps=args.plasticity_steps,
            plasticity_lr=args.plasticity_lr,
            require_admissible=not bool(args.allow_unadmitted),
        )
    print(json.dumps(report.manifest_dict, indent=2))
    return 0


def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        if args.subcommand == "check-providers":
            return _cmd_check_providers(args)
        if args.subcommand == "mine-replays":
            return _cmd_mine_replays(args)
        if args.subcommand == "synthesize-archetype":
            return _cmd_synthesize_archetype(args)
        if args.subcommand == "compose-boss-matrix":
            return _cmd_compose_boss_matrix(args)
        if args.subcommand == "compile-play-in":
            return _cmd_compile_play_in(args)
        if args.subcommand == "fuzz-tensor":
            return _cmd_fuzz_tensor(args)
        if args.subcommand == "audit-set":
            return _cmd_audit_set(args)
        if args.subcommand == "auto-curriculum":
            return _cmd_auto_curriculum(args)
    except PermissionError as exc:
        print(f"PermissionError: {exc}", file=sys.stderr)
        return 2
    parser.error(f"Unhandled subcommand {args.subcommand!r}")
    return 2
