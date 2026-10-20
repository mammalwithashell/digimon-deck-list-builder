"""Structured blocker/owner registry with tracker reconciliation checks."""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Mapping

from .errors import ValidationError
from .models import CARD_ID_RE


ALLOWED_OWNER_KINDS = frozenset({"openspec_change", "tracker"})
ALLOWED_BLOCKER_CATEGORIES = frozenset(
    {"engine", "dsl", "engine_dsl", "rules", "data", "test"}
)
ALLOWED_BLOCKER_STATUSES = frozenset({"active", "resolved", "superseded"})
_SYNTHETIC_CARD_PREFIXES = frozenset({"TEST", "TST", "TOKEN"})


def _is_canonical_real_card_id(value: object) -> bool:
    if not isinstance(value, str) or value != value.upper():
        return False
    match = CARD_ID_RE.fullmatch(value)
    return bool(match and match.group("prefix") not in _SYNTHETIC_CARD_PREFIXES)


def _require_mapping(value: object, message: str, *, path: Path | None) -> Mapping[str, Any]:
    if not isinstance(value, Mapping):
        raise ValidationError(message, path=path)
    return value


def _require_non_empty_string(
    value: object,
    message: str,
    *,
    path: Path | None,
    details: Mapping[str, object] | None = None,
) -> str:
    if not isinstance(value, str) or not value.strip():
        raise ValidationError(message, path=path, details=details)
    return value


@dataclass(frozen=True, slots=True)
class BlockerOwner:
    owner_id: str
    kind: str
    reference: str


@dataclass(frozen=True, slots=True)
class Blocker:
    blocker_id: str
    category: str
    status: str
    owner_id: str
    affected_card_ids: tuple[str, ...]
    tracker_refs: tuple[Mapping[str, str], ...]


@dataclass(frozen=True, slots=True)
class BlockerRegistry:
    owners: Mapping[str, BlockerOwner]
    blockers: tuple[Blocker, ...]

    @property
    def active_by_card(self) -> dict[str, tuple[str, ...]]:
        result: dict[str, list[str]] = {}
        for blocker in self.blockers:
            if blocker.status != "active":
                continue
            for card_id in blocker.affected_card_ids:
                result.setdefault(card_id, []).append(blocker.blocker_id)
        return {card_id: tuple(sorted(values)) for card_id, values in sorted(result.items())}

    def reconcile(
        self,
        repo_root: Path,
        *,
        canonical_card_ids: set[str] | frozenset[str] | None = None,
    ) -> None:
        if canonical_card_ids is not None:
            unknown_by_blocker = {
                blocker.blocker_id: tuple(
                    sorted(set(blocker.affected_card_ids) - set(canonical_card_ids))
                )
                for blocker in self.blockers
                if set(blocker.affected_card_ids) - set(canonical_card_ids)
            }
            if unknown_by_blocker:
                raise ValidationError(
                    "blocker registry references cards outside the canonical universe",
                    details={"unknown_affected_card_ids": unknown_by_blocker},
                )
        for owner in self.owners.values():
            if owner.kind == "openspec_change" and not (
                repo_root / "openspec/changes" / owner.reference
            ).is_dir():
                raise ValidationError(
                    "blocker owner references an unknown OpenSpec change",
                    details={"owner_id": owner.owner_id, "reference": owner.reference},
                )
            if owner.kind == "tracker" and not (repo_root / owner.reference).is_file():
                raise ValidationError(
                    "blocker owner tracker does not exist",
                    details={"owner_id": owner.owner_id, "reference": owner.reference},
                )
        for blocker in self.blockers:
            if blocker.owner_id not in self.owners:
                raise ValidationError(
                    "blocker references an unknown owner",
                    details={"blocker_id": blocker.blocker_id, "owner_id": blocker.owner_id},
                )
            for reference in blocker.tracker_refs:
                path = repo_root / reference["path"]
                try:
                    text = path.read_text(encoding="utf-8")
                except OSError as exc:
                    raise ValidationError(
                        "blocker tracker reference is unavailable", path=path
                    ) from exc
                if reference["marker"] not in text:
                    raise ValidationError(
                        "blocker tracker marker no longer reconciles",
                        path=path,
                        details={
                            "blocker_id": blocker.blocker_id,
                            "marker": reference["marker"],
                        },
                    )


