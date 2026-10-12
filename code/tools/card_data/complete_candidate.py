"""Assemble one complete, review-only card-data publication candidate.

The live reconciliation command intentionally stops at immutable source
observations and source-plan review.  This module is the next offline step: it
consumes those already-acquired official and secondary full-pool trees,
normalizes their union, renders every artifact that is safe at the current
review boundary, and seals the exact file inventory.  It never publishes a
candidate and it never creates migration approvals.

Campaign and pool artifacts are deliberately downstream of canonical
readiness.  A blocked candidate remains useful for review, but its report
explicitly records that derived compatibility, campaign, and pool outputs were
omitted rather than presenting partial outputs as a complete revision.
"""

from __future__ import annotations

import argparse
import json
import shutil
import tempfile
from dataclasses import dataclass, replace
from pathlib import Path
from typing import Any, Iterable, Mapping

from card_data_reader import load_json_artifacts
from ..build_tested_cards import build_snapshot as build_tested_cards_snapshot

from .adjudications import (
    _row as _adjudication_from_payload,
    load_migration_adjudications,
    validate_migration_adjudications,
)
from .campaign_generate import (
    CampaignSources,
    build_campaign_manifest,
    render_manifest,
)
from .compatibility_diff import approve_runtime_diffs, diff_runtime_fields
from .compatibility_conditions import refresh_official_conditions
from .compatibility_selections import (
    CompatibilitySourceSelection, SELECTION_LEDGER_PATH, load_compatibility_selections,
    selection_ledger_payload, selections_from_bytes, validate_compatibility_selections,
)
from .compatibility_corrections import (
    CorrectionDocument, candidate_document, correction_document_from_bytes,
    correction_document_payload, correction_review_rows, load_correction_document,
    load_document_images, require_document_extension, validate_image_artifacts,
)
from .derived_artifacts import generate_derived_artifacts
from .errors import AdjudicationError, CardDataError, ValidationError
from .generate import (
    COMPATIBILITY_PROJECTION_PATH,
    CompatibilityProjection,
    bootstrap_legacy_canonical_cards,
    bootstrap_legacy_compatibility_fields,
    build_compatibility_projection,
    generate_cards_json,
    load_committed_generation_stack,
    render_legacy_cards_json,
)
from .live_reconcile import (
    OFFICIAL_MANIFEST,
    SECONDARY_MANIFEST,
    _load_manifest,
    _load_observations,
    _manifest_from_dict,
    _observation_from_dict,
    build_reconciliation,
)
from .models import (
    SCHEMA_VERSION,
    AdmissionStatus,
    AdjudicationKind,
    AuthorityClass,
    CardFace,
    CanonicalCard,
    FaceKind,
    FieldConflict,
    FieldConflictKind,
    FieldProvenance,
    MigrationAdjudication,
    Printing,
    ReviewedCorrection,
    ScopeManifest,
    SourceName,
    SourceObservation,
)
from .normalization import canonical_conflict_review_rows, normalize_card
from .official_candidate import _apply_current_corrections, _source_bootstrap
from .pool_generate import build_pool_artifacts
from .pool_impact import (
    build_current_production_lane_report,
    render_report as render_pool_impact_report,
)
from .production_registry_generate import validate_production_registry_payload
from .publication import (
    CANDIDATE_INVENTORY_NAME,
    candidate_inventory_payload,
    load_candidate_artifacts,
)
from .registry import (
    StableCardRegistry,
    allocate_append_only,
    is_gameplay_card_id,
    read_registry,
)
from .serialization import (
    canonical_json_bytes,
    canonical_json_text,
    content_digest,
    natural_card_id_key,
    sha256_hex,
)
from .source_manifest import AggregateSourceManifest, build_full_pool_source_manifest
from .sources.official import OfficialScopeSnapshot
from .staging import ScopeKind, SyncScope, stage_source_observations
from .sync_plan import (
    CompositeSyncPlan,
    SourcePlan,
    load_source_plan,
    source_plan_components,
)
from .transaction import (
    EMPTY_REVISION,
    RevisionMarker,
    committed_revision_protocol_active,
    current_revision,
    write_revision_marker,
)
from .validation import validate_canonical_cards


COMPLETE_CANDIDATE_REPORT = "qa/qa-reports/official-canonical-candidate.json"
CAMPAIGN_MANIFEST_PATH = "qa/qa-reports/card_implementation_manifest.json"
POOL_IMPACT_PATH = "qa/qa-reports/registered_vs_validated_pool_impact.json"
CURRENT_PRODUCTION_LANE_IMPACT_PATH = (
    "qa/qa-reports/tested_cards_vs_production_lane.json"
)
RUNTIME_POOL_PATH = "data/runtime_registered_cards.json"
VALIDATED_POOL_PATH = "data/validated_playable_cards.json"
PRODUCTION_REGISTRY_PATH = "data/production_runtime_registry.json"
TESTED_CARDS_PATH = "data/tested_cards.json"


@dataclass(frozen=True, slots=True)
class SourceTransitionBinding:
    """Exact non-circular source inputs for one admission/retirement review."""

    card_id: str
    old_input_revision: str
    new_input_revision: str
    old_observation_ids: tuple[str, ...]
    new_observation_ids: tuple[str, ...]

    @staticmethod
    def _membership_digest(card_id: str, observation_ids: Iterable[str]) -> str:
        return content_digest(
            {
                "card_id": card_id,
                "observation_ids": tuple(sorted(observation_ids)),
            }
        )

    @property
    def old_digest(self) -> str:
        return self._membership_digest(self.card_id, self.old_observation_ids)

    @property
    def new_digest(self) -> str:
        return self._membership_digest(self.card_id, self.new_observation_ids)

    def normalization_options(self) -> dict[str, str]:
        return {
            "old_input_revision": self.old_input_revision,
            "new_input_revision": self.new_input_revision,
            "old_digest": self.old_digest,
            "new_digest": self.new_digest,
        }

    def to_dict(self) -> dict[str, Any]:
        return {
            "card_id": self.card_id,
            "old_input_revision": self.old_input_revision,
            "new_input_revision": self.new_input_revision,
            "old_digest": self.old_digest,
            "new_digest": self.new_digest,
            "old_observation_ids": list(self.old_observation_ids),
            "new_observation_ids": list(self.new_observation_ids),
        }


@dataclass(frozen=True, slots=True)
class CompleteCandidate:
    canonical_cards: Mapping[str, CanonicalCard]
    observations: Mapping[str, SourceObservation]
    source_manifest: AggregateSourceManifest
    registry: StableCardRegistry
    artifacts: Mapping[str, bytes]
    report: Mapping[str, Any]


def _snapshot(root: Path, relative_manifest: str, source: SourceName) -> OfficialScopeSnapshot:
    manifest_path = root / relative_manifest
    manifest = _load_manifest(manifest_path)
    if manifest is None:
        raise ValidationError("full-pool source manifest is unavailable", path=manifest_path)
    if manifest.source is not source:
        raise ValidationError(
            "full-pool source manifest is in the wrong source lane",
            path=manifest_path,
            details={"expected": source.value, "actual": manifest.source.value},
        )
    observations = _load_observations(root, source)
    missing = sorted(set(manifest.observation_ids) - set(observations))
    if missing:
        raise ValidationError(
            "full-pool source manifest has unresolved observations",
            path=manifest_path,
            details={"observation_ids": missing},
        )
    return OfficialScopeSnapshot(
        manifest=manifest,
        observations=tuple(observations[item] for item in manifest.observation_ids),
    )


