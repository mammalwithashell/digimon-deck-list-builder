"""Migrate frozen whole-field overrides into reviewed sparse corrections.

The historical override ledger predates committed raw source observations.  To
avoid inventing raw HTTP provenance, this migration creates transparent
bootstrap observations from the exact source-shaped records already committed
in ``data/card_official.json``.  Each record payload is deterministically
serialized and hashed; parser/schema/source metadata explicitly identifies it
as a legacy official-mirror bootstrap and the report records that no raw HTTP
body is retained.

Before producer cutover the command deterministically materializes and checks
those migration artifacts. After a committed revision marker is active it is
check-only: the frozen report is reconciled with the marker-owned canonical
cards, reviewed corrections, scope, and evidence without replaying retired
legacy values into generated ``cards.json``.
"""

from __future__ import annotations

import argparse
import dataclasses
import hashlib
import json
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Mapping

from .corrections import (
    CORRECTIONS_DOCUMENT_SCHEMA_VERSION,
    apply_correction,
    correction_is_already_applied,
    load_reviewed_corrections,
    merge_reviewed_corrections,
    write_reviewed_corrections,
)
from .errors import ValidationError, WriteError
from .legacy_overrides import load_legacy_overrides
from .models import (
    SCHEMA_VERSION,
    ReviewedCorrection,
    ScopeManifest,
    ScopeObservationRef,
    SourceName,
    SourceObservation,
)
from .serialization import (
    canonical_json_bytes,
    canonical_json_text,
    content_digest,
    natural_card_id_key,
    observation_digest,
    sha256_hex,
)
from .validation import validate_canonical_card


BOOTSTRAP_SCOPE_NAME = "explicit-ids:legacy-official-mirror-overrides-v1"
BOOTSTRAP_PARSER_VERSION = "legacy-official-mirror-bootstrap-v1"
BOOTSTRAP_SOURCE_SCHEMA_VERSION = "legacy-card-official-json-record-v1"
BOOTSTRAP_FIRST_ACQUIRED_AT = "2026-07-12T00:00:00Z"
BOOTSTRAP_REVIEWER = "canonical-card-data-pipeline-bootstrap-review"
BOOTSTRAP_REVIEWED_AT = "2026-07-12"
BOOTSTRAP_CORRECTION_PREFIX = "legacy-override-bootstrap-"
BOOTSTRAP_SOURCE_ROOT = "repository://legacy-official-mirror/data/card_official.json"
BOOTSTRAP_RECORD_SERIALIZATION = (
    "UTF-8 canonical_json_bytes of the exact parsed cards[card_id] object; "
    "not a retained raw HTTP body"
)
BOOTSTRAP_SCOPE_PATH = (
    "data/card_sources/scopes/official/"
    "legacy-official-mirror-overrides-v1.json"
)
REVISION_MARKER_PATH = "data/card_data_revision.json"
TRANSACTION_JOURNAL_PATH = "data/.card_data_transaction.json"
HISTORICAL_LEGACY_OVERRIDES_SHA256 = (
    "5040bd9a5edc26bf38325f7a700a6dcd617d2bf86c3489d1096f0dc0002dd0f6"
)
HISTORICAL_OFFICIAL_MIRROR_SHA256 = (
    "6d21c9348bd3c6f0099cdbd9cd814dcd7d2e0ac129792764e201231f2eef0e5c"
)
HISTORICAL_COMPATIBILITY_CARDS_SHA256 = (
    "577c43f68a4d30ebe8c39388b08bb919bf0ca1b00e1c41912370405fd43f21cf"
)
HISTORICAL_CORRECTIONS_SHA256 = (
    "396a39d7e81ed753f35eba477f291993b2be85f6181d56e61124af83a3e7fd97"
)
HISTORICAL_BOOTSTRAP_SCOPE_SHA256 = (
    "46bdec054a6f5fcde2c3bb81ba9fef26099f6a8b8584e7a08817f79378275404"
)
HISTORICAL_BOOTSTRAP_OBSERVATION_IDS_SHA256 = (
    "2c76d4bbcc4dec37dd741acc895f3d6496cff6eaddde866f08c4f9c49d62e49e"
)

COLOR_CODES = {
    "Red": 0,
    "Blue": 1,
    "Yellow": 2,
    "Green": 3,
    "White": 4,
    "Black": 5,
    "Purple": 6,
}


@dataclass(frozen=True, slots=True)
class MigrationArtifacts:
    observations: Mapping[str, SourceObservation]
    manifest: ScopeManifest
    corrections: tuple[ReviewedCorrection, ...]
    migration_corrections: tuple[ReviewedCorrection, ...]
    report: Mapping[str, Any]


@dataclass(frozen=True, slots=True)
class CommittedMigrationCheck:
    """Validated post-cutover reconciliation evidence."""

    revision: str
    legacy_card_count: int
    legacy_field_count: int
    active_bootstrap_correction_count: int
    normalized_no_op_count: int
    rejected_correction_count: int


def _load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ValidationError(
            "failed to read legacy-correction migration input",
            path=path,
            details={"error": str(exc)},
        ) from exc


def _input_sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _split_official_line(value: Any) -> list[str]:
    return [part.strip() for part in str(value or "").split("/") if part.strip()]


def _official_projection(field_path: str, official: Mapping[str, Any]) -> Any:
    if field_path == "card_colors":
        return [
            COLOR_CODES[color]
            for color in str(official.get("colors") or "").split()
            if color in COLOR_CODES
        ]
    if field_path == "evo_costs":
        return official.get("digivolve_costs") or []
    if field_path == "type_eng":
        return _split_official_line(official.get("type"))
    if field_path == "form_eng":
        return _split_official_line(official.get("form"))
    if field_path == "attribute_eng":
        return _split_official_line(official.get("attribute"))
    # Printed effect sections do not round-trip to the legacy flattened field
    # without a projection policy.  Retain the source-shaped sections for
    # review and classify the comparison honestly.
    if field_path == "effect_description_eng":
        return official.get("text_sections") or []
    return None


def _official_relation(legacy_value: Any, projection: Any, field_path: str) -> str:
    if field_path == "effect_description_eng" or projection is None:
        return "not_directly_comparable"
    if legacy_value == projection:
        return "exact"
    if (
        isinstance(legacy_value, list)
        and isinstance(projection, list)
        and all(isinstance(item, str) for item in legacy_value)
        and all(isinstance(item, str) for item in projection)
    ):
        legacy_set = {item.casefold() for item in legacy_value}
        official_set = {item.casefold() for item in projection}
        if official_set <= legacy_set:
            return "legacy_superset"
    return "conflict"


def _correction_reason(relation: str) -> str:
    if relation == "exact":
        return (
            "Legacy override migration; the committed official-mirror bootstrap "
            "supports the retained compatibility value."
        )
    if relation == "legacy_superset":
        return (
            "Legacy override migration; retain the shipped superset while the "
            "official-mirror bootstrap preserves the printed baseline for review."
        )
    return (
        "Legacy override migration preserves the shipped compatibility value; "
        "the official-mirror bootstrap differs or needs a structured projection, "
        "and the reconciliation report records the follow-up."
    )


