"""Deterministic, write-free sync-plan construction."""

from __future__ import annotations

from dataclasses import dataclass
from collections.abc import Mapping
from typing import Iterable

from .errors import (
    Diagnostic,
    DiagnosticCategory,
    DiagnosticCode,
    DiagnosticSeverity,
    ValidationError,
)
from .models import ScopeManifest
from .serialization import canonicalize, content_digest
from .staging import SourceDiff, StagedSourceState, diff_source_manifests


@dataclass(frozen=True, slots=True)
class SyncPlanDraft:
    base_committed_revision: str
    requested_scope: str
    staged_digest: str
    diff: SourceDiff
    diagnostics: tuple[Diagnostic, ...]
    rejected_observation_ids: tuple[str, ...]
    authoring_run_id: str | None = None
    reviewed_removal_card_ids: tuple[str, ...] = ()

    def __post_init__(self) -> None:
        reviewed = tuple(sorted(set(self.reviewed_removal_card_ids)))
        unknown = set(reviewed) - set(self.diff.removed_gameplay_card_ids)
        if unknown:
            raise ValidationError(
                "reviewed removals must be present in the source diff",
                details={"gameplay_card_ids": sorted(unknown)},
            )
        object.__setattr__(self, "reviewed_removal_card_ids", reviewed)

    @property
    def diff_digest(self) -> str:
        return self.diff.digest

    @property
    def can_apply(self) -> bool:
        return (
            not self.rejected_observation_ids
            and set(self.diff.removed_gameplay_card_ids)
            <= set(self.reviewed_removal_card_ids)
            and not any(
                diagnostic.severity is DiagnosticSeverity.ERROR
                for diagnostic in self.diagnostics
            )
        )

    @property
    def draft_digest(self) -> str:
        return content_digest(self)

    def to_dict(self) -> dict[str, object]:
        payload = canonicalize(self)
        assert isinstance(payload, dict)
        return {
            **payload,
            "diff_digest": self.diff_digest,
            "draft_digest": self.draft_digest,
            "can_apply": self.can_apply,
        }

    @classmethod
    def from_dict(cls, payload: dict[str, object]) -> "SyncPlanDraft":
        raw_diff = payload["diff"]
        if not isinstance(raw_diff, dict):
            raise ValidationError("sync plan diff must be an object")
        raw_diagnostics = payload.get("diagnostics", [])
        if not isinstance(raw_diagnostics, list):
            raise ValidationError("sync plan diagnostics must be an array")
        diagnostics = tuple(
            Diagnostic(
                code=DiagnosticCode(item["code"]),
                category=DiagnosticCategory(item["category"]),
                severity=DiagnosticSeverity(item["severity"]),
                message=item["message"],
                card_id=item.get("card_id"),
                source=item.get("source"),
                path=item.get("path"),
                field_path=item.get("field_path"),
                details=item.get("details", {}),
            )
            for item in raw_diagnostics
        )
        plan = cls(
            base_committed_revision=str(payload["base_committed_revision"]),
            requested_scope=str(payload["requested_scope"]),
            staged_digest=str(payload["staged_digest"]),
            diff=SourceDiff(
                added_observation_ids=tuple(raw_diff["added_observation_ids"]),
                removed_observation_ids=tuple(raw_diff["removed_observation_ids"]),
                retained_observation_ids=tuple(raw_diff["retained_observation_ids"]),
                added_gameplay_card_ids=tuple(raw_diff["added_gameplay_card_ids"]),
                removed_gameplay_card_ids=tuple(raw_diff["removed_gameplay_card_ids"]),
                retained_gameplay_card_ids=tuple(raw_diff["retained_gameplay_card_ids"]),
            ),
            diagnostics=diagnostics,
            rejected_observation_ids=tuple(payload.get("rejected_observation_ids", ())),
            authoring_run_id=(
                str(payload["authoring_run_id"])
                if payload.get("authoring_run_id") is not None
                else None
            ),
            reviewed_removal_card_ids=tuple(
                payload.get("reviewed_removal_card_ids", ())
            ),
        )
        declared_digest = payload.get("draft_digest")
        if declared_digest is not None and declared_digest != plan.draft_digest:
            raise ValidationError("sync plan draft digest does not match its inputs")
        declared_diff_digest = payload.get("diff_digest")
        if declared_diff_digest is not None and declared_diff_digest != plan.diff_digest:
            raise ValidationError("sync plan diff digest does not match its inputs")
        declared_can_apply = payload.get("can_apply")
        if declared_can_apply is not None and declared_can_apply is not plan.can_apply:
            raise ValidationError("sync plan applicability does not match its inputs")
        return plan