def load_review_snapshots(root: Path) -> tuple[OfficialScopeSnapshot, OfficialScopeSnapshot]:
    """Load both already-acquired complete source scopes without network access."""

    return (
        _snapshot(root, OFFICIAL_MANIFEST, SourceName.OFFICIAL),
        _snapshot(root, SECONDARY_MANIFEST, SourceName.DIGIMONCARD_IO),
    )


def rebuild_source_plan_from_snapshots(
    repo_root: Path,
    official: OfficialScopeSnapshot,
    secondary: OfficialScopeSnapshot,
) -> CompositeSyncPlan:
    """Reseal a current plan from offline snapshots and committed inputs.

    This intentionally does not copy review metadata from an older plan whose
    digest no longer validates.  Any newly detected removals therefore remain
    blocked until they are reviewed against the freshly sealed plan.
    """

    payload, candidates = build_reconciliation(
        repo_root.resolve(),
        official=official,
        secondary=secondary,
    )
    raw_plan = payload.get("source_plan")
    if not isinstance(raw_plan, Mapping):
        raise ValidationError("rebuilt reconciliation lacks a source plan")
    plan = load_source_plan(raw_plan)
    if not isinstance(plan, CompositeSyncPlan):
        raise ValidationError("complete candidate requires a composite source plan")
    candidate_plan = CompositeSyncPlan(tuple(item.plan for item in candidates))
    if plan != candidate_plan:
        raise ValidationError("rebuilt source plan disagrees with staged snapshots")
    return plan


def _scope_key(manifest: ScopeManifest) -> SyncScope:
    prefix, separator, name = manifest.scope.partition(":")
    if separator != ":" or prefix != ScopeKind.FULL_POOL.value or not name:
        raise ValidationError(
            "complete candidate requires a full-pool source scope",
            source=manifest.source.value,
            details={"scope": manifest.scope},
        )
    return SyncScope(ScopeKind.FULL_POOL, name)


def _validate_source_plan(
    source_plan: SourcePlan,
    snapshots: Iterable[OfficialScopeSnapshot],
    *,
    committed_revision: str,
) -> None:
    if source_plan.base_committed_revision != committed_revision:
        raise ValidationError(
            "source plan base revision is stale for candidate assembly",
            details={
                "source_plan_base": source_plan.base_committed_revision,
                "committed_revision": committed_revision,
            },
        )
    by_scope = {item.requested_scope: item for item in source_plan_components(source_plan)}
    expected_scopes: set[str] = set()
    for snapshot in snapshots:
        staged = stage_source_observations(
            scope=_scope_key(snapshot.manifest),
            manifest=snapshot.manifest,
            observations={item.observation_id: item for item in snapshot.observations},
        )
        expected_scopes.add(staged.scope.manifest_scope)
        component = by_scope.get(staged.scope.manifest_scope)
        if component is None or component.staged_digest != staged.digest:
            raise ValidationError(
                "source plan does not bind the supplied review snapshot",
                source=snapshot.manifest.source.value,
                details={"requested_scope": staged.scope.manifest_scope},
            )
    unexpected = sorted(set(by_scope) - expected_scopes)
    if unexpected:
        raise ValidationError(
            "source plan contains scopes outside the complete candidate",
            details={"requested_scopes": unexpected},
        )


def _observation_membership(manifests: Iterable[ScopeManifest]) -> dict[str, tuple[str, ...]]:
    grouped: dict[str, set[str]] = {}
    for manifest in manifests:
        for ref in manifest.observation_refs:
            grouped.setdefault(ref.gameplay_card_id, set()).add(ref.observation_id)
    return {card_id: tuple(sorted(values)) for card_id, values in grouped.items()}


def _transition_binding(
    card_id: str,
    *,
    source_plan: SourcePlan,
    old_membership: Mapping[str, tuple[str, ...]],
    new_membership: Mapping[str, tuple[str, ...]],
) -> SourceTransitionBinding:
    return SourceTransitionBinding(
        card_id=card_id,
        old_input_revision=source_plan.base_committed_revision,
        new_input_revision=source_plan.staged_digest,
        old_observation_ids=old_membership.get(card_id, ()),
        new_observation_ids=new_membership.get(card_id, ()),
    )


def _transition_status(
    *,
    kind: AdjudicationKind,
    card_id: str,
    binding: SourceTransitionBinding,
    adjudications: Iterable[MigrationAdjudication],
) -> tuple[str, list[str]]:
    relevant = [
        item
        for item in adjudications
        if item.kind is kind and item.card_id == card_id
    ]
    current = [item for item in relevant if item.is_current(**binding.normalization_options())]
    if len(current) > 1:
        raise AdjudicationError(
            "multiple current source-transition adjudications match one card",
            card_id=card_id,
            details={"kind": kind.value},
        )
    if current:
        return "current", [current[0].adjudication_id]
    return ("stale" if relevant else "missing"), sorted(
        item.adjudication_id for item in relevant
    )


def _preserved_admission_ids(
    *,
    card_id: str,
    binding: SourceTransitionBinding,
    prior_card: CanonicalCard | None,
    adjudications: Iterable[MigrationAdjudication],
) -> list[str]:
    """Reuse a reviewed admission only across exact source-membership continuity."""

    if (
        prior_card is None
        or prior_card.admission is not AdmissionStatus.ADMITTED
        or not binding.new_observation_ids
        or binding.old_observation_ids != binding.new_observation_ids
    ):
        return []
    matches = [
        item
        for item in adjudications
        if item.kind is AdjudicationKind.SOURCE_ADMISSION
        and item.card_id == card_id
        and item.new_digest == binding.new_digest
    ]
    if len(matches) > 1:
        raise AdjudicationError(
            "multiple reviewed source admissions match unchanged membership",
            card_id=card_id,
            details={"adjudication_ids": sorted(item.adjudication_id for item in matches)},
        )
    return [item.adjudication_id for item in matches]


def _merge_observations(
    committed: Mapping[str, SourceObservation],
    snapshots: Iterable[OfficialScopeSnapshot],
) -> dict[str, SourceObservation]:
    merged = dict(committed)
    for snapshot in snapshots:
        for observation in snapshot.observations:
            prior = merged.get(observation.observation_id)
            if prior is not None and prior != observation:
                raise ValidationError(
                    "content-addressed observation identity collision",
                    source=observation.source.value,
                    details={"observation_id": observation.observation_id},
                )
            merged[observation.observation_id] = observation
    return dict(sorted(merged.items()))


