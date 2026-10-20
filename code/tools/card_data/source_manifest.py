"""Deterministic aggregate manifest for complete full-pool source scopes.

The aggregate at ``data/card_sources/manifest.json`` is a generated input
index, not a replacement for either source-specific scope manifest.  It binds
the exact non-operational digest and membership of both required source lanes
so downstream artifacts can name one source-manifest revision.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Iterable

from .errors import CompletenessError, ValidationError
from .models import SCHEMA_VERSION, ScopeManifest, ScopeObservationRef, SourceName
from .serialization import canonical_json_bytes, content_digest, natural_card_id_key


SOURCE_MANIFEST_PATH = "data/card_sources/manifest.json"

_REQUIRED_SOURCES = (SourceName.OFFICIAL, SourceName.DIGIMONCARD_IO)
_SCOPE_MANIFEST_PATHS = {
    SourceName.OFFICIAL: "data/card_sources/scopes/official/full-pool.json",
    SourceName.DIGIMONCARD_IO: (
        "data/card_sources/scopes/digimoncard_io/full-pool.json"
    ),
}


def _sorted_refs(
    refs: Iterable[ScopeObservationRef],
) -> tuple[ScopeObservationRef, ...]:
    return tuple(
        sorted(
            refs,
            key=lambda ref: (
                natural_card_id_key(ref.gameplay_card_id),
                ref.printing_id,
                ref.observation_id,
            ),
        )
    )


def _card_ids(manifest: ScopeManifest) -> tuple[str, ...]:
    return tuple(
        sorted(
            {ref.gameplay_card_id for ref in manifest.observation_refs},
            key=natural_card_id_key,
        )
    )


@dataclass(frozen=True, slots=True)
class SourceScopeBinding:
    """One source-specific scope bound into the aggregate revision."""

    source: SourceName
    scope_manifest_path: str
    scope: str
    scope_manifest_digest: str
    parser_version: str
    source_schema_version: str
    observation_ids: tuple[str, ...]
    observation_refs: tuple[ScopeObservationRef, ...]
    gameplay_card_ids: tuple[str, ...]

    def to_dict(self) -> dict[str, Any]:
        return {
            "source": self.source.value,
            "scope_manifest_path": self.scope_manifest_path,
            "scope": self.scope,
            "scope_manifest_digest": self.scope_manifest_digest,
            "parser_version": self.parser_version,
            "source_schema_version": self.source_schema_version,
            "observation_count": len(self.observation_ids),
            "gameplay_card_count": len(self.gameplay_card_ids),
            "observation_ids": list(self.observation_ids),
            "observation_refs": [
                {
                    "observation_id": ref.observation_id,
                    "gameplay_card_id": ref.gameplay_card_id,
                    "printing_id": ref.printing_id,
                }
                for ref in self.observation_refs
            ],
            "gameplay_card_ids": list(self.gameplay_card_ids),
        }


@dataclass(frozen=True, slots=True)
class AggregateSourceManifest:
    """Complete official + secondary source membership for one revision."""

    sources: tuple[SourceScopeBinding, ...]
    observation_ids: tuple[str, ...]
    gameplay_card_ids: tuple[str, ...]
    shared_gameplay_card_ids: tuple[str, ...]
    official_only_gameplay_card_ids: tuple[str, ...]
    secondary_only_gameplay_card_ids: tuple[str, ...]

    def _revision_payload(self) -> dict[str, Any]:
        return {
            "schema_version": SCHEMA_VERSION,
            "manifest_kind": "full-pool-source-aggregate",
            "sources": {
                source.source.value: source.to_dict() for source in self.sources
            },
            "membership": {
                "observation_count": len(self.observation_ids),
                "gameplay_card_count": len(self.gameplay_card_ids),
                "observation_ids": list(self.observation_ids),
                "gameplay_card_ids": list(self.gameplay_card_ids),
                "shared_gameplay_card_ids": list(self.shared_gameplay_card_ids),
                "official_only_gameplay_card_ids": list(
                    self.official_only_gameplay_card_ids
                ),
                "secondary_only_gameplay_card_ids": list(
                    self.secondary_only_gameplay_card_ids
                ),
            },
        }

    @property
    def manifest_revision(self) -> str:
        return content_digest(self._revision_payload(), exclude_operational=True)

    def to_dict(self) -> dict[str, Any]:
        return {
            **self._revision_payload(),
            "manifest_revision": self.manifest_revision,
        }

    def to_bytes(self) -> bytes:
        return canonical_json_bytes(self.to_dict(), exclude_operational=True)


def _validate_complete_full_pool(manifest: ScopeManifest) -> None:
    if not manifest.scope.startswith("full-pool:") or not manifest.scope.partition(":")[2]:
        raise CompletenessError(
            "aggregate source manifest requires full-pool scope manifests",
            source=manifest.source.value,
            details={"scope": manifest.scope},
        )
    if not manifest.complete or manifest.failed_ids:
        raise CompletenessError(
            "aggregate source manifest requires complete source scopes",
            source=manifest.source.value,
            details={
                "scope": manifest.scope,
                "failed_ids": list(manifest.failed_ids),
            },
        )

    card_ids = _card_ids(manifest)
    if not manifest.observation_ids or not card_ids:
        raise CompletenessError(
            "aggregate source manifest cannot bind an empty full-pool scope",
            source=manifest.source.value,
            details={"scope": manifest.scope},
        )
    if len(card_ids) != manifest.expected_count:
        raise CompletenessError(
            "full-pool gameplay-card count does not match expected_count",
            source=manifest.source.value,
            details={
                "scope": manifest.scope,
                "expected_count": manifest.expected_count,
                "actual_count": len(card_ids),
            },
        )

    printing_owners: dict[str, str] = {}
    for ref in manifest.observation_refs:
        owner = printing_owners.setdefault(ref.printing_id, ref.gameplay_card_id)
        if owner != ref.gameplay_card_id:
            raise ValidationError(
                "one source printing_id cannot belong to multiple gameplay cards",
                source=manifest.source.value,
                details={
                    "printing_id": ref.printing_id,
                    "gameplay_card_ids": sorted(
                        {owner, ref.gameplay_card_id}, key=natural_card_id_key
                    ),
                },
            )


def build_full_pool_source_manifest(
    manifests: Iterable[ScopeManifest],
) -> AggregateSourceManifest:
    """Merge exactly one complete full-pool manifest from each source lane.

    Gameplay-card overlap between official and secondary sources is expected
    and retained. Observation-ID overlap is rejected because provenance and
    correction records identify observations by digest without a source
    namespace, so one digest cannot safely name two source records.
    """

    by_source: dict[SourceName, ScopeManifest] = {}
    for manifest in manifests:
        if manifest.source in by_source:
            raise ValidationError(
                "aggregate source manifest received duplicate source scopes",
                source=manifest.source.value,
                details={"source": manifest.source.value},
            )
        by_source[manifest.source] = manifest

    missing = [source.value for source in _REQUIRED_SOURCES if source not in by_source]
    if missing:
        raise CompletenessError(
            "aggregate source manifest is missing required source scopes",
            details={"missing_sources": missing},
        )

    for source in _REQUIRED_SOURCES:
        _validate_complete_full_pool(by_source[source])

    official_observations = set(by_source[SourceName.OFFICIAL].observation_ids)
    secondary_observations = set(
        by_source[SourceName.DIGIMONCARD_IO].observation_ids
    )
    overlapping_observations = official_observations & secondary_observations
    if overlapping_observations:
        raise ValidationError(
            "observation IDs cannot overlap across source namespaces",
            details={"observation_ids": sorted(overlapping_observations)},
        )

    bindings = tuple(
        SourceScopeBinding(
            source=source,
            scope_manifest_path=_SCOPE_MANIFEST_PATHS[source],
            scope=by_source[source].scope,
            scope_manifest_digest=content_digest(
                by_source[source], exclude_operational=True
            ),
            parser_version=by_source[source].parser_version,
            source_schema_version=by_source[source].source_schema_version,
            observation_ids=tuple(sorted(by_source[source].observation_ids)),
            observation_refs=_sorted_refs(by_source[source].observation_refs),
            gameplay_card_ids=_card_ids(by_source[source]),
        )
        for source in _REQUIRED_SOURCES
    )
    official_cards = set(bindings[0].gameplay_card_ids)
    secondary_cards = set(bindings[1].gameplay_card_ids)
    card_key = natural_card_id_key
    return AggregateSourceManifest(
        sources=bindings,
        observation_ids=tuple(sorted(official_observations | secondary_observations)),
        gameplay_card_ids=tuple(sorted(official_cards | secondary_cards, key=card_key)),
        shared_gameplay_card_ids=tuple(
            sorted(official_cards & secondary_cards, key=card_key)
        ),
        official_only_gameplay_card_ids=tuple(
            sorted(official_cards - secondary_cards, key=card_key)
        ),
        secondary_only_gameplay_card_ids=tuple(
            sorted(secondary_cards - official_cards, key=card_key)
        ),
    )
