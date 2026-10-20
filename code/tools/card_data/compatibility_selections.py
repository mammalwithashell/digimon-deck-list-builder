"""Versioned, evidence-bound reviews for compatibility-only official conditions."""
from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime
import json
from pathlib import Path
import re
from types import MappingProxyType
from typing import Any, Iterable, Mapping

from .errors import AdjudicationError
from .models import CARD_ID_RE
from .serialization import content_digest

SELECTION_LEDGER_PATH = "data/card_compatibility_selections.json"
SELECTION_SCHEMA_VERSION = 1
_SHA256 = re.compile(r"[0-9a-f]{64}\Z")


@dataclass(frozen=True, slots=True)
class CompatibilitySourceSelection:
    schema_version: int
    selection_id: str
    selection_kind: str
    card_id: str
    field_path: str
    source_manifest_revision: str
    candidate_values: Mapping[str, str]
    candidate_set_digest: str
    selected_observation_id: str
    selected_value: str
    selected_value_digest: str
    reviewer: str
    reason: str
    reviewed_at: str

    def __post_init__(self) -> None:
        if (type(self.schema_version) is not int or self.schema_version != SELECTION_SCHEMA_VERSION
                or self.selection_kind != "official_condition_projection" or self.field_path != "xros_req"):
            raise AdjudicationError("unsupported compatibility source selection schema or target")
        for name in ("selection_id", "reviewer", "reason", "reviewed_at"):
            if not isinstance(getattr(self, name), str) or not getattr(self, name).strip():
                raise AdjudicationError(f"compatibility selection requires {name}")
        if (not isinstance(self.card_id, str) or self.card_id != self.card_id.upper()
                or not CARD_ID_RE.fullmatch(self.card_id)):
            raise AdjudicationError("compatibility selection has invalid card identity")
        try:
            datetime.fromisoformat(self.reviewed_at.replace("Z", "+00:00"))
        except ValueError as exc:
            raise AdjudicationError("compatibility selection has invalid reviewed_at") from exc
        for name in ("source_manifest_revision", "candidate_set_digest", "selected_observation_id", "selected_value_digest"):
            if not isinstance(getattr(self, name), str) or not _SHA256.fullmatch(getattr(self, name)):
                raise AdjudicationError(f"compatibility selection has invalid {name}")
        if not isinstance(self.candidate_values, Mapping) or not self.candidate_values:
            raise AdjudicationError("compatibility selection requires the complete candidate value set")
        if any(not isinstance(oid, str) or not _SHA256.fullmatch(oid) or not isinstance(value, str)
                for oid, value in self.candidate_values.items()):
            raise AdjudicationError("compatibility selection has invalid candidate values")
        if content_digest(self.candidate_values) != self.candidate_set_digest:
            raise AdjudicationError("compatibility selection candidate set digest mismatch")
        if (self.selected_observation_id not in self.candidate_values
                or self.candidate_values[self.selected_observation_id] != self.selected_value
                or content_digest(self.selected_value) != self.selected_value_digest):
            raise AdjudicationError("compatibility selection selected evidence/value mismatch")
        object.__setattr__(self, "candidate_values", MappingProxyType(dict(sorted(self.candidate_values.items()))))


def validate_compatibility_selections(rows: Iterable[CompatibilitySourceSelection]) -> tuple[CompatibilitySourceSelection, ...]:
    ordered = tuple(sorted(rows, key=lambda row: (row.card_id, row.field_path, row.selection_id)))
    if len({row.selection_id for row in ordered}) != len(ordered):
        raise AdjudicationError("compatibility selection IDs must be unique")
    return ordered


def selection_ledger_payload(rows: Iterable[CompatibilitySourceSelection]) -> dict[str, Any]:
    return {"schema_version": SELECTION_SCHEMA_VERSION, "selections": validate_compatibility_selections(rows)}


def selections_from_payload(payload: Any) -> tuple[CompatibilitySourceSelection, ...]:
    if (not isinstance(payload, Mapping) or set(payload) != {"schema_version", "selections"}
            or type(payload["schema_version"]) is not int or payload["schema_version"] != SELECTION_SCHEMA_VERSION
            or not isinstance(payload["selections"], list)):
        raise AdjudicationError("invalid compatibility selection ledger schema")
    try:
        return validate_compatibility_selections(CompatibilitySourceSelection(**row) for row in payload["selections"])
    except (TypeError, KeyError, ValueError) as exc:
        raise AdjudicationError("invalid compatibility selection ledger row", details={"error": str(exc)}) from exc


def _unique_object(pairs):
    value = {}
    for key, item in pairs:
        if key in value:
            raise AdjudicationError("duplicate key in compatibility selection ledger", details={"key": key})
        value[key] = item
    return value


def selections_from_bytes(raw: bytes) -> tuple[CompatibilitySourceSelection, ...]:
    try:
        return selections_from_payload(json.loads(raw.decode("utf-8"), object_pairs_hook=_unique_object))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise AdjudicationError("invalid compatibility selection ledger JSON", details={"error": str(exc)}) from exc


def load_compatibility_selections(path: Path) -> tuple[CompatibilitySourceSelection, ...]:
    return selections_from_bytes(path.read_bytes()) if path.exists() else ()