def _normalize_source_union(
    *,
    official: OfficialScopeSnapshot,
    secondary: OfficialScopeSnapshot,
    source_plan: SourcePlan,
    aggregate_revision: str,
    prior_full_pool_manifests: Iterable[ScopeManifest],
    prior_canonical_cards: Mapping[str, CanonicalCard],
    adjudications: tuple[MigrationAdjudication, ...],
) -> tuple[dict[str, CanonicalCard], list[dict[str, Any]]]:
    observations = (*official.observations, *secondary.observations)
    grouped: dict[str, list[SourceObservation]] = {}
    for observation in observations:
        card_id = str(observation.extracted.get("card_id", "")).upper()
        if not card_id:
            raise ValidationError(
                "source observation lacks gameplay card ID",
                source=observation.source.value,
                details={"observation_id": observation.observation_id},
            )
        grouped.setdefault(card_id, []).append(observation)
    old_membership = _observation_membership(prior_full_pool_manifests)
    new_membership = _observation_membership((official.manifest, secondary.manifest))
    cards: dict[str, CanonicalCard] = {}
    rows: list[dict[str, Any]] = []
    for card_id in sorted(grouped, key=natural_card_id_key):
        binding = _transition_binding(
            card_id,
            source_plan=source_plan,
            old_membership=old_membership,
            new_membership=new_membership,
        )
        prior_card = prior_canonical_cards.get(card_id)
        preserved_admission_ids = _preserved_admission_ids(
            card_id=card_id,
            binding=binding,
            prior_card=prior_card,
            adjudications=adjudications,
        )
        card = normalize_card(
            card_id,
            grouped[card_id],
            adjudications=adjudications,
            prior_card=prior_card,
            field_selection_input_revision=aggregate_revision,
            defer_admission_if_missing_faces=True,
            preserve_reviewed_admission=bool(preserved_admission_ids),
            **binding.normalization_options(),
        )
        cards[card_id] = card
        if not any(item.source is SourceName.OFFICIAL for item in grouped[card_id]):
            if preserved_admission_ids:
                status, matching = "unchanged", preserved_admission_ids
            else:
                status, matching = _transition_status(
                    kind=AdjudicationKind.SOURCE_ADMISSION,
                    card_id=card_id,
                    binding=binding,
                    adjudications=adjudications,
                )
            rows.append(
                {
                    **binding.to_dict(),
                    "kind": AdjudicationKind.SOURCE_ADMISSION.value,
                    "status": status,
                    "adjudication_ids": matching,
                    "canonical_admission": card.admission.value,
                }
            )

    for card_id in sorted(set(prior_canonical_cards) - set(grouped), key=natural_card_id_key):
        binding = _transition_binding(
            card_id,
            source_plan=source_plan,
            old_membership=old_membership,
            new_membership=new_membership,
        )
        status, matching = _transition_status(
            kind=AdjudicationKind.SOURCE_RETIREMENT,
            card_id=card_id,
            binding=binding,
            adjudications=adjudications,
        )
        row = {
            **binding.to_dict(),
            "kind": AdjudicationKind.SOURCE_RETIREMENT.value,
            "status": status,
            "adjudication_ids": matching,
        }
        try:
            card = normalize_card(
                card_id,
                (),
                prior_card=prior_canonical_cards[card_id],
                adjudications=adjudications,
                field_selection_input_revision=aggregate_revision,
                **binding.normalization_options(),
            )
        except AdjudicationError:
            # Retain the previous complete record in a blocked review candidate;
            # never turn an unapproved upstream removal into a deletion.
            card = prior_canonical_cards[card_id]
            row["canonical_admission"] = card.admission.value
        else:
            row["canonical_admission"] = card.admission.value
        cards[card_id] = card
        rows.append(row)
    return cards, rows


def _scope_artifacts(
    retained_scopes: Mapping[str, ScopeManifest],
    official: OfficialScopeSnapshot,
    secondary: OfficialScopeSnapshot,
) -> dict[str, bytes]:
    scopes = dict(retained_scopes)
    scopes[OFFICIAL_MANIFEST] = official.manifest
    scopes[SECONDARY_MANIFEST] = secondary.manifest
    return {
        relative: canonical_json_bytes(manifest)
        for relative, manifest in sorted(scopes.items())
    }


def _diagnostic(exc: Exception) -> dict[str, Any]:
    if isinstance(exc, CardDataError):
        return exc.diagnostic.to_dict()
    return {
        "code": "COMPLETE_CANDIDATE_DOWNSTREAM_FAILURE",
        "category": "validation",
        "severity": "error",
        "message": str(exc),
        "details": {"exception_type": type(exc).__name__},
    }


def _copy_file(source: Path, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)


def _materialize_campaign_context(repo_root: Path, overlay: Path, blockers_path: Path) -> None:
    """Copy only repository-relative inputs that campaign fingerprinting resolves."""

    for relative in (
        "code/digimon-engine/src",
        "code/digimon-engine/tests",
        "code/digimon-dsl/src",
    ):
        source = repo_root / relative
        destination = overlay / relative
        if source.is_dir():
            shutil.copytree(source, destination)
    for relative in (
        "Cargo.toml",
        "Cargo.lock",
        "code/digimon-engine/Cargo.toml",
        "code/digimon-engine/build.rs",
        "code/digimon-engine/cards/source_lanes.json",
        "code/digimon-engine/cards/core_validation_baseline.json",
        "code/digimon-dsl/Cargo.toml",
        "code/digimon-dsl/build.rs",
    ):
        source = repo_root / relative
        if source.is_file():
            _copy_file(source, overlay / relative)

    payload = json.loads(blockers_path.read_text(encoding="utf-8"))
    for owner in payload.get("owners", {}).values():
        reference = str(owner.get("reference", ""))
        if owner.get("kind") == "openspec_change":
            source = repo_root / "openspec/changes" / reference
            if source.is_dir():
                (overlay / "openspec/changes" / reference).mkdir(parents=True, exist_ok=True)
        elif owner.get("kind") == "tracker" and reference:
            _copy_file(repo_root / reference, overlay / reference)
    for blocker in payload.get("blockers", []):
        for reference in blocker.get("tracker_refs", []):
            relative = str(reference.get("path", ""))
            if relative:
                _copy_file(repo_root / relative, overlay / relative)


