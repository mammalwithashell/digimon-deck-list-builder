"""Load and validate reviewed migration adjudication ledgers."""

from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any, Iterable, Mapping

from .errors import AdjudicationError, ValidationError, WriteError
from .models import SCHEMA_VERSION, AdjudicationKind, MigrationAdjudication
from .serialization import canonical_json_text


def _row(payload: Mapping[str, Any], *, path: Path) -> MigrationAdjudication:
    try:
        return MigrationAdjudication(
            schema_version=int(payload["schema_version"]),
            adjudication_id=str(payload["adjudication_id"]),
            kind=AdjudicationKind(str(payload["kind"])),
            card_id=str(payload["card_id"]),
            field_path=(
                str(payload["field_path"])
                if payload.get("field_path") is not None
                else None
            ),
            selected_observation_id=(
                str(payload["selected_observation_id"])
                if payload.get("selected_observation_id") is not None
                else None
            ),
            old_input_revision=str(payload["old_input_revision"]),
            new_input_revision=str(payload["new_input_revision"]),
            old_digest=str(payload["old_digest"]),
            new_digest=str(payload["new_digest"]),
            old_value=payload.get("old_value"),
            new_value=payload.get("new_value"),
            provenance_observation_ids=tuple(
                str(value) for value in payload.get("provenance_observation_ids", ())
            ),
            affected_deck_ids=tuple(
                str(value) for value in payload.get("affected_deck_ids", ())
            ),
            reason=str(payload["reason"]),
            reviewer=str(payload["reviewer"]),
            reviewed_at=str(payload["reviewed_at"]),
        )
    except (KeyError, TypeError, ValueError) as exc:
        raise AdjudicationError(
            "migration adjudication row is invalid",
            path=path,
            details={"error": str(exc)},
        ) from exc


def load_migration_adjudications(path: Path) -> tuple[MigrationAdjudication, ...]:
    """Load a deterministic reviewed ledger; an absent pre-cutover file is empty."""

    if not path.exists():
        return ()
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ValidationError(
            "could not read migration adjudication ledger",
            path=path,
            details={"error": str(exc)},
        ) from exc
    if not isinstance(payload, Mapping) or payload.get("schema_version") != SCHEMA_VERSION:
        raise AdjudicationError(
            "unsupported migration adjudication ledger schema",
            path=path,
        )
    raw_rows = payload.get("adjudications", ())
    if not isinstance(raw_rows, list):
        raise AdjudicationError(
            "migration adjudication ledger requires an adjudications array",
            path=path,
        )
    rows: list[MigrationAdjudication] = []
    for raw in raw_rows:
        if not isinstance(raw, Mapping):
            raise AdjudicationError(
                "migration adjudication row must be an object",
                path=path,
            )
        rows.append(_row(raw, path=path))
    return validate_migration_adjudications(rows, path=path)


def validate_migration_adjudications(
    adjudications: Iterable[MigrationAdjudication],
    *,
    path: Path | None = None,
) -> tuple[MigrationAdjudication, ...]:
    """Validate identities and return the ledger's deterministic row order."""

    rows = tuple(
        sorted(
            adjudications,
            key=lambda row: (
                row.kind.value,
                row.card_id,
                row.field_path or "",
                row.adjudication_id,
            ),
        )
    )
    ids = [row.adjudication_id for row in rows]
    if len(ids) != len(set(ids)):
        raise AdjudicationError(
            "migration adjudication IDs must be unique",
            path=path,
        )
    return rows


def merge_migration_adjudications(
    existing: Iterable[MigrationAdjudication],
    reviewed: Iterable[MigrationAdjudication],
) -> tuple[MigrationAdjudication, ...]:
    """Upsert re-reviewed rows without allowing an identity to be reassigned."""

    merged = {row.adjudication_id: row for row in existing}
    for row in reviewed:
        prior = merged.get(row.adjudication_id)
        if prior is not None and (
            prior.kind is not row.kind
            or prior.card_id != row.card_id
            or prior.field_path != row.field_path
        ):
            raise AdjudicationError(
                "an adjudication ID cannot be reassigned to another target",
                card_id=row.card_id,
                field_path=row.field_path,
                details={"adjudication_id": row.adjudication_id},
            )
        merged[row.adjudication_id] = row
    return validate_migration_adjudications(merged.values())


def write_migration_adjudications(
    path: Path,
    adjudications: Iterable[MigrationAdjudication],
) -> None:
    """Atomically write a validated, deterministic migration ledger."""

    rows = validate_migration_adjudications(adjudications, path=path)
    temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        temporary.write_text(
            canonical_json_text(
                {"schema_version": SCHEMA_VERSION, "adjudications": rows}
            ),
            encoding="utf-8",
        )
        os.replace(temporary, path)
    except OSError as exc:
        try:
            temporary.unlink(missing_ok=True)
        except OSError:
            pass
        raise WriteError(
            "failed to write migration adjudication ledger",
            path=path,
            details={"error": str(exc)},
        ) from exc
