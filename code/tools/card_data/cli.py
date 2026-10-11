"""Compatibility command-line entry point for the card-data pipeline.

The command surface is intentionally small while the canonical pipeline is
introduced.  Future sync, generation, and campaign commands should register
their handlers here so legacy wrappers and the module entry point share one
error-reporting boundary.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from collections.abc import Sequence

from . import __version__
from .adjudications import load_migration_adjudications
from .candidate_dsl_validation import validate_candidate_production_dsl
from .errors import CardDataError, ValidationError
from .publication import (
    apply_publication,
    build_publication_plan,
    load_candidate_artifacts,
    load_publication_plan,
    publication_plan_payload,
    rollback_committed_publication,
)
from .serialization import canonical_json_text, content_digest
from .sync_plan import load_source_plan


PROGRAM_NAME = "card-data"
OFFICIAL_CANDIDATE_REPORT = Path(
    "qa/qa-reports/official-canonical-candidate.json"
)
MIGRATION_ADJUDICATION_LEDGER = Path(
    "data/card_migration_adjudications.json"
)
COMPLETE_CANDIDATE_KIND = "complete-full-pool-card-data"
REQUIRED_COMPLETE_CANDIDATE_ARTIFACTS = frozenset(
    {
        "data/author_set_lexicons.json",
        "data/card_compatibility_projection.json",
        "data/card_corrections.json",
        "data/card_data_derived_artifacts.json",
        "data/card_evolution_costs.json",
        "data/card_migration_adjudications.json",
        "data/card_official.json",
        "data/card_registry.json",
        "data/card_sources/manifest.json",
        "data/cards.json",
        "data/cards_canonical.json",
        "data/production_runtime_registry.json",
        "data/runtime_registered_cards.json",
        "data/tested_cards.json",
        "data/validated_playable_cards.json",
        "qa/qa-reports/card_implementation_manifest.json",
        "qa/qa-reports/live-card-data-sync-plan.json",
        "qa/qa-reports/registered_vs_validated_pool_impact.json",
        "qa/qa-reports/tested_cards_vs_production_lane.json",
    }
)


def _require_candidate_semantic_readiness(candidate_dir: Path) -> None:
    """Require a ready complete-candidate report and its downstream contract.

    Candidate assembly is intentionally separate from transactional planning,
    but only the complete full-pool assembler owns publication candidates.
    Failing closed here prevents a missing report or a ready-looking legacy
    official-only preview from becoming a complete committed revision.
    """

    path = candidate_dir / OFFICIAL_CANDIDATE_REPORT
    if not path.is_file():
        raise ValidationError(
            "complete canonical candidate report is required for publication",
            path=path,
        )
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ValidationError(
            "official canonical candidate report is unreadable",
            path=path,
            details={"error": str(exc)},
        ) from exc
    if not isinstance(payload, dict):
        raise ValidationError(
            "official canonical candidate report must be an object",
            path=path,
        )
    if payload.get("candidate_kind") != COMPLETE_CANDIDATE_KIND:
        raise ValidationError(
            "publication requires a complete full-pool canonical candidate",
            path=path,
            details={"candidate_kind": payload.get("candidate_kind")},
        )
    if payload.get("status") not in {"ready", "blocked"}:
        raise ValidationError(
            "official canonical candidate report has an invalid status",
            path=path,
        )
    if payload["status"] != "ready":
        raise ValidationError(
            "official canonical candidate is not ready for publication",
            path=path,
            details={
                "status": payload["status"],
                "runtime_field_diff_count": payload.get(
                    "runtime_field_diff_count"
                ),
                "approved_runtime_field_diff_count": payload.get(
                    "approved_runtime_field_diff_count"
                ),
            },
        )
    source_plan = payload.get("source_plan")
    downstream = payload.get("downstream")
    if payload.get("canonical_readiness") != "ready":
        raise ValidationError(
            "complete canonical candidate is not canonically ready",
            path=path,
        )
    if not isinstance(source_plan, dict) or source_plan.get("can_apply") is not True:
        raise ValidationError(
            "complete canonical candidate source plan is not applicable",
            path=path,
        )
    if not isinstance(downstream, dict) or downstream.get("status") != "included":
        raise ValidationError(
            "complete canonical candidate downstream artifacts are not included",
            path=path,
        )
    missing = sorted(
        relative
        for relative in REQUIRED_COMPLETE_CANDIDATE_ARTIFACTS
        if not (candidate_dir / relative).is_file()
    )
    if missing:
        raise ValidationError(
            "complete canonical candidate is missing required artifacts",
            path=candidate_dir,
            details={"missing": missing},
        )


def _candidate_adjudication_digests(candidate_dir: Path) -> tuple[str, ...]:
    """Return deterministic identities for adjudications shipped in a candidate."""

    ledger = candidate_dir / MIGRATION_ADJUDICATION_LEDGER
    return tuple(
        content_digest(adjudication)
        for adjudication in load_migration_adjudications(ledger)
    )


def build_parser() -> argparse.ArgumentParser:
    """Build the compatibility parser without performing any work."""

    parser = argparse.ArgumentParser(
        prog=PROGRAM_NAME,
        description="canonical card-data pipeline",
    )
    parser.add_argument(
        "--diagnostic-format",
        choices=("text", "json"),
        default="text",
        help="render diagnostics as human-readable text or machine-readable JSON",
    )
    parser.add_argument(
        "--version",
        action="version",
        version=f"%(prog)s {__version__}",
    )
    subcommands = parser.add_subparsers(dest="command")
    sync = subcommands.add_parser("sync", help="plan or apply one complete candidate revision")
    mode = sync.add_mutually_exclusive_group(required=True)
    mode.add_argument("--plan", action="store_true")
    mode.add_argument("--apply", action="store_true")
    sync.add_argument("--repo-root", type=Path, default=Path.cwd())
    sync.add_argument("--candidate-dir", type=Path, required=True)
    sync.add_argument("--plan-file", type=Path, required=True)
    sync.add_argument("--source-plan", type=Path)
    sync.add_argument(
        "--marker", type=Path, default=Path("data/card_data_revision.json")
    )
    sync.add_argument(
        "--journal", type=Path, default=Path("data/.card_data_transaction.json")
    )
    sync.add_argument("--recover-stale-lock", action="store_true")
    rollback = subcommands.add_parser(
        "rollback-committed",
        help="explicitly restore the revision before the latest successful publication",
    )
    rollback.add_argument("--repo-root", type=Path, default=Path.cwd())
    rollback.add_argument(
        "--expected-current-revision",
        required=True,
        help="required digest pin for the currently committed revision",
    )
    rollback.add_argument(
        "--reason",
        required=True,
        help="operator reason persisted in the transaction journal",
    )
    rollback.add_argument(
        "--marker", type=Path, default=Path("data/card_data_revision.json")
    )
    rollback.add_argument(
        "--journal", type=Path, default=Path("data/.card_data_transaction.json")
    )
    rollback.add_argument("--recover-stale-lock", action="store_true")
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    """Run the compatibility CLI.

    With no command registered yet, a bare invocation is a successful help
    request.  Concrete command handlers will be added by their owning OpenSpec
    tasks rather than being represented by misleading no-op placeholders.
    """

    parser = build_parser()
    args = parser.parse_args(argv)
    if args.command is None:
        parser.print_help()
        return 0
    if args.command == "sync":
        root = args.repo_root.resolve()
        candidate_dir = args.candidate_dir.resolve()
        marker = args.marker if args.marker.is_absolute() else root / args.marker
        journal = args.journal if args.journal.is_absolute() else root / args.journal
        plan_file = args.plan_file.resolve()
        try:
            _require_candidate_semantic_readiness(candidate_dir)
            if args.plan:
                if args.source_plan is None:
                    parser.error("sync --plan requires --source-plan")
                source_payload = json.loads(args.source_plan.read_text(encoding="utf-8"))
                source_plan = load_source_plan(source_payload)
                artifacts = load_candidate_artifacts(candidate_dir)
                validate_candidate_production_dsl(
                    repo_root=root,
                    candidate_dir=candidate_dir,
                    artifact_bytes=artifacts,
                )
                plan = build_publication_plan(
                    repo_root=root,
                    source_plan=source_plan,
                    artifact_bytes=artifacts,
                    marker_path=marker,
                    journal_path=journal,
                    reviewed_adjudication_digests=(
                        _candidate_adjudication_digests(candidate_dir)
                    ),
                )
                rendered = canonical_json_text(publication_plan_payload(plan))
                plan_file.parent.mkdir(parents=True, exist_ok=True)
                plan_file.write_text(rendered, encoding="utf-8", newline="\n")
                print(plan.token.token_id)
                return 0
            payload = json.loads(plan_file.read_text(encoding="utf-8"))
            plan = load_publication_plan(payload, candidate_dir=candidate_dir)
            apply_publication(
                plan,
                repo_root=root,
                marker_path=marker,
                journal_path=journal,
                recover_stale_lock=args.recover_stale_lock,
            )
            print(plan.marker.revision)
            return 0
        except (OSError, json.JSONDecodeError, CardDataError, ValueError) as exc:
            parser.error(str(exc))
    if args.command == "rollback-committed":
        root = args.repo_root.resolve()
        marker = args.marker if args.marker.is_absolute() else root / args.marker
        journal_path = (
            args.journal if args.journal.is_absolute() else root / args.journal
        )
        try:
            journal = rollback_committed_publication(
                repo_root=root,
                marker_path=marker,
                journal_path=journal_path,
                expected_current_revision=args.expected_current_revision,
                reason=args.reason,
                recover_stale_lock=args.recover_stale_lock,
            )
            print(journal.base_revision)
            return 0
        except (OSError, CardDataError, ValueError) as exc:
            parser.error(str(exc))
    raise AssertionError(f"unhandled command: {args.command}")


if __name__ == "__main__":
    raise SystemExit(main())