def _downstream_artifacts(
    *,
    repo_root: Path,
    canonical_bytes: bytes,
    cards_bytes: bytes,
    production_registry_bytes: bytes,
    production_registered_ids: tuple[str, ...],
    tested_cards_generated_at: str,
    saved_deck_paths: Iterable[Path] = (),
) -> tuple[dict[str, bytes], dict[str, Any]]:
    """Exercise campaign/pool committed-reader contracts in an isolated overlay."""

    saved_deck_paths = tuple(saved_deck_paths)
    with tempfile.TemporaryDirectory(prefix="digimon-card-candidate-") as temporary:
        overlay = Path(temporary)
        canonical_path = overlay / "data/cards_canonical.json"
        cards_path = overlay / "data/cards.json"
        registry_path = overlay / PRODUCTION_REGISTRY_PATH
        for path, body in (
            (canonical_path, canonical_bytes),
            (cards_path, cards_bytes),
            (registry_path, production_registry_bytes),
        ):
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(body)
        marker = RevisionMarker.create(
            {
                "data/cards_canonical.json": sha256_hex(canonical_bytes),
                "data/cards.json": sha256_hex(cards_bytes),
            }
        )
        write_revision_marker(overlay / "data/card_data_revision.json", marker)

        blockers = repo_root / "data/card_blockers.json"
        _materialize_campaign_context(repo_root, overlay, blockers)
        tested_cards_bytes = canonical_json_bytes(
            build_tested_cards_snapshot(
                sorted(production_registered_ids),
                generated_at=tested_cards_generated_at,
            )
        )
        tested_cards_path = overlay / TESTED_CARDS_PATH
        tested_cards_path.parent.mkdir(parents=True, exist_ok=True)
        tested_cards_path.write_bytes(tested_cards_bytes)
        manifest = build_campaign_manifest(
            CampaignSources(
                repo_root=overlay,
                canonical_path=canonical_path,
                generated_cards_path=cards_path,
                cards_root=repo_root / "code/digimon-engine/cards",
                source_lanes_path=repo_root / "code/digimon-engine/cards/source_lanes.json",
                runtime_registry_path=registry_path,
                verdict_path=repo_root / "qa/qa-reports/validated_cards_dsl.json",
                evidence_path=repo_root / "qa/qa-reports/card_behavioral_evidence.json",
                blockers_path=blockers,
                deck_library_path=repo_root / "data/deck_library.json",
                test_root=overlay / "code/digimon-engine/tests",
            )
        )
        manifest_bytes = render_manifest(manifest).encode("utf-8")
        manifest_path = overlay / CAMPAIGN_MANIFEST_PATH
        manifest_path.parent.mkdir(parents=True, exist_ok=True)
        manifest_path.write_bytes(manifest_bytes)
        pools = build_pool_artifacts(
            campaign_manifest_path=manifest_path,
            canonical_path=canonical_path,
            generated_cards_path=cards_path,
            cards_root=repo_root / "code/digimon-engine/cards",
            source_lanes_path=repo_root
            / "code/digimon-engine/cards/source_lanes.json",
            production_registry_path=registry_path,
            verdict_path=repo_root / "qa/qa-reports/validated_cards_dsl.json",
            evidence_path=repo_root
            / "qa/qa-reports/card_behavioral_evidence.json",
            blockers_path=blockers,
            deck_library_path=repo_root / "data/deck_library.json",
            test_root=overlay / "code/digimon-engine/tests",
            starter_decks_path=repo_root / "data/starter_decks.json",
            saved_deck_paths=saved_deck_paths,
            repo_root=overlay,
        )
        current_production_lane_impact = build_current_production_lane_report(
            tested_cards_path=tested_cards_path,
            cards_root=repo_root / "code/digimon-engine/cards",
            source_lanes_path=repo_root
            / "code/digimon-engine/cards/source_lanes.json",
            deck_library_path=repo_root / "data/deck_library.json",
            starter_decks_path=repo_root / "data/starter_decks.json",
            saved_deck_paths=saved_deck_paths,
            display_root=repo_root,
        )
        artifacts = {
            PRODUCTION_REGISTRY_PATH: production_registry_bytes,
            TESTED_CARDS_PATH: tested_cards_bytes,
            CAMPAIGN_MANIFEST_PATH: manifest_bytes,
            RUNTIME_POOL_PATH: canonical_json_text(pools.runtime_registered).encode("utf-8"),
            VALIDATED_POOL_PATH: canonical_json_text(pools.validated_playable).encode("utf-8"),
            POOL_IMPACT_PATH: canonical_json_text(pools.registered_vs_validated).encode("utf-8"),
            CURRENT_PRODUCTION_LANE_IMPACT_PATH: render_pool_impact_report(
                current_production_lane_impact
            ).encode("utf-8"),
        }
        return artifacts, {
            "status": "included",
            "campaign_manifest_revision": content_digest(
                json.loads(manifest_bytes.decode("utf-8"))
            ),
            "campaign_card_count": manifest.summary.canonical_real_card_count,
            "campaign_state_counts": dict(manifest.summary.state_counts),
            "runtime_registered_count": pools.runtime_registered["card_count"],
            "tested_cards_count": len(production_registered_ids),
            "validated_playable_count": pools.validated_playable["card_count"],
        }


