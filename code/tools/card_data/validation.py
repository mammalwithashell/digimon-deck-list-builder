"""Aggregate canonical-card invariants before generation or publication."""

from __future__ import annotations

from typing import Iterable, Mapping

from .errors import Diagnostic, DiagnosticReport, DiagnosticSeverity, ValidationError
from .models import (
    AdmissionStatus,
    AuthorityClass,
    CanonicalCard,
    FaceKind,
    SourceObservation,
)


_CARD_KINDS = frozenset({"Digimon", "Digi-Egg", "Tamer", "Option", "Dual"})
_TEXT_FIELDS = frozenset(
    {
        "effect_description_eng",
        "inherited_effect_description_eng",
        "security_effect_description_eng",
        "link_effect_description_eng",
    }
)


def _error(
    message: str,
    *,
    card_id: str,
    field_path: str | None = None,
    details: Mapping[str, object] | None = None,
) -> Diagnostic:
    return ValidationError(
        message,
        card_id=card_id,
        field_path=field_path,
        details=details,
    ).diagnostic


def validate_canonical_card(
    card: CanonicalCard,
    *,
    observations: Mapping[str, SourceObservation] | None = None,
    include_conflict_diagnostics: bool = True,
    canonical_field_selection_approvals: Mapping[tuple[str, str], str] | None = None,
) -> DiagnosticReport:
    diagnostics: list[Diagnostic] = []
    fields = card.fields

    kind = fields.get("card_kind")
    if kind is not None and kind not in _CARD_KINDS:
        diagnostics.append(
            _error(
                "canonical card_kind is unsupported",
                card_id=card.card_id,
                field_path="card_kind",
                details={"value": kind},
            )
        )
    colors = fields.get("card_colors")
    if colors is not None and (
        not isinstance(colors, (list, tuple))
        or not colors
        or not all(isinstance(color, str) and color for color in colors)
    ):
        diagnostics.append(
            _error(
                "canonical card_colors must be a non-empty string array",
                card_id=card.card_id,
                field_path="card_colors",
            )
        )
    for field_path in ("play_cost", "level", "dp", "max_count_in_deck"):
        value = fields.get(field_path)
        if value is not None and (
            not isinstance(value, int) or isinstance(value, bool) or value < 0
        ):
            diagnostics.append(
                _error(
                    "canonical numeric field must be a non-negative integer or null",
                    card_id=card.card_id,
                    field_path=field_path,
                )
            )
    for field_path in ("type_eng", "form_eng", "attribute_eng"):
        value = fields.get(field_path)
        if value is not None and (
            not isinstance(value, (list, tuple))
            or not all(isinstance(item, str) and item for item in value)
        ):
            diagnostics.append(
                _error(
                    "canonical trait/form/attribute field must be a string array",
                    card_id=card.card_id,
                    field_path=field_path,
                )
            )
    for field_path in _TEXT_FIELDS:
        value = fields.get(field_path)
        if value is not None and not isinstance(value, str):
            diagnostics.append(
                _error(
                    "canonical printed text field must be a string",
                    card_id=card.card_id,
                    field_path=field_path,
                )
            )

    routes = fields.get("evo_costs")
    if routes is not None:
        if not isinstance(routes, (list, tuple)):
            diagnostics.append(
                _error(
                    "canonical evo_costs must be an array",
                    card_id=card.card_id,
                    field_path="evo_costs",
                )
            )
        else:
            for ordinal, route in enumerate(routes):
                if not isinstance(route, Mapping):
                    diagnostics.append(
                        _error(
                            "canonical evolution route must be an object",
                            card_id=card.card_id,
                            field_path=f"evo_costs.{ordinal}",
                        )
                    )
                    continue
                cost = route.get("cost", route.get("memory_cost"))
                if cost is None or not isinstance(cost, int) or isinstance(cost, bool) or cost < 0:
                    diagnostics.append(
                        _error(
                            "canonical evolution route requires a non-negative cost",
                            card_id=card.card_id,
                            field_path=f"evo_costs.{ordinal}",
                        )
                    )

    if not card.printings and card.admission is not AdmissionStatus.RETIRED:
        diagnostics.append(
            _error("active canonical card requires a printing", card_id=card.card_id, field_path="printings")
        )
    printing_ids = [printing.printing_id for printing in card.printings]
    if len(printing_ids) != len(set(printing_ids)):
        diagnostics.append(
            _error("canonical printing IDs must be unique", card_id=card.card_id, field_path="printings")
        )
    if len(card.faces) == 2 and {face.kind for face in card.faces} != {
        FaceKind.DIGIMON,
        FaceKind.OPTION,
    }:
        diagnostics.append(
            _error(
                "two-face canonical card must contain Digimon and Option faces",
                card_id=card.card_id,
                field_path="faces",
            )
        )
    for ordinal, face in enumerate(card.faces):
        if not face.colors:
            diagnostics.append(
                _error(
                    "canonical face requires at least one color",
                    card_id=card.card_id,
                    field_path=f"faces.{ordinal}.colors",
                )
            )

    missing_provenance = set(fields) - set(card.provenance)
    for field_path in sorted(missing_provenance):
        diagnostics.append(
            _error(
                "canonical field has no source provenance",
                card_id=card.card_id,
                field_path=field_path,
            )
        )
    if include_conflict_diagnostics:
        for conflict in card.conflicts:
            if not conflict.requires_adjudication(card.admission):
                continue
            approval_id = (canonical_field_selection_approvals or {}).get(
                (card.card_id, conflict.field_path)
            )
            if approval_id:
                continue
            provenance = card.provenance.get(conflict.field_path)
            # Normalization sets adjudicated=True only after it has matched a
            # digest-bound canonical_field_selection entry.  A sparse
            # correction also uses adjudicated provenance, but with the
            # ADJUDICATED authority class; that must not approve a printing
            # choice.  Candidate assemblers pass the exact current selection
            # IDs separately when a later correction has replaced provenance.
            if (
                provenance is not None
                and provenance.adjudicated
                and provenance.authority is not AuthorityClass.ADJUDICATED
            ):
                continue
            diagnostics.append(
                _error(
                    "canonical printing conflict requires adjudication",
                    card_id=card.card_id,
                    field_path=conflict.field_path,
                    details={
                        "conflict_kind": conflict.kind.value,
                        "candidate_observation_ids": list(
                            conflict.candidate_observation_ids
                        ),
                        "reason": conflict.reason,
                    },
                )
            )
    if observations is not None:
        for field_path, provenance in card.provenance.items():
            missing = set(provenance.observation_ids) - observations.keys()
            if missing:
                diagnostics.append(
                    _error(
                        "canonical provenance references missing observations",
                        card_id=card.card_id,
                        field_path=field_path,
                        details={"observation_ids": sorted(missing)},
                    )
                )
        for ordinal, printing in enumerate(card.printings):
            missing = set(printing.observation_ids) - observations.keys()
            if missing:
                diagnostics.append(
                    _error(
                        "canonical printing references missing observations",
                        card_id=card.card_id,
                        field_path=f"printings.{ordinal}",
                        details={"observation_ids": sorted(missing)},
                    )
                )

    return DiagnosticReport(tuple(diagnostics))