COMPOSITE_SYNC_PLAN_KIND = "composite-sync-plan"
COMPOSITE_SYNC_PLAN_SCHEMA_VERSION = 1


def composite_sync_plan_digest(component_digests: Iterable[str]) -> str:
    """Return the deterministic identity for an ordered component set."""

    return content_digest(
        {
            "schema_version": COMPOSITE_SYNC_PLAN_SCHEMA_VERSION,
            "plan_kind": COMPOSITE_SYNC_PLAN_KIND,
            "component_digests": tuple(component_digests),
        }
    )


@dataclass(frozen=True, slots=True)
class CompositeSyncPlan:
    """One publication source plan composed from multiple complete scopes."""

    components: tuple[SyncPlanDraft, ...]

    def __post_init__(self) -> None:
        if len(self.components) < 2:
            raise ValidationError(
                "a composite sync plan requires at least two component drafts"
            )
        ordered = tuple(
            sorted(
                self.components,
                key=lambda item: (item.requested_scope, item.draft_digest),
            )
        )
        scopes = tuple(item.requested_scope for item in ordered)
        if len(set(scopes)) != len(scopes):
            raise ValidationError(
                "composite sync plan component scopes must be unique",
                details={"requested_scopes": scopes},
            )
        bases = {item.base_committed_revision for item in ordered}
        if len(bases) != 1:
            raise ValidationError(
                "composite sync plan components must share one base revision",
                details={"base_committed_revisions": sorted(bases)},
            )
        object.__setattr__(self, "components", ordered)

    @property
    def base_committed_revision(self) -> str:
        return self.components[0].base_committed_revision

    @property
    def requested_scope(self) -> str:
        scopes = ",".join(item.requested_scope for item in self.components)
        return f"composite:[{scopes}]"

    @property
    def component_digests(self) -> tuple[str, ...]:
        return tuple(item.draft_digest for item in self.components)

    @property
    def staged_digest(self) -> str:
        return content_digest(
            {
                "component_staged_digests": tuple(
                    item.staged_digest for item in self.components
                )
            }
        )

    @property
    def diff_digest(self) -> str:
        return content_digest(
            {
                "component_diff_digests": tuple(
                    item.diff_digest for item in self.components
                )
            }
        )

    @property
    def can_apply(self) -> bool:
        return all(item.can_apply for item in self.components)

    @property
    def draft_digest(self) -> str:
        return composite_sync_plan_digest(self.component_digests)

    def to_dict(self) -> dict[str, object]:
        return {
            "schema_version": COMPOSITE_SYNC_PLAN_SCHEMA_VERSION,
            "plan_kind": COMPOSITE_SYNC_PLAN_KIND,
            "base_committed_revision": self.base_committed_revision,
            "requested_scope": self.requested_scope,
            "staged_digest": self.staged_digest,
            "diff_digest": self.diff_digest,
            "component_digests": list(self.component_digests),
            "draft_digest": self.draft_digest,
            "can_apply": self.can_apply,
            "components": [item.to_dict() for item in self.components],
        }

    @classmethod
    def from_dict(cls, payload: Mapping[str, object]) -> "CompositeSyncPlan":
        if payload.get("schema_version") != COMPOSITE_SYNC_PLAN_SCHEMA_VERSION:
            raise ValidationError("unsupported composite sync plan schema version")
        if payload.get("plan_kind") != COMPOSITE_SYNC_PLAN_KIND:
            raise ValidationError("unsupported composite sync plan kind")
        raw_components = payload.get("components")
        if not isinstance(raw_components, list) or not all(
            isinstance(item, dict) for item in raw_components
        ):
            raise ValidationError("composite sync plan components must be an array")
        plan = cls(
            tuple(SyncPlanDraft.from_dict(item) for item in raw_components)
        )
        declared_fields = {
            "base_committed_revision": plan.base_committed_revision,
            "requested_scope": plan.requested_scope,
            "staged_digest": plan.staged_digest,
            "diff_digest": plan.diff_digest,
            "component_digests": list(plan.component_digests),
            "draft_digest": plan.draft_digest,
            "can_apply": plan.can_apply,
        }
        for field_name, expected in declared_fields.items():
            if payload.get(field_name) != expected:
                raise ValidationError(
                    f"composite sync plan {field_name} does not match its components"
                )
        return plan


