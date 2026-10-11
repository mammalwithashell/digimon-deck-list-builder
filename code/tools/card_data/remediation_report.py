"""Publish deterministic, actionable migration-remediation queues.

This report is deliberately a review artifact rather than a producer input.  It
turns the live official candidate plus the current DSL/verdict/evidence lanes
into explicit work queues without treating any card as complete.
"""

from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Mapping

from .errors import ValidationError, WriteError
from .serialization import canonical_json_text, natural_card_id_key, sha256_hex


SCHEMA_VERSION = 1
ACCEPTED_VERDICTS = frozenset({"IMPLEMENTED", "AUDITED-OK"})


def _load_object(path: Path) -> Mapping[str, Any]:
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ValidationError(
            "failed to read remediation-report input",
            path=path,
            details={"error": str(exc)},
        ) from exc
    if not isinstance(payload, Mapping):
        raise ValidationError("remediation-report input must be an object", path=path)
    return payload


def _set_id(card_id: str) -> str:
    return card_id.split("-", 1)[0]


def build_remediation_report(
    *,
    official_candidate: Mapping[str, Any],
    lint_baseline: Mapping[str, Any],
    pool_comparison: Mapping[str, Any],
    verdicts: Mapping[str, Any],
    evidence: Mapping[str, Any],
    blockers: Mapping[str, Any],
    input_digests: Mapping[str, str],
) -> dict[str, Any]:
    official_ids = set(official_candidate.get("official_only_ids", ()))
    legacy_only_ids = set(official_candidate.get("legacy_only_ids", ()))
    universe = official_ids | legacy_only_ids
    for row in official_candidate.get("unapproved_runtime_field_diffs", ()):
        universe.add(str(row["card_id"]))

    verdict_rows = verdicts.get("cards", {})
    if not isinstance(verdict_rows, Mapping):
        raise ValidationError("validated-card report must contain a cards object")
    universe.update(str(card_id) for card_id in verdict_rows)

    evidence_rows = evidence.get("evidence", ())
    if not isinstance(evidence_rows, list):
        raise ValidationError("behavioral evidence must contain an evidence array")
    passing_current: set[str] = set()
    for row in evidence_rows:
        if not isinstance(row, Mapping):
            continue
        if row.get("current") is True and row.get("result") == "passed":
            passing_current.update(str(card_id) for card_id in row.get("card_ids", ()))

    # The live official candidate count can exceed the union represented by its
    # diff queues, so missing verdicts are derived from its explicit official
    # universe when present and otherwise from the review-visible union.
    candidate_universe = official_candidate.get("official_card_ids")
    if isinstance(candidate_universe, list):
        official_universe = {str(card_id) for card_id in candidate_universe}
    else:
        official_universe = universe

    missing_verdict_ids = sorted(
        official_universe - set(verdict_rows), key=natural_card_id_key
    )
    stale_verdict_rows = []
    status_rows = []
    for card_id, row in verdict_rows.items():
        if not isinstance(row, Mapping):
            continue
        status = str(row.get("status", ""))
        if status in {"PARTIAL", "BLOCKED"}:
            status_rows.append(
                {
                    "card_id": card_id,
                    "status": status,
                    "gap_kind": row.get("gap_kind"),
                    "yaml_path": row.get("yaml_path"),
                    "test_path": row.get("test_path"),
                    "notes": row.get("notes"),
                }
            )
        if status in ACCEPTED_VERDICTS and card_id not in passing_current:
            stale_verdict_rows.append(
                {
                    "card_id": card_id,
                    "verdict_status": status,
                    "reason": "no current passing digest-bound behavioral evidence",
                    "yaml_path": row.get("yaml_path"),
                    "test_path": row.get("test_path"),
                }
            )

    active_blockers = [
        row
        for row in blockers.get("blockers", ())
        if isinstance(row, Mapping) and row.get("status") == "active"
    ]
    blocker_by_card: dict[str, list[str]] = defaultdict(list)
    for row in active_blockers:
        for card_id in row.get("affected_card_ids", ()):
            blocker_by_card[str(card_id)].append(str(row.get("blocker_id")))
    for row in status_rows:
        row["active_blocker_ids"] = sorted(blocker_by_card.get(row["card_id"], ()))

    runtime_diffs_by_card: dict[str, list[str]] = defaultdict(list)
    runtime_diff_fields: Counter[str] = Counter()
    runtime_diff_sets: Counter[str] = Counter()
    for row in official_candidate.get("unapproved_runtime_field_diffs", ()):
        card_id = str(row["card_id"])
        field_path = str(row["field_path"])
        runtime_diff_fields[field_path] += 1
        runtime_diff_sets[_set_id(card_id)] += 1
        runtime_diffs_by_card[card_id].append(field_path)

    correction_review = [
        row
        for row in official_candidate.get("corrections", ())
        if isinstance(row, Mapping) and row.get("status") != "applied"
    ]
    lint_entries = list(lint_baseline.get("entries", ()))
    tested_not_production = list(pool_comparison.get("tested_not_production", ()))
    production_not_tested = list(pool_comparison.get("production_not_tested", ()))

    report = {
        "schema_version": SCHEMA_VERSION,
        "status": "review-required",
        "completion_claimed": False,
        "candidate_canonical_revision": official_candidate.get(
            "candidate_canonical_revision"
        ),
        "source_manifest_digest": official_candidate.get("source_manifest_digest"),
        "reproduce": {
            "official_candidate": (
                "python -m tools.card_data.official_candidate --repo-root . "
                "--output-dir .codex/target-card-data/official-candidate"
            ),
            "remediation_report": (
                "python -m tools.card_data.remediation_report --repo-root . "
                "--official-candidate .codex/target-card-data/official-candidate/"
                "qa/qa-reports/official-canonical-candidate.json --write"
            ),
        },
        "input_digests": dict(sorted(input_digests.items())),
        "counts": {
            "strict_lint_diagnostics": len(lint_entries),
            "example_only_runtime_cards": len(tested_not_production),
            "production_not_compatibility_pool": len(production_not_tested),
            "missing_verdicts": len(missing_verdict_ids),
            "stale_verdicts": len(stale_verdict_rows),
            "partial_cards": sum(row["status"] == "PARTIAL" for row in status_rows),
            "blocked_cards": sum(row["status"] == "BLOCKED" for row in status_rows),
            "active_capability_blockers": len(active_blockers),
            "official_only_cards": len(official_ids),
            "legacy_only_cards": len(legacy_only_ids),
            "unapproved_runtime_field_diffs": sum(runtime_diff_fields.values()),
            "corrections_requiring_review": len(correction_review),
        },
        "strict_lint": {
            "owner": "DSL/card-script maintainers",
            "action": "remediate each diagnostic, then regenerate the clean baseline",
            "items": lint_entries,
        },
        "example_only_runtime_cards": {
            "owner": "DSL/card-script maintainers",
            "action": "promote faithfully or approve a digest-bound pool removal",
            "items": tested_not_production,
            "production_not_compatibility_pool": production_not_tested,
        },
        "missing_or_stale_verdicts": {
            "owner": "card QA maintainers",
            "action": "add a current verdict and passing digest-bound behavioral evidence",
            "missing_verdict_ids": missing_verdict_ids,
            "stale_verdicts": sorted(
                stale_verdict_rows, key=lambda row: natural_card_id_key(row["card_id"])
            ),
        },
        "partial_or_blocked_cards": {
            "owner": "engine, DSL, and card-script maintainers",
            "action": "resolve the referenced capability or card implementation; do not mark DONE early",
            "items": sorted(
                status_rows, key=lambda row: natural_card_id_key(row["card_id"])
            ),
            "active_capability_blockers": active_blockers,
        },
        "source_discrepancies": {
            "owner": "card-data reviewers",
            "action": "review admissions, retirements, corrections, and each semantic runtime-field diff before producer cutover",
            "official_only_ids": sorted(official_ids, key=natural_card_id_key),
            "legacy_only_ids": sorted(legacy_only_ids, key=natural_card_id_key),
            "corrections_requiring_review": correction_review,
            "runtime_diff_counts_by_field": dict(sorted(runtime_diff_fields.items())),
            "runtime_diff_counts_by_set": dict(sorted(runtime_diff_sets.items())),
            "unapproved_runtime_fields_by_card": {
                card_id: sorted(rows)
                for card_id, rows in sorted(
                    runtime_diffs_by_card.items(),
                    key=lambda item: natural_card_id_key(item[0]),
                )
            },
        },
    }
    return report


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--official-candidate", type=Path, required=True)
    parser.add_argument(
        "--lint-baseline",
        type=Path,
        default=Path("code/digimon-engine/cards/core_validation_baseline.json"),
    )
    parser.add_argument(
        "--pool-comparison",
        type=Path,
        default=Path("qa/qa-reports/tested_cards_vs_production_lane.json"),
    )
    parser.add_argument(
        "--verdicts",
        type=Path,
        default=Path("qa/qa-reports/validated_cards_dsl.json"),
    )
    parser.add_argument(
        "--evidence",
        type=Path,
        default=Path("qa/qa-reports/card_behavioral_evidence.json"),
    )
    parser.add_argument(
        "--blockers", type=Path, default=Path("data/card_blockers.json")
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("qa/qa-reports/card-data-cutover-remediation.json"),
    )
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)
    if args.write == args.check:
        parser.error("choose exactly one of --write or --check")
    root = args.repo_root.resolve()

    def resolve(path: Path) -> Path:
        return path if path.is_absolute() else root / path

    inputs = {
        "official_candidate": resolve(args.official_candidate),
        "lint_baseline": resolve(args.lint_baseline),
        "pool_comparison": resolve(args.pool_comparison),
        "verdicts": resolve(args.verdicts),
        "evidence": resolve(args.evidence),
        "blockers": resolve(args.blockers),
    }
    payloads = {name: _load_object(path) for name, path in inputs.items()}
    report = build_remediation_report(
        official_candidate=payloads["official_candidate"],
        lint_baseline=payloads["lint_baseline"],
        pool_comparison=payloads["pool_comparison"],
        verdicts=payloads["verdicts"],
        evidence=payloads["evidence"],
        blockers=payloads["blockers"],
        input_digests={name: sha256_hex(path.read_bytes()) for name, path in inputs.items()},
    )
    rendered = canonical_json_text(report)
    output = resolve(args.output)
    if args.check:
        if not output.exists() or output.read_text(encoding="utf-8") != rendered:
            raise ValidationError("cutover remediation report is stale", path=output)
        return 0
    try:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(rendered, encoding="utf-8", newline="\n")
    except OSError as exc:
        raise WriteError("could not write cutover remediation report", path=output) from exc
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
