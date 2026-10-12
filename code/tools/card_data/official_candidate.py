"""Build a review-only official canonical/compatibility producer candidate."""

from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Mapping

from .adjudications import load_migration_adjudications
from .compatibility_diff import approve_runtime_diffs, diff_runtime_fields
from .compatibility_conditions import official_condition_projection, refresh_official_conditions
from .compatibility_selections import (
    CompatibilitySourceSelection, SELECTION_LEDGER_PATH, load_compatibility_selections,
    selection_ledger_payload, selections_from_bytes, validate_compatibility_selections,
)
from .corrections import (
    apply_correction,
    correction_is_already_applied,
)
from .errors import ValidationError
from .compatibility_corrections import (
    CorrectionDocument, candidate_document, correction_document_payload,
    load_correction_document, load_document_images, correction_review_rows,
)
from .generate import (
    bootstrap_legacy_canonical_cards,
    bootstrap_legacy_compatibility_fields,
    generate_cards_json,
    render_legacy_cards_json,
)
from .models import (
    SCHEMA_VERSION,
    AdmissionStatus,
    AuthorityClass,
    CanonicalCard,
    MigrationAdjudication,
    ReviewedCorrection,
    SourceName,
    SourceObservation,
)
from .normalization import canonical_conflict_review_rows, normalize_cards
from .publication import CANDIDATE_INVENTORY_NAME, candidate_inventory_payload
from .registry import allocate_append_only, is_gameplay_card_id, read_registry
from .serialization import canonical_json_bytes, canonical_json_text, content_digest, natural_card_id_key
from .sources.official import OfficialScopeSnapshot, OfficialSourceAdapter
from .validation import validate_canonical_cards
from .validation import validate_canonical_card


@dataclass(frozen=True, slots=True)
class OfficialCandidate:
    canonical_cards: Mapping[str, CanonicalCard]
    observations: Mapping[str, SourceObservation]
    artifacts: Mapping[str, bytes]
    report: Mapping[str, Any]


def _source_bootstrap(
    card: CanonicalCard,
    observations: Mapping[str, SourceObservation],
    *, condition_value: str | None = None,
) -> dict[str, Any]:
    selected = card.provenance.get("card_name_eng")
    observation = (
        observations[selected.selected_observation_id]
        if selected is not None
        else observations[card.printings[0].observation_ids[0]]
    )
    xros_req = condition_value
    if xros_req is None:
        xros_req, _ = official_condition_projection(card, observations)
    return {
        "card_name_jpn": "",
        "rarity": observation.extracted.get("rarity"),
        "xros_req": xros_req,
    }


def _apply_current_corrections(
    cards: Mapping[str, CanonicalCard],
    corrections: tuple[ReviewedCorrection, ...],
    observations: Mapping[str, SourceObservation],
    *,
    allow_retired_replay: bool = False,
) -> tuple[dict[str, CanonicalCard], list[dict[str, Any]]]:
    output = dict(cards)
    rows: list[dict[str, Any]] = []
    for correction in corrections:
        card = output.get(correction.card_id)
        if card is None:
            rows.append(
                {
                    "correction_id": correction.correction_id,
                    "card_id": correction.card_id,
                    "field_path": correction.field_path,
                    "status": "orphaned",
                    "message": "card is absent from the official canonical scope",
                }
            )
            continue
        try:
            replayed = bool(
                allow_retired_replay
                and card.admission is AdmissionStatus.RETIRED
                and correction_is_already_applied(
                    card, correction, observations=observations
                )
            )
            candidate = (
                card
                if replayed
                else apply_correction(card, correction, observations=observations)
            )
        except ValidationError as exc:
            rows.append(
                {
                    "correction_id": correction.correction_id,
                    "card_id": correction.card_id,
                    "field_path": correction.field_path,
                    "status": "stale-or-noop",
                    "message": str(exc),
                }
            )
        else:
            correction_validation = validate_canonical_card(
                candidate,
                observations=observations,
                include_conflict_diagnostics=False,
            )
            if correction_validation.has_errors:
                rows.append(
                    {
                        "correction_id": correction.correction_id,
                        "card_id": correction.card_id,
                        "field_path": correction.field_path,
                        "status": "stale-or-invalid",
                        "message": "correction violates current canonical invariants",
                        "diagnostics": correction_validation.to_dict()["diagnostics"],
                    }
                )
                continue
            output[correction.card_id] = candidate
            rows.append(
                {
                    "correction_id": correction.correction_id,
                    "card_id": correction.card_id,
                    "field_path": correction.field_path,
                    "status": "applied",
                    **({"replayed_from_retired_record": True} if replayed else {}),
                }
            )
    return output, rows


