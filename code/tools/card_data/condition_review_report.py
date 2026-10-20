"""Read-only complete-source condition review; never generates source approvals."""
from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any, Mapping

from .compatibility_conditions import official_condition_projection
from .compatibility_corrections import (
    CorrectionDocument, apply_image_correction, correction_document_payload,
    load_correction_document, load_document_images, validate_image_artifacts,
)
from .compatibility_selections import (
    CompatibilitySourceSelection, load_compatibility_selections,
    selection_ledger_payload, validate_compatibility_selections,
)
from .complete_candidate import load_review_snapshots
from .errors import SourceSchemaError, ValidationError
from .models import SourceName
from .normalization import normalize_cards
from .serialization import canonical_json_text, content_digest, natural_card_id_key
from .source_manifest import build_full_pool_source_manifest
from .sources.official import OfficialScopeSnapshot
from .staging import stage_source_observations, ScopeKind, SyncScope


def _validate_snapshot(snapshot: OfficialScopeSnapshot, source: SourceName):
    manifest = snapshot.manifest
    if manifest.source is not source:
        raise SourceSchemaError("condition review source lanes are mismatched")
    prefix, separator, name = manifest.scope.partition(":")
    if prefix != "full-pool" or separator != ":" or not name:
        raise SourceSchemaError("condition review requires complete full-pool scopes")
    ids = [row.observation_id for row in snapshot.observations]
    if len(ids) != len(set(ids)) or set(ids) != set(manifest.observation_ids):
        raise SourceSchemaError("condition review observation inventory does not exactly match manifest")
    observations = {row.observation_id: row for row in snapshot.observations}
    stage_source_observations(scope=SyncScope(ScopeKind.FULL_POOL, name),
        manifest=manifest, observations=observations)
    for ref in manifest.observation_refs:
        row = observations[ref.observation_id]
        if row.extracted.get("card_id") != ref.gameplay_card_id:
            raise SourceSchemaError("condition review manifest card reference is inconsistent",
                details={"observation_id": ref.observation_id})
        if row.parser_version != manifest.parser_version or row.source_schema_version != manifest.source_schema_version:
            raise SourceSchemaError("condition review parser/schema version differs from manifest")
        if source is SourceName.OFFICIAL:
            printing = row.extracted.get("printing")
            if not isinstance(printing, Mapping) or printing.get("printing_id") != ref.printing_id:
                raise SourceSchemaError("condition review official printing reference is inconsistent",
                    details={"observation_id": ref.observation_id})
    return observations


