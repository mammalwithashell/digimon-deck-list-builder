"""Generate deterministic full/set campaign views from committed inputs."""

from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path

from card_data_reader import load_json_artifacts

from .campaign import (
    BehavioralEvidence,
    CampaignInputRevision,
    CampaignInputs,
    CampaignManifest,
    LaneSets,
    compute_lane_discrepancies,
)
from .campaign_adapters import (
    load_blockers,
    load_canonical_real_cards,
    load_deck_usage,
    load_evidence_inventory,
    load_runtime_registry_ids,
    load_verdict_inventory,
    load_yaml_inventory,
    behavioral_test_tree_revision,
    engine_behavior_revision,
    reconcile_evidence_currentness,
    scan_ignored_tests,
)
from .errors import ValidationError, WriteError
from .serialization import canonical_json_text, canonicalize, content_digest, sha256_hex
from .transaction import standalone_output_write_guard


@dataclass(frozen=True, slots=True)
class CampaignSources:
    repo_root: Path
    canonical_path: Path
    generated_cards_path: Path
    cards_root: Path
    source_lanes_path: Path
    runtime_registry_path: Path
    verdict_path: Path
    evidence_path: Path
    blockers_path: Path
    deck_library_path: Path
    test_root: Path


def _digest(path: Path) -> str:
    return sha256_hex(path.read_bytes())


def _load_campaign_input(path: Path) -> tuple[object, bytes]:
    """Read one non-transactional campaign input exactly once.

    Verdicts, behavioral results, blockers, deck usage, and the engine registry
    evolve independently of the canonical card-data publication transaction.
    Their exact bytes are still bound into ``CampaignInputRevision``, but they
    must not be required to appear in ``card_data_revision.json``.
    """

    try:
        raw = path.read_bytes()
        return json.loads(raw.decode("utf-8")), raw
    except OSError as exc:
        raise ValidationError("campaign input is unavailable", path=path) from exc
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ValidationError("campaign input is not valid JSON", path=path) from exc


def _evidence_model(row) -> BehavioralEvidence:
    return BehavioralEvidence(
        evidence_id=row["evidence_id"],
        card_ids=tuple(row["card_ids"]),
        test_targets=tuple(row["test_targets"]),
        command=row["command"],
        profile=row["profile"],
        engine_revision=row["engine_revision"],
        source_revision=row["source_revision"],
        result=row["result"],
        artifact_digest=row["artifact_digest"],
        canonical_data_fingerprint=row["canonical_data_fingerprint"],
        yaml_fingerprint=row["yaml_fingerprint"],
        test_fingerprint=row["test_fingerprint"],
        current=row["current"],
    )