def build_official_candidate(
    snapshot: OfficialScopeSnapshot,
    *,
    legacy_cards: Mapping[str, Mapping[str, Any]],
    registry,
    corrections: tuple[ReviewedCorrection, ...] = (),
    adjudications: tuple[MigrationAdjudication, ...] = (),
    correction_evidence: Mapping[str, SourceObservation] | None = None,
    compatibility_selections: tuple[CompatibilitySourceSelection, ...] = (),
    correction_document: CorrectionDocument | None = None,
    correction_image_artifacts: Mapping[str, bytes] | None = None,
) -> OfficialCandidate:
    correction_document, correction_images = candidate_document(
        corrections, correction_document, correction_image_artifacts or {})
    corrections = correction_document.corrections
    compatibility_selections = validate_compatibility_selections(compatibility_selections)
    correction_evidence = correction_evidence or {}
    live_observations = {item.observation_id: item for item in snapshot.observations}
    all_observations = {**correction_evidence, **live_observations}
    source_manifest_digest = content_digest(
        snapshot.manifest, exclude_operational=True
    )
    # Canonical conflict approvals bind only stable pre-adjudication source
    # inputs.  They deliberately do not depend on the resulting canonical
    # digest, which would make the approval revision circular.
    normalized = normalize_cards(
        snapshot.observations,
        adjudications=adjudications,
        field_selection_input_revision=source_manifest_digest,
    )
    conflict_rows = canonical_conflict_review_rows(
        normalized,
        snapshot.observations,
        adjudications=adjudications,
        input_revision=source_manifest_digest,
    )
    corrected, correction_rows = _apply_current_corrections(
        normalized, corrections, all_observations
    )
    for row in conflict_rows:
        provenance = corrected[row["card_id"]].provenance.get(row["field_path"])
        if (
            provenance is not None
            and provenance.adjudicated
            and provenance.authority is AuthorityClass.ADJUDICATED
        ):
            corrected_card = corrected[row["card_id"]]
            row["canonical_value_after_corrections"] = (
                corrected_card.faces
                if row["field_path"] == "faces"
                else corrected_card.fields[row["field_path"]]
            )
            row["correction_applied"] = True
            if not row["approval_required"]:
                row["status"] = "resolved_by_correction"
    current_conflict_approvals = {
        (row["card_id"], row["field_path"]): row["adjudication_id"]
        for row in conflict_rows
        if row["status"] == "current" and row["adjudication_id"] is not None
    }
    validation = validate_canonical_cards(
        corrected,
        observations=all_observations,
        canonical_field_selection_approvals=current_conflict_approvals,
    )

    candidate_registry = allocate_append_only(registry, corrected)
    legacy_bootstrap = bootstrap_legacy_compatibility_fields(legacy_cards)
    missing_bootstrap_ids = corrected.keys() - legacy_bootstrap.keys()
    legacy_bootstrap, compatibility_evidence = refresh_official_conditions(
        corrected, live_observations, legacy_bootstrap, selections=compatibility_selections,
        source_manifest_revision=source_manifest_digest,
        compatibility_corrections=correction_document.compatibility_corrections,
        correction_image_artifacts=correction_images,
    )
    for card_id in missing_bootstrap_ids:
        legacy_bootstrap[card_id] = _source_bootstrap(corrected[card_id], live_observations,
            condition_value=legacy_bootstrap[card_id]["xros_req"])
    generation_cards = dict(corrected)
    legacy_canonical = bootstrap_legacy_canonical_cards(legacy_cards)
    for card_id in candidate_registry.assignments:
        if card_id not in generation_cards:
            generation_cards[card_id] = legacy_canonical[card_id]
    generated = generate_cards_json(
        generation_cards,
        candidate_registry,
        legacy_bootstrap=legacy_bootstrap,
    )
    generated = dict(sorted(generated.items(), key=lambda item: natural_card_id_key(item[0])))

    raw_diffs = diff_runtime_fields(legacy_cards, generated, corrected,
        compatibility_field_provenance=compatibility_evidence)
    old_input_revision = content_digest(legacy_cards)
    new_input_revision = content_digest({"canonical": corrected,
        "compatibility_selections": selection_ledger_payload(compatibility_selections),
        **({'compatibility_corrections': correction_document.compatibility_corrections}
           if correction_document.compatibility_corrections else {})})
    approved = approve_runtime_diffs(
        raw_diffs,
        corrections=corrections,
        adjudications=adjudications,
        observations=all_observations,
        old_input_revision=old_input_revision,
        new_input_revision=new_input_revision,
    )
    official_ids = set(corrected)
    legacy_real_ids = {card_id for card_id in legacy_cards if is_gameplay_card_id(card_id)}
    legacy_only_ids = legacy_real_ids - official_ids
    canonical_payload = {
        "schema_version": 1,
        "source_manifest_digest": source_manifest_digest,
        "source_manifests": {"official": source_manifest_digest},
        "secondary_only_ids": [],
        "cards": corrected,
    }
    artifacts = {
        **correction_images,
        SELECTION_LEDGER_PATH: canonical_json_bytes(selection_ledger_payload(compatibility_selections)),
        "data/cards_canonical.json": canonical_json_bytes(canonical_payload),
        "data/card_corrections.json": canonical_json_bytes(
            correction_document_payload(correction_document)
        ),
        "data/card_migration_adjudications.json": canonical_json_bytes(
            {
                "schema_version": SCHEMA_VERSION,
                "adjudications": adjudications,
            }
        ),
        "data/card_registry.json": canonical_json_bytes(candidate_registry.to_dict()),
        "data/cards.json": render_legacy_cards_json(generated),
        "data/card_sources/scopes/official/full-pool.json": canonical_json_bytes(
            snapshot.manifest
        ),
    }
    unapproved = [item for item in approved if not item.approved]
    unresolved_conflicts = [
        row
        for row in conflict_rows
        if row["approval_required"] and row["status"] != "current"
    ]
    approval_required_conflicts = [
        row for row in conflict_rows if row["approval_required"]
    ]
    report = {
        'compatibility_corrections': correction_review_rows(correction_document, source_manifest_digest),
        "schema_version": 1,
        "status": "blocked" if validation.has_errors or unapproved or legacy_only_ids or unresolved_conflicts or any(row["status"] != "applied" for row in correction_rows) else "ready",
        "official_card_count": len(official_ids),
        "legacy_real_card_count": len(legacy_real_ids),
        "official_card_ids": sorted(official_ids, key=natural_card_id_key),
        "official_only_ids": sorted(official_ids - legacy_real_ids, key=natural_card_id_key),
        "legacy_only_ids": sorted(legacy_only_ids, key=natural_card_id_key),
        "registry_additions": len(candidate_registry.assignments) - len(registry.assignments),
        "runtime_field_diff_count": len(approved),
        "approved_runtime_field_diff_count": len(approved) - len(unapproved),
        "runtime_field_diffs": [item.to_dict() for item in approved],
        "unapproved_runtime_field_diffs": [item.to_dict() for item in unapproved],
        "corrections": correction_rows,
        "canonical_conflict_count": len(conflict_rows),
        "approved_canonical_conflict_count": (
            len(approval_required_conflicts) - len(unresolved_conflicts)
        ),
        "approval_required_canonical_conflict_count": len(
            approval_required_conflicts
        ),
        "informational_canonical_conflict_count": (
            len(conflict_rows) - len(approval_required_conflicts)
        ),
        "unresolved_canonical_conflict_count": len(unresolved_conflicts),
        "canonical_conflicts": conflict_rows,
        "canonical_conflict_input_revision": source_manifest_digest,
        "canonical_validation": validation.to_dict(),
        "source_manifest_digest": source_manifest_digest,
        "compatibility_selection_ledger_revision": content_digest(selection_ledger_payload(compatibility_selections)),
        "old_input_revision": old_input_revision,
        "new_input_revision": new_input_revision,
        "candidate_canonical_revision": content_digest(corrected),
    }
    return OfficialCandidate(corrected, all_observations, artifacts, report)