def validate_canonical_cards(
    cards: Mapping[str, CanonicalCard],
    *,
    observations: Mapping[str, SourceObservation] | None = None,
    include_conflict_diagnostics: bool = True,
    canonical_field_selection_approvals: Mapping[tuple[str, str], str] | None = None,
) -> DiagnosticReport:
    diagnostics: list[Diagnostic] = []
    for key, card in cards.items():
        if key != card.card_id:
            diagnostics.append(
                _error(
                    "canonical mapping key does not match card_id",
                    card_id=card.card_id,
                    details={"mapping_key": key},
                )
            )
        diagnostics.extend(
            validate_canonical_card(
                card,
                observations=observations,
                include_conflict_diagnostics=include_conflict_diagnostics,
                canonical_field_selection_approvals=(
                    canonical_field_selection_approvals
                ),
            ).diagnostics
        )
    return DiagnosticReport(tuple(diagnostics))


def require_valid_canonical_cards(
    cards: Mapping[str, CanonicalCard],
    *,
    observations: Mapping[str, SourceObservation] | None = None,
    canonical_field_selection_approvals: Mapping[tuple[str, str], str] | None = None,
) -> None:
    report = validate_canonical_cards(
        cards,
        observations=observations,
        canonical_field_selection_approvals=canonical_field_selection_approvals,
    )
    if report.has_errors:
        raise ValidationError(
            "canonical card invariants failed",
            details=report.to_dict(),
        )
