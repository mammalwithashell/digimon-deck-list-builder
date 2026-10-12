"""Sparse reviewed correction validation and immutable application."""

from __future__ import annotations

import json
import os
from dataclasses import replace
from pathlib import Path
from typing import Any, Iterable, Mapping

from .errors import ValidationError, WriteError
from .models import (
    AuthorityClass,
    CanonicalCard,
    CardFace,
    FaceKind,
    FieldProvenance,
    ReviewedCorrection,
    SourceObservation,
)
from .serialization import canonical_json_text


CORRECTIONS_DOCUMENT_SCHEMA_VERSION = 1
OPTIONAL_INSERTABLE_ROOT_TYPES: Mapping[str, type] = {
    # Digi-Egg observations correctly omit a play cost.  A reviewed migration
    # correction may nevertheless need to materialize the compatibility value
    # (zero) so that the generated runtime field has explicit provenance.
    "play_cost": int,
}


def _correction_from_dict(value: Any, *, index: int) -> ReviewedCorrection:
    if not isinstance(value, Mapping):
        raise ValidationError(
            "reviewed correction entry must be an object",
            details={"index": index, "actual_type": type(value).__name__},
        )
    required_keys = {
        "schema_version",
        "correction_id",
        "card_id",
        "field_path",
        "value",
        "evidence_observation_ids",
        "reason",
        "reviewer",
        "reviewed_at",
    }
    actual_keys = set(value)
    if actual_keys != required_keys:
        raise ValidationError(
            "reviewed correction entry has missing or unknown fields",
            details={
                "index": index,
                "missing": sorted(required_keys - actual_keys),
                "unknown": sorted(actual_keys - required_keys),
            },
        )
    try:
        evidence = value["evidence_observation_ids"]
        if not isinstance(evidence, list):
            raise TypeError("evidence_observation_ids must be an array")
        return ReviewedCorrection(
            schema_version=value["schema_version"],
            correction_id=value["correction_id"],
            card_id=value["card_id"],
            field_path=value["field_path"],
            value=value.get("value"),
            evidence_observation_ids=tuple(evidence),
            reason=value["reason"],
            reviewer=value["reviewer"],
            reviewed_at=value["reviewed_at"],
        )
    except (KeyError, TypeError, ValueError) as exc:
        if isinstance(exc, ValidationError):
            raise
        raise ValidationError(
            "reviewed correction entry violates the versioned schema",
            details={"index": index, "error": str(exc)},
        ) from exc


def validate_correction_set(
    corrections: Iterable[ReviewedCorrection],
) -> tuple[ReviewedCorrection, ...]:
    """Validate correction IDs and sparse card-field ownership."""

    ordered = tuple(
        sorted(
            corrections,
            key=lambda item: (item.card_id, item.field_path, item.correction_id),
        )
    )
    ids: set[str] = set()
    targets: set[tuple[str, str]] = set()
    for correction in ordered:
        if correction.correction_id in ids:
            raise ValidationError(
                "duplicate reviewed correction ID",
                card_id=correction.card_id,
                field_path=correction.field_path,
                details={"correction_id": correction.correction_id},
            )
        ids.add(correction.correction_id)
        target = (correction.card_id, correction.field_path)
        if target in targets:
            raise ValidationError(
                "multiple reviewed corrections target the same card field",
                card_id=correction.card_id,
                field_path=correction.field_path,
            )
        targets.add(target)
    return ordered


