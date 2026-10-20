"""Run review-only live official and secondary full-pool reconciliation.

This command is intentionally excluded from ordinary pull-request validation.
It fetches both configured upstream catalogs, validates each complete staged
scope in memory, builds deterministic sync-plan drafts, and only then writes a
review bundle.  It never applies a publication plan or writes committed data
paths directly.
"""

from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable, Mapping

from .ci_status import build_ci_status
from .errors import CardDataError
from .models import (
    SCHEMA_VERSION,
    ScopeManifest,
    ScopeObservationRef,
    SourceName,
    SourceObservation,
)
from .serialization import (
    canonical_json_text,
    content_digest,
    natural_card_id_key,
)
from .sources.digimoncard_io import (
    PARSER_VERSION as SECONDARY_PARSER_VERSION,
    SOURCE_SCHEMA_VERSION as SECONDARY_SCHEMA_VERSION,
    DigimonCardIOAcquisition,
    DigimonCardIOAdapter,
)
from .sources.official import (
    OfficialScopeSnapshot,
    OfficialSourceAdapter,
    gameplay_set_prefix,
)
from .staging import ScopeKind, StagedSourceState, SyncScope, stage_source_observations
from .sync_plan import (
    CompositeSyncPlan,
    SyncPlanDraft,
    build_sync_plan_draft,
    load_source_plan,
)
from .transaction import EMPTY_REVISION, read_consistent_revision


OFFICIAL_MANIFEST = "data/card_sources/scopes/official/full-pool.json"
SECONDARY_MANIFEST = "data/card_sources/scopes/digimoncard_io/full-pool.json"
SYNC_PLAN_REPORT = "qa/qa-reports/live-card-data-sync-plan.json"
SYNC_SUMMARY_REPORT = "qa/qa-reports/live-card-data-sync-plan.md"
SOURCE_PLAN_BUNDLE = "source-plan.json"


@dataclass(frozen=True, slots=True)
class SourceCandidate:
    name: SourceName
    staged: StagedSourceState
    plan: SyncPlanDraft
    manifest_path: str


def _utc_now() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")


def _observation_from_dict(payload: Mapping[str, Any]) -> SourceObservation:
    return SourceObservation(
        schema_version=payload["schema_version"],
        observation_id=payload["observation_id"],
        source=SourceName(payload["source"]),
        source_record_key=payload["source_record_key"],
        source_url=payload["source_url"],
        raw_payload_digest=payload["raw_payload_digest"],
        parser_version=payload["parser_version"],
        source_schema_version=payload["source_schema_version"],
        first_acquired_at=payload["first_acquired_at"],
        extracted=payload["extracted"],
    )


def _manifest_from_dict(payload: Mapping[str, Any]) -> ScopeManifest:
    return ScopeManifest(
        schema_version=payload["schema_version"],
        source=SourceName(payload["source"]),
        scope=payload["scope"],
        source_locations=tuple(payload["source_locations"]),
        first_acquired_at=payload["first_acquired_at"],
        parser_version=payload["parser_version"],
        source_schema_version=payload["source_schema_version"],
        observation_ids=tuple(payload["observation_ids"]),
        observation_refs=tuple(
            ScopeObservationRef(
                observation_id=row["observation_id"],
                gameplay_card_id=row["gameplay_card_id"],
                printing_id=row["printing_id"],
            )
            for row in payload["observation_refs"]
        ),
        complete=payload["complete"],
        expected_count=payload["expected_count"],
        failed_ids=tuple(payload.get("failed_ids", ())),
    )


def _load_manifest(path: Path) -> ScopeManifest | None:
    if not path.is_file():
        return None
    payload = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(payload, Mapping):
        raise ValueError(f"scope manifest must be an object: {path}")
    return _manifest_from_dict(payload)


def _load_observations(repo_root: Path, source: SourceName) -> dict[str, SourceObservation]:
    root = repo_root / "data/card_sources/observations" / source.value
    observations: dict[str, SourceObservation] = {}
    if not root.is_dir():
        return observations
    for path in sorted(root.glob("*.json")):
        payload = json.loads(path.read_text(encoding="utf-8"))
        if not isinstance(payload, Mapping):
            raise ValueError(f"source observation must be an object: {path}")
        observation = _observation_from_dict(payload)
        if path.stem != observation.observation_id:
            raise ValueError(f"source observation filename does not match identity: {path}")
        observations[observation.observation_id] = observation
    return observations