def _bootstrap_observation(
    card_id: str,
    official_record: Mapping[str, Any],
) -> SourceObservation:
    record_bytes = canonical_json_bytes(official_record)
    draft = SourceObservation(
        schema_version=SCHEMA_VERSION,
        observation_id="0" * 64,
        source=SourceName.OFFICIAL,
        source_record_key=f"legacy-official-mirror:{card_id}",
        source_url=f"{BOOTSTRAP_SOURCE_ROOT}#/cards/{card_id}",
        raw_payload_digest=sha256_hex(record_bytes),
        parser_version=BOOTSTRAP_PARSER_VERSION,
        source_schema_version=BOOTSTRAP_SOURCE_SCHEMA_VERSION,
        first_acquired_at=BOOTSTRAP_FIRST_ACQUIRED_AT,
        extracted=official_record,
    )
    return dataclasses.replace(draft, observation_id=observation_digest(draft))


def _corrections_payload(
    corrections: tuple[ReviewedCorrection, ...],
) -> dict[str, Any]:
    return {
        "schema_version": CORRECTIONS_DOCUMENT_SCHEMA_VERSION,
        "corrections": corrections,
    }


def _canonical_reconciled_corrections(
    canonical_root: Path,
    mechanical_corrections: tuple[ReviewedCorrection, ...],
    existing: tuple[ReviewedCorrection, ...],
    *,
    legacy_cards: Mapping[str, Mapping[str, Any]],
) -> tuple[tuple[ReviewedCorrection, ...], dict[str, dict[str, Any]]]:
    """Keep only legacy candidates proven active in a canonical candidate.

    The canonical record carries the decisive distinction that the frozen
    legacy inputs cannot: an applied correction has adjudicated provenance
    bound to its evidence, while a naturally normalized no-op or rejected
    invalid candidate does not.  Non-bootstrap reviewed corrections are
    outside this one-time migration and are always preserved verbatim.
    """

    from .complete_candidate import load_prior_canonical_cards
    from .live_reconcile import _load_observations

    canonical_path = canonical_root / "data/cards_canonical.json"
    canonical_cards = load_prior_canonical_cards(canonical_path)
    report_path = (
        canonical_root / "qa/qa-reports/official-canonical-candidate.json"
    )
    report = _load_json(report_path)
    if not isinstance(report, Mapping) or (
        report.get("status") != "ready"
        or report.get("canonical_readiness") != "ready"
    ):
        raise ValidationError(
            "canonical correction reconciliation requires a ready candidate report",
            path=report_path,
        )
    expected_old_revision = content_digest(legacy_cards)
    if report.get("old_input_revision") != expected_old_revision:
        raise ValidationError(
            "canonical correction reconciliation targets another compatibility input",
            path=report_path,
            details={
                "expected_old_input_revision": expected_old_revision,
                "actual_old_input_revision": report.get("old_input_revision"),
            },
        )
    candidate_canonical_revision = content_digest(canonical_cards)
    if report.get("candidate_canonical_revision") != candidate_canonical_revision:
        raise ValidationError(
            "canonical correction reconciliation report is stale",
            path=report_path,
            details={
                "expected_canonical_revision": candidate_canonical_revision,
                "actual_canonical_revision": report.get(
                    "candidate_canonical_revision"
                ),
            },
        )
    observations = {
        **_load_observations(canonical_root, SourceName.OFFICIAL),
        **_load_observations(canonical_root, SourceName.DIGIMONCARD_IO),
    }
    candidate_corrections = load_reviewed_corrections(
        canonical_root / "data/card_corrections.json"
    )
    report_correction_ids = {
        str(row.get("correction_id"))
        for row in report.get("corrections", ())
        if isinstance(row, Mapping) and row.get("status") == "applied"
    }
    candidate_correction_ids = {
        item.correction_id for item in candidate_corrections
    }
    if report_correction_ids != candidate_correction_ids:
        raise ValidationError(
            "canonical correction ledger disagrees with its ready candidate report",
            path=report_path,
            details={
                "report_only_ids": sorted(
                    report_correction_ids - candidate_correction_ids
                ),
                "ledger_only_ids": sorted(
                    candidate_correction_ids - report_correction_ids
                ),
            },
        )
    existing_by_id = {item.correction_id: item for item in existing}
    preserved = tuple(
        item
        for item in existing
        if not item.correction_id.startswith(BOOTSTRAP_CORRECTION_PREFIX)
    )
    candidate_preserved = tuple(
        item
        for item in candidate_corrections
        if not item.correction_id.startswith(BOOTSTRAP_CORRECTION_PREFIX)
    )
    if preserved != candidate_preserved:
        raise ValidationError(
            "canonical correction reconciliation would change unrelated reviewed rows",
            path=canonical_root / "data/card_corrections.json",
        )
    preserved_targets = {
        (item.card_id, item.field_path): item for item in preserved
    }
    candidate_bootstrap = {
        item.correction_id: item
        for item in candidate_corrections
        if item.correction_id.startswith(BOOTSTRAP_CORRECTION_PREFIX)
    }
    mechanical_by_id = {
        item.correction_id: item for item in mechanical_corrections
    }
    expected_ids = set(mechanical_by_id)
    unexpected_candidate_ids = sorted(set(candidate_bootstrap) - expected_ids)
    if unexpected_candidate_ids:
        raise ValidationError(
            "canonical candidate contains unknown legacy bootstrap corrections",
            details={"correction_ids": unexpected_candidate_ids},
        )
    changed_candidate_ids = sorted(
        correction_id
        for correction_id, correction in candidate_bootstrap.items()
        if (
            correction.schema_version
            != mechanical_by_id[correction_id].schema_version
            or correction.card_id != mechanical_by_id[correction_id].card_id
            or correction.field_path
            != mechanical_by_id[correction_id].field_path
            or correction.value != mechanical_by_id[correction_id].value
        )
    )
    if changed_candidate_ids:
        raise ValidationError(
            "canonical candidate redefines legacy bootstrap corrections",
            details={"correction_ids": changed_candidate_ids},
        )
    active: list[ReviewedCorrection] = []
    missing_cards: list[str] = []
    dispositions: dict[str, dict[str, Any]] = {}
    for expected in mechanical_corrections:
        target = (expected.card_id, expected.field_path)
        superseding = preserved_targets.get(target)
        if superseding is not None:
            dispositions[expected.correction_id] = {
                "correction_disposition_reason": (
                    "superseded_by_reviewed_sparse_correction"
                ),
                "superseding_correction_id": superseding.correction_id,
            }
            continue
        card = canonical_cards.get(expected.card_id)
        if card is None:
            missing_cards.append(expected.card_id)
            continue
        candidate = candidate_bootstrap.get(
            expected.correction_id,
            existing_by_id.get(expected.correction_id, expected),
        )
        if correction_is_already_applied(
            card,
            candidate,
            observations=observations,
        ):
            active.append(candidate)
            dispositions[expected.correction_id] = {
                "correction_disposition_reason": (
                    "required_to_preserve_legacy_compatibility_value"
                )
            }
            continue
        try:
            corrected = apply_correction(
                card,
                candidate,
                observations=observations,
            )
        except ValidationError as exc:
            if "no-op" not in str(exc):
                raise ValidationError(
                    "canonical correction reconciliation could not classify "
                    "an inactive legacy correction",
                    card_id=candidate.card_id,
                    field_path=candidate.field_path,
                    details={
                        "correction_id": candidate.correction_id,
                        "error": str(exc),
                    },
                ) from exc
            dispositions[expected.correction_id] = {
                "correction_disposition_reason": (
                    "normalized_value_already_matches"
                )
            }
            continue
        validation = validate_canonical_card(
            corrected,
            observations=observations,
            include_conflict_diagnostics=False,
        )
        if not validation.has_errors:
            raise ValidationError(
                "ready canonical candidate omits a valid legacy correction",
                card_id=candidate.card_id,
                field_path=candidate.field_path,
                details={"correction_id": candidate.correction_id},
            )
        diagnostics = validation.to_dict()["diagnostics"]
        dispositions[expected.correction_id] = {
            "correction_disposition_reason": (
                "rejected_by_canonical_validation"
            ),
            "canonical_validation_diagnostic_codes": sorted(
                {
                    str(row["code"])
                    for row in diagnostics
                    if row.get("severity") == "error"
                }
            ),
            "canonical_validation_messages": sorted(
                {
                    str(row["message"])
                    for row in diagnostics
                    if row.get("severity") == "error"
                }
            ),
        }
    if missing_cards:
        raise ValidationError(
            "canonical correction reconciliation is missing legacy cards",
            path=canonical_path,
            details={"card_ids": sorted(set(missing_cards), key=natural_card_id_key)},
        )
    active_ids = {item.correction_id for item in active}
    if active_ids != set(candidate_bootstrap):
        raise ValidationError(
            "canonical correction provenance disagrees with its reviewed ledger",
            details={
                "provenance_only_ids": sorted(
                    active_ids - set(candidate_bootstrap)
                ),
                "ledger_only_ids": sorted(
                    set(candidate_bootstrap) - active_ids
                ),
            },
        )
    return merge_reviewed_corrections(preserved, active), dispositions