def _load_correction_evidence(root: Path) -> dict[str, SourceObservation]:
    from .live_reconcile import _load_observations
    from .models import SourceName

    return _load_observations(root, SourceName.OFFICIAL)


def _load_official_snapshot(root: Path) -> OfficialScopeSnapshot:
    """Reload a previously acquired candidate without contacting upstream."""

    from .live_reconcile import OFFICIAL_MANIFEST, _load_manifest, _load_observations

    manifest_path = root / OFFICIAL_MANIFEST
    manifest = _load_manifest(manifest_path)
    if manifest is None:
        raise ValidationError("official full-pool manifest is unavailable", path=manifest_path)
    observations = _load_observations(root, SourceName.OFFICIAL)
    missing = sorted(set(manifest.observation_ids) - set(observations))
    if missing:
        raise ValidationError(
            "official full-pool manifest has unresolved observations",
            path=manifest_path,
            details={"observation_ids": missing},
        )
    return OfficialScopeSnapshot(
        manifest=manifest,
        observations=tuple(observations[item] for item in manifest.observation_ids),
    )


def write_candidate(candidate: OfficialCandidate, output_dir: Path) -> None:
    written: dict[str, bytes] = {}

    def write_artifact(relative: str, body: bytes) -> None:
        path = output_dir / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(body)
        written[relative] = body

    for relative, body in candidate.artifacts.items():
        write_artifact(relative, body)
    for observation_id, observation in candidate.observations.items():
        if observation.source.value != "official":
            continue
        write_artifact(
            f"data/card_sources/observations/official/{observation_id}.json",
            canonical_json_text(observation).encode("utf-8"),
        )
    write_artifact(
        "qa/qa-reports/official-canonical-candidate.json",
        canonical_json_text(candidate.report).encode("utf-8"),
    )
    inventory_path = output_dir / CANDIDATE_INVENTORY_NAME
    inventory_path.parent.mkdir(parents=True, exist_ok=True)
    inventory_path.write_text(
        canonical_json_text(candidate_inventory_payload(written)),
        encoding="utf-8",
        newline="\n",
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument(
        "--snapshot-root",
        type=Path,
        help="reuse an acquired official candidate tree without network access",
    )
    parser.add_argument("--acquired-at", default="2026-07-12T00:00:00Z")
    args = parser.parse_args(argv)
    root = args.repo_root.resolve()
    snapshot = (
        _load_official_snapshot(args.snapshot_root.resolve())
        if args.snapshot_root is not None
        else OfficialSourceAdapter().acquire_catalog(
            first_acquired_at=args.acquired_at,
            existing_observations=_load_correction_evidence(root),
        )
    )
    legacy_cards = json.loads((root / "data/cards.json").read_text(encoding="utf-8"))
    document = load_correction_document(root / 'data/card_corrections.json')
    candidate = build_official_candidate(
        snapshot,
        legacy_cards=legacy_cards,
        registry=read_registry(root / "data/card_registry.json"),
        correction_document=document,
        correction_image_artifacts=load_document_images(document, root),
        adjudications=load_migration_adjudications(
            root / "data/card_migration_adjudications.json"
        ),
        correction_evidence=_load_correction_evidence(root),
        compatibility_selections=load_compatibility_selections(root / SELECTION_LEDGER_PATH),
    )
    write_candidate(candidate, args.output_dir.resolve())
    compact_summary = {
        "status": candidate.report["status"],
        "candidate_canonical_revision": candidate.report[
            "candidate_canonical_revision"
        ],
        "official_card_count": candidate.report["official_card_count"],
        "official_only_count": len(candidate.report["official_only_ids"]),
        "legacy_only_count": len(candidate.report["legacy_only_ids"]),
        "registry_additions": candidate.report["registry_additions"],
        "runtime_field_diff_count": candidate.report["runtime_field_diff_count"],
        "approved_runtime_field_diff_count": candidate.report[
            "approved_runtime_field_diff_count"
        ],
        "canonical_validation_error_count": candidate.report[
            "canonical_validation"
        ]["summary"]["by_severity"].get("error", 0),
        "report_path": (
            args.output_dir.resolve()
            / "qa/qa-reports/official-canonical-candidate.json"
        ).as_posix(),
    }
    print(canonical_json_text(compact_summary), end="")
    return 0 if candidate.report["status"] == "ready" else 2


if __name__ == "__main__":
    raise SystemExit(main())