def _secondary_snapshot(
    acquisitions: Iterable[DigimonCardIOAcquisition],
    *,
    acquired_at: str,
) -> OfficialScopeSnapshot:
    observations: dict[str, SourceObservation] = {}
    locations: set[str] = set()
    for acquisition in acquisitions:
        locations.add(acquisition.parsed.source_url)
        for observation in acquisition.observations:
            observations[observation.observation_id] = observation
    refs = tuple(
        ScopeObservationRef(
            observation_id=observation.observation_id,
            gameplay_card_id=str(observation.extracted["card_id"]),
            printing_id=observation.source_record_key,
        )
        for observation in sorted(
            observations.values(),
            key=lambda item: (
                natural_card_id_key(str(item.extracted["card_id"])),
                item.observation_id,
            ),
        )
    )
    gameplay_ids = {ref.gameplay_card_id for ref in refs}
    manifest = ScopeManifest(
        schema_version=SCHEMA_VERSION,
        source=SourceName.DIGIMONCARD_IO,
        scope="full-pool:digimoncard-io-catalog",
        source_locations=tuple(sorted(locations)),
        first_acquired_at=min(
            (observation.first_acquired_at for observation in observations.values()),
            default=acquired_at,
        ),
        parser_version=SECONDARY_PARSER_VERSION,
        source_schema_version=SECONDARY_SCHEMA_VERSION,
        observation_ids=tuple(ref.observation_id for ref in refs),
        observation_refs=refs,
        complete=True,
        expected_count=len(gameplay_ids),
        failed_ids=(),
    )
    return OfficialScopeSnapshot(
        manifest=manifest,
        observations=tuple(observations[ref.observation_id] for ref in refs),
    )


def acquire_live_snapshots(
    repo_root: Path,
    *,
    acquired_at: str,
) -> tuple[OfficialScopeSnapshot, OfficialScopeSnapshot]:
    """Fetch complete official and secondary scopes without writing outputs."""

    official_existing = _load_observations(repo_root, SourceName.OFFICIAL)
    official = OfficialSourceAdapter().acquire_catalog(
        first_acquired_at=acquired_at,
        existing_observations=official_existing,
    )
    secondary_existing = _load_observations(repo_root, SourceName.DIGIMONCARD_IO)
    adapter = DigimonCardIOAdapter(existing_observations=secondary_existing)
    acquisitions = (adapter.acquire_catalog(),)
    return official, _secondary_snapshot(acquisitions, acquired_at=acquired_at)


def _candidate(
    *,
    repo_root: Path,
    snapshot: OfficialScopeSnapshot,
    manifest_path: str,
    scope: SyncScope,
    base_revision: str,
    committed_manifest: ScopeManifest | None,
) -> SourceCandidate:
    observations = {item.observation_id: item for item in snapshot.observations}
    staged = stage_source_observations(
        scope=scope,
        manifest=snapshot.manifest,
        observations=observations,
    )
    plan = build_sync_plan_draft(
        base_committed_revision=base_revision,
        staged=staged,
        committed_manifest=committed_manifest,
    )
    return SourceCandidate(snapshot.manifest.source, staged, plan, manifest_path)


def _current_card_ids(payload: object) -> set[str]:
    if not isinstance(payload, Mapping):
        raise ValueError("data/cards.json must be an object")
    return {str(card_id) for card_id in payload}


def _committed_reconciliation_inputs(
    repo_root: Path,
) -> tuple[str, ScopeManifest | None, ScopeManifest | None, Mapping[str, Any]]:
    """Load every committed planning input from one card-data revision."""

    marker = repo_root / "data/card_data_revision.json"
    journal = repo_root / "data/.card_data_transaction.json"
    if not marker.is_file():
        # The pre-cutover checkout has no multi-file marker yet. Its legacy
        # cards reader still performs the available journal/shape checks.
        build_ci_status(repo_root)
        cards_payload = json.loads(
            (repo_root / "data/cards.json").read_text(encoding="utf-8")
        )
        if not isinstance(cards_payload, Mapping):
            raise ValueError("data/cards.json must be an object")
        return (
            EMPTY_REVISION,
            _load_manifest(repo_root / OFFICIAL_MANIFEST),
            _load_manifest(repo_root / SECONDARY_MANIFEST),
            cards_payload,
        )

    required = "data/cards.json"
    optional_manifests = (OFFICIAL_MANIFEST, SECONDARY_MANIFEST)
    revision = read_consistent_revision(
        repo_root,
        marker_path=marker,
        journal_path=journal,
        loaders={
            relative: lambda raw: json.loads(raw.decode("utf-8"))
            for relative in (required, *optional_manifests)
        },
    )
    if required not in revision.values:
        raise ValueError("committed revision does not declare data/cards.json")
    for relative in optional_manifests:
        if (repo_root / relative).is_file() and relative not in revision.values:
            raise ValueError(
                f"committed source manifest is outside revision coverage: {relative}"
            )

    def manifest(relative: str) -> ScopeManifest | None:
        payload = revision.values.get(relative)
        if payload is None:
            return None
        if not isinstance(payload, Mapping):
            raise ValueError(f"scope manifest must be an object: {relative}")
        return _manifest_from_dict(payload)

    cards_payload = revision.values[required]
    if not isinstance(cards_payload, Mapping):
        raise ValueError("data/cards.json must be an object")
    return (
        revision.revision,
        manifest(OFFICIAL_MANIFEST),
        manifest(SECONDARY_MANIFEST),
        cards_payload,
    )