def build_campaign_manifest(sources: CampaignSources) -> CampaignManifest:
    # Canonical and generated compatibility data are one card-data publication
    # and therefore use the marker/journal consistent-reader protocol.
    fixed_paths = (
        sources.canonical_path,
        sources.generated_cards_path,
    )
    committed = load_json_artifacts(
        fixed_paths,
        repository_root=sources.repo_root,
        marker_path=sources.repo_root / "data/card_data_revision.json",
        journal_path=sources.repo_root / "data/.card_data_transaction.json",
    )
    canonical_payload = committed[sources.canonical_path].value
    canonical = load_canonical_real_cards(
        sources.canonical_path, payload=canonical_payload
    )
    runtime_payload, runtime_raw = _load_campaign_input(sources.runtime_registry_path)
    verdict_payload, verdict_raw = _load_campaign_input(sources.verdict_path)
    evidence_payload, evidence_raw = _load_campaign_input(sources.evidence_path)
    blockers_payload, blockers_raw = _load_campaign_input(sources.blockers_path)
    deck_payload, deck_raw = _load_campaign_input(sources.deck_library_path)
    yaml = load_yaml_inventory(sources.cards_root, sources.source_lanes_path)
    raw_runtime = set(
        load_runtime_registry_ids(
            sources.runtime_registry_path,
            payload=runtime_payload,
        )
    )
    verdicts = load_verdict_inventory(
        sources.verdict_path, payload=verdict_payload
    )
    evidence = load_evidence_inventory(
        sources.evidence_path, payload=evidence_payload
    )
    evidence = reconcile_evidence_currentness(
        evidence,
        canonical_records=canonical,
        yaml_inventory=yaml,
        repo_root=sources.repo_root,
        engine_source_root=sources.repo_root / "code/digimon-engine/src",
        test_root=sources.test_root,
        source_revision=sha256_hex(committed[sources.generated_cards_path].raw_bytes),
    )
    blockers = load_blockers(
        sources.blockers_path,
        repo_root=sources.repo_root,
        payload=blockers_payload,
        canonical_card_ids=set(canonical),
    )
    ignored = scan_ignored_tests(
        sources.test_root,
        evidence=evidence,
        repo_root=sources.repo_root,
    )
    deck_usage = load_deck_usage(
        sources.deck_library_path,
        set(canonical),
        payload=deck_payload,
    )
    generated_payload = committed[sources.generated_cards_path].value
    generated_ids = set(generated_payload.get("cards", generated_payload))
    secondary_only = set(canonical_payload.get("secondary_only_ids", []))

    discrepancies = compute_lane_discrepancies(
        LaneSets(
            secondary_observations=frozenset(secondary_only),
            canonical_cards=frozenset(canonical),
            generated_cards=frozenset(generated_ids),
            production_yaml=frozenset(yaml.production_paths),
            example_yaml=frozenset(yaml.example_paths),
            raw_runtime_registry=frozenset(raw_runtime),
            verdicts=verdicts.current_ids,
            production_yaml_paths=yaml.production_paths,
            stale_evidence=evidence.stale_or_nonpassing_ids,
            ignored_tests=frozenset(ignored),
        )
    )
    discrepancy_by_card = discrepancies.membership_by_card()

    inputs_by_card: dict[str, CampaignInputs] = {}
    for card_id, canonical_record in canonical.items():
        verdict = verdicts.rows.get(card_id)
        evidence_rows = tuple(
            _evidence_model(row) for row in evidence.rows_by_card.get(card_id, ())
        )
        prod_paths = tuple(yaml.production_paths.get(card_id, ()))
        example_paths = tuple(yaml.example_paths.get(card_id, ()))
        all_paths = prod_paths + example_paths
        inputs_by_card[card_id] = CampaignInputs(
            card_id=card_id,
            canonical_admission=str(canonical_record.get("admission", "admitted")),
            canonical_record_digest=content_digest(canonical_record),
            production_yaml_paths=prod_paths,
            example_yaml_paths=example_paths,
            yaml_digests={path: yaml.path_digests[path] for path in all_paths},
            runtime_registered=card_id in raw_runtime,
            verdict_status=verdict.get("status") if verdict else None,
            verdict_current=verdict is not None,
            historical_verdict=verdict is not None,
            behavioral_evidence=evidence_rows,
            ignored_tests=ignored.get(card_id, ()),
            blocker_ids=blockers.active_by_card.get(card_id, ()),
            deck_usage=tuple(deck_usage.get(card_id, ())),
        )

    revision = CampaignInputRevision(
        canonical_revision=sha256_hex(committed[sources.canonical_path].raw_bytes),
        source_manifest_digest=str(
            canonical_payload.get("source_manifest_digest")
            or content_digest(canonical_payload.get("source_manifests", {}))
        ),
        yaml_tree_digest=content_digest(
            {
                "source_lanes": _digest(sources.source_lanes_path),
                "yaml_files": yaml.path_digests,
            }
        ),
        runtime_registry_digest=sha256_hex(runtime_raw),
        verdict_digest=sha256_hex(verdict_raw),
        behavioral_evidence_digest=sha256_hex(evidence_raw),
        ignored_test_digest=content_digest(ignored),
        blocker_owner_digest=sha256_hex(blockers_raw),
        deck_usage_digest=sha256_hex(deck_raw),
        engine_revision=engine_behavior_revision(
            sources.repo_root,
            engine_source_root=sources.repo_root / "code/digimon-engine/src",
        ),
        test_tree_digest=behavioral_test_tree_revision(
            sources.repo_root,
            sources.test_root,
        ),
    )
    return CampaignManifest.create(
        input_revision=revision,
        inputs_by_card=inputs_by_card,
        discrepancies_by_card=discrepancy_by_card,
        lane_discrepancies=discrepancies,
    )