def _committed_bootstrap_evidence(
    *,
    artifact_bytes: Mapping[str, bytes],
    legacy_entries: Mapping[str, Mapping[str, Any]],
) -> tuple[
    ScopeManifest,
    dict[str, SourceObservation],
    dict[str, SourceObservation],
]:
    """Load the frozen migration evidence from one marker-owned revision."""

    from .live_reconcile import _manifest_from_dict, _observation_from_dict

    scope_raw = artifact_bytes.get(BOOTSTRAP_SCOPE_PATH)
    if not isinstance(scope_raw, bytes):
        raise ValidationError(
            "committed revision does not own the legacy bootstrap scope",
            path=BOOTSTRAP_SCOPE_PATH,
        )
    try:
        scope_payload = json.loads(scope_raw.decode("utf-8"))
        if not isinstance(scope_payload, Mapping):
            raise TypeError("scope manifest root must be an object")
        manifest = _manifest_from_dict(scope_payload)
    except (UnicodeDecodeError, json.JSONDecodeError, KeyError, TypeError, ValueError) as exc:
        raise ValidationError(
            "committed legacy bootstrap scope is invalid",
            path=BOOTSTRAP_SCOPE_PATH,
            details={"error": str(exc)},
        ) from exc

    observations: dict[str, SourceObservation] = {}
    observations_by_card: dict[str, SourceObservation] = {}
    for ref in manifest.observation_refs:
        expected_printing_id = f"{ref.gameplay_card_id}:legacy-official-mirror"
        if ref.printing_id != expected_printing_id:
            raise ValidationError(
                "legacy bootstrap scope has a stale printing identity",
                card_id=ref.gameplay_card_id,
                details={
                    "expected_printing_id": expected_printing_id,
                    "actual_printing_id": ref.printing_id,
                },
            )
        relative = (
            "data/card_sources/observations/official/"
            f"{ref.observation_id}.json"
        )
        raw = artifact_bytes.get(relative)
        if not isinstance(raw, bytes):
            raise ValidationError(
                "legacy bootstrap observation is not marker-owned",
                path=relative,
                card_id=ref.gameplay_card_id,
            )
        try:
            payload = json.loads(raw.decode("utf-8"))
            if not isinstance(payload, Mapping):
                raise TypeError("observation root must be an object")
            observation = _observation_from_dict(payload)
        except (
            UnicodeDecodeError,
            json.JSONDecodeError,
            KeyError,
            TypeError,
            ValueError,
        ) as exc:
            raise ValidationError(
                "committed legacy bootstrap observation is invalid",
                path=relative,
                card_id=ref.gameplay_card_id,
                details={"error": str(exc)},
            ) from exc
        expected = _bootstrap_observation(
            ref.gameplay_card_id,
            observation.extracted,
        )
        if observation != expected or observation.observation_id != ref.observation_id:
            raise ValidationError(
                "legacy bootstrap observation does not match its frozen identity",
                path=relative,
                card_id=ref.gameplay_card_id,
            )
        extracted_card_id = str(
            observation.extracted.get("card_id", "")
        ).upper()
        if extracted_card_id != ref.gameplay_card_id:
            raise ValidationError(
                "legacy bootstrap observation belongs to another card",
                path=relative,
                card_id=ref.gameplay_card_id,
                details={"extracted_card_id": extracted_card_id},
            )
        if ref.gameplay_card_id in observations_by_card:
            raise ValidationError(
                "legacy bootstrap scope has duplicate card evidence",
                card_id=ref.gameplay_card_id,
            )
        observations[observation.observation_id] = observation
        observations_by_card[ref.gameplay_card_id] = observation

    expected_cards = set(legacy_entries)
    actual_cards = set(observations_by_card)
    if actual_cards != expected_cards:
        raise ValidationError(
            "legacy bootstrap scope does not cover the frozen override ledger",
            details={
                "missing_card_ids": sorted(
                    expected_cards - actual_cards, key=natural_card_id_key
                ),
                "unexpected_card_ids": sorted(
                    actual_cards - expected_cards, key=natural_card_id_key
                ),
            },
        )

    expected_refs = tuple(
        ScopeObservationRef(
            observation_id=observations_by_card[card_id].observation_id,
            gameplay_card_id=card_id,
            printing_id=f"{card_id}:legacy-official-mirror",
        )
        for card_id in sorted(legacy_entries, key=natural_card_id_key)
    )
    expected_manifest = ScopeManifest(
        schema_version=SCHEMA_VERSION,
        source=SourceName.OFFICIAL,
        scope=BOOTSTRAP_SCOPE_NAME,
        source_locations=(BOOTSTRAP_SOURCE_ROOT,),
        first_acquired_at=BOOTSTRAP_FIRST_ACQUIRED_AT,
        parser_version=BOOTSTRAP_PARSER_VERSION,
        source_schema_version=BOOTSTRAP_SOURCE_SCHEMA_VERSION,
        observation_ids=tuple(ref.observation_id for ref in expected_refs),
        observation_refs=expected_refs,
        complete=True,
        expected_count=len(legacy_entries),
        failed_ids=(),
    )
    if manifest != expected_manifest:
        raise ValidationError(
            "committed legacy bootstrap scope differs from frozen migration evidence",
            path=BOOTSTRAP_SCOPE_PATH,
        )
    return manifest, observations, observations_by_card