SourcePlan = SyncPlanDraft | CompositeSyncPlan


def load_source_plan(payload: Mapping[str, object]) -> SourcePlan:
    """Load a composite envelope or a backward-compatible single draft."""

    if not isinstance(payload, Mapping):
        raise ValidationError("source plan must be a JSON object")
    if payload.get("plan_kind") == COMPOSITE_SYNC_PLAN_KIND:
        return CompositeSyncPlan.from_dict(payload)
    return SyncPlanDraft.from_dict(dict(payload))


def source_plan_components(plan: SourcePlan) -> tuple[SyncPlanDraft, ...]:
    if isinstance(plan, CompositeSyncPlan):
        return plan.components
    return (plan,)


def _diagnostic_sort_key(item: Diagnostic) -> tuple[str, str, str, str, str]:
    return (
        item.card_id or "",
        item.source or "",
        item.field_path or "",
        item.code.value,
        item.message,
    )


def build_sync_plan_draft(
    *,
    base_committed_revision: str,
    staged: StagedSourceState,
    committed_manifest: ScopeManifest | None,
    diagnostics: Iterable[Diagnostic] = (),
    rejected_observation_ids: Iterable[str] = (),
    authoring_run_id: str | None = None,
    reviewed_removal_card_ids: Iterable[str] = (),
) -> SyncPlanDraft:
    """Build a complete plan value without touching committed or staged files."""

    if len(base_committed_revision) != 64 or any(
        char not in "0123456789abcdef" for char in base_committed_revision
    ):
        raise ValidationError("base committed revision must be a lowercase sha256")

    rejected = tuple(sorted(set(rejected_observation_ids)))
    if authoring_run_id is not None and not authoring_run_id.strip():
        raise ValidationError("authoring_run_id must be non-empty when supplied")
    known = set(staged.observation_ids)
    unknown_rejections = set(rejected) - known
    if unknown_rejections:
        raise ValidationError(
            "rejected observation diagnostics reference observations outside the staged scope",
            details={"observation_ids": sorted(unknown_rejections)},
        )
    diagnostic_rows = tuple(sorted(diagnostics, key=_diagnostic_sort_key))
    if rejected and not diagnostic_rows:
        raise ValidationError(
            "rejected observations require precise plan diagnostics"
        )

    return SyncPlanDraft(
        base_committed_revision=base_committed_revision,
        requested_scope=staged.scope.manifest_scope,
        staged_digest=staged.digest,
        diff=diff_source_manifests(committed_manifest, staged.manifest),
        diagnostics=diagnostic_rows,
        rejected_observation_ids=rejected,
        authoring_run_id=authoring_run_id,
        reviewed_removal_card_ids=tuple(reviewed_removal_card_ids),
    )