def manifest_payload(manifest: CampaignManifest) -> dict[str, object]:
    payload = canonicalize(manifest)
    assert isinstance(payload, dict)
    return {**payload, "manifest_revision": content_digest(payload)}


def render_manifest(manifest: CampaignManifest) -> str:
    return canonical_json_text(manifest_payload(manifest))


def _sources(repo_root: Path, args) -> CampaignSources:
    def resolve(path: Path) -> Path:
        return path if path.is_absolute() else repo_root / path

    return CampaignSources(
        repo_root=repo_root,
        canonical_path=resolve(args.canonical),
        generated_cards_path=resolve(args.generated_cards),
        cards_root=resolve(args.cards_root),
        source_lanes_path=resolve(args.source_lanes),
        runtime_registry_path=resolve(args.runtime_registry),
        verdict_path=resolve(args.verdicts),
        evidence_path=resolve(args.evidence),
        blockers_path=resolve(args.blockers),
        deck_library_path=resolve(args.deck_library),
        test_root=resolve(args.test_root),
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument("--canonical", type=Path, default=Path("data/cards_canonical.json"))
    parser.add_argument("--generated-cards", type=Path, default=Path("data/cards.json"))
    parser.add_argument("--cards-root", type=Path, default=Path("code/digimon-engine/cards"))
    parser.add_argument("--source-lanes", type=Path, default=Path("code/digimon-engine/cards/source_lanes.json"))
    parser.add_argument(
        "--runtime-registry",
        type=Path,
        default=Path("data/production_runtime_registry.json"),
        help=(
            "Raw production engine-registry snapshot used as campaign input; "
            "not the generated runtime_registered_cards.json output"
        ),
    )
    parser.add_argument("--verdicts", type=Path, default=Path("qa/qa-reports/validated_cards_dsl.json"))
    parser.add_argument("--evidence", type=Path, default=Path("qa/qa-reports/card_behavioral_evidence.json"))
    parser.add_argument("--blockers", type=Path, default=Path("data/card_blockers.json"))
    parser.add_argument("--deck-library", type=Path, default=Path("data/deck_library.json"))
    parser.add_argument("--test-root", type=Path, default=Path("code/digimon-engine/tests"))
    parser.add_argument("--output", type=Path, default=Path("qa/qa-reports/card_implementation_manifest.json"))
    parser.add_argument("--set")
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)
    if args.write and args.check:
        parser.error("--write and --check are mutually exclusive")
    repo_root = args.repo_root.resolve()
    manifest = build_campaign_manifest(_sources(repo_root, args))
    if args.set:
        manifest = manifest.filter_set(args.set)
    rendered = render_manifest(manifest)
    output = args.output if args.output.is_absolute() else repo_root / args.output
    if args.check:
        if not output.exists() or output.read_text(encoding="utf-8") != rendered:
            raise ValidationError("campaign manifest is stale", path=output)
        return 0
    if args.write:
        try:
            with standalone_output_write_guard(
                repo_root,
                (output,),
                marker_path=repo_root / "data/card_data_revision.json",
                journal_path=repo_root / "data/.card_data_transaction.json",
            ):
                output.parent.mkdir(parents=True, exist_ok=True)
                output.write_text(rendered, encoding="utf-8", newline="\n")
        except OSError as exc:
            raise WriteError("could not write campaign manifest", path=output) from exc
        return 0
    print(rendered, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
