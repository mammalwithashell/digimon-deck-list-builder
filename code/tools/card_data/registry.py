"""Append-only global card-index registry and compatibility derivations."""

from __future__ import annotations

import json
import os
import uuid
from dataclasses import dataclass
from pathlib import Path
from types import MappingProxyType
from typing import Any, Iterable, Mapping

from .errors import ValidationError, WriteError
from .models import CARD_ID_RE, SCHEMA_VERSION
from .serialization import canonical_json_bytes, natural_card_id_key


REGISTRY_VERSION = 1
REGISTRY_CAPACITY = 20_000
PADDING_INDEX = 0
_SYNTHETIC_PREFIXES = frozenset({"TEST", "TST", "TOKEN"})


class RegistryValidationError(ValidationError, ValueError):
    """Raised when registry or derived compatibility data is invalid."""


def strict_json_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    """JSON object hook that refuses duplicate keys instead of overwriting."""

    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise RegistryValidationError(f"duplicate JSON object key: {key}")
        result[key] = value
    return result


def _canonical_card_id(card_id: str) -> str:
    if not isinstance(card_id, str) or not CARD_ID_RE.fullmatch(card_id.upper()):
        raise RegistryValidationError(f"invalid card ID for stable registry: {card_id!r}")
    canonical = card_id.upper()
    if card_id != canonical:
        raise RegistryValidationError(
            f"stable registry card IDs must be uppercase: {card_id!r}"
        )
    return canonical


def is_gameplay_card_id(card_id: str) -> bool:
    """Return whether an ID is eligible for a new production assignment."""

    match = CARD_ID_RE.fullmatch(card_id.upper()) if isinstance(card_id, str) else None
    return bool(match and match.group("prefix") not in _SYNTHETIC_PREFIXES)


def numeric_card_suffix(card_id: str) -> int:
    """Derive the intentionally non-global ``card_index`` value."""

    match = CARD_ID_RE.fullmatch(card_id.upper()) if isinstance(card_id, str) else None
    if match is None:
        raise RegistryValidationError(
            f"cannot derive numeric card suffix from {card_id!r}"
        )
    return int(match.group("card_no"))


@dataclass(frozen=True, slots=True)
class StableCardRegistry:
    schema_version: int
    registry_version: int
    capacity: int
    padding_index: int
    assignments: Mapping[str, int]

    def __post_init__(self) -> None:
        if self.schema_version != SCHEMA_VERSION:
            raise RegistryValidationError(
                f"unsupported registry schema_version {self.schema_version!r}"
            )
        if self.registry_version != REGISTRY_VERSION:
            raise RegistryValidationError(
                f"unsupported registry_version {self.registry_version!r}"
            )
        if self.capacity != REGISTRY_CAPACITY:
            raise RegistryValidationError(
                f"registry capacity is fixed at {REGISTRY_CAPACITY}"
            )
        if self.padding_index != PADDING_INDEX:
            raise RegistryValidationError("registry padding index is fixed at 0")
        if not isinstance(self.assignments, Mapping):
            raise RegistryValidationError("registry assignments must be a mapping")

        normalized: dict[str, int] = {}
        index_to_id: dict[int, str] = {}
        for card_id, index in self.assignments.items():
            card_id = _canonical_card_id(card_id)
            if not isinstance(index, int) or isinstance(index, bool):
                raise RegistryValidationError(
                    f"registry index for {card_id} must be an integer"
                )
            if index <= self.padding_index or index > self.capacity:
                raise RegistryValidationError(
                    f"registry index for {card_id} is outside 1..{self.capacity}"
                )
            existing = index_to_id.get(index)
            if existing is not None:
                raise RegistryValidationError(
                    f"duplicate global index {index} assigned to {existing} and {card_id}"
                )
            normalized[card_id] = index
            index_to_id[index] = card_id

        object.__setattr__(
            self,
            "assignments",
            MappingProxyType(
                dict(sorted(normalized.items(), key=lambda item: natural_card_id_key(item[0])))
            ),
        )

    @property
    def maximum_index(self) -> int:
        return max(self.assignments.values(), default=self.padding_index)

    @property
    def next_index(self) -> int:
        return self.maximum_index + 1

    def to_dict(self) -> dict[str, Any]:
        return {
            "schema_version": self.schema_version,
            "registry_version": self.registry_version,
            "capacity": self.capacity,
            "padding_index": self.padding_index,
            "assignment_count": len(self.assignments),
            "assignments": dict(self.assignments),
        }

    @classmethod
    def from_dict(cls, payload: Mapping[str, Any]) -> "StableCardRegistry":
        registry = cls(
            schema_version=payload["schema_version"],
            registry_version=payload["registry_version"],
            capacity=payload["capacity"],
            padding_index=payload["padding_index"],
            assignments=payload["assignments"],
        )
        declared_count = payload.get("assignment_count")
        if declared_count != len(registry.assignments):
            raise RegistryValidationError(
                "registry assignment_count does not match assignments"
            )
        return registry

    @classmethod
    def from_cards_json(
        cls, cards: Mapping[str, Mapping[str, Any]]
    ) -> "StableCardRegistry":
        """Losslessly extract the legacy global mapping from ``cards.json``."""

        if not isinstance(cards, Mapping) or not cards:
            raise RegistryValidationError("cards.json must be a non-empty mapping")
        assignments: dict[str, int] = {}
        for card_id, row in cards.items():
            card_id = _canonical_card_id(card_id)
            if not isinstance(row, Mapping):
                raise RegistryValidationError(f"cards.json row {card_id} must be an object")
            if row.get("card_id") != card_id:
                raise RegistryValidationError(
                    f"cards.json key/card_id mismatch for {card_id}"
                )
            index = row.get("index")
            if not isinstance(index, int) or isinstance(index, bool) or index <= 0:
                raise RegistryValidationError(
                    f"cards.json row {card_id} has no stable positive index"
                )
            assignments[card_id] = index
        return cls(
            schema_version=SCHEMA_VERSION,
            registry_version=REGISTRY_VERSION,
            capacity=REGISTRY_CAPACITY,
            padding_index=PADDING_INDEX,
            assignments=assignments,
        )


