"""Deterministic serialization and hashing for card-data artifacts."""

from __future__ import annotations

import dataclasses
import hashlib
import json
import re
from collections.abc import Mapping
from enum import Enum
from pathlib import Path
from typing import Any, Iterable

from .errors import ValidationError
from .models import CARD_ID_RE, SourceObservation


OPERATIONAL_KEYS = frozenset({"last_checked"})


def natural_card_id_key(value: str) -> tuple[str, int, int, str, str]:
    """Sort canonical IDs by set prefix/number and numeric card suffix."""

    match = CARD_ID_RE.fullmatch(value.upper())
    if not match:
        return (value.upper(), -1, -1, "", value)
    set_no = int(match.group("set_no") or -1)
    card_no = int(match.group("card_no"))
    return (
        match.group("prefix"),
        set_no,
        card_no,
        match.group("suffix"),
        value,
    )


def _looks_like_card_id_map(keys: list[str]) -> bool:
    return bool(keys) and all(CARD_ID_RE.fullmatch(key.upper()) for key in keys)


def canonicalize(value: Any, *, exclude_operational: bool = False) -> Any:
    """Convert supported values into a stable JSON-compatible structure."""

    if dataclasses.is_dataclass(value) and not isinstance(value, type):
        value = {field.name: getattr(value, field.name) for field in dataclasses.fields(value)}
    if isinstance(value, Enum):
        return value.value
    if isinstance(value, Path):
        return value.as_posix()
    if isinstance(value, Mapping):
        keys = [str(key) for key in value]
        if exclude_operational:
            keys = [key for key in keys if key not in OPERATIONAL_KEYS]
        key_fn = natural_card_id_key if _looks_like_card_id_map(keys) else lambda item: item
        return {
            key: canonicalize(value[key], exclude_operational=exclude_operational)
            for key in sorted(keys, key=key_fn)
        }
    if isinstance(value, (tuple, list)):
        return [canonicalize(item, exclude_operational=exclude_operational) for item in value]
    if isinstance(value, (str, int, float, bool)) or value is None:
        return value
    raise TypeError(f"unsupported canonical JSON value: {type(value).__name__}")


def canonical_json_text(value: Any, *, exclude_operational: bool = False) -> str:
    normalized = canonicalize(value, exclude_operational=exclude_operational)
    return json.dumps(normalized, ensure_ascii=False, indent=2, separators=(",", ": ")) + "\n"


def canonical_json_bytes(value: Any, *, exclude_operational: bool = False) -> bytes:
    return canonical_json_text(value, exclude_operational=exclude_operational).encode("utf-8")


def sha256_hex(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def content_digest(value: Any, *, exclude_operational: bool = False) -> str:
    return sha256_hex(
        canonical_json_bytes(value, exclude_operational=exclude_operational)
    )


def observation_digest(observation: SourceObservation) -> str:
    return content_digest(observation.identity_payload(), exclude_operational=True)


def resolve_observation_references(
    observation_ids: Iterable[str],
    observations: Mapping[str, SourceObservation],
) -> tuple[SourceObservation, ...]:
    """Resolve immutable observation references without hiding missing inputs.

    Reference order is retained because scope manifests and provenance records
    are already deterministically ordered by their producers.  Store keys must
    match each object's declared identity so a misplaced content-addressed file
    cannot satisfy a reference accidentally.
    """

    resolved: list[SourceObservation] = []
    for observation_id in observation_ids:
        observation = observations.get(observation_id)
        if observation is None:
            raise ValidationError(
                "source observation reference does not exist",
                details={"observation_id": observation_id},
            )
        if observation.observation_id != observation_id:
            raise ValidationError(
                "source observation store key does not match record identity",
                details={
                    "store_key": observation_id,
                    "record_observation_id": observation.observation_id,
                },
            )
        resolved.append(observation)
    return tuple(resolved)