def build_reconciliation(
    repo_root: Path,
    *,
    official: OfficialScopeSnapshot,
    secondary: OfficialScopeSnapshot,
) -> tuple[dict[str, Any], tuple[SourceCandidate, ...]]:
    """Build deterministic plans and cross-source drift from staged snapshots."""

    root = repo_root.resolve()
    (
        base_revision,
        committed_official,
        committed_secondary,
        committed_cards,
    ) = _committed_reconciliation_inputs(root)
    candidates = (
        _candidate(
            repo_root=root,
            snapshot=official,
            manifest_path=OFFICIAL_MANIFEST,
            scope=SyncScope(ScopeKind.FULL_POOL, "official-catalog"),
            base_revision=base_revision,
            committed_manifest=committed_official,
        ),
        _candidate(
            repo_root=root,
            snapshot=secondary,
            manifest_path=SECONDARY_MANIFEST,
            scope=SyncScope(ScopeKind.FULL_POOL, "digimoncard-io-catalog"),
            base_revision=base_revision,
            committed_manifest=committed_secondary,
        ),
    )
    official_ids = set(candidates[0].staged.gameplay_card_ids)
    secondary_ids = set(candidates[1].staged.gameplay_card_ids)
    current_ids = _current_card_ids(committed_cards)
    added_to_canonical_inputs = sorted(official_ids - current_ids, key=natural_card_id_key)
    retired_from_official = sorted(current_ids - official_ids, key=natural_card_id_key)
    changed = any(candidate.plan.diff.changed for candidate in candidates)
    source_plan = CompositeSyncPlan(tuple(candidate.plan for candidate in candidates))
    payload = {
        "schema_version": SCHEMA_VERSION,
        "status": "drift" if changed else "current",
        "changed": changed,
        "protected_branch_mutated": False,
        "base_committed_revision": base_revision,
        "source_plan": source_plan.to_dict(),
        "sources": {
            candidate.name.value: {
                "manifest_revision": content_digest(
                    candidate.staged.manifest,
                    exclude_operational=True,
                ),
                "staged_revision": candidate.staged.digest,
                "card_count": len(candidate.staged.gameplay_card_ids),
                "plan": candidate.plan.to_dict(),
            }
            for candidate in candidates
        },
        "cross_source": {
            "official_not_secondary": sorted(
                official_ids - secondary_ids,
                key=natural_card_id_key,
            ),
            "secondary_not_official": sorted(
                secondary_ids - official_ids,
                key=natural_card_id_key,
            ),
        },
        "generation_impact": {
            "official_ids_absent_from_cards_json": added_to_canonical_inputs,
            "cards_json_ids_absent_from_official": retired_from_official,
            "affected_sets": sorted(
                {
                    gameplay_set_prefix(card_id)
                    for card_id in (*added_to_canonical_inputs, *retired_from_official)
                }
            ),
            "canonical_regeneration_required": bool(
                added_to_canonical_inputs or retired_from_official
            ),
        },
    }
    return payload, candidates