def allocate_append_only(
    registry: StableCardRegistry, card_ids: Iterable[str]
) -> StableCardRegistry:
    """Preserve every assignment and allocate new gameplay IDs after max index."""

    requested = list(card_ids)
    canonical: list[str] = []
    seen: set[str] = set()
    for raw_card_id in requested:
        card_id = _canonical_card_id(raw_card_id)
        if card_id in seen:
            raise RegistryValidationError(
                f"duplicate card ID in allocation request: {card_id}"
            )
        seen.add(card_id)
        canonical.append(card_id)

    assignments = dict(registry.assignments)
    new_ids = sorted(
        (card_id for card_id in canonical if card_id not in assignments),
        key=natural_card_id_key,
    )
    for card_id in new_ids:
        if not is_gameplay_card_id(card_id):
            raise RegistryValidationError(
                f"new synthetic/test ID cannot receive a global assignment: {card_id}"
            )
        next_index = max(assignments.values(), default=PADDING_INDEX) + 1
        if next_index > registry.capacity:
            raise RegistryValidationError(
                f"global registry capacity {registry.capacity} is exhausted"
            )
        assignments[card_id] = next_index

    candidate = StableCardRegistry(
        schema_version=registry.schema_version,
        registry_version=registry.registry_version,
        capacity=registry.capacity,
        padding_index=registry.padding_index,
        assignments=assignments,
    )
    validate_append_only(registry, candidate)
    return candidate


