"""Official condition text and its explicit compatibility-field evidence."""
from __future__ import annotations

from typing import Any, Mapping

from .errors import SourceSchemaError
from .models import CanonicalCard, SourceName, SourceObservation
from .serialization import observation_digest, content_digest
from .compatibility_selections import CompatibilitySourceSelection

_CONDITION_LABELS = (
    "special play condition", "special digivolution condition",
    "special evolution condition", "dna digivolution", "digixros",
)


def official_condition_projection(
    card: CanonicalCard, observations: Mapping[str, SourceObservation],
    *, selections: tuple[CompatibilitySourceSelection, ...] = (),
    source_manifest_revision: str | None = None,
) -> tuple[str, tuple[str, ...]]:
    """Require agreeing official sections; never choose using name provenance.

    Evidence comes only from this card's printing membership. Even an empty
    projection requires an explicit, well-formed official section collection.
    """
    member_ids = {oid for printing in card.printings for oid in printing.observation_ids}
    missing = sorted(member_ids - observations.keys())
    if missing:
        raise SourceSchemaError("condition projection has missing observation evidence",
            card_id=card.card_id, details={"observation_ids": missing})
    values: dict[str, str] = {}
    for oid in sorted(member_ids):
        source = observations[oid]
        if source.source is not SourceName.OFFICIAL:
            continue
        if source.observation_id != oid or observation_digest(source) != oid:
            raise SourceSchemaError("condition projection has invalid observation identity",
                card_id=card.card_id, details={"observation_id": oid})
        if str(source.extracted.get("card_id", "")).upper() != card.card_id:
            raise SourceSchemaError("condition projection has foreign card evidence",
                card_id=card.card_id, details={"observation_id": oid})
        sections = source.extracted.get("text_sections")
        if not isinstance(sections, list) or any(
            not isinstance(row, Mapping) or not isinstance(row.get("label"), str)
            or not isinstance(row.get("text"), str) for row in sections
        ):
            raise SourceSchemaError("condition projection requires official text sections",
                card_id=card.card_id, details={"observation_id": oid})
        values[oid] = "\n".join(
            f"[{row['label']}] {row['text']}".strip()
            for row in sections
            if any(label in row["label"].lower() for label in _CONDITION_LABELS)
        )
    if not values:
        raise SourceSchemaError("condition projection lacks official observation evidence",
            card_id=card.card_id)
    matches = [row for row in selections
        if row.card_id == card.card_id and row.source_manifest_revision == source_manifest_revision
        and dict(row.candidate_values) == values
        and row.candidate_set_digest == content_digest(values)]
    if len(matches) > 1:
        raise SourceSchemaError("multiple current compatibility source selections",
            card_id=card.card_id, details={"selection_ids": [row.selection_id for row in matches]})
    if matches:
        selected = matches[0]
        return selected.selected_value, tuple(oid for oid, value in values.items() if value == selected.selected_value)
    if len(set(values.values())) != 1:
        raise SourceSchemaError("official condition sections disagree",
            card_id=card.card_id, details={"candidate_conditions": values})
    return next(iter(values.values())), tuple(values)


def refresh_official_conditions(
    cards: Mapping[str, CanonicalCard],
    observations: Mapping[str, SourceObservation],
    compatibility_fields: Mapping[str, Mapping[str, Any]],
    *, selections: tuple[CompatibilitySourceSelection, ...] = (),
    source_manifest_revision: str | None = None,
    compatibility_corrections: tuple = (),
    correction_image_artifacts: Mapping[str, bytes] | None = None,
) -> tuple[dict[str, dict[str, Any]], dict[str, dict[str, tuple[str, ...]]]]:
    """Refresh only xros_req, preserving unrelated compatibility-only inputs."""
    output = {cid: dict(row) for cid, row in compatibility_fields.items()}
    evidence: dict[str, dict[str, tuple[str, ...]]] = {}
    conflicts: list[dict[str, Any]] = []
    from .compatibility_corrections import CorrectionDocument, apply_image_correction, validate_image_artifacts
    from .errors import ValidationError
    document = CorrectionDocument(compatibility_corrections=compatibility_corrections)
    validate_image_artifacts(document, correction_image_artifacts or {})
    for correction in compatibility_corrections:
        if correction.source_manifest_revision == source_manifest_revision and correction.card_id not in cards:
            raise ValidationError('current compatibility correction targets an absent card')
    for cid, card in cards.items():
        reviewed = [row for row in compatibility_corrections if row.card_id == cid]
        current = [row for row in reviewed if row.source_manifest_revision == source_manifest_revision]
        if reviewed and not current:
            raise ValidationError('compatibility correction is stale for this source revision', card_id=cid)
        if current:
            if any(row.card_id == cid and row.source_manifest_revision == source_manifest_revision for row in selections):
                raise ValidationError('source selection overlaps a compatibility image correction', card_id=cid)
            value, ids = apply_image_correction(card, observations, current[0],
                source_manifest_revision=source_manifest_revision,
                image_artifacts=correction_image_artifacts or {})
            output.setdefault(cid, {})['xros_req'] = value
            evidence[cid] = {'xros_req': ids}
            continue
        try:
            value, ids = official_condition_projection(card, observations,
                selections=selections, source_manifest_revision=source_manifest_revision)
        except SourceSchemaError as exc:
            conflicts.append({"card_id": cid, "message": str(exc),
                "details": dict(exc.diagnostic.details)})
            continue
        output.setdefault(cid, {})["xros_req"] = value
        evidence[cid] = {"xros_req": ids}
    if conflicts:
        raise SourceSchemaError("official condition projection preflight failed",
            details={"condition_conflicts": conflicts})
    return output, evidence
