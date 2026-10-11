"""Write-free source staging and deterministic scope diffs."""

from __future__ import annotations

from dataclasses import dataclass
from enum import StrEnum
from typing import Mapping

from .errors import CompletenessError, SourceSchemaError, ValidationError
from .models import ScopeManifest, ScopeObservationRef, SourceName, SourceObservation
from .serialization import (
    content_digest,
    natural_card_id_key,
    observation_digest,
    resolve_observation_references,
)


class ScopeKind(StrEnum):
    FULL_POOL = "full-pool"
    RELEASE_SET = "release-set"
    EXPLICIT_IDS = "explicit-ids"


@dataclass(frozen=True, slots=True)
class SyncScope:
    kind: ScopeKind
    value: str
    requested_card_ids: tuple[str, ...] = ()

    def __post_init__(self) -> None:
        if not self.value.strip():
            raise ValidationError("sync scope value must not be empty")
        if self.kind is ScopeKind.EXPLICIT_IDS and not self.requested_card_ids:
            raise ValidationError("explicit-ID scope requires requested_card_ids")
        if len(set(self.requested_card_ids)) != len(self.requested_card_ids):
            raise ValidationError("requested_card_ids must be unique")

    @property
    def manifest_scope(self) -> str:
        return f"{self.kind.value}:{self.value}"


@dataclass(frozen=True, slots=True)
class StagedSourceState:
    scope: SyncScope
    manifest: ScopeManifest
    observations: Mapping[str, SourceObservation]

    @property
    def observation_ids(self) -> tuple[str, ...]:
        return self.manifest.observation_ids

    @property
    def gameplay_card_ids(self) -> tuple[str, ...]:
        return tuple(
            sorted(
                {ref.gameplay_card_id for ref in self.manifest.observation_refs},
                key=natural_card_id_key,
            )
        )

    @property
    def digest(self) -> str:
        return content_digest(
            {
                "scope": self.scope,
                "manifest": self.manifest,
                "observations": {
                    observation_id: self.observations[observation_id]
                    for observation_id in self.manifest.observation_ids
                },
            },
            exclude_operational=True,
        )


@dataclass(frozen=True, slots=True)
class SourceDiff:
    added_observation_ids: tuple[str, ...]
    removed_observation_ids: tuple[str, ...]
    retained_observation_ids: tuple[str, ...]
    added_gameplay_card_ids: tuple[str, ...]
    removed_gameplay_card_ids: tuple[str, ...]
    retained_gameplay_card_ids: tuple[str, ...]

    @property
    def digest(self) -> str:
        return content_digest(self)

    @property
    def changed(self) -> bool:
        return bool(
            self.added_observation_ids
            or self.removed_observation_ids
            or self.added_gameplay_card_ids
            or self.removed_gameplay_card_ids
        )


def _ref_card_ids(refs: tuple[ScopeObservationRef, ...]) -> set[str]:
    return {ref.gameplay_card_id for ref in refs}


def stage_source_observations(
    *,
    scope: SyncScope,
    manifest: ScopeManifest,
    observations: Mapping[str, SourceObservation],
) -> StagedSourceState:
    """Validate a requested source scope without performing any writes."""

    if manifest.scope != scope.manifest_scope:
        raise SourceSchemaError(
            "source manifest scope does not match the requested scope",
            source=manifest.source.value,
            details={"requested": scope.manifest_scope, "actual": manifest.scope},
        )
    if not manifest.complete:
        raise CompletenessError(
            "source manifest is incomplete",
            source=manifest.source.value,
            details={"scope": manifest.scope, "failed_ids": list(manifest.failed_ids)},
        )

    resolved = resolve_observation_references(manifest.observation_ids, observations)
    for observation in resolved:
        if observation.source is not manifest.source:
            raise SourceSchemaError(
                "source observation does not match its scope manifest source",
                source=manifest.source.value,
                details={
                    "observation_id": observation.observation_id,
                    "observation_source": observation.source.value,
                },
            )
        calculated = observation_digest(observation)
        if calculated != observation.observation_id:
            raise ValidationError(
                "source observation identity does not match its content",
                source=manifest.source.value,
                details={
                    "observation_id": observation.observation_id,
                    "calculated_observation_id": calculated,
                },
            )

    gameplay_ids = _ref_card_ids(manifest.observation_refs)
    if len(gameplay_ids) != manifest.expected_count:
        raise CompletenessError(
            "scope gameplay-card count does not match expected_count",
            source=manifest.source.value,
            details={
                "scope": manifest.scope,
                "expected_count": manifest.expected_count,
                "actual_count": len(gameplay_ids),
            },
        )
    if scope.kind is ScopeKind.EXPLICIT_IDS:
        requested = set(scope.requested_card_ids)
        if gameplay_ids != requested:
            raise CompletenessError(
                "explicit-ID scope did not return exactly the requested gameplay IDs",
                source=manifest.source.value,
                details={
                    "missing_ids": sorted(requested - gameplay_ids, key=natural_card_id_key),
                    "unexpected_ids": sorted(gameplay_ids - requested, key=natural_card_id_key),
                },
            )

    return StagedSourceState(
        scope=scope,
        manifest=manifest,
        observations={key: observations[key] for key in manifest.observation_ids},
    )


def diff_source_manifests(
    committed: ScopeManifest | None,
    staged: ScopeManifest,
) -> SourceDiff:
    """Compare only membership for one source scope, never global artifacts."""

    if committed is not None and (
        committed.source is not staged.source or committed.scope != staged.scope
    ):
        raise ValidationError("cannot diff manifests for different source scopes")

    old_observations = set(committed.observation_ids if committed else ())
    new_observations = set(staged.observation_ids)
    old_cards = _ref_card_ids(committed.observation_refs) if committed else set()
    new_cards = _ref_card_ids(staged.observation_refs)
    card_key = natural_card_id_key
    return SourceDiff(
        added_observation_ids=tuple(sorted(new_observations - old_observations)),
        removed_observation_ids=tuple(sorted(old_observations - new_observations)),
        retained_observation_ids=tuple(sorted(old_observations & new_observations)),
        added_gameplay_card_ids=tuple(sorted(new_cards - old_cards, key=card_key)),
        removed_gameplay_card_ids=tuple(sorted(old_cards - new_cards, key=card_key)),
        retained_gameplay_card_ids=tuple(sorted(old_cards & new_cards, key=card_key)),
    )