def build_complete_candidate(
    *,
    repo_root: Path,
    official: OfficialScopeSnapshot,
    secondary: OfficialScopeSnapshot,
    source_plan: SourcePlan,
    legacy_cards: Mapping[str, Mapping[str, Any]],
    registry: StableCardRegistry,
    corrections: tuple[ReviewedCorrection, ...] = (),
    correction_document: CorrectionDocument | None = None,
    correction_image_artifacts: Mapping[str, bytes] | None = None,
    adjudications: tuple[MigrationAdjudication, ...] = (),
    committed_observations: Mapping[str, SourceObservation] | None = None,
    retained_scope_manifests: Mapping[str, ScopeManifest] | None = None,
    prior_canonical_cards: Mapping[str, CanonicalCard] | None = None,
    prior_compatibility_projection: CompatibilityProjection | None = None,
    committed_revision: str | None = None,
    production_registry_path: Path | None = None,
    saved_deck_paths: Iterable[Path] = (),
    compatibility_selections: tuple[CompatibilitySourceSelection, ...] = (),
) -> CompleteCandidate:
    """Build a deterministic review tree without mutating committed paths."""

    document, correction_images = candidate_document(
        corrections, correction_document, correction_image_artifacts or {})
    corrections = document.corrections
    compatibility_selections = validate_compatibility_selections(compatibility_selections)
    root = repo_root.resolve()
    if committed_revision is None:
        committed_revision = current_revision(root / "data/card_data_revision.json")
    _validate_source_plan(
        source_plan,
        (official, secondary),
        committed_revision=committed_revision,
    )
    aggregate = build_full_pool_source_manifest(
        (official.manifest, secondary.manifest)
    )
    retained_scope_manifests = retained_scope_manifests or {}
    prior_canonical_cards = prior_canonical_cards or {}
    observations = _merge_observations(
        committed_observations or {}, (official, secondary)
    )
    base_cards, transition_rows = _normalize_source_union(
        official=official,
        secondary=secondary,
        source_plan=source_plan,
        aggregate_revision=aggregate.manifest_revision,
        prior_full_pool_manifests=(
            manifest
            for manifest in retained_scope_manifests.values()
            if manifest.scope in {official.manifest.scope, secondary.manifest.scope}
        ),
        prior_canonical_cards=prior_canonical_cards,
        adjudications=adjudications,
    )
    conflict_rows = canonical_conflict_review_rows(
        base_cards,
        (*official.observations, *secondary.observations),
        adjudications=adjudications,
        input_revision=aggregate.manifest_revision,
    )
    corrected, correction_rows = _apply_current_corrections(
        base_cards,
        corrections,
        observations,
        allow_retired_replay=True,
    )
    for row in transition_rows:
        if (
            row["kind"] != AdjudicationKind.SOURCE_ADMISSION.value
            or row["status"] not in {"current", "unchanged"}
        ):
            continue
        card = corrected[row["card_id"]]
        if card.admission is AdmissionStatus.PROVISIONAL and card.faces:
            card = replace(card, admission=AdmissionStatus.ADMITTED)
            corrected[row["card_id"]] = card
        row["canonical_admission"] = card.admission.value
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
        observations=observations,
        canonical_field_selection_approvals=current_conflict_approvals,
    )
    admitted = {
        card_id: card
        for card_id, card in corrected.items()
        if card.admission in {AdmissionStatus.ADMITTED, AdmissionStatus.RETIRED}
    }
    provisional = {
        card_id: card
        for card_id, card in corrected.items()
        if card.admission is AdmissionStatus.PROVISIONAL
    }
    candidate_registry = allocate_append_only(registry, admitted)

    generation_cards = dict(admitted)
    hidden_legacy_fallback_ids: list[str] = []
    compatibility_fixture_fallback_ids: list[str] = []
    compatibility_fixture_rows: dict[str, Mapping[str, Any]] = {}
    if prior_compatibility_projection is None:
        # First-cutover shadow mode only.  The generated legacy snapshot is an
        # explicit migration input until marker ownership is established.
        legacy_canonical = bootstrap_legacy_canonical_cards(legacy_cards)
        for card_id in candidate_registry.assignments:
            if card_id in generation_cards:
                continue
            if card_id not in legacy_canonical:
                raise ValidationError(
                    "stable registry card lacks canonical or legacy compatibility data",
                    card_id=card_id,
                )
            generation_cards[card_id] = legacy_canonical[card_id]
            if is_gameplay_card_id(card_id):
                hidden_legacy_fallback_ids.append(card_id)
            else:
                compatibility_fixture_fallback_ids.append(card_id)
                compatibility_fixture_rows[card_id] = legacy_cards[card_id]
        legacy_bootstrap = bootstrap_legacy_compatibility_fields(legacy_cards)
    else:
        # Post-cutover mode.  cards.json remains the comparison baseline only;
        # all compatibility-only generation inputs come from the validated,
        # marker-owned sidecar.
        if prior_compatibility_projection.registry_revision != content_digest(
            registry
        ):
            raise ValidationError(
                "prior compatibility projection targets another stable registry"
            )
        if prior_canonical_cards and (
            prior_compatibility_projection.canonical_revision
            != content_digest(prior_canonical_cards)
        ):
            raise ValidationError(
                "prior compatibility projection targets another canonical revision"
            )
        legacy_bootstrap = {
            card_id: dict(row)
            for card_id, row in prior_compatibility_projection.compatibility_fields.items()
        }
        compatibility_fixture_rows = {
            card_id: dict(row)
            for card_id, row in prior_compatibility_projection.fixture_rows.items()
        }
        compatibility_fixture_fallback_ids.extend(compatibility_fixture_rows)
        generation_cards.update(
            bootstrap_legacy_canonical_cards(compatibility_fixture_rows)
        )
        missing_assignments = set(candidate_registry.assignments) - set(
            generation_cards
        )
        if missing_assignments:
            raise ValidationError(
                "post-cutover registry card lacks canonical or typed fixture data",
                details={
                    "card_ids": sorted(
                        missing_assignments, key=natural_card_id_key
                    )
                },
            )

    missing_bootstrap_ids = admitted.keys() - legacy_bootstrap.keys()
    legacy_bootstrap, compatibility_evidence = refresh_official_conditions(
        {cid: card for cid, card in admitted.items() if card.admission is AdmissionStatus.ADMITTED},
        observations, legacy_bootstrap, selections=compatibility_selections,
        source_manifest_revision=aggregate.manifest_revision,
        compatibility_corrections=document.compatibility_corrections,
        correction_image_artifacts=correction_images,
    )
    for card_id in missing_bootstrap_ids:
        legacy_bootstrap[card_id] = _source_bootstrap(admitted[card_id], observations,
            condition_value=legacy_bootstrap.get(card_id, {}).get("xros_req"))
    generated = generate_cards_json(
        generation_cards,
        candidate_registry,
        legacy_bootstrap=legacy_bootstrap,
    )
    generated = dict(
        sorted(generated.items(), key=lambda item: natural_card_id_key(item[0]))
    )
    cards_bytes = render_legacy_cards_json(generated)
    compatibility_projection = None
    if not hidden_legacy_fallback_ids:
        compatibility_projection = build_compatibility_projection(
            corrected,
            candidate_registry,
            corrections=corrections,
            source_manifest_revision=aggregate.manifest_revision,
            compatibility_fields={
                card_id: legacy_bootstrap[card_id]
                for card_id in candidate_registry.assignments
            },
            fixture_rows={
                card_id: compatibility_fixture_rows[card_id]
                for card_id in compatibility_fixture_fallback_ids
            },
            projected_cards=cards_bytes,
        )
    old_input_revision = content_digest(legacy_cards)
    input_payload = {"canonical": admitted,
        "compatibility_selections": selection_ledger_payload(compatibility_selections)}
    if document.compatibility_corrections:
        input_payload["compatibility_corrections"] = document.compatibility_corrections
    new_input_revision = content_digest(input_payload)
    runtime_diffs = approve_runtime_diffs(
        diff_runtime_fields(legacy_cards, generated, corrected,
            compatibility_field_provenance=compatibility_evidence),
        corrections=corrections,
        adjudications=adjudications,
        observations=observations,
        old_input_revision=old_input_revision,
        new_input_revision=new_input_revision,
    )
    unapproved_runtime_diffs = [item for item in runtime_diffs if not item.approved]
    unresolved_transitions = [
        row
        for row in transition_rows
        if row["kind"] == AdjudicationKind.SOURCE_RETIREMENT.value
        and row["status"] != "current"
    ]
    unresolved_conflicts = [
        row
        for row in conflict_rows
        if row["approval_required"]
        and row["status"] != "current"
    ]
    approval_required_conflicts = [
        row for row in conflict_rows if row["approval_required"]
    ]
    canonical_ready = bool(
        source_plan.can_apply
        and not validation.has_errors
        and not unapproved_runtime_diffs
        and not hidden_legacy_fallback_ids
        and not unresolved_transitions
        and not unresolved_conflicts
        and all(row["status"] == "applied" for row in correction_rows)
    )

    canonical_payload = {
        "schema_version": 1,
        "source_manifest_digest": aggregate.manifest_revision,
        "source_manifests": {
            binding.source.value: binding.scope_manifest_digest
            for binding in aggregate.sources
        },
        "secondary_only_ids": list(aggregate.secondary_only_gameplay_card_ids),
        "cards": corrected,
    }
    canonical_bytes = canonical_json_bytes(canonical_payload)
    artifacts: dict[str, bytes] = {
        **correction_images,
        SELECTION_LEDGER_PATH: canonical_json_bytes(selection_ledger_payload(compatibility_selections)),
        "data/card_sources/manifest.json": aggregate.to_bytes(),
        **_scope_artifacts(retained_scope_manifests, official, secondary),
        "data/cards_canonical.json": canonical_bytes,
        "data/card_corrections.json": canonical_json_bytes(
            correction_document_payload(document)
        ),
        "data/card_migration_adjudications.json": canonical_json_bytes(
            {"schema_version": SCHEMA_VERSION, "adjudications": adjudications}
        ),
        "data/card_registry.json": canonical_json_bytes(candidate_registry.to_dict()),
        "data/cards.json": cards_bytes,
        "qa/qa-reports/live-card-data-sync-plan.json": canonical_json_bytes(
            source_plan.to_dict()
        ),
    }
    if compatibility_projection is not None:
        artifacts[COMPATIBILITY_PROJECTION_PATH] = canonical_json_bytes(
            compatibility_projection.to_dict()
        )
    for observation in observations.values():
        artifacts[
            f"data/card_sources/observations/{observation.source.value}/"
            f"{observation.observation_id}.json"
        ] = canonical_json_bytes(observation)

    downstream: dict[str, Any]
    if not canonical_ready:
        downstream = {
            "status": "omitted",
            "reason": "canonical readiness gates are blocked",
            "omitted_artifacts": [
                "derived compatibility artifacts",
                CAMPAIGN_MANIFEST_PATH,
                TESTED_CARDS_PATH,
                RUNTIME_POOL_PATH,
                VALIDATED_POOL_PATH,
                POOL_IMPACT_PATH,
                CURRENT_PRODUCTION_LANE_IMPACT_PATH,
            ],
        }
    elif production_registry_path is None or not production_registry_path.is_file():
        downstream = {
            "status": "omitted",
            "reason": "explicit production runtime-registry snapshot is unavailable",
            "omitted_artifacts": [
                CAMPAIGN_MANIFEST_PATH,
                TESTED_CARDS_PATH,
                RUNTIME_POOL_PATH,
                VALIDATED_POOL_PATH,
                POOL_IMPACT_PATH,
                CURRENT_PRODUCTION_LANE_IMPACT_PATH,
            ],
        }
    else:
        try:
            # Preserve the structured engine snapshot and prove it was emitted
            # from these exact candidate compatibility bytes before it becomes
            # a digest-bound campaign input.
            registry_payload = json.loads(
                production_registry_path.read_text(encoding="utf-8")
            )
            production_registered_ids = validate_production_registry_payload(
                registry_payload,
                cards_payload=generated,
                cards_raw=cards_bytes,
                path=production_registry_path,
            )
            production_registry_bytes = canonical_json_bytes(registry_payload)
            derived = generate_derived_artifacts(
                corrected,
                candidate_registry,
                observations,
                source_manifest_revision=aggregate.manifest_revision,
                corrections=corrections,
                compatibility_projection=compatibility_projection,
                corrections_already_applied=True,
            )
            artifacts.update(derived.all_files())
            generated_downstream, downstream = _downstream_artifacts(
                repo_root=root,
                canonical_bytes=canonical_bytes,
                cards_bytes=cards_bytes,
                production_registry_bytes=production_registry_bytes,
                production_registered_ids=production_registered_ids,
                tested_cards_generated_at=max(
                    official.manifest.first_acquired_at,
                    secondary.manifest.first_acquired_at,
                ),
                saved_deck_paths=saved_deck_paths,
            )
            artifacts.update(generated_downstream)
            downstream["derived_artifact_revision"] = derived.artifact_revision
        except (OSError, ValueError, CardDataError) as exc:
            downstream = {
                "status": "omitted",
                "reason": "downstream generation failed",
                "diagnostics": [_diagnostic(exc)],
                "omitted_artifacts": [
                    CAMPAIGN_MANIFEST_PATH,
                    TESTED_CARDS_PATH,
                    RUNTIME_POOL_PATH,
                    VALIDATED_POOL_PATH,
                    POOL_IMPACT_PATH,
                    CURRENT_PRODUCTION_LANE_IMPACT_PATH,
                ],
            }

    overall_ready = canonical_ready and downstream.get("status") == "included"
    legacy_real_ids = {
        card_id for card_id in legacy_cards if is_gameplay_card_id(card_id)
    }
    report = {
        "schema_version": 1,
        "candidate_kind": "complete-full-pool-card-data",
        "status": "ready" if overall_ready else "blocked",
        "protected_branch_mutated": False,
        "canonical_readiness": "ready" if canonical_ready else "blocked",
        "source_plan": {
            "base_committed_revision": source_plan.base_committed_revision,
            "staged_digest": source_plan.staged_digest,
            "draft_digest": source_plan.draft_digest,
            "can_apply": source_plan.can_apply,
        },
        "source_manifest_digest": aggregate.manifest_revision,
        "source_counts": {
            binding.source.value: {
                "observations": len(binding.observation_ids),
                "gameplay_cards": len(binding.gameplay_card_ids),
            }
            for binding in aggregate.sources
        },
        "canonical_counts": {
            "records": len(corrected),
            "admitted_or_retired": len(admitted),
            "provisional": len(provisional),
            "legacy_real_cards": len(legacy_real_ids),
        },
        "secondary_only_ids": list(aggregate.secondary_only_gameplay_card_ids),
        "legacy_only_ids": sorted(
            legacy_real_ids - set(admitted), key=natural_card_id_key
        ),
        "hidden_legacy_fallback_ids": sorted(
            hidden_legacy_fallback_ids, key=natural_card_id_key
        ),
        "compatibility_fixture_fallback_ids": sorted(
            compatibility_fixture_fallback_ids, key=natural_card_id_key
        ),
        "source_transitions": transition_rows,
        "canonical_conflicts": conflict_rows,
        "canonical_conflict_counts": {
            "total": len(conflict_rows),
            "informational": len(conflict_rows) - len(approval_required_conflicts),
            "approval_required": len(approval_required_conflicts),
            "approval_required_unresolved": len(unresolved_conflicts),
        },
        "corrections": correction_rows,
        "compatibility_corrections": correction_review_rows(document, aggregate.manifest_revision),
        "canonical_validation": validation.to_dict(),
        "runtime_field_diff_count": len(runtime_diffs),
        "approved_runtime_field_diff_count": len(runtime_diffs)
        - len(unapproved_runtime_diffs),
        "runtime_field_diffs": [item.to_dict() for item in runtime_diffs],
        "unapproved_runtime_field_diffs": [
            item.to_dict() for item in unapproved_runtime_diffs
        ],
        "old_input_revision": old_input_revision,
        "new_input_revision": new_input_revision,
        "compatibility_selection_ledger_revision": content_digest(selection_ledger_payload(compatibility_selections)),
        "candidate_canonical_revision": content_digest(corrected),
        "downstream": downstream,
    }
    artifacts[COMPLETE_CANDIDATE_REPORT] = canonical_json_bytes(report)
    return CompleteCandidate(
        canonical_cards=corrected,
        observations=observations,
        source_manifest=aggregate,
        registry=candidate_registry,
        artifacts=dict(sorted(artifacts.items())),
        report=report,
    )