def _mechanical_correction(
    *,
    card_id: str,
    field_path: str,
    value: Any,
    observation: SourceObservation,
) -> tuple[ReviewedCorrection, Any, str]:
    projection = _official_projection(
        field_path,
        observation.extracted,
    )
    relation = _official_relation(value, projection, field_path)
    return (
        ReviewedCorrection(
            schema_version=SCHEMA_VERSION,
            correction_id=f"{BOOTSTRAP_CORRECTION_PREFIX}{card_id}-{field_path}",
            card_id=card_id,
            field_path=field_path,
            value=value,
            evidence_observation_ids=(observation.observation_id,),
            reason=_correction_reason(relation),
            reviewer=BOOTSTRAP_REVIEWER,
            reviewed_at=BOOTSTRAP_REVIEWED_AT,
        ),
        projection,
        relation,
    )


def _card_validation_observations(
    *,
    artifact_bytes: Mapping[str, bytes],
    card: Any,
    seed: Mapping[str, SourceObservation],
) -> dict[str, SourceObservation]:
    """Load only the marker-owned observations referenced by one card."""

    from .live_reconcile import _observation_from_dict

    observation_ids = {
        observation_id
        for provenance in card.provenance.values()
        for observation_id in provenance.observation_ids
    }
    observation_ids.update(
        observation_id
        for printing in card.printings
        for observation_id in printing.observation_ids
    )
    observation_ids.update(
        observation_id
        for conflict in card.conflicts
        for observation_id in conflict.candidate_observation_ids
    )
    result = dict(seed)
    for observation_id in sorted(observation_ids - result.keys()):
        matches = [
            relative
            for relative in artifact_bytes
            if relative.startswith("data/card_sources/observations/")
            and relative.endswith(f"/{observation_id}.json")
        ]
        if len(matches) != 1:
            raise ValidationError(
                "canonical validation evidence is not uniquely marker-owned",
                card_id=card.card_id,
                details={
                    "observation_id": observation_id,
                    "paths": sorted(matches),
                },
            )
        relative = matches[0]
        try:
            payload = json.loads(artifact_bytes[relative].decode("utf-8"))
            if not isinstance(payload, Mapping):
                raise TypeError("observation root must be an object")
            observation = _observation_from_dict(payload)
        except (
            UnicodeDecodeError,
            json.JSONDecodeError,
            KeyError,
            TypeError,
            ValueError,
        ) as exc:
            raise ValidationError(
                "canonical validation evidence is invalid",
                path=relative,
                card_id=card.card_id,
                details={"error": str(exc)},
            ) from exc
        if (
            observation.observation_id != observation_id
            or observation_digest(observation) != observation_id
        ):
            raise ValidationError(
                "canonical validation evidence has a stale identity",
                path=relative,
                card_id=card.card_id,
            )
        result[observation_id] = observation
    return result


def _require_historical_input(
    inputs: Mapping[str, Any],
    name: str,
    *,
    expected_path: str,
    expected_digest: str,
) -> Mapping[str, Any]:
    value = inputs.get(name)
    if not isinstance(value, Mapping):
        raise ValidationError(
            "legacy reconciliation report has an invalid input binding",
            details={"input": name},
        )
    digest = value.get("sha256")
    if (
        value.get("path") != expected_path
        or digest != expected_digest
    ):
        raise ValidationError(
            "legacy reconciliation report has an invalid input binding",
            details={"input": name},
        )
    return value