def render_markdown(payload: Mapping[str, Any]) -> str:
    sources = payload["sources"]
    impact = payload["generation_impact"]
    lines = [
        "# Live card-data reconciliation",
        "",
        f"Status: **{payload['status']}**",
        "",
        "This is a review-only sync plan. It did not apply a publication or mutate main.",
        "",
        "## Sources",
        "",
    ]
    for name in sorted(sources):
        source = sources[name]
        diff = source["plan"]["diff"]
        lines.append(
            f"- `{name}`: {source['card_count']} cards; "
            f"+{len(diff['added_gameplay_card_ids'])} / "
            f"-{len(diff['removed_gameplay_card_ids'])} gameplay IDs"
        )
    lines.extend(
        [
            "",
            "## Cross-source and generation impact",
            "",
            "- Official IDs absent from secondary: "
            f"{len(payload['cross_source']['official_not_secondary'])}",
            "- Secondary IDs absent from official: "
            f"{len(payload['cross_source']['secondary_not_official'])}",
            "- Official IDs absent from `cards.json`: "
            f"{len(impact['official_ids_absent_from_cards_json'])}",
            "- `cards.json` IDs absent from official: "
            f"{len(impact['cards_json_ids_absent_from_official'])}",
            f"- Affected sets: {', '.join(impact['affected_sets']) or 'none'}",
            "",
        ]
    )
    return "\n".join(lines)


def write_reconciliation_bundle(
    output_dir: Path,
    payload: Mapping[str, Any],
    candidates: Iterable[SourceCandidate],
) -> None:
    output_dir.mkdir(parents=True, exist_ok=True)
    raw_source_plan = payload.get("source_plan")
    if not isinstance(raw_source_plan, Mapping):
        raise ValueError("reconciliation payload is missing its composite source plan")
    source_plan = load_source_plan(raw_source_plan)
    if not isinstance(source_plan, CompositeSyncPlan):
        raise ValueError("live reconciliation requires a composite source plan")
    source_plan_text = canonical_json_text(source_plan.to_dict())
    (output_dir / "reconciliation.json").write_text(
        canonical_json_text(payload),
        encoding="utf-8",
        newline="\n",
    )
    (output_dir / SOURCE_PLAN_BUNDLE).write_text(
        source_plan_text,
        encoding="utf-8",
        newline="\n",
    )
    (output_dir / "reconciliation.md").write_text(
        render_markdown(payload),
        encoding="utf-8",
        newline="\n",
    )
    if not payload["changed"]:
        return
    review = output_dir / "review-tree"
    for candidate in candidates:
        manifest_path = review / candidate.manifest_path
        manifest_path.parent.mkdir(parents=True, exist_ok=True)
        manifest_path.write_text(
            canonical_json_text(candidate.staged.manifest),
            encoding="utf-8",
            newline="\n",
        )
        observation_root = (
            review / "data/card_sources/observations" / candidate.name.value
        )
        observation_root.mkdir(parents=True, exist_ok=True)
        for observation in candidate.staged.observations.values():
            (observation_root / f"{observation.observation_id}.json").write_text(
                canonical_json_text(observation),
                encoding="utf-8",
                newline="\n",
            )
    for relative, body in (
        (SYNC_PLAN_REPORT, source_plan_text),
        (SYNC_SUMMARY_REPORT, render_markdown(payload)),
    ):
        path = review / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(body, encoding="utf-8", newline="\n")


def _write_failure(output_dir: Path, exc: Exception) -> None:
    output_dir.mkdir(parents=True, exist_ok=True)
    diagnostic = (
        exc.diagnostic.to_dict()
        if isinstance(exc, CardDataError)
        else {
            "code": "UNEXPECTED_RECONCILIATION_FAILURE",
            "category": "validation",
            "severity": "error",
            "message": str(exc),
            "details": {"exception_type": type(exc).__name__},
        }
    )
    payload = {
        "schema_version": SCHEMA_VERSION,
        "status": "failed",
        "changed": False,
        "protected_branch_mutated": False,
        "diagnostics": [diagnostic],
    }
    (output_dir / "reconciliation.json").write_text(
        canonical_json_text(payload),
        encoding="utf-8",
        newline="\n",
    )
    (output_dir / "reconciliation.md").write_text(
        "# Live card-data reconciliation\n\nStatus: **failed**\n\n"
        f"- `{diagnostic['code']}`: {diagnostic['message']}\n",
        encoding="utf-8",
        newline="\n",
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args(argv)
    root = args.repo_root.resolve()
    output = args.output_dir.resolve()
    try:
        acquired_at = _utc_now()
        official, secondary = acquire_live_snapshots(root, acquired_at=acquired_at)
        payload, candidates = build_reconciliation(
            root,
            official=official,
            secondary=secondary,
        )
        write_reconciliation_bundle(output, payload, candidates)
    except Exception as exc:
        _write_failure(output, exc)
        raise
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