def blocker_registry_from_payload(
    payload: Mapping[str, object], *, path: Path | None = None
) -> BlockerRegistry:
    if not isinstance(payload, Mapping):
        raise ValidationError("blocker registry root must be an object", path=path)
    if type(payload.get("schema_version")) is not int or payload.get("schema_version") != 1:
        raise ValidationError("unsupported blocker registry schema version", path=path)
    raw_owners = _require_mapping(
        payload.get("owners"), "blocker registry owners must be an object", path=path
    )
    owners: dict[str, BlockerOwner] = {}
    for raw_owner_id, raw_row in raw_owners.items():
        owner_id = _require_non_empty_string(
            raw_owner_id, "blocker owner IDs must be non-empty strings", path=path
        )
        row = _require_mapping(
            raw_row,
            "blocker owner entry must be an object",
            path=path,
        )
        kind = _require_non_empty_string(
            row.get("kind"),
            "blocker owner kind must be a non-empty string",
            path=path,
            details={"owner_id": owner_id},
        )
        if kind not in ALLOWED_OWNER_KINDS:
            raise ValidationError(
                "blocker owner kind is not supported",
                path=path,
                details={
                    "owner_id": owner_id,
                    "kind": kind,
                    "allowed": sorted(ALLOWED_OWNER_KINDS),
                },
            )
        reference = _require_non_empty_string(
            row.get("reference"),
            "blocker owner reference must be a non-empty string",
            path=path,
            details={"owner_id": owner_id},
        )
        owners[owner_id] = BlockerOwner(owner_id, kind, reference)

    raw_blockers = payload.get("blockers")
    if not isinstance(raw_blockers, list):
        raise ValidationError("blocker registry blockers must be an array", path=path)
    blockers: list[Blocker] = []
    blocker_ids: set[str] = set()
    for raw_row in raw_blockers:
        row = _require_mapping(
            raw_row, "blocker entry must be an object", path=path
        )
        blocker_id = _require_non_empty_string(
            row.get("blocker_id"),
            "blocker_id must be a non-empty string",
            path=path,
        )
        if blocker_id in blocker_ids:
            raise ValidationError(
                "blocker IDs must be unique",
                path=path,
                details={"blocker_id": blocker_id},
            )
        blocker_ids.add(blocker_id)
        category = _require_non_empty_string(
            row.get("category"),
            "blocker category must be a non-empty string",
            path=path,
            details={"blocker_id": blocker_id},
        )
        if category not in ALLOWED_BLOCKER_CATEGORIES:
            raise ValidationError(
                "blocker category is not supported",
                path=path,
                details={
                    "blocker_id": blocker_id,
                    "category": category,
                    "allowed": sorted(ALLOWED_BLOCKER_CATEGORIES),
                },
            )
        status = _require_non_empty_string(
            row.get("status"),
            "blocker status must be a non-empty string",
            path=path,
            details={"blocker_id": blocker_id},
        )
        if status not in ALLOWED_BLOCKER_STATUSES:
            raise ValidationError(
                "blocker status is not supported",
                path=path,
                details={
                    "blocker_id": blocker_id,
                    "status": status,
                    "allowed": sorted(ALLOWED_BLOCKER_STATUSES),
                },
            )
        owner_id = _require_non_empty_string(
            row.get("owner_id"),
            "blocker owner_id must be a non-empty string",
            path=path,
            details={"blocker_id": blocker_id},
        )
        raw_card_ids = row.get("affected_card_ids")
        if (
            not isinstance(raw_card_ids, list)
            or not raw_card_ids
            or not all(_is_canonical_real_card_id(card_id) for card_id in raw_card_ids)
            or len(raw_card_ids) != len(set(raw_card_ids))
        ):
            raise ValidationError(
                "blocker affected_card_ids must be unique canonical real card IDs",
                path=path,
                details={"blocker_id": blocker_id},
            )
        raw_tracker_refs = row.get("tracker_refs")
        if not isinstance(raw_tracker_refs, list):
            raise ValidationError(
                "blocker tracker_refs must be an array",
                path=path,
                details={"blocker_id": blocker_id},
            )
        tracker_refs: list[Mapping[str, str]] = []
        for raw_reference in raw_tracker_refs:
            reference = _require_mapping(
                raw_reference,
                "blocker tracker reference must be an object",
                path=path,
            )
            tracker_path = _require_non_empty_string(
                reference.get("path"),
                "blocker tracker path must be a non-empty string",
                path=path,
                details={"blocker_id": blocker_id},
            )
            marker = _require_non_empty_string(
                reference.get("marker"),
                "blocker tracker marker must be a non-empty string",
                path=path,
                details={"blocker_id": blocker_id},
            )
            tracker_refs.append({"path": tracker_path, "marker": marker})
        blockers.append(
            Blocker(
                blocker_id=blocker_id,
                category=category,
                status=status,
                owner_id=owner_id,
                affected_card_ids=tuple(raw_card_ids),
                tracker_refs=tuple(tracker_refs),
            )
        )
    return BlockerRegistry(owners=owners, blockers=blockers)


def load_blocker_registry(path: Path) -> BlockerRegistry:
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ValidationError(
            "blocker registry is unavailable or invalid JSON",
            path=path,
            details={"error": str(exc)},
        ) from exc
    return blocker_registry_from_payload(payload, path=path)