def write_complete_candidate(candidate: CompleteCandidate, output_dir: Path) -> None:
    """Write or safely replace one previously sealed candidate directory."""

    output = output_dir.resolve()
    if output.exists() and any(output.iterdir()):
        # Only replace a directory whose entire prior contents were already
        # authenticated by its inventory.  Unknown files are never deleted.
        previous = load_candidate_artifacts(output)
        for relative in previous:
            (output / relative).unlink()
        (output / CANDIDATE_INVENTORY_NAME).unlink()
        for directory in sorted(
            (path for path in output.rglob("*") if path.is_dir()),
            key=lambda path: len(path.parts),
            reverse=True,
        ):
            try:
                directory.rmdir()
            except OSError:
                pass
    output.mkdir(parents=True, exist_ok=True)
    for relative, body in candidate.artifacts.items():
        path = output / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(body)
    (output / CANDIDATE_INVENTORY_NAME).write_bytes(
        canonical_json_bytes(candidate_inventory_payload(candidate.artifacts))
    )


def _load_retained_scopes(repo_root: Path) -> dict[str, ScopeManifest]:
    root = repo_root / "data/card_sources/scopes"
    result: dict[str, ScopeManifest] = {}
    if not root.is_dir():
        return result
    for path in sorted(root.rglob("*.json")):
        manifest = _load_manifest(path)
        if manifest is not None:
            result[path.relative_to(repo_root).as_posix()] = manifest
    return result


def _load_committed_observations(repo_root: Path) -> dict[str, SourceObservation]:
    observations: dict[str, SourceObservation] = {}
    for source in (SourceName.OFFICIAL, SourceName.DIGIMONCARD_IO):
        for observation_id, observation in _load_observations(repo_root, source).items():
            prior = observations.get(observation_id)
            if prior is not None and prior != observation:
                raise ValidationError(
                    "committed observation identity overlaps source lanes",
                    details={"observation_id": observation_id},
                )
            observations[observation_id] = observation
    return observations


def _json_from_committed_artifact(
    artifacts: Mapping[str, bytes], relative: str
) -> Any:
    raw = artifacts.get(relative)
    if not isinstance(raw, bytes):
        raise ValidationError(
            "committed revision does not own a required candidate input",
            path=relative,
        )
    try:
        return json.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ValidationError(
            "committed candidate input is not valid JSON",
            path=relative,
            details={"error": str(exc)},
        ) from exc