def check_committed_migration(repo_root: Path) -> CommittedMigrationCheck:
    """Validate the reviewed migration after canonical producer cutover.

    ``card_overrides.json`` and the reconciliation report are frozen historical
    evidence.  Marker-owned canonical records and reviewed corrections are the
    live authority.  Consequently this check validates each durable
    disposition instead of requiring the historical override value to remain
    byte-for-byte visible in the generated compatibility projection.
    """

    from .generate import load_committed_generation_stack

    marker_path = repo_root / REVISION_MARKER_PATH
    journal_path = repo_root / TRANSACTION_JOURNAL_PATH
    stack = load_committed_generation_stack(
        repo_root,
        marker_path=marker_path,
        journal_path=journal_path,
    )

    override_path = repo_root / "data/card_overrides.json"
    report_relative = "qa/qa-reports/card_overrides_reconciliation.json"
    report_path = repo_root / report_relative
    legacy_document = load_legacy_overrides(override_path)
    legacy_entries = {
        card_id: fields
        for card_id, fields in legacy_document.items()
        if not card_id.startswith("_")
    }
    report_raw = stack.artifact_bytes.get(report_relative)
    if isinstance(report_raw, bytes):
        try:
            report = json.loads(report_raw.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as exc:
            raise ValidationError(
                "marker-owned legacy reconciliation report is invalid",
                path=report_path,
                details={"error": str(exc)},
            ) from exc
    else:
        # The cutover report predates marker ownership and is intentionally
        # frozen. A later candidate may bring it under the marker; when that
        # happens the branch above keeps the read revision-consistent.
        report = _load_json(report_path)
    if not isinstance(report, Mapping):
        raise ValidationError(
            "legacy reconciliation report root must be an object",
            path=report_path,
        )

    manifest, observations, observations_by_card = _committed_bootstrap_evidence(
        artifact_bytes=stack.artifact_bytes,
        legacy_entries=legacy_entries,
    )
    if content_digest(manifest) != HISTORICAL_BOOTSTRAP_SCOPE_SHA256:
        raise ValidationError(
            "committed legacy bootstrap scope differs from the historical cutover",
            path=BOOTSTRAP_SCOPE_PATH,
        )
    if (
        content_digest(manifest.observation_ids)
        != HISTORICAL_BOOTSTRAP_OBSERVATION_IDS_SHA256
    ):
        raise ValidationError(
            "committed legacy bootstrap observations differ from the historical cutover",
            path=BOOTSTRAP_SCOPE_PATH,
        )

    mechanical: dict[str, ReviewedCorrection] = {}
    projections: dict[str, Any] = {}
    relations: dict[str, str] = {}
    for card_id in sorted(legacy_entries, key=natural_card_id_key):
        observation = observations_by_card[card_id]
        for field_path in sorted(legacy_entries[card_id]):
            correction, projection, relation = _mechanical_correction(
                card_id=card_id,
                field_path=field_path,
                value=legacy_entries[card_id][field_path],
                observation=observation,
            )
            mechanical[correction.correction_id] = correction
            projections[correction.correction_id] = projection
            relations[correction.correction_id] = relation

    correction_by_id = {
        correction.correction_id: correction for correction in stack.corrections
    }
    correction_by_target = {
        (correction.card_id, correction.field_path): correction
        for correction in stack.corrections
    }
    active_bootstrap = {
        correction_id: correction
        for correction_id, correction in correction_by_id.items()
        if correction_id.startswith(BOOTSTRAP_CORRECTION_PREFIX)
    }
    unexpected_bootstrap = sorted(set(active_bootstrap) - set(mechanical))
    if unexpected_bootstrap:
        raise ValidationError(
            "committed correction ledger contains unknown legacy bootstrap rows",
            details={"correction_ids": unexpected_bootstrap},
        )

    expected_rows: list[dict[str, Any]] = []
    disposition_counts: Counter[str] = Counter()
    reason_counts: Counter[str] = Counter()
    relation_counts: Counter[str] = Counter()
    followups: list[dict[str, str]] = []
    for expected in mechanical.values():
        card = stack.canonical_cards.get(expected.card_id)
        if card is None:
            raise ValidationError(
                "committed canonical data is missing a legacy override card",
                card_id=expected.card_id,
            )
        current = correction_by_id.get(expected.correction_id)
        target_correction = correction_by_target.get(
            (expected.card_id, expected.field_path)
        )
        disposition_details: dict[str, Any]
        if current is not None:
            if current != expected:
                raise ValidationError(
                    "committed legacy bootstrap correction differs from its frozen migration",
                    card_id=expected.card_id,
                    field_path=expected.field_path,
                    details={"correction_id": expected.correction_id},
                )
            if not correction_is_already_applied(
                card,
                current,
                observations=observations,
            ):
                raise ValidationError(
                    "committed canonical card does not preserve its legacy correction",
                    card_id=expected.card_id,
                    field_path=expected.field_path,
                    details={"correction_id": expected.correction_id},
                )
            disposition = "active_bootstrap_correction"
            disposition_details = {
                "correction_disposition_reason": (
                    "required_to_preserve_legacy_compatibility_value"
                )
            }
        elif target_correction is not None:
            disposition = "superseded_by_reviewed_correction"
            disposition_details = {
                "correction_disposition_reason": (
                    "superseded_by_reviewed_sparse_correction"
                ),
                "superseding_correction_id": target_correction.correction_id,
            }
        else:
            disposition = "reviewed_no_active_correction"
            try:
                corrected = apply_correction(
                    card,
                    expected,
                    observations=observations,
                )
            except ValidationError as exc:
                if "no-op" not in str(exc):
                    raise ValidationError(
                        "committed migration cannot revalidate an inactive legacy correction",
                        card_id=expected.card_id,
                        field_path=expected.field_path,
                        details={
                            "correction_id": expected.correction_id,
                            "error": str(exc),
                        },
                    ) from exc
                disposition_details = {
                    "correction_disposition_reason": (
                        "normalized_value_already_matches"
                    )
                }
            else:
                validation_observations = _card_validation_observations(
                    artifact_bytes=stack.artifact_bytes,
                    card=card,
                    seed=observations,
                )
                current_validation = validate_canonical_card(
                    card,
                    observations=validation_observations,
                    include_conflict_diagnostics=False,
                )
                if current_validation.has_errors:
                    raise ValidationError(
                        "committed canonical card is invalid before legacy "
                        "correction classification",
                        card_id=expected.card_id,
                        field_path=expected.field_path,
                        details={"correction_id": expected.correction_id},
                    )
                validation = validate_canonical_card(
                    corrected,
                    observations=validation_observations,
                    include_conflict_diagnostics=False,
                )
                if not validation.has_errors:
                    raise ValidationError(
                        "committed correction ledger omits a valid legacy correction",
                        card_id=expected.card_id,
                        field_path=expected.field_path,
                        details={"correction_id": expected.correction_id},
                    )
                diagnostics = validation.to_dict()["diagnostics"]
                disposition_details = {
                    "correction_disposition_reason": (
                        "rejected_by_canonical_validation"
                    ),
                    "canonical_validation_diagnostic_codes": sorted(
                        {
                            str(row["code"])
                            for row in diagnostics
                            if row.get("severity") == "error"
                        }
                    ),
                    "canonical_validation_messages": sorted(
                        {
                            str(row["message"])
                            for row in diagnostics
                            if row.get("severity") == "error"
                        }
                    ),
                }

        correction_id = expected.correction_id
        relation = relations[correction_id]
        row = {
            "card_id": expected.card_id,
            "field_path": expected.field_path,
            "legacy_value": expected.value,
            # These two fields describe the frozen pre-cutover compatibility
            # snapshot bound in report.inputs; they are deliberately not
            # recomputed from marker-owned post-cutover cards.json.
            "compatibility_value": expected.value,
            "compatibility_equivalent": True,
            "official_projection": projections[correction_id],
            "official_relation": relation,
            "correction_id": correction_id,
            "evidence_observation_id": expected.evidence_observation_ids[0],
            "correction_disposition": disposition,
            "active_correction_id": (
                target_correction.correction_id
                if target_correction is not None
                else None
            ),
            **disposition_details,
        }
        expected_rows.append(row)
        disposition_counts[disposition] += 1
        reason_counts[row["correction_disposition_reason"]] += 1
        relation_counts[relation] += 1
        if relation in {"conflict", "not_directly_comparable"}:
            followups.append(
                {
                    "card_id": expected.card_id,
                    "field_path": expected.field_path,
                    "official_relation": relation,
                }
            )

    inputs = report.get("inputs")
    if not isinstance(inputs, Mapping):
        raise ValidationError(
            "legacy reconciliation report requires input bindings",
            path=report_path,
        )
    override_input = _require_historical_input(
        inputs,
        "legacy_overrides",
        expected_path="data/card_overrides.json",
        expected_digest=HISTORICAL_LEGACY_OVERRIDES_SHA256,
    )
    _require_historical_input(
        inputs,
        "official_mirror",
        expected_path="data/card_official.json",
        expected_digest=HISTORICAL_OFFICIAL_MIRROR_SHA256,
    )
    _require_historical_input(
        inputs,
        "compatibility_cards",
        expected_path="data/cards.json",
        expected_digest=HISTORICAL_COMPATIBILITY_CARDS_SHA256,
    )
    if override_input["sha256"] != _input_sha256(override_path):
        raise ValidationError(
            "frozen legacy override input differs from its reconciliation report",
            path=override_path,
        )

    expected_bootstrap = {
        "kind": "legacy_official_mirror_bootstrap",
        "source": "official",
        "source_url_root": BOOTSTRAP_SOURCE_ROOT,
        "parser_version": BOOTSTRAP_PARSER_VERSION,
        "source_schema_version": BOOTSTRAP_SOURCE_SCHEMA_VERSION,
        "record_bytes_contract": BOOTSTRAP_RECORD_SERIALIZATION,
        "raw_http_retained": False,
        "first_acquired_at": BOOTSTRAP_FIRST_ACQUIRED_AT,
        "first_acquired_at_semantics": (
            "deterministic migration epoch; original upstream acquisition "
            "time is unavailable"
        ),
        "reviewer": BOOTSTRAP_REVIEWER,
        "reviewed_at": BOOTSTRAP_REVIEWED_AT,
        "scope": BOOTSTRAP_SCOPE_NAME,
        "scope_manifest": BOOTSTRAP_SCOPE_PATH,
    }
    expected_summary = {
        "legacy_card_count": len(legacy_entries),
        "legacy_field_count": len(expected_rows),
        "migration_correction_count": len(active_bootstrap),
        "active_legacy_field_correction_count": (
            disposition_counts["active_bootstrap_correction"]
            + disposition_counts["superseded_by_reviewed_correction"]
        ),
        "reviewed_no_active_correction_count": disposition_counts[
            "reviewed_no_active_correction"
        ],
        "superseded_by_reviewed_correction_count": disposition_counts[
            "superseded_by_reviewed_correction"
        ],
        "bootstrap_observation_count": len(observations),
        "compatibility_equivalent_field_count": len(expected_rows),
        "unmigrated_field_count": 0,
        "orphan_reference_count": 0,
        "official_relation_counts": {
            key: relation_counts[key]
            for key in (
                "exact",
                "legacy_superset",
                "conflict",
                "not_directly_comparable",
            )
        },
        "official_followup_count": len(followups),
    }
    expected_output_digests = {
        "scope_manifest_sha256": HISTORICAL_BOOTSTRAP_SCOPE_SHA256,
        # This is the one-time migration output digest, not a promise that
        # unrelated reviewed corrections can never be added later. The live
        # bootstrap subset is validated row-by-row above.
        "corrections_sha256": HISTORICAL_CORRECTIONS_SHA256,
        "observation_ids_sha256": (
            HISTORICAL_BOOTSTRAP_OBSERVATION_IDS_SHA256
        ),
    }
    expected_root_keys = {
        "schema_version",
        "migration",
        "inputs",
        "bootstrap_evidence",
        "summary",
        "output_digests",
        "legacy_metadata",
        "official_followups",
        "fields",
    }
    if set(report) != expected_root_keys:
        raise ValidationError(
            "legacy reconciliation report has missing or unknown fields",
            path=report_path,
            details={
                "missing": sorted(expected_root_keys - set(report)),
                "unknown": sorted(set(report) - expected_root_keys),
            },
        )
    expected_sections = {
        "schema_version": 1,
        "migration": "legacy-card-overrides-to-reviewed-sparse-corrections",
        "bootstrap_evidence": expected_bootstrap,
        "summary": expected_summary,
        "output_digests": expected_output_digests,
        "legacy_metadata": {
            key: value
            for key, value in legacy_document.items()
            if key.startswith("_")
        },
        "official_followups": followups,
        "fields": expected_rows,
    }
    stale_sections = sorted(
        key for key, value in expected_sections.items() if report.get(key) != value
    )
    if stale_sections:
        row_differences: list[dict[str, Any]] = []
        if "fields" in stale_sections and isinstance(report.get("fields"), list):
            actual_rows = report["fields"]
            for index in range(max(len(expected_rows), len(actual_rows))):
                expected_row = expected_rows[index] if index < len(expected_rows) else None
                actual_row = actual_rows[index] if index < len(actual_rows) else None
                if expected_row == actual_row:
                    continue
                keys = (
                    set(expected_row or {}) | set(actual_row or {})
                    if isinstance(actual_row, Mapping)
                    else set(expected_row or {})
                )
                row_differences.append(
                    {
                        "index": index,
                        "correction_id": (
                            expected_row.get("correction_id")
                            if expected_row is not None
                            else None
                        ),
                        "fields": sorted(
                            key
                            for key in keys
                            if (expected_row or {}).get(key)
                            != (actual_row or {}).get(key)
                        ),
                    }
                )
                if len(row_differences) == 10:
                    break
        raise ValidationError(
            "legacy reconciliation report disagrees with the committed correction ledger",
            path=report_path,
            details={
                "sections": stale_sections,
                "row_differences": row_differences,
            },
        )

    return CommittedMigrationCheck(
        revision=stack.revision,
        legacy_card_count=len(legacy_entries),
        legacy_field_count=len(expected_rows),
        active_bootstrap_correction_count=len(active_bootstrap),
        normalized_no_op_count=reason_counts["normalized_value_already_matches"],
        rejected_correction_count=reason_counts[
            "rejected_by_canonical_validation"
        ],
    )


def build_migration(
    repo_root: Path,
    *,
    canonical_root: Path | None = None,
) -> MigrationArtifacts:
    """Build every deterministic migration artifact without writing files."""

    data_dir = repo_root / "data"
    override_path = data_dir / "card_overrides.json"
    official_path = data_dir / "card_official.json"
    cards_path = data_dir / "cards.json"
    correction_path = data_dir / "card_corrections.json"
    reconciliation_path = (
        repo_root / "qa/qa-reports/card_overrides_reconciliation.json"
    )

    legacy_document = load_legacy_overrides(override_path)
    legacy_entries = {
        card_id: fields
        for card_id, fields in legacy_document.items()
        if not card_id.startswith("_")
    }
    official_payload = _load_json(official_path)
    official_cards = official_payload.get("cards") if isinstance(official_payload, dict) else None
    if not isinstance(official_cards, dict):
        raise ValidationError("card_official.json requires a cards object", path=official_path)
    cards = _load_json(cards_path)
    if not isinstance(cards, dict):
        raise ValidationError("cards.json must be an object", path=cards_path)

    missing_official = sorted(set(legacy_entries) - set(official_cards), key=natural_card_id_key)
    missing_cards = sorted(set(legacy_entries) - set(cards), key=natural_card_id_key)
    if missing_official or missing_cards:
        raise ValidationError(
            "legacy override migration has orphaned card entries",
            details={
                "missing_official_records": missing_official,
                "missing_compatibility_records": missing_cards,
            },
        )

    observations_by_card = {
        card_id: _bootstrap_observation(card_id, official_cards[card_id])
        for card_id in sorted(legacy_entries, key=natural_card_id_key)
    }
    observations = {
        observation.observation_id: observation
        for observation in observations_by_card.values()
    }
    if len(observations) != len(observations_by_card):
        raise ValidationError("bootstrap observation identity collision")

    refs = tuple(
        ScopeObservationRef(
            observation_id=observations_by_card[card_id].observation_id,
            gameplay_card_id=card_id,
            printing_id=f"{card_id}:legacy-official-mirror",
        )
        for card_id in sorted(legacy_entries, key=natural_card_id_key)
    )
    manifest = ScopeManifest(
        schema_version=SCHEMA_VERSION,
        source=SourceName.OFFICIAL,
        scope=BOOTSTRAP_SCOPE_NAME,
        source_locations=(BOOTSTRAP_SOURCE_ROOT,),
        first_acquired_at=BOOTSTRAP_FIRST_ACQUIRED_AT,
        parser_version=BOOTSTRAP_PARSER_VERSION,
        source_schema_version=BOOTSTRAP_SOURCE_SCHEMA_VERSION,
        observation_ids=tuple(ref.observation_id for ref in refs),
        observation_refs=refs,
        complete=True,
        expected_count=len(legacy_entries),
        failed_ids=(),
    )

    field_rows: list[dict[str, Any]] = []
    mechanical_corrections: list[ReviewedCorrection] = []
    relation_counts: Counter[str] = Counter()
    incompatible_rows: list[dict[str, str]] = []
    for card_id in sorted(legacy_entries, key=natural_card_id_key):
        observation = observations_by_card[card_id]
        for field_path in sorted(legacy_entries[card_id]):
            legacy_value = legacy_entries[card_id][field_path]
            compatibility_value = cards[card_id].get(field_path)
            compatible = compatibility_value == legacy_value
            if not compatible:
                incompatible_rows.append(
                    {"card_id": card_id, "field_path": field_path}
                )
            projection = _official_projection(field_path, official_cards[card_id])
            relation = _official_relation(legacy_value, projection, field_path)
            relation_counts[relation] += 1
            correction = ReviewedCorrection(
                schema_version=SCHEMA_VERSION,
                correction_id=(
                    f"{BOOTSTRAP_CORRECTION_PREFIX}{card_id}-{field_path}"
                ),
                card_id=card_id,
                field_path=field_path,
                value=legacy_value,
                evidence_observation_ids=(observation.observation_id,),
                reason=_correction_reason(relation),
                reviewer=BOOTSTRAP_REVIEWER,
                reviewed_at=BOOTSTRAP_REVIEWED_AT,
            )
            mechanical_corrections.append(correction)
            field_rows.append(
                {
                    "card_id": card_id,
                    "field_path": field_path,
                    "legacy_value": legacy_value,
                    "compatibility_value": compatibility_value,
                    "compatibility_equivalent": compatible,
                    "official_projection": projection,
                    "official_relation": relation,
                    "correction_id": correction.correction_id,
                    "evidence_observation_id": observation.observation_id,
                }
            )

    if incompatible_rows:
        raise ValidationError(
            "legacy overrides are not fully reflected in committed cards.json",
            details={"fields": incompatible_rows},
        )

    existing_document = correction_path.is_file()
    existing = load_reviewed_corrections(correction_path)
    mechanical_by_id = {
        correction.correction_id: correction
        for correction in mechanical_corrections
    }
    stale_bootstrap_rows = []
    if existing_document:
        for correction in existing:
            if not correction.correction_id.startswith(
                BOOTSTRAP_CORRECTION_PREFIX
            ):
                continue
            expected = mechanical_by_id.get(correction.correction_id)
            if expected is None or (
                correction.schema_version != expected.schema_version
                or correction.card_id != expected.card_id
                or correction.field_path != expected.field_path
                or correction.value != expected.value
            ):
                stale_bootstrap_rows.append(correction.correction_id)
    if stale_bootstrap_rows:
        raise ValidationError(
            "reviewed legacy bootstrap corrections differ from the "
            "current mechanical migration",
            details={"correction_ids": sorted(stale_bootstrap_rows)},
        )

    prior_dispositions: dict[str, dict[str, Any]] = {}
    if reconciliation_path.is_file():
        prior_report = _load_json(reconciliation_path)
        if isinstance(prior_report, Mapping):
            for prior_row in prior_report.get("fields", ()):
                if not isinstance(prior_row, Mapping):
                    continue
                correction_id = prior_row.get("correction_id")
                reason = prior_row.get("correction_disposition_reason")
                if not isinstance(correction_id, str) or not isinstance(
                    reason, str
                ):
                    continue
                prior_dispositions[correction_id] = {
                    key: prior_row[key]
                    for key in (
                        "correction_disposition_reason",
                        "superseding_correction_id",
                        "canonical_validation_diagnostic_codes",
                        "canonical_validation_messages",
                    )
                    if key in prior_row
                }

    canonical_dispositions: dict[str, dict[str, Any]] = {}
    if canonical_root is not None:
        all_corrections, canonical_dispositions = _canonical_reconciled_corrections(
            canonical_root.resolve(),
            tuple(mechanical_corrections),
            existing,
            legacy_cards=cards,
        )
    elif existing_document:
        # The first mechanical pass intentionally produced one candidate
        # correction for every legacy field.  That ledger was then reviewed
        # against canonical normalization, where stale/no-op and invalid
        # candidates were removed.  Once the reviewed ledger exists, its
        # active bootstrap subset is authoritative: recreating absent rows
        # would undo that review and make a clean canonical candidate fail its
        # own no-op/invariant gates.
        all_corrections = existing
    else:
        # Initial bootstrap only.  Subsequent runs preserve explicit review
        # removals instead of treating absence as an invitation to recreate a
        # stale correction.
        all_corrections = merge_reviewed_corrections((), mechanical_corrections)

    active_by_target = {
        (correction.card_id, correction.field_path): correction
        for correction in all_corrections
    }
    disposition_counts: Counter[str] = Counter()
    for row in field_rows:
        target = (row["card_id"], row["field_path"])
        active = active_by_target.get(target)
        if active is None:
            disposition = "reviewed_no_active_correction"
        elif active.correction_id == row["correction_id"]:
            disposition = "active_bootstrap_correction"
        else:
            disposition = "superseded_by_reviewed_correction"
        row["correction_disposition"] = disposition
        row["active_correction_id"] = (
            active.correction_id if active is not None else None
        )
        details = canonical_dispositions.get(
            row["correction_id"],
            prior_dispositions.get(row["correction_id"], {}),
        )
        if details:
            row.update(details)
        elif disposition == "active_bootstrap_correction":
            row["correction_disposition_reason"] = (
                "required_to_preserve_legacy_compatibility_value"
            )
        elif disposition == "superseded_by_reviewed_correction":
            row["correction_disposition_reason"] = (
                "superseded_by_reviewed_sparse_correction"
            )
        else:
            row["correction_disposition_reason"] = (
                "retained_reviewed_ledger_disposition"
            )
        disposition_counts[disposition] += 1

    migration_corrections = tuple(
        correction
        for correction in all_corrections
        if correction.correction_id.startswith(BOOTSTRAP_CORRECTION_PREFIX)
    )
    correction_payload = _corrections_payload(all_corrections)

    followup_rows = [
        {
            "card_id": row["card_id"],
            "field_path": row["field_path"],
            "official_relation": row["official_relation"],
        }
        for row in field_rows
        if row["official_relation"] in {"conflict", "not_directly_comparable"}
    ]
    report = {
        "schema_version": 1,
        "migration": "legacy-card-overrides-to-reviewed-sparse-corrections",
        "inputs": {
            "legacy_overrides": {
                "path": "data/card_overrides.json",
                "sha256": _input_sha256(override_path),
            },
            "official_mirror": {
                "path": "data/card_official.json",
                "sha256": _input_sha256(official_path),
            },
            "compatibility_cards": {
                "path": "data/cards.json",
                "sha256": _input_sha256(cards_path),
            },
        },
        "bootstrap_evidence": {
            "kind": "legacy_official_mirror_bootstrap",
            "source": "official",
            "source_url_root": BOOTSTRAP_SOURCE_ROOT,
            "parser_version": BOOTSTRAP_PARSER_VERSION,
            "source_schema_version": BOOTSTRAP_SOURCE_SCHEMA_VERSION,
            "record_bytes_contract": BOOTSTRAP_RECORD_SERIALIZATION,
            "raw_http_retained": False,
            "first_acquired_at": BOOTSTRAP_FIRST_ACQUIRED_AT,
            "first_acquired_at_semantics": (
                "deterministic migration epoch; original upstream acquisition "
                "time is unavailable"
            ),
            "reviewer": BOOTSTRAP_REVIEWER,
            "reviewed_at": BOOTSTRAP_REVIEWED_AT,
            "scope": BOOTSTRAP_SCOPE_NAME,
            "scope_manifest": (
                "data/card_sources/scopes/official/"
                "legacy-official-mirror-overrides-v1.json"
            ),
        },
        "summary": {
            "legacy_card_count": len(legacy_entries),
            "legacy_field_count": len(field_rows),
            "migration_correction_count": len(migration_corrections),
            "active_legacy_field_correction_count": sum(
                1
                for row in field_rows
                if row["active_correction_id"] is not None
            ),
            "reviewed_no_active_correction_count": disposition_counts[
                "reviewed_no_active_correction"
            ],
            "superseded_by_reviewed_correction_count": disposition_counts[
                "superseded_by_reviewed_correction"
            ],
            "bootstrap_observation_count": len(observations),
            "compatibility_equivalent_field_count": sum(
                1 for row in field_rows if row["compatibility_equivalent"]
            ),
            "unmigrated_field_count": 0,
            "orphan_reference_count": 0,
            "official_relation_counts": {
                key: relation_counts[key]
                for key in (
                    "exact",
                    "legacy_superset",
                    "conflict",
                    "not_directly_comparable",
                )
            },
            "official_followup_count": len(followup_rows),
        },
        "output_digests": {
            "scope_manifest_sha256": content_digest(manifest),
            "corrections_sha256": content_digest(correction_payload),
            "observation_ids_sha256": content_digest(manifest.observation_ids),
        },
        "legacy_metadata": {
            key: value
            for key, value in legacy_document.items()
            if key.startswith("_")
        },
        "official_followups": followup_rows,
        "fields": field_rows,
    }
    return MigrationArtifacts(
        observations=observations,
        manifest=manifest,
        corrections=all_corrections,
        migration_corrections=migration_corrections,
        report=report,
    )


def _paths(repo_root: Path) -> tuple[Path, Path, Path, Path]:
    observation_dir = repo_root / "data/card_sources/observations/official"
    scope_path = (
        repo_root
        / "data/card_sources/scopes/official/legacy-official-mirror-overrides-v1.json"
    )
    correction_path = repo_root / "data/card_corrections.json"
    report_path = repo_root / "qa/qa-reports/card_overrides_reconciliation.json"
    return observation_dir, scope_path, correction_path, report_path


def _expected_texts(
    repo_root: Path,
    artifacts: MigrationArtifacts,
) -> dict[Path, str]:
    observation_dir, scope_path, correction_path, report_path = _paths(repo_root)
    expected = {
        observation_dir / f"{observation_id}.json": canonical_json_text(observation)
        for observation_id, observation in artifacts.observations.items()
    }
    expected[scope_path] = canonical_json_text(artifacts.manifest)
    expected[correction_path] = canonical_json_text(
        _corrections_payload(artifacts.corrections)
    )
    expected[report_path] = canonical_json_text(artifacts.report)
    return expected


def _stale_bootstrap_observations(
    observation_dir: Path,
    expected: set[Path],
) -> tuple[Path, ...]:
    stale: list[Path] = []
    if not observation_dir.exists():
        return ()
    for path in observation_dir.glob("*.json"):
        if path in expected:
            continue
        try:
            payload = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            continue
        if payload.get("parser_version") == BOOTSTRAP_PARSER_VERSION:
            stale.append(path)
    return tuple(sorted(stale))


def check_migration(repo_root: Path, artifacts: MigrationArtifacts) -> tuple[str, ...]:
    expected = _expected_texts(repo_root, artifacts)
    failures: list[str] = []
    for path, text in expected.items():
        try:
            actual = path.read_text(encoding="utf-8")
        except OSError:
            failures.append(f"missing: {path.relative_to(repo_root).as_posix()}")
            continue
        if actual != text:
            failures.append(f"stale: {path.relative_to(repo_root).as_posix()}")
    observation_dir, _, _, _ = _paths(repo_root)
    for path in _stale_bootstrap_observations(observation_dir, set(expected)):
        failures.append(f"stale: {path.relative_to(repo_root).as_posix()}")
    return tuple(failures)


def write_migration(repo_root: Path, artifacts: MigrationArtifacts) -> None:
    expected = _expected_texts(repo_root, artifacts)
    observation_dir, scope_path, correction_path, report_path = _paths(repo_root)
    try:
        observation_dir.mkdir(parents=True, exist_ok=True)
        scope_path.parent.mkdir(parents=True, exist_ok=True)
        report_path.parent.mkdir(parents=True, exist_ok=True)
        for path, text in expected.items():
            if path == correction_path:
                continue
            path.write_text(text, encoding="utf-8")
        write_reviewed_corrections(correction_path, artifacts.corrections)
        for path in _stale_bootstrap_observations(observation_dir, set(expected)):
            path.unlink()
    except (OSError, WriteError) as exc:
        if isinstance(exc, WriteError):
            raise
        raise WriteError(
            "failed to write legacy override migration artifacts",
            details={"error": str(exc)},
        ) from exc


def main(argv: list[str] | None = None) -> int:
    from .transaction import committed_revision_protocol_active

    default_root = Path(__file__).resolve().parents[3]
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, default=default_root)
    parser.add_argument(
        "--canonical-root",
        type=Path,
        help=(
            "pre-cutover-only canonical candidate/repository root used to retire "
            "legacy migration rows that are not actively applied with "
            "adjudicated provenance"
        ),
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help=(
            "check exact generated migration artifacts before cutover, or "
            "validate the frozen dispositions against one consistent "
            "marker-owned canonical revision after cutover"
        ),
    )
    args = parser.parse_args(argv)
    repo_root = args.repo_root.resolve()
    revision_protocol_active = committed_revision_protocol_active(
        marker_path=repo_root / REVISION_MARKER_PATH,
        journal_path=repo_root / TRANSACTION_JOURNAL_PATH,
    )
    if revision_protocol_active:
        if not args.check:
            raise WriteError(
                "legacy override migration cannot write marker-owned post-cutover artifacts; "
                "publish reviewed updates through a complete candidate"
            )
        if args.canonical_root is not None:
            raise ValidationError(
                "--canonical-root is a pre-cutover candidate reconciliation option"
            )
        committed = check_committed_migration(repo_root)
        print(
            "validated committed legacy override reconciliation at "
            f"revision {committed.revision}: "
            f"{committed.active_bootstrap_correction_count} active, "
            f"{committed.normalized_no_op_count} normalized no-op, "
            f"{committed.rejected_correction_count} rejected"
        )
        return 0
    artifacts = build_migration(
        repo_root,
        canonical_root=(
            args.canonical_root.resolve()
            if args.canonical_root is not None
            else (
                repo_root
                if (repo_root / "data/cards_canonical.json").is_file()
                else None
            )
        ),
    )
    if args.check:
        failures = check_migration(repo_root, artifacts)
        for failure in failures:
            print(failure)
        return 1 if failures else 0
    write_migration(repo_root, artifacts)
    print(
        f"migrated {len(artifacts.migration_corrections)} fields across "
        f"{artifacts.manifest.expected_count} cards"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
