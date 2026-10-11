"""Convert sealed semantic-review decisions into exact migration approvals.

Review helpers deliberately do not write production ledgers.  This module is
the narrow bridge between those read-only artifacts and the migration ledger:
it verifies the sealed candidate report byte-for-byte, rechecks every reviewed
row against the report, and materializes approvals only for rows explicitly
marked safe.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Mapping, Sequence

from .adjudications import (
    load_migration_adjudications,
    merge_migration_adjudications,
    write_migration_adjudications,
)
from .errors import ValidationError
from .models import SCHEMA_VERSION, AdjudicationKind, MigrationAdjudication
from .transaction import standalone_output_write_guard


RUNTIME_REVIEW_KIND = "sealed-runtime-field-diff-semantic-triage"
RUNTIME_REVIEW_SCHEMA_VERSIONS = frozenset({1, 2})


@dataclass(frozen=True, slots=True)
class ReviewedAdjudicationBatch:
    kind: AdjudicationKind
    targets: frozenset[tuple[str, str]]
    adjudications: tuple[MigrationAdjudication, ...]


def _load_sealed_object(path: Path) -> tuple[Mapping[str, Any], str]:
    try:
        raw = path.read_bytes()
        value = json.loads(raw.decode("utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ValidationError(
            "semantic review input is unreadable",
            path=path,
            details={"error": str(exc)},
        ) from exc
    if not isinstance(value, Mapping):
        raise ValidationError("semantic review input must be an object", path=path)
    return value, hashlib.sha256(raw).hexdigest()


def _resolve_review_dependency(review_path: Path, declared: str) -> Path:
    relative = Path(declared)
    if relative.is_absolute() or ".." in relative.parts:
        raise ValidationError(
            "semantic review dependency must be a repository-relative path",
            path=review_path,
            details={"dependency": declared},
        )
    matches: list[Path] = []
    seen: set[Path] = set()
    for parent in (review_path.parent, *review_path.parents):
        candidate = (parent / relative).resolve()
        if candidate in seen:
            continue
        seen.add(candidate)
        if candidate.is_file():
            matches.append(candidate)
    if len(matches) != 1:
        raise ValidationError(
            "semantic review dependency could not be resolved uniquely",
            path=review_path,
            details={"dependency": declared, "matches": [str(item) for item in matches]},
        )
    return matches[0]


def _validate_runtime_review_dependencies(
    review: Mapping[str, Any],
    *,
    review_path: Path,
    candidate_report_sha256: str,
) -> None:
    schema_version = review.get("schema_version")
    if schema_version not in RUNTIME_REVIEW_SCHEMA_VERSIONS:
        raise ValidationError("unsupported semantic review schema", path=review_path)
    if schema_version == 1:
        return
    declared = review.get("runtime_hold_review")
    expected_digest = review.get("runtime_hold_review_sha256")
    if not isinstance(declared, str) or not declared:
        raise ValidationError(
            "runtime review v2 requires its hold-review dependency",
            path=review_path,
        )
    if not isinstance(expected_digest, str) or len(expected_digest) != 64:
        raise ValidationError(
            "runtime review v2 requires its hold-review digest",
            path=review_path,
        )
    hold_path = _resolve_review_dependency(review_path, declared)
    hold, hold_digest = _load_sealed_object(hold_path)
    if hold_digest != expected_digest.lower():
        raise ValidationError(
            "runtime hold-review dependency digest does not match",
            path=hold_path,
        )
    inputs = hold.get("inputs")
    if (
        hold.get("schema_version") != 1
        or hold.get("candidate_canonical_revision")
        != review.get("candidate_canonical_revision")
        or not isinstance(inputs, Mapping)
        or inputs.get("candidate_report_sha256") != candidate_report_sha256
        or hold.get("unresolved_rows") != []
    ):
        raise ValidationError(
            "runtime hold-review dependency is not bound to the sealed candidate",
            path=hold_path,
        )


def _unique_rows(
    rows: Any,
    *,
    path: Path,
    label: str,
) -> dict[tuple[str, str], Mapping[str, Any]]:
    if not isinstance(rows, list):
        raise ValidationError(f"{label} must be an array", path=path)
    output: dict[tuple[str, str], Mapping[str, Any]] = {}
    for index, row in enumerate(rows):
        if not isinstance(row, Mapping):
            raise ValidationError(
                f"{label} row must be an object",
                path=path,
                details={"index": index},
            )
        try:
            key = (str(row["card_id"]), str(row["field_path"]))
        except KeyError as exc:
            raise ValidationError(
                f"{label} row is missing its target",
                path=path,
                details={"index": index},
            ) from exc
        if key in output:
            raise ValidationError(
                f"{label} contains a duplicate target",
                path=path,
                card_id=key[0],
                field_path=key[1],
            )
        output[key] = row
    return output


def _validate_runtime_safety_consistency(
    review: Mapping[str, Any],
    decisions: Mapping[tuple[str, str], Mapping[str, Any]],
    *,
    path: Path,
) -> None:
    for key, decision in decisions.items():
        safe = decision.get("safe_to_approve")
        flags = decision.get("flags")
        if not isinstance(safe, bool):
            raise ValidationError(
                "runtime decision requires a boolean safe_to_approve",
                path=path,
                card_id=key[0],
                field_path=key[1],
            )
        if (
            not isinstance(flags, list)
            or any(not isinstance(flag, str) or not flag for flag in flags)
            or len(flags) != len(set(flags))
            or safe is not (len(flags) == 0)
        ):
            raise ValidationError(
                "runtime decision safety contradicts its flags",
                path=path,
                card_id=key[0],
                field_path=key[1],
            )
    unsafe = {
        key: decision
        for key, decision in decisions.items()
        if not decision["safe_to_approve"]
    }
    not_safe = _unique_rows(
        review.get("not_safe_rows"), path=path, label="runtime not-safe rows"
    )
    if not_safe.keys() != unsafe.keys():
        raise ValidationError(
            "runtime review not-safe index contradicts decision safety",
            path=path,
        )
    for key, row in not_safe.items():
        decision = unsafe[key]
        for field in ("old_digest", "new_digest", "category", "flags"):
            if row.get(field) != decision.get(field):
                raise ValidationError(
                    "runtime not-safe row contradicts its decision",
                    path=path,
                    card_id=key[0],
                    field_path=key[1],
                    details={"field": field},
                )
    summary = review.get("summary")
    if not isinstance(summary, Mapping):
        raise ValidationError("runtime review summary must be an object", path=path)
    expected_approval_counts = {
        key: value
        for key, value in {
            "safe": len(decisions) - len(unsafe),
            "not_safe": len(unsafe),
        }.items()
        if value
    }
    if summary.get("approval_counts") != expected_approval_counts:
        raise ValidationError(
            "runtime review summary contradicts decision safety",
            path=path,
        )
    expected_flag_counts = dict(
        sorted(Counter(flag for row in unsafe.values() for flag in row.get("flags", ())).items())
    )
    if summary.get("unsafe_flag_counts") != expected_flag_counts:
        raise ValidationError(
            "runtime review unsafe-flag summary is inconsistent",
            path=path,
        )


def _validate_official_safety_consistency(
    review: Mapping[str, Any],
    decisions: Mapping[tuple[str, str], Mapping[str, Any]],
    *,
    path: Path,
) -> None:
    counts: Counter[str] = Counter()
    unsafe_keys: set[str] = set()
    uncertain_keys: set[str] = set()
    for key, decision in decisions.items():
        safe = decision.get("safe_to_approve")
        selection = decision.get("selection_safety")
        unsafe = decision.get("unsafe")
        uncertain = decision.get("uncertain")
        if (
            not isinstance(safe, bool)
            or selection not in {"safe", "unsafe", "uncertain"}
            or not isinstance(unsafe, bool)
            or not isinstance(uncertain, bool)
            or safe is not (selection == "safe")
            or unsafe is not (selection == "unsafe")
            or uncertain is not (selection == "uncertain")
        ):
            raise ValidationError(
                "official-printing safety fields are contradictory",
                path=path,
                card_id=key[0],
                field_path=key[1],
            )
        if unsafe and decision.get("expected_value") is None:
            raise ValidationError(
                "unsafe official-printing decision requires an expected value",
                path=path,
                card_id=key[0],
                field_path=key[1],
            )
        if (
            unsafe
            and "expected_value_if_unsafe" in decision
            and decision.get("expected_value_if_unsafe")
            != decision.get("expected_value")
        ):
            raise ValidationError(
                "unsafe official-printing correction values are contradictory",
                path=path,
                card_id=key[0],
                field_path=key[1],
            )
        if safe and decision.get("expected_value") is not None:
            raise ValidationError(
                "safe official-printing decision cannot carry a correction value",
                path=path,
                card_id=key[0],
                field_path=key[1],
            )
        counts[str(selection)] += 1
        rendered = f"{key[0]}:{key[1]}"
        if unsafe:
            unsafe_keys.add(rendered)
        if uncertain:
            uncertain_keys.add(rendered)
    summary = review.get("summary")
    if not isinstance(summary, Mapping):
        raise ValidationError(
            "official-printing review summary must be an object", path=path
        )
    if summary.get("by_selection_safety") != dict(sorted(counts.items())):
        raise ValidationError(
            "official-printing summary contradicts decision safety",
            path=path,
        )
    aggregate_counts = {
        "safe_decision_count": counts.get("safe", 0),
        "unsafe_decision_count": counts.get("unsafe", 0),
        "unresolved_decision_count": counts.get("uncertain", 0),
    }
    for field, expected in aggregate_counts.items():
        actual = review.get(field)
        if (
            not isinstance(actual, int)
            or isinstance(actual, bool)
            or actual != expected
        ):
            raise ValidationError(
                "official-printing aggregate safety count is inconsistent",
                path=path,
                details={"field": field},
            )
    unsafe_rows = summary.get("unsafe_rows")
    uncertain_rows = summary.get("uncertain_rows")
    if (
        not isinstance(unsafe_rows, list)
        or len(unsafe_rows) != len(set(unsafe_rows))
        or set(unsafe_rows) != unsafe_keys
        or not isinstance(uncertain_rows, list)
        or len(uncertain_rows) != len(set(uncertain_rows))
        or set(uncertain_rows) != uncertain_keys
    ):
        raise ValidationError(
            "official-printing unsafe/uncertain index is inconsistent",
            path=path,
        )


def _runtime_review_batch(
    *,
    candidate_report_path: Path,
    review_path: Path,
    reviewer: str,
    reviewed_at: str,
) -> ReviewedAdjudicationBatch:
    """Validate a sealed runtime review and return its explicitly safe rows."""

    report, candidate_report_sha256 = _load_sealed_object(candidate_report_path)
    review, _review_sha256 = _load_sealed_object(review_path)
    _validate_runtime_review_dependencies(
        review,
        review_path=review_path,
        candidate_report_sha256=candidate_report_sha256,
    )
    if review.get("review_kind") != RUNTIME_REVIEW_KIND:
        raise ValidationError("unexpected runtime review kind", path=review_path)
    if review.get("source_report_sha256") != candidate_report_sha256:
        raise ValidationError(
            "runtime review is not bound to the exact candidate report",
            path=review_path,
        )
    for field in ("candidate_canonical_revision", "source_manifest_digest"):
        if review.get(field) != report.get(field):
            raise ValidationError(
                "runtime review revision does not match candidate report",
                path=review_path,
                details={"field": field},
            )

    report_rows = _unique_rows(
        report.get("unapproved_runtime_field_diffs"),
        path=candidate_report_path,
        label="candidate runtime differences",
    )
    decisions = _unique_rows(
        review.get("decisions"), path=review_path, label="runtime decisions"
    )
    if review.get("decision_count") != len(decisions):
        raise ValidationError("runtime review decision count is inconsistent", path=review_path)
    if decisions.keys() != report_rows.keys():
        raise ValidationError(
            "runtime review does not cover the exact candidate difference set",
            path=review_path,
            details={
                "missing": sorted(set(report_rows) - set(decisions)),
                "unexpected": sorted(set(decisions) - set(report_rows)),
            },
        )
    _validate_runtime_safety_consistency(review, decisions, path=review_path)

    rows: list[MigrationAdjudication] = []
    for key in sorted(decisions):
        decision = decisions[key]
        candidate = report_rows[key]
        for field in ("old_digest", "new_digest", "old_value", "new_value"):
            if decision.get(field) != candidate.get(field):
                raise ValidationError(
                    "runtime decision does not match its candidate row",
                    path=review_path,
                    card_id=key[0],
                    field_path=key[1],
                    details={"field": field},
                )
        safe = decision.get("safe_to_approve")
        if not isinstance(safe, bool):
            raise ValidationError(
                "runtime decision requires a boolean safe_to_approve",
                path=review_path,
                card_id=key[0],
                field_path=key[1],
            )
        if not safe:
            continue
        provenance = candidate.get("provenance_observation_ids")
        if not isinstance(provenance, list) or not provenance:
            raise ValidationError(
                "a safe runtime decision requires candidate provenance",
                path=review_path,
                card_id=key[0],
                field_path=key[1],
            )
        category = str(decision.get("category") or "reviewed semantic change")
        rows.append(
            MigrationAdjudication(
                schema_version=SCHEMA_VERSION,
                adjudication_id=(
                    f"runtime-cutover-{key[0].lower()}-{key[1].replace('_', '-')}"
                ),
                kind=AdjudicationKind.RUNTIME_FIELD_CUTOVER,
                card_id=key[0],
                field_path=key[1],
                old_input_revision=str(report["old_input_revision"]),
                new_input_revision=str(report["new_input_revision"]),
                old_digest=str(candidate["old_digest"]),
                new_digest=str(candidate["new_digest"]),
                old_value=candidate.get("old_value"),
                new_value=candidate.get("new_value"),
                provenance_observation_ids=tuple(str(item) for item in provenance),
                reason=f"Reviewed runtime migration: {category}",
                reviewer=reviewer,
                reviewed_at=reviewed_at,
            )
        )
    return ReviewedAdjudicationBatch(
        kind=AdjudicationKind.RUNTIME_FIELD_CUTOVER,
        targets=frozenset(decisions),
        adjudications=tuple(rows),
    )


def runtime_adjudications_from_review(
    *,
    candidate_report_path: Path,
    review_path: Path,
    reviewer: str,
    reviewed_at: str,
) -> tuple[MigrationAdjudication, ...]:
    """Validate a sealed runtime review and return its explicitly safe rows."""

    return _runtime_review_batch(
        candidate_report_path=candidate_report_path,
        review_path=review_path,
        reviewer=reviewer,
        reviewed_at=reviewed_at,
    ).adjudications


def _official_printing_review_batch(
    *,
    candidate_report_path: Path,
    review_path: Path,
    reviewer: str,
    reviewed_at: str,
) -> ReviewedAdjudicationBatch:
    """Validate a sealed printing review and return its explicitly safe rows."""

    report, candidate_report_sha256 = _load_sealed_object(candidate_report_path)
    review, _review_sha256 = _load_sealed_object(review_path)
    if review.get("schema_version") != SCHEMA_VERSION:
        raise ValidationError("unsupported semantic review schema", path=review_path)
    if review.get("source_report_sha256") != candidate_report_sha256:
        raise ValidationError(
            "official-printing review is not bound to the exact candidate report",
            path=review_path,
        )
    scope = review.get("scope")
    if not isinstance(scope, Mapping) or (
        scope.get("conflict_kind") != "official_printing"
        or scope.get("approval_required") is not True
        or scope.get("status") != "missing"
    ):
        raise ValidationError("unexpected official-printing review scope", path=review_path)
    for field in ("candidate_canonical_revision", "source_manifest_digest"):
        if review.get(field) != report.get(field):
            raise ValidationError(
                "official-printing review revision does not match candidate report",
                path=review_path,
                details={"field": field},
            )

    raw_conflicts = report.get("canonical_conflicts")
    if not isinstance(raw_conflicts, list):
        raise ValidationError(
            "candidate canonical conflicts must be an array",
            path=candidate_report_path,
        )
    required = [
        row
        for row in raw_conflicts
        if isinstance(row, Mapping)
        and row.get("conflict_kind") == "official_printing"
        and row.get("approval_required") is True
        and row.get("status") == "missing"
    ]
    report_rows = _unique_rows(
        required,
        path=candidate_report_path,
        label="approval-required official-printing conflicts",
    )
    decisions = _unique_rows(
        review.get("decisions"),
        path=review_path,
        label="official-printing decisions",
    )
    if review.get("decision_count") != len(decisions):
        raise ValidationError(
            "official-printing review decision count is inconsistent",
            path=review_path,
        )
    if decisions.keys() != report_rows.keys():
        raise ValidationError(
            "official-printing review does not cover the exact conflict set",
            path=review_path,
            details={
                "missing": sorted(set(report_rows) - set(decisions)),
                "unexpected": sorted(set(decisions) - set(report_rows)),
            },
        )
    _validate_official_safety_consistency(review, decisions, path=review_path)

    rows: list[MigrationAdjudication] = []
    for key in sorted(decisions):
        decision = decisions[key]
        candidate = report_rows[key]
        template = candidate.get("adjudication_template")
        if not isinstance(template, Mapping):
            raise ValidationError(
                "approval-required conflict lacks an adjudication template",
                path=candidate_report_path,
                card_id=key[0],
                field_path=key[1],
            )
        expected = {
            "card_id": key[0],
            "field_path": key[1],
            "new_value": candidate.get("selected_value"),
            "new_digest": template.get("new_digest"),
            "selected_observation_id": template.get("selected_observation_id"),
        }
        actual = {
            "card_id": decision.get("card_id"),
            "field_path": decision.get("field_path"),
            "new_value": decision.get("new_value"),
            "new_digest": decision.get("new_value_digest"),
            "selected_observation_id": decision.get("selected_observation_id"),
        }
        if actual != expected or template.get("new_value") != expected["new_value"]:
            raise ValidationError(
                "official-printing decision does not match its candidate template",
                path=review_path,
                card_id=key[0],
                field_path=key[1],
            )
        safe = decision.get("safe_to_approve")
        if not isinstance(safe, bool):
            raise ValidationError(
                "official-printing decision requires a boolean safe_to_approve",
                path=review_path,
                card_id=key[0],
                field_path=key[1],
            )
        correction_backed = False
        if not safe:
            # A printing-selection adjudication records the exact deterministic
            # source choice.  It does not make that selected value the final
            # canonical value: a reviewed sparse correction may supersede it.
            # Import an unsafe selection only when this exact candidate proves
            # that such a correction is current and its effective value is the
            # reviewer's expected printed value.  Otherwise leave the conflict
            # unresolved so candidate publication remains blocked.
            correction_backed = bool(
                decision.get("selection_safety") == "unsafe"
                and candidate.get("correction_applied") is True
                and candidate.get("canonical_value_after_corrections")
                == decision.get("expected_value")
            )
            if not correction_backed:
                continue
        provenance = template.get("provenance_observation_ids")
        if not isinstance(provenance, list) or not provenance:
            raise ValidationError(
                "a safe official-printing decision requires candidate provenance",
                path=review_path,
                card_id=key[0],
                field_path=key[1],
            )
        classification = str(
            decision.get("classification") or "reviewed official printing selection"
        )
        reason_prefix = (
            "Reviewed deterministic official printing selection, superseded by "
            "a current reviewed correction"
            if correction_backed
            else "Reviewed official printing selection"
        )
        rows.append(
            MigrationAdjudication(
                schema_version=SCHEMA_VERSION,
                adjudication_id=(
                    f"canonical-selection-{key[0].lower()}-{key[1].replace('_', '-')}"
                ),
                kind=AdjudicationKind.CANONICAL_FIELD_SELECTION,
                card_id=key[0],
                field_path=key[1],
                selected_observation_id=str(template["selected_observation_id"]),
                old_input_revision=str(template["old_input_revision"]),
                new_input_revision=str(template["new_input_revision"]),
                old_digest=str(template["old_digest"]),
                new_digest=str(template["new_digest"]),
                old_value=candidate.get("old_value"),
                new_value=template.get("new_value"),
                provenance_observation_ids=tuple(str(item) for item in provenance),
                reason=f"{reason_prefix}: {classification}",
                reviewer=reviewer,
                reviewed_at=reviewed_at,
            )
        )
    return ReviewedAdjudicationBatch(
        kind=AdjudicationKind.CANONICAL_FIELD_SELECTION,
        targets=frozenset(decisions),
        adjudications=tuple(rows),
    )


def official_printing_adjudications_from_review(
    *,
    candidate_report_path: Path,
    review_path: Path,
    reviewer: str,
    reviewed_at: str,
) -> tuple[MigrationAdjudication, ...]:
    """Return safe selections plus unsafe selections superseded by corrections."""

    return _official_printing_review_batch(
        candidate_report_path=candidate_report_path,
        review_path=review_path,
        reviewer=reviewer,
        reviewed_at=reviewed_at,
    ).adjudications


def reconcile_reviewed_adjudications(
    existing: Sequence[MigrationAdjudication],
    batches: Sequence[ReviewedAdjudicationBatch],
) -> tuple[MigrationAdjudication, ...]:
    """Replace every reviewed target so a later unsafe decision revokes approval."""

    reviewed_targets = {
        (batch.kind, card_id, field_path)
        for batch in batches
        for card_id, field_path in batch.targets
    }
    retained = tuple(
        row
        for row in existing
        if (row.kind, row.card_id, row.field_path) not in reviewed_targets
    )
    reviewed = tuple(
        adjudication
        for batch in batches
        for adjudication in batch.adjudications
    )
    return merge_migration_adjudications(retained, reviewed)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="materialize sealed semantic-review decisions"
    )
    parser.add_argument("--candidate-report", type=Path, required=True)
    parser.add_argument("--runtime-review", type=Path)
    parser.add_argument("--official-printing-review", type=Path)
    parser.add_argument("--ledger", type=Path, required=True)
    parser.add_argument("--reviewer", required=True)
    parser.add_argument("--reviewed-at", required=True)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--recover-stale-lock", action="store_true")
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    if args.runtime_review is None and args.official_printing_review is None:
        raise ValidationError("at least one sealed semantic review is required")
    runtime_batch = (
        _runtime_review_batch(
            candidate_report_path=args.candidate_report,
            review_path=args.runtime_review,
            reviewer=args.reviewer,
            reviewed_at=args.reviewed_at,
        )
        if args.runtime_review is not None
        else None
    )
    printing_batch = (
        _official_printing_review_batch(
            candidate_report_path=args.candidate_report,
            review_path=args.official_printing_review,
            reviewer=args.reviewer,
            reviewed_at=args.reviewed_at,
        )
        if args.official_printing_review is not None
        else None
    )
    batches = tuple(
        batch for batch in (runtime_batch, printing_batch) if batch is not None
    )
    root = args.repo_root.resolve()
    ledger = args.ledger if args.ledger.is_absolute() else root / args.ledger
    ledger = ledger.resolve()
    if not ledger.is_relative_to(root):
        raise ValidationError("migration ledger must stay within the repository", path=ledger)
    with standalone_output_write_guard(
        root,
        (ledger,),
        marker_path=root / "data/card_data_revision.json",
        journal_path=root / "data/.card_data_transaction.json",
        recover_stale_lock=args.recover_stale_lock,
    ):
        existing = load_migration_adjudications(ledger)
        merged = reconcile_reviewed_adjudications(existing, batches)
        write_migration_adjudications(ledger, merged)
    runtime_rows = runtime_batch.adjudications if runtime_batch is not None else ()
    printing_rows = printing_batch.adjudications if printing_batch is not None else ()
    print(
        json.dumps(
            {
                "imported_runtime_adjudications": len(runtime_rows),
                "imported_official_printing_adjudications": len(printing_rows),
                "ledger_adjudication_count": len(merged),
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