def build_condition_review_report(
    official: OfficialScopeSnapshot, secondary: OfficialScopeSnapshot, *,
    selections: tuple[CompatibilitySourceSelection, ...] = (),
    correction_document: CorrectionDocument | None = None,
    correction_image_artifacts: Mapping[str, bytes] | None = None,
) -> dict[str, Any]:
    """Inspect the entire bound source universe without candidate generation.

    All official cards get a row. Secondary-only members are reported separately
    as lacking official condition authority; they are never silently admitted.
    """
    selections = validate_compatibility_selections(selections)
    observations = _validate_snapshot(official, SourceName.OFFICIAL)
    secondary_observations = _validate_snapshot(secondary, SourceName.DIGIMONCARD_IO)
    aggregate = build_full_pool_source_manifest((official.manifest, secondary.manifest))
    # The property hashes the entire immutable source inventory. Bind it once
    # for this report rather than repeating whole-pool work for every card.
    aggregate_revision = aggregate.manifest_revision
    cards = normalize_cards(observations.values())
    official_ids = {ref.gameplay_card_id for ref in official.manifest.observation_refs}
    secondary_ids = {ref.gameplay_card_id for ref in secondary.manifest.observation_refs}
    if set(cards) != official_ids:
        raise ValidationError("condition normalization did not preserve the full official inventory")
    images = correction_image_artifacts or {}
    current_corrections = {}
    if correction_document is not None:
        validate_image_artifacts(correction_document, images)
        for item in correction_document.compatibility_corrections:
            if item.source_manifest_revision == aggregate_revision:
                if item.card_id not in cards:
                    raise ValidationError("current compatibility correction targets an absent card")
                current_corrections[item.card_id] = item
        for item in correction_document.compatibility_corrections:
            if item.card_id in cards and item.card_id not in current_corrections:
                raise ValidationError("compatibility correction is stale for this source revision")
        if any(item.card_id in current_corrections and item.source_manifest_revision == aggregate_revision
               for item in selections):
            raise ValidationError("source selection overlaps a compatibility image correction")
    elif images:
        raise ValidationError("correction image evidence requires a complete correction document")
    rows = []
    for cid in sorted(cards, key=natural_card_id_key):
        card = cards[cid]
        problem = None
        try:
            value, evidence = official_condition_projection(card, observations)
            values = {oid: value for oid in evidence}
        except SourceSchemaError as exc:
            values = dict(exc.details.get("candidate_conditions", {}))
            problem = {"message": str(exc), "details": dict(exc.details)}
        groups: dict[str, list[str]] = {}
        for oid, value in sorted(values.items()):
            groups.setdefault(value, []).append(oid)
        row: dict[str, Any] = {
            "card_id": cid, "field_path": "xros_req", "status": "unresolved" if problem else "consensus",
            "canonical_conflicts": [{"field_path": conflict.field_path, "kind": conflict.kind.value,
                "reason": conflict.reason, "observation_ids": list(conflict.candidate_observation_ids),
                "requires_adjudication": conflict.requires_adjudication(card.admission)} for conflict in card.conflicts],
            "value_groups": [{"value": value, "value_digest": content_digest(value), "observation_ids": ids}
                for value, ids in sorted(groups.items())],
            "review_binding": {"card_id": cid, "field_path": "xros_req",
                "source_manifest_revision": aggregate_revision,
                "candidate_values": dict(sorted(values.items())), "candidate_set_digest": content_digest(values)},
            "observations": [{"observation_id": oid, "raw_payload_digest": observations[oid].raw_payload_digest,
                "source_url": observations[oid].source_url, "source_record_key": observations[oid].source_record_key,
                "printing_id": observations[oid].extracted["printing"]["printing_id"],
                "parser_version": observations[oid].parser_version} for oid in sorted(values)],
        }
        if problem:
            row["source_issue"] = problem
        if selections:
            try:
                selected_value, evidence = official_condition_projection(card, observations,
                    selections=selections, source_manifest_revision=aggregate_revision)
            except SourceSchemaError as exc:
                row["status"] = "unresolved"
                row["source_issue"] = {"message": str(exc), "details": dict(exc.details)}
            else:
                matching = [item for item in selections if item.card_id == cid
                    and item.source_manifest_revision == aggregate_revision
                    and dict(item.candidate_values) == values]
                if matching:
                    row.update(status="reviewed", selection_id=matching[0].selection_id,
                        selected_value=selected_value, selected_evidence=list(evidence))
        if cid in current_corrections:
            correction = current_corrections[cid]
            corrected, evidence = apply_image_correction(card, observations, correction,
                source_manifest_revision=aggregate_revision, image_artifacts=images)
            # Keep the uncorrected groups, issue and bindings. This explicitly
            # records image authority instead of claiming source consensus.
            row.update(status="reviewed_image_correction", correction_id=correction.correction_id,
                corrected_value=corrected, corrected_evidence=list(evidence),
                image_artifact_path=correction.image_artifact_path, image_sha256=correction.image_sha256)
        rows.append(row)
    unresolved = sum(row["status"] == "unresolved" for row in rows)
    report = {"schema_version": 1,
        "status": "blocked" if unresolved else "condition_consensus_pass",
        "aggregate_source_revision": aggregate_revision,
        "official_manifest_digest": content_digest(official.manifest),
        "secondary_manifest_digest": content_digest(secondary.manifest),
        "official_card_count": len(official_ids), "official_observation_count": len(observations),
        "secondary_card_count": len(secondary_ids), "secondary_observation_count": len(secondary_observations),
        "aggregate_card_count": len(official_ids | secondary_ids),
        "secondary_only_card_ids": sorted(secondary_ids - official_ids, key=natural_card_id_key),
        "unresolved_card_count": unresolved, "cards": rows,
        "canonical_required_conflict_count": sum(c["requires_adjudication"] for row in rows for c in row["canonical_conflicts"]),
        "selection_ledger_revision": content_digest(selection_ledger_payload(selections)),
        "selections_created": 0, "training_readiness_assessed": False, "runtime_cutover_approved": False}
    if correction_document is not None:
        report["correction_document_revision"] = content_digest(correction_document_payload(correction_document))
        report["image_correction_count"] = len(current_corrections)
        if not unresolved:
            report["status"] = "condition_projection_review_pass"
    report["report_digest"] = content_digest(report)
    return report


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--review-tree", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--compatibility-selections", type=Path)
    parser.add_argument("--corrections", type=Path)
    parser.add_argument("--correction-evidence-root", type=Path)
    args = parser.parse_args(argv)
    tree, output = args.review_tree.resolve(), args.output.resolve()
    if output.is_relative_to(tree):
        raise ValueError("condition review output must be outside the immutable source tree")
    if output.exists():
        raise FileExistsError(output)
    if args.compatibility_selections is not None and not args.compatibility_selections.is_file():
        raise FileNotFoundError(args.compatibility_selections)
    if (args.corrections is None) != (args.correction_evidence_root is None):
        raise ValueError("corrections and correction-evidence-root must be supplied together")
    if args.corrections is not None and not args.corrections.is_file():
        raise FileNotFoundError(args.corrections)
    official, secondary = load_review_snapshots(tree)
    selections = load_compatibility_selections(args.compatibility_selections) if args.compatibility_selections else ()
    document = load_correction_document(args.corrections) if args.corrections else None
    images = load_document_images(document, args.correction_evidence_root) if document is not None else None
    report = build_condition_review_report(official, secondary, selections=selections,
        correction_document=document, correction_image_artifacts=images)
    with output.open("x", encoding="utf-8") as stream:
        stream.write(canonical_json_text(report))
    print(canonical_json_text({key: report[key] for key in (
        "status", "aggregate_source_revision", "official_card_count", "secondary_card_count", "unresolved_card_count")}))
    return 2 if report["unresolved_card_count"] else 0


if __name__ == "__main__":
    raise SystemExit(main())