def validate_append_only(
    previous: StableCardRegistry, candidate: StableCardRegistry
) -> None:
    """Reject deletion, reassignment, index reuse, gaps, or non-deterministic adds."""

    if (
        previous.schema_version,
        previous.registry_version,
        previous.capacity,
        previous.padding_index,
    ) != (
        candidate.schema_version,
        candidate.registry_version,
        candidate.capacity,
        candidate.padding_index,
    ):
        raise RegistryValidationError("registry compatibility contract cannot change")

    for card_id, old_index in previous.assignments.items():
        new_index = candidate.assignments.get(card_id)
        if new_index is None:
            raise RegistryValidationError(
                f"append-only registry cannot remove assignment for {card_id}"
            )
        if new_index != old_index:
            raise RegistryValidationError(
                f"append-only registry cannot reassign {card_id}: {old_index} -> {new_index}"
            )

    additions = {
        card_id: index
        for card_id, index in candidate.assignments.items()
        if card_id not in previous.assignments
    }
    synthetic_additions = sorted(
        card_id for card_id in additions if not is_gameplay_card_id(card_id)
    )
    if synthetic_additions:
        raise RegistryValidationError(
            "new synthetic/test IDs cannot receive global assignments: "
            + ", ".join(synthetic_additions)
        )
    expected_ids = sorted(additions, key=natural_card_id_key)
    expected_indices = list(
        range(previous.maximum_index + 1, previous.maximum_index + 1 + len(additions))
    )
    actual_indices = [additions[card_id] for card_id in expected_ids]
    if actual_indices != expected_indices:
        raise RegistryValidationError(
            "new global indices must be contiguous, append-only, and natural-ID ordered"
        )


def compatibility_values(
    card_id: str, index: int, *, capacity: int = REGISTRY_CAPACITY
) -> dict[str, int | float]:
    if capacity != REGISTRY_CAPACITY:
        raise RegistryValidationError(
            f"registry capacity is fixed at {REGISTRY_CAPACITY}"
        )
    if not isinstance(index, int) or isinstance(index, bool) or not 0 < index <= capacity:
        raise RegistryValidationError("global index is outside the registry capacity")
    return {
        "index": index,
        "norm_id": index / capacity,
        "card_index": numeric_card_suffix(card_id),
    }


def apply_compatibility_values(
    cards: Mapping[str, Mapping[str, Any]],
    registry: StableCardRegistry,
) -> dict[str, dict[str, Any]]:
    """Inject registry-owned values without changing printed card metadata.

    Existing synthetic fixture ``card_index`` exceptions are retained because
    they are part of the characterized ``cards.json`` ABI.  New assignments are
    restricted to gameplay IDs, so no new exception can enter the registry.
    """

    output: dict[str, dict[str, Any]] = {}
    for raw_card_id, raw_row in cards.items():
        card_id = _canonical_card_id(raw_card_id)
        if not isinstance(raw_row, Mapping):
            raise RegistryValidationError(f"cards.json row {card_id} must be an object")
        if raw_row.get("card_id") != card_id:
            raise RegistryValidationError(
                f"cards.json key/card_id mismatch for {card_id}"
            )
        index = registry.assignments.get(card_id)
        if index is None:
            raise RegistryValidationError(
                f"card {card_id} has no global registry assignment"
            )
        existing_index = raw_row.get("index")
        if existing_index not in (None, PADDING_INDEX, index):
            raise RegistryValidationError(
                f"cards.json attempts to reassign {card_id}: {existing_index} -> {index}"
            )

        derived = compatibility_values(card_id, index, capacity=registry.capacity)
        row = dict(raw_row)
        row["index"] = derived["index"]
        row["norm_id"] = derived["norm_id"]
        if is_gameplay_card_id(card_id):
            existing_card_index = row.get("card_index")
            if existing_card_index not in (None, derived["card_index"]):
                raise RegistryValidationError(
                    f"cards.json card_index for {card_id} does not match numeric suffix"
                )
            row["card_index"] = derived["card_index"]
        output[card_id] = row
    return output


def read_registry(path: Path) -> StableCardRegistry:
    try:
        payload = json.loads(
            path.read_text(encoding="utf-8"), object_pairs_hook=strict_json_object
        )
        return StableCardRegistry.from_dict(payload)
    except RegistryValidationError:
        raise
    except (OSError, KeyError, TypeError, ValueError, json.JSONDecodeError) as exc:
        raise RegistryValidationError(
            "stable card registry cannot be read",
            path=path,
            details={"error": str(exc)},
        ) from exc


def write_registry(path: Path, registry: StableCardRegistry) -> None:
    temporary = path.with_name(f".{path.name}.{os.getpid()}.{uuid.uuid4().hex}.tmp")
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        temporary.write_bytes(canonical_json_bytes(registry.to_dict()))
        os.replace(temporary, path)
    except OSError as exc:
        try:
            temporary.unlink(missing_ok=True)
        except OSError:
            pass
        raise WriteError(
            "failed to write stable card registry",
            path=path,
            details={"error": str(exc)},
        ) from exc