def _observations_from_committed_artifacts(
    artifacts: Mapping[str, bytes],
) -> dict[str, SourceObservation]:
    observations: dict[str, SourceObservation] = {}
    prefix = "data/card_sources/observations/"
    for relative in sorted(artifacts):
        if not relative.startswith(prefix) or not relative.endswith(".json"):
            continue
        payload = _json_from_committed_artifact(artifacts, relative)
        if not isinstance(payload, Mapping):
            raise ValidationError(
                "committed source observation must be an object", path=relative
            )
        observation = _observation_from_dict(payload)
        if Path(relative).stem != observation.observation_id:
            raise ValidationError(
                "committed source observation filename does not match identity",
                path=relative,
            )
        prior = observations.get(observation.observation_id)
        if prior is not None and prior != observation:
            raise ValidationError(
                "committed observation identity overlaps source lanes",
                details={"observation_id": observation.observation_id},
            )
        observations[observation.observation_id] = observation
    return observations


def _scopes_from_committed_artifacts(
    artifacts: Mapping[str, bytes],
) -> dict[str, ScopeManifest]:
    scopes: dict[str, ScopeManifest] = {}
    prefix = "data/card_sources/scopes/"
    for relative in sorted(artifacts):
        if not relative.startswith(prefix) or not relative.endswith(".json"):
            continue
        payload = _json_from_committed_artifact(artifacts, relative)
        if not isinstance(payload, Mapping):
            raise ValidationError(
                "committed source scope must be an object", path=relative
            )
        scopes[relative] = _manifest_from_dict(payload)
    return scopes


def _compatibility_selections_from_committed_artifacts(
    artifacts: Mapping[str, bytes],
) -> tuple[CompatibilitySourceSelection, ...]:
    # Older marker-owned revisions predate this ledger. Never read an unowned
    # fixed-path file as a fallback once a revision protocol is active.
    raw = artifacts.get(SELECTION_LEDGER_PATH)
    return selections_from_bytes(raw) if raw is not None else ()


def _adjudications_from_committed_artifacts(
    artifacts: Mapping[str, bytes],
) -> tuple[MigrationAdjudication, ...]:
    relative = "data/card_migration_adjudications.json"
    payload = _json_from_committed_artifact(artifacts, relative)
    if not isinstance(payload, Mapping) or payload.get("schema_version") != SCHEMA_VERSION:
        raise AdjudicationError(
            "unsupported migration adjudication ledger schema", path=relative
        )
    raw_rows = payload.get("adjudications")
    if not isinstance(raw_rows, list):
        raise AdjudicationError(
            "migration adjudication ledger requires an adjudications array",
            path=relative,
        )
    rows = []
    for raw in raw_rows:
        if not isinstance(raw, Mapping):
            raise AdjudicationError(
                "migration adjudication row must be an object", path=relative
            )
        rows.append(_adjudication_from_payload(raw, path=Path(relative)))
    return validate_migration_adjudications(rows, path=Path(relative))


def _require_staged_ledger_extension(
    committed: Iterable[Any],
    staged: Iterable[Any],
    *,
    identity_field: str,
    description: str,
) -> tuple[Any, ...]:
    """Require an explicit whole-ledger candidate to preserve committed rows."""

    committed_rows = {
        getattr(row, identity_field): row for row in committed
    }
    staged_rows = {getattr(row, identity_field): row for row in staged}
    missing = sorted(set(committed_rows) - set(staged_rows))
    changed = sorted(
        row_id
        for row_id in set(committed_rows) & set(staged_rows)
        if committed_rows[row_id] != staged_rows[row_id]
    )
    if missing or changed:
        raise ValidationError(
            f"staged {description} is not an append-only extension of the committed ledger",
            details={"missing_ids": missing, "changed_ids": changed},
        )
    return tuple(staged)


def _canonical_card_from_payload(
    card_id: str,
    payload: Mapping[str, Any],
    *,
    path: Path,
) -> CanonicalCard:
    """Reconstruct one typed prior canonical record without weakening checks."""

    try:
        provenance_payload = payload["provenance"]
        if not isinstance(provenance_payload, Mapping):
            raise TypeError("provenance must be an object")
        provenance = {
            str(field_path): FieldProvenance(
                selected_observation_id=row["selected_observation_id"],
                observation_ids=tuple(row["observation_ids"]),
                authority=AuthorityClass(row["authority"]),
                derived=row.get("derived", False),
                adjudicated=row.get("adjudicated", False),
            )
            for field_path, row in provenance_payload.items()
        }
        printings = tuple(
            Printing(
                printing_id=row["printing_id"],
                observation_ids=tuple(row["observation_ids"]),
                language=row.get("language", "en"),
                product=row.get("product"),
                alternate_art=row.get("alternate_art", False),
            )
            for row in payload["printings"]
        )
        faces = tuple(
            CardFace(
                face_id=row["face_id"],
                kind=FaceKind(row["kind"]),
                colors=tuple(row["colors"]),
                play_cost=row.get("play_cost"),
                level=row.get("level"),
                dp=row.get("dp"),
                effect_text=row.get("effect_text", ""),
                inherited_text=row.get("inherited_text", ""),
                security_text=row.get("security_text", ""),
            )
            for row in payload["faces"]
        )
        conflicts = tuple(
            FieldConflict(
                field_path=row["field_path"],
                candidate_observation_ids=tuple(row["candidate_observation_ids"]),
                kind=FieldConflictKind(row["kind"]),
                reason=row["reason"],
            )
            for row in payload.get("conflicts", ())
        )
        card = CanonicalCard(
            schema_version=payload["schema_version"],
            card_id=payload["card_id"],
            admission=AdmissionStatus(payload["admission"]),
            fields=payload["fields"],
            provenance=provenance,
            printings=printings,
            faces=faces,
            conflicts=conflicts,
            tombstone_reason=payload.get("tombstone_reason"),
        )
    except (KeyError, TypeError, ValueError, ValidationError) as exc:
        raise ValidationError(
            "committed canonical card record is invalid",
            path=path,
            card_id=card_id,
            details={"error": str(exc)},
        ) from exc
    if card.card_id != card_id:
        raise ValidationError(
            "committed canonical card key does not match its record",
            path=path,
            card_id=card_id,
            details={"record_card_id": card.card_id},
        )
    return card


