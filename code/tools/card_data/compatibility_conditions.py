"""Official condition text and its explicit compatibility-field evidence."""
from __future__ import annotations

from typing import Any, Mapping

from .errors import SourceSchemaError
from .models import CanonicalCard, SourceName, SourceObservation
from .serialization import observation_digest

_CONDITION_LABELS = (
    "special play condition", "special digivolution condition",
    "special evolution condition", "dna digivolution", "digixros",
)


def official_condition_projection(
    card: CanonicalCard, observations: Mapping[str, SourceObservation],
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
    if len(set(values.values())) != 1:
        raise SourceSchemaError("official condition sections disagree",
            card_id=card.card_id, details={"candidate_conditions": values})
    return next(iter(values.values())), tuple(values)


def refresh_official_conditions(
    cards: Mapping[str, CanonicalCard],
    observations: Mapping[str, SourceObservation],
    compatibility_fields: Mapping[str, Mapping[str, Any]],
) -> tuple[dict[str, dict[str, Any]], dict[str, dict[str, tuple[str, ...]]]]:
    """Refresh only xros_req, preserving unrelated compatibility-only inputs."""
    output = {cid: dict(row) for cid, row in compatibility_fields.items()}
    evidence: dict[str, dict[str, tuple[str, ...]]] = {}
    conflicts: list[dict[str, Any]] = []
    for cid, card in cards.items():
        try:
            value, ids = official_condition_projection(card, observations)
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
