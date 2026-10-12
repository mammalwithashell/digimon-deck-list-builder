"""Field-level runtime cutover diffs and durable approval validation."""

from __future__ import annotations

from dataclasses import dataclass, replace
from typing import Any, Iterable, Mapping

from .errors import AdjudicationError
from .models import (
    AdjudicationKind,
    CanonicalCard,
    MigrationAdjudication,
    ReviewedCorrection,
    SourceObservation,
)
from .serialization import content_digest, natural_card_id_key


RUNTIME_FIELDS = (
    "card_name_eng",
    "dual",
    "xros_req",
    "link_effect_description_eng",
    "max_count_in_deck",
    "dna_costs",
    "digixros_costs",
    "card_colors",
    "card_kind",
    "play_cost",
    "dp",
    "level",
    "type_eng",
    "form_eng",
    "attribute_eng",
    "evo_costs",
    "effect_description_eng",
    "inherited_effect_description_eng",
    "security_effect_description_eng",
)


@dataclass(frozen=True, slots=True)
class RuntimeFieldDiff:
    card_id: str
    field_path: str
    old_value: Any
    new_value: Any
    old_digest: str
    new_digest: str
    provenance_observation_ids: tuple[str, ...]
    approval_kind: str | None = None
    approval_id: str | None = None

    @property
    def approved(self) -> bool:
        return self.approval_id is not None

    def to_dict(self) -> dict[str, Any]:
        """Render the complete cutover-audit row, including approval identity."""

        return {
            "card_id": self.card_id,
            "field_path": self.field_path,
            "old_value": self.old_value,
            "new_value": self.new_value,
            "old_digest": self.old_digest,
            "new_digest": self.new_digest,
            "provenance_observation_ids": list(self.provenance_observation_ids),
            "approved": self.approved,
            "approval_kind": self.approval_kind,
            "approval_id": self.approval_id,
        }


def diff_runtime_fields(
    old_cards: Mapping[str, Mapping[str, Any]],
    new_cards: Mapping[str, Mapping[str, Any]],
    canonical_cards: Mapping[str, CanonicalCard],
    *,
    compatibility_field_provenance: Mapping[str, Mapping[str, tuple[str, ...]]] | None = None,
) -> tuple[RuntimeFieldDiff, ...]:
    diffs: list[RuntimeFieldDiff] = []
    for card_id in sorted(set(old_cards) | set(new_cards), key=natural_card_id_key):
        old = old_cards.get(card_id, {})
        new = new_cards.get(card_id, {})
        canonical = canonical_cards.get(card_id)
        for field_path in RUNTIME_FIELDS:
            old_value = old.get(field_path)
            new_value = new.get(field_path)
            if old_value == new_value:
                continue
            provenance = canonical.provenance.get(field_path) if canonical else None
            observation_ids = provenance.observation_ids if provenance else ()
            if canonical is not None and field_path == "dual":
                # project_canonical_card derives dual from structural faces,
                # plus the Digimon face's canonical traits and evolution costs.
                # Include nested reviewed corrections to those inputs only.
                observation_ids = tuple(sorted({
                    observation_id
                    for path, source in canonical.provenance.items()
                    if path.split(".", 1)[0] in {"faces", "type_eng", "evo_costs"}
                    for observation_id in source.observation_ids
                }))
            if field_path == "xros_req":
                observation_ids = tuple(sorted(set(
                    (compatibility_field_provenance or {}).get(card_id, {}).get(field_path, ())
                )))
            diffs.append(
                RuntimeFieldDiff(
                    card_id=card_id,
                    field_path=field_path,
                    old_value=old_value,
                    new_value=new_value,
                    old_digest=content_digest(old_value),
                    new_digest=content_digest(new_value),
                    provenance_observation_ids=observation_ids,
                )
            )
    return tuple(diffs)


def approve_runtime_diffs(
    diffs: Iterable[RuntimeFieldDiff],
    *,
    corrections: Iterable[ReviewedCorrection],
    adjudications: Iterable[MigrationAdjudication],
    observations: Mapping[str, SourceObservation],
    old_input_revision: str,
    new_input_revision: str,
) -> tuple[RuntimeFieldDiff, ...]:
    correction_by_field = {
        (item.card_id, item.field_path): item for item in corrections
    }
    adjudications = tuple(adjudications)
    output: list[RuntimeFieldDiff] = []
    for diff in diffs:
        correction = correction_by_field.get((diff.card_id, diff.field_path))
        correction_evidence = (
            tuple(
                observations[observation_id]
                for observation_id in correction.evidence_observation_ids
            )
            if correction is not None
            and set(correction.evidence_observation_ids) <= observations.keys()
            else ()
        )
        if correction is not None and (
            correction.value == diff.new_value
            and bool(correction_evidence)
            and all(
                str(observation.extracted.get("card_id", "")).upper()
                == diff.card_id
                for observation in correction_evidence
            )
        ):
            output.append(
                replace(
                    diff,
                    approval_kind="correction",
                    approval_id=correction.correction_id,
                )
            )
            continue
        matches = [
            item
            for item in adjudications
            if item.kind is AdjudicationKind.RUNTIME_FIELD_CUTOVER
            and item.card_id == diff.card_id
            and item.field_path == diff.field_path
            and item.is_current(
                old_input_revision=old_input_revision,
                new_input_revision=new_input_revision,
                old_digest=diff.old_digest,
                new_digest=diff.new_digest,
            )
            and bool(item.provenance_observation_ids)
            and set(item.provenance_observation_ids) <= observations.keys()
            and set(item.provenance_observation_ids)
            <= set(diff.provenance_observation_ids)
        ]
        if len(matches) > 1:
            raise AdjudicationError(
                "multiple current runtime cutover adjudications match one field",
                card_id=diff.card_id,
                field_path=diff.field_path,
            )
        approval = matches[0] if matches else None
        output.append(
            replace(
                diff,
                approval_kind="migration_adjudication" if approval else None,
                approval_id=approval.adjudication_id if approval else None,
            )
        )
    return tuple(output)


def require_approved_runtime_diffs(diffs: Iterable[RuntimeFieldDiff]) -> None:
    unapproved = [diff for diff in diffs if not diff.approved]
    if unapproved:
        raise AdjudicationError(
            "runtime-field cutover contains unapproved differences",
            details={
                "differences": [
                    {"card_id": item.card_id, "field_path": item.field_path}
                    for item in unapproved
                ]
            },
        )