def load_prior_canonical_cards(
    path: Path,
    *,
    payload: Any | None = None,
) -> dict[str, CanonicalCard]:
    """Load committed canonical records so reviewed removals become tombstones."""

    if payload is None:
        try:
            payload = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
            raise ValidationError(
                "committed canonical cards artifact is unavailable or invalid",
                path=path,
                details={"error": str(exc)},
            ) from exc
    if not isinstance(payload, Mapping) or payload.get("schema_version") != SCHEMA_VERSION:
        raise ValidationError(
            "committed canonical cards artifact has an unsupported schema",
            path=path,
        )
    raw_cards = payload.get("cards")
    if not isinstance(raw_cards, Mapping):
        raise ValidationError(
            "committed canonical cards artifact must contain a cards object",
            path=path,
        )
    result: dict[str, CanonicalCard] = {}
    for card_id, row in sorted(
        raw_cards.items(), key=lambda item: natural_card_id_key(str(item[0]))
    ):
        if not isinstance(card_id, str) or not isinstance(row, Mapping):
            raise ValidationError(
                "committed canonical cards artifact contains an invalid record",
                path=path,
                details={"card_id": card_id},
            )
        result[card_id] = _canonical_card_from_payload(card_id, row, path=path)
    return result


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--review-tree", type=Path, required=True)
    plan_group = parser.add_mutually_exclusive_group(required=True)
    plan_group.add_argument(
        "--source-plan",
        type=Path,
        help="previously sealed source plan; all declared digests are validated",
    )
    plan_group.add_argument(
        "--rebuild-source-plan",
        action="store_true",
        help="offline reseal from the supplied snapshots and current committed inputs",
    )
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--compatibility-selections", type=Path,
        help="reviewed whole-ledger extension for exact official condition source selections")
    parser.add_argument(
        "--corrections",
        type=Path,
        help=(
            "explicit reviewed whole-ledger candidate; it must preserve every "
            "committed correction and is sealed into the output candidate"
        ),
    )
    parser.add_argument(
        "--adjudications",
        type=Path,
        help=(
            "explicit reviewed whole-ledger candidate; it must preserve every "
            "committed adjudication and is sealed into the output candidate"
        ),
    )
    parser.add_argument(
        "--production-registry",
        type=Path,
        default=Path(PRODUCTION_REGISTRY_PATH),
        help="engine-emitted production registry snapshot; downstream artifacts are omitted when absent",
    )
    parser.add_argument("--saved-decks", type=Path, action="append", default=[])
    args = parser.parse_args(argv)
    root = args.repo_root.resolve()

    def resolved(path: Path) -> Path:
        return path if path.is_absolute() else root / path

    official, secondary = load_review_snapshots(args.review_tree.resolve())
    if args.rebuild_source_plan:
        source_plan = rebuild_source_plan_from_snapshots(root, official, secondary)
    else:
        assert args.source_plan is not None
        source_payload = json.loads(args.source_plan.read_text(encoding="utf-8"))
        source_plan = load_source_plan(source_payload)
    cards_path = root / "data/cards.json"
    prior_canonical_path = root / "data/cards_canonical.json"
    marker_path = root / "data/card_data_revision.json"
    journal_path = root / "data/.card_data_transaction.json"
    revision_protocol_present = committed_revision_protocol_active(
        marker_path=marker_path,
        journal_path=journal_path,
    )
    prior_compatibility_projection = None
    if revision_protocol_present:
        stack = load_committed_generation_stack(
            root,
            marker_path=marker_path,
            journal_path=journal_path,
        )
        if stack.generated_cards_bytes != stack.committed_cards_bytes:
            raise ValidationError(
                "committed cards.json is stale for its canonical projection",
                path=cards_path,
            )
        try:
            legacy_cards = json.loads(stack.committed_cards_bytes.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as exc:
            raise ValidationError(
                "committed cards.json is not valid JSON",
                path=cards_path,
                details={"error": str(exc)},
            ) from exc
        if not isinstance(legacy_cards, Mapping):
            raise ValidationError("cards.json must be an object", path=cards_path)
        prior_canonical_cards = dict(stack.canonical_cards)
        loaded_committed_revision = stack.revision
        prior_compatibility_projection = stack.compatibility_projection
        registry = stack.registry
        compatibility_selections = _compatibility_selections_from_committed_artifacts(stack.artifact_bytes)
        document = correction_document_from_bytes(stack.artifact_bytes["data/card_corrections.json"])
        correction_images = {r.image_artifact_path: stack.artifact_bytes[r.image_artifact_path]
                             for r in document.compatibility_corrections}
        adjudications = _adjudications_from_committed_artifacts(
            stack.artifact_bytes
        )
        committed_observations = _observations_from_committed_artifacts(
            stack.artifact_bytes
        )
        retained_scope_manifests = _scopes_from_committed_artifacts(
            stack.artifact_bytes
        )
    else:
        committed_paths = [cards_path]
        if prior_canonical_path.is_file():
            committed_paths.append(prior_canonical_path)
        committed = load_json_artifacts(
            committed_paths,
            repository_root=root,
            marker_path=marker_path,
            journal_path=journal_path,
        )
        legacy_cards = committed[cards_path].value
        if not isinstance(legacy_cards, Mapping):
            raise ValidationError("cards.json must be an object", path=cards_path)
        prior_canonical_cards = (
            load_prior_canonical_cards(
                prior_canonical_path,
                payload=committed[prior_canonical_path].value,
            )
            if prior_canonical_path in committed
            else {}
        )
        registry = read_registry(root / "data/card_registry.json")
        compatibility_selections = load_compatibility_selections(root / SELECTION_LEDGER_PATH)
        document = load_correction_document(root / "data/card_corrections.json")
        correction_images = load_document_images(document, root)
        adjudications = load_migration_adjudications(
            root / "data/card_migration_adjudications.json"
        )
        committed_observations = _load_committed_observations(root)
        retained_scope_manifests = _load_retained_scopes(root)
        loaded_committed_revision = current_revision(marker_path)

    if args.compatibility_selections is not None:
        staged_path = resolved(args.compatibility_selections)
        if not staged_path.is_file():
            raise ValidationError("explicit compatibility selection ledger is unavailable", path=staged_path)
        compatibility_selections = _require_staged_ledger_extension(
            compatibility_selections, load_compatibility_selections(staged_path),
            identity_field="selection_id", description="compatibility source selection ledger",
        )

    if args.corrections is not None:
        staged_corrections_path = resolved(args.corrections)
        if not staged_corrections_path.is_file():
            raise ValidationError(
                "explicit staged correction ledger is unavailable",
                path=staged_corrections_path,
            )
        staged_document = load_correction_document(staged_corrections_path)
        require_document_extension(document, staged_document)
        correction_images.update(load_document_images(staged_document, staged_corrections_path.parent))
        document = staged_document
    if args.adjudications is not None:
        staged_adjudications_path = resolved(args.adjudications)
        if not staged_adjudications_path.is_file():
            raise ValidationError(
                "explicit staged adjudication ledger is unavailable",
                path=staged_adjudications_path,
            )
        staged_adjudications = load_migration_adjudications(
            staged_adjudications_path
        )
        adjudications = _require_staged_ledger_extension(
            adjudications,
            staged_adjudications,
            identity_field="adjudication_id",
            description="migration adjudication ledger",
        )
    production_registry = resolved(args.production_registry)
    candidate = build_complete_candidate(
        repo_root=root,
        official=official,
        secondary=secondary,
        source_plan=source_plan,
        legacy_cards=legacy_cards,
        registry=registry,
        correction_document=document,
        correction_image_artifacts=correction_images,
        adjudications=adjudications,
        compatibility_selections=compatibility_selections,
        committed_observations=committed_observations,
        retained_scope_manifests=retained_scope_manifests,
        prior_canonical_cards=prior_canonical_cards,
        prior_compatibility_projection=prior_compatibility_projection,
        committed_revision=loaded_committed_revision,
        production_registry_path=(
            production_registry if production_registry.is_file() else None
        ),
        saved_deck_paths=tuple(resolved(path) for path in args.saved_decks),
    )
    write_complete_candidate(candidate, args.output_dir)
    print(
        canonical_json_text(
            {
                "status": candidate.report["status"],
                "canonical_readiness": candidate.report["canonical_readiness"],
                "source_manifest_digest": candidate.report["source_manifest_digest"],
                "canonical_card_count": candidate.report["canonical_counts"][
                    "admitted_or_retired"
                ],
                "runtime_field_diff_count": candidate.report[
                    "runtime_field_diff_count"
                ],
                "approved_runtime_field_diff_count": candidate.report[
                    "approved_runtime_field_diff_count"
                ],
                "downstream_status": candidate.report["downstream"]["status"],
                "report_path": (
                    args.output_dir.resolve() / COMPLETE_CANDIDATE_REPORT
                ).as_posix(),
            }
        ),
        end="",
    )
    return 0 if candidate.report["status"] == "ready" else 2


if __name__ == "__main__":
    raise SystemExit(main())
