"""Scope-local membership replacement with cross-scope observation retention."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Mapping

from .errors import CompletenessError, ValidationError
from .models import CanonicalCard, ScopeManifest, SourceObservation
from .normalization import normalize_card
from .serialization import natural_card_id_key
from .staging import StagedSourceState


@dataclass(frozen=True, slots=True)
class MergedScopeState:
    manifests: Mapping[str, ScopeManifest]
    observations: Mapping[str, SourceObservation]
    affected_card_ids: tuple[str, ...]

    def observation_ids_for(self, card_id: str) -> tuple[str, ...]:
        ids = {
            ref.observation_id
            for manifest in self.manifests.values()
            for ref in manifest.observation_refs
            if ref.gameplay_card_id == card_id
        }
        return tuple(sorted(ids))

    def observations_for(self, card_id: str) -> tuple[SourceObservation, ...]:
        return tuple(self.observations[item] for item in self.observation_ids_for(card_id))

    def recompute_affected_cards(
        self,
        prior_cards: Mapping[str, CanonicalCard],
        **normalization_options,
    ) -> dict[str, CanonicalCard]:
        """Recompute affected IDs from the union of every retained scope."""

        result = dict(prior_cards)
        for card_id in self.affected_card_ids:
            result[card_id] = normalize_card(
                card_id,
                self.observations_for(card_id),
                prior_card=prior_cards.get(card_id),
                **normalization_options,
            )
        return result


def merge_scope_membership(
    *,
    committed_manifests: Mapping[str, ScopeManifest],
    committed_observations: Mapping[str, SourceObservation],
    staged: StagedSourceState,
    reviewed_removal_card_ids: frozenset[str] = frozenset(),
) -> MergedScopeState:
    """Replace exactly one scope while retaining all observations used elsewhere."""

    scope_key = staged.scope.manifest_scope
    if staged.manifest.scope != scope_key:
        raise ValidationError("staged manifest does not match its requested scope")
    if not staged.manifest.complete or staged.manifest.failed_ids:
        raise CompletenessError(
            "set-scoped apply requires a complete scope with no failed IDs",
            details={"scope": scope_key, "failed_ids": list(staged.manifest.failed_ids)},
        )

    prior = committed_manifests.get(scope_key)
    prior_cards = (
        {ref.gameplay_card_id for ref in prior.observation_refs} if prior else set()
    )
    staged_cards = {ref.gameplay_card_id for ref in staged.manifest.observation_refs}
    removed_cards = prior_cards - staged_cards
    unreviewed = removed_cards - set(reviewed_removal_card_ids)
    if unreviewed:
        raise CompletenessError(
            "set-scoped apply would shrink the requested scope without reviewed removals",
            details={
                "scope": scope_key,
                "removed_card_ids": sorted(unreviewed, key=natural_card_id_key),
            },
        )

    manifests = dict(committed_manifests)
    manifests[scope_key] = staged.manifest
    referenced_after = {
        ref.observation_id
        for manifest in manifests.values()
        for ref in manifest.observation_refs
    }
    prior_scope_ids = set(prior.observation_ids) if prior else set()
    observations = dict(committed_observations)
    observations.update(staged.observations)
    # Only observations previously owned by this scope and no longer referenced
    # anywhere may leave membership. Every other committed object is untouched.
    for observation_id in prior_scope_ids - referenced_after:
        observations.pop(observation_id, None)
    missing = referenced_after - observations.keys()
    if missing:
        raise ValidationError(
            "merged scope manifests reference unavailable observations",
            details={"observation_ids": sorted(missing)},
        )
    return MergedScopeState(
        manifests=dict(sorted(manifests.items())),
        observations=observations,
        affected_card_ids=tuple(
            sorted(prior_cards | staged_cards, key=natural_card_id_key)
        ),
    )