def load_reviewed_corrections(
    path: str | os.PathLike[str] | None = None,
) -> tuple[ReviewedCorrection, ...]:
    """Load only the versioned reviewed sparse-correction document."""

    if path is None:
        from data_paths import CARD_CORRECTIONS

        path = CARD_CORRECTIONS
    source = Path(path)
    if not source.exists():
        return ()
    try:
        payload = json.loads(source.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ValidationError(
            "failed to read reviewed correction document",
            path=source,
            details={"error": str(exc)},
        ) from exc
    if not isinstance(payload, Mapping):
        raise ValidationError("reviewed correction document root must be an object")
    if payload.get("schema_version") != CORRECTIONS_DOCUMENT_SCHEMA_VERSION:
        raise ValidationError(
            "unsupported reviewed correction document schema version",
            path=source,
            details={"schema_version": payload.get("schema_version")},
        )
    entries = payload.get("corrections")
    if not isinstance(entries, list):
        raise ValidationError("reviewed correction document requires a corrections array")
    return validate_correction_set(
        _correction_from_dict(entry, index=index)
        for index, entry in enumerate(entries)
    )


def merge_reviewed_corrections(
    existing: Iterable[ReviewedCorrection],
    reviewed: Iterable[ReviewedCorrection],
) -> tuple[ReviewedCorrection, ...]:
    """Upsert explicitly re-reviewed corrections by stable correction ID."""

    merged = {item.correction_id: item for item in existing}
    for correction in reviewed:
        prior = merged.get(correction.correction_id)
        if prior is not None and (
            prior.card_id != correction.card_id
            or prior.field_path != correction.field_path
        ):
            raise ValidationError(
                "a correction ID cannot be reassigned to another card field",
                card_id=correction.card_id,
                field_path=correction.field_path,
                details={"correction_id": correction.correction_id},
            )
        merged[correction.correction_id] = correction
    return validate_correction_set(merged.values())


def write_reviewed_corrections(
    path: str | os.PathLike[str],
    corrections: Iterable[ReviewedCorrection],
) -> None:
    """Write a validated deterministic reviewed-correction document.

    This is the only supported correction-ledger writer.  It cannot accept the
    legacy ``{card_id: {field: value}}`` override shape because every entry is
    already a typed :class:`ReviewedCorrection` with evidence and review data.
    """

    destination = Path(path)
    if destination.exists():
        # The legacy API has no representation for compatibility corrections.
        # Refuse a complete-document downgrade rather than erase reviewed data.
        from .compatibility_corrections import load_correction_document
        if load_correction_document(destination).compatibility_corrections:
            raise ValidationError('legacy correction writer cannot erase compatibility corrections')
    ordered = validate_correction_set(corrections)
    payload = {
        "schema_version": CORRECTIONS_DOCUMENT_SCHEMA_VERSION,
        "corrections": ordered,
    }
    temporary = destination.with_name(f".{destination.name}.{os.getpid()}.tmp")
    try:
        destination.parent.mkdir(parents=True, exist_ok=True)
        temporary.write_text(canonical_json_text(payload), encoding="utf-8")
        os.replace(temporary, destination)
    except OSError as exc:
        try:
            temporary.unlink(missing_ok=True)
        except OSError:
            pass
        raise WriteError(
            "failed to write reviewed correction document",
            path=destination,
            details={"error": str(exc)},
        ) from exc


def _compatible_type(current: Any, replacement: Any) -> bool:
    if current is None:
        return replacement is None or isinstance(replacement, (str, int, list, dict))
    if isinstance(current, bool):
        return isinstance(replacement, bool)
    if isinstance(current, int):
        return isinstance(replacement, int) and not isinstance(replacement, bool)
    if isinstance(current, tuple):
        return isinstance(replacement, (tuple, list))
    return isinstance(replacement, type(current))


def _validate_missing_root_insertion(
    root: str,
    tail: list[str],
    replacement: Any,
) -> None:
    expected = OPTIONAL_INSERTABLE_ROOT_TYPES.get(root)
    if tail or expected is None:
        raise ValidationError(
            "correction field path does not exist on the canonical card"
        )
    if not isinstance(replacement, expected) or isinstance(replacement, bool):
        raise ValidationError(
            "correction replacement type does not match the optional canonical field"
        )


def _replace_nested(value: Any, segments: list[str], replacement: Any) -> Any:
    if not segments:
        if isinstance(value, tuple) and isinstance(replacement, list):
            replacement = tuple(replacement)
        if not _compatible_type(value, replacement):
            raise ValidationError(
                "correction replacement type does not match the normalized field",
                details={
                    "expected_type": type(value).__name__,
                    "actual_type": type(replacement).__name__,
                },
            )
        if value == replacement:
            raise ValidationError("correction is a no-op and must be removed")
        return replacement

    segment = segments[0]
    tail = segments[1:]
    if isinstance(value, Mapping):
        if segment not in value:
            raise ValidationError("correction field path does not exist")
        copied = dict(value)
        copied[segment] = _replace_nested(value[segment], tail, replacement)
        return copied
    if isinstance(value, (tuple, list)):
        if not segment.isdigit() or int(segment) >= len(value):
            raise ValidationError("correction array index is out of range")
        copied = list(value)
        copied[int(segment)] = _replace_nested(copied[int(segment)], tail, replacement)
        return tuple(copied) if isinstance(value, tuple) else copied
    if isinstance(value, CardFace):
        if not hasattr(value, segment):
            raise ValidationError("correction face field path does not exist")
        return replace(
            value,
            **{segment: _replace_nested(getattr(value, segment), tail, replacement)},
        )
    raise ValidationError("correction field path traverses a scalar value")


def _coerce_face_correction(value: Any) -> tuple[CardFace, ...]:
    if not isinstance(value, (list, tuple)):
        raise ValidationError("faces correction must be an array")
    faces: list[CardFace] = []
    for index, raw in enumerate(value):
        typed_face = isinstance(raw, CardFace)
        allowed = {
            "face_id", "kind", "colors", "play_cost", "level", "dp",
            "effect_text", "inherited_text", "security_text",
        }
        data = {key: getattr(raw, key) for key in allowed} if typed_face else raw
        if not isinstance(data, Mapping):
            raise ValidationError(
                "faces correction entries must be objects",
                details={"index": index},
            )
        try:
            if set(data) - allowed:
                raise TypeError("unknown face correction fields")
            if not isinstance(data["face_id"], str) or not isinstance(data["kind"], str):
                raise TypeError("face_id and kind must be strings")
            kind = FaceKind(data["kind"].lower().replace("-", "_").replace(" ", "_"))
            colors = data["colors"]
            if not isinstance(colors, (list, tuple)) or any(not isinstance(color, str) for color in colors):
                raise TypeError("colors must be an array of strings")
            for name in ("effect_text", "inherited_text", "security_text"):
                if name in data and not isinstance(data[name], str):
                    raise TypeError(f"{name} must be a string")
            for name in ("play_cost", "level", "dp"):
                number = data.get(name)
                if number is not None and (type(number) is not int or number < 0):
                    raise TypeError(f"{name} must be a non-negative integer or null")
            face = CardFace(
                face_id=data["face_id"],
                kind=kind,
                colors=tuple(colors),
                play_cost=data.get("play_cost"),
                level=data.get("level"),
                dp=data.get("dp"),
                effect_text=data.get("effect_text", ""),
                inherited_text=data.get("inherited_text", ""),
                security_text=data.get("security_text", ""),
            )
            faces.append(raw if typed_face and isinstance(raw.kind, FaceKind) else face)
        except (KeyError, TypeError, ValueError) as exc:
            raise ValidationError(
                "faces correction entry is invalid",
                details={"index": index, "error": str(exc)},
            ) from exc
    return tuple(faces)


def _read_nested(value: Any, segments: list[str]) -> Any:
    if not segments:
        return value
    segment = segments[0]
    tail = segments[1:]
    if isinstance(value, Mapping):
        if segment not in value:
            raise ValidationError("correction field path does not exist")
        return _read_nested(value[segment], tail)
    if isinstance(value, (tuple, list)):
        if not segment.isdigit() or int(segment) >= len(value):
            raise ValidationError("correction array index is out of range")
        return _read_nested(value[int(segment)], tail)
    if isinstance(value, CardFace):
        if not hasattr(value, segment):
            raise ValidationError("correction face field path does not exist")
        return _read_nested(getattr(value, segment), tail)
    raise ValidationError("correction field path traverses a scalar value")


def _validate_correction_evidence(
    card: CanonicalCard,
    correction: ReviewedCorrection,
    *,
    observations: Mapping[str, SourceObservation],
) -> None:
    if correction.card_id != card.card_id:
        raise ValidationError(
            "correction card ID does not match the canonical record",
            card_id=correction.card_id,
        )
    missing_evidence = set(correction.evidence_observation_ids) - observations.keys()
    if missing_evidence:
        raise ValidationError(
            "correction references missing or stale source observations",
            card_id=card.card_id,
            field_path=correction.field_path,
            details={"observation_ids": sorted(missing_evidence)},
        )
    foreign_evidence = sorted(
        observation_id
        for observation_id in correction.evidence_observation_ids
        if str(observations[observation_id].extracted.get("card_id", "")).upper()
        != card.card_id
    )
    if foreign_evidence:
        raise ValidationError(
            "correction evidence belongs to another gameplay card",
            card_id=card.card_id,
            field_path=correction.field_path,
            details={"observation_ids": foreign_evidence},
        )


def correction_is_already_applied(
    card: CanonicalCard,
    correction: ReviewedCorrection,
    *,
    observations: Mapping[str, SourceObservation],
) -> bool:
    """Return whether a prior canonical record preserves this exact correction.

    This is intentionally stricter than value equality.  A retained tombstone
    may already contain its committed corrected value, but replay is accepted
    only when the exact field provenance remains adjudicated and bound to the
    correction's still-valid evidence observations.
    """

    _validate_correction_evidence(card, correction, observations=observations)
    segments = correction.field_path.split(".")
    root, tail = segments[0], segments[1:]
    if root == "faces":
        current = _read_nested(card.faces, tail)
        expected_value = (
            _coerce_face_correction(correction.value)
            if not tail
            else correction.value
        )
    elif root == "printings":
        raise ValidationError(
            "printing structure cannot be replaced by a field correction",
            card_id=card.card_id,
            field_path=correction.field_path,
        )
    else:
        if root not in card.fields:
            _validate_missing_root_insertion(root, tail, correction.value)
            return False
        current = _read_nested(card.fields[root], tail)
        expected_value = correction.value
    if current != expected_value:
        return False
    provenance = card.provenance.get(correction.field_path)
    expected_observations = tuple(sorted(correction.evidence_observation_ids))
    return bool(
        provenance is not None
        and provenance.authority is AuthorityClass.ADJUDICATED
        and provenance.adjudicated
        and provenance.selected_observation_id == expected_observations[0]
        and provenance.observation_ids == expected_observations
    )


def apply_correction(
    card: CanonicalCard,
    correction: ReviewedCorrection,
    *,
    observations: Mapping[str, SourceObservation],
) -> CanonicalCard:
    _validate_correction_evidence(card, correction, observations=observations)

    segments = correction.field_path.split(".")
    root, tail = segments[0], segments[1:]
    if root == "faces":
        if not tail:
            corrected_faces = _coerce_face_correction(correction.value)
            if corrected_faces == card.faces:
                raise ValidationError("correction is a no-op and must be removed")
        else:
            corrected_faces = _replace_nested(card.faces, tail, correction.value)
        assert isinstance(corrected_faces, tuple)
        corrected = replace(card, faces=corrected_faces)
    elif root == "printings":
        raise ValidationError(
            "printing structure cannot be replaced by a field correction",
            card_id=card.card_id,
            field_path=correction.field_path,
        )
    else:
        fields = dict(card.fields)
        if root not in fields:
            _validate_missing_root_insertion(root, tail, correction.value)
            fields[root] = correction.value
        else:
            fields[root] = _replace_nested(fields[root], tail, correction.value)
        corrected = replace(card, fields=fields)

    provenance = dict(corrected.provenance)
    prior = provenance.get(correction.field_path) or provenance.get(root)
    observation_ids = tuple(sorted(correction.evidence_observation_ids))
    provenance[correction.field_path] = FieldProvenance(
        selected_observation_id=observation_ids[0],
        observation_ids=observation_ids,
        authority=AuthorityClass.ADJUDICATED,
        derived=prior.derived if prior is not None else False,
        adjudicated=True,
    )
    return replace(corrected, provenance=provenance)


def apply_corrections(
    cards: Mapping[str, CanonicalCard],
    corrections: Iterable[ReviewedCorrection],
    *,
    observations: Mapping[str, SourceObservation],
) -> dict[str, CanonicalCard]:
    result = dict(cards)
    seen: set[tuple[str, str]] = set()
    for correction in corrections:
        key = (correction.card_id, correction.field_path)
        if key in seen:
            raise ValidationError(
                "multiple corrections target the same card field",
                card_id=correction.card_id,
                field_path=correction.field_path,
            )
        seen.add(key)
        card = result.get(correction.card_id)
        if card is None:
            raise ValidationError(
                "correction references an unknown canonical card",
                card_id=correction.card_id,
            )
        result[correction.card_id] = apply_correction(
            card, correction, observations=observations
        )
    return result
