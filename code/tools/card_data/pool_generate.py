"""Generate separate mechanical and faithfully validated card-pool artifacts.

This module never changes ``tested_cards.json`` and never selects a pool for a
consumer.  It derives the production mechanical pool from an explicit raw
engine-registry snapshot intersected with the admitted canonical universe, and
derives the candidate validated pool only from campaign ``DONE`` records.
"""

from __future__ import annotations

import argparse
import json
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable, Mapping

from card_data_reader import load_json_artifact

from .campaign import CampaignState, campaign_manifest_from_payload
from .campaign_adapters import load_canonical_real_cards
from .campaign_generate import (
    CampaignSources,
    build_campaign_manifest,
    manifest_payload,
)
from .errors import ValidationError, WriteError
from .pool_impact import find_deck_usage
from .serialization import (
    canonical_json_text,
    content_digest,
    natural_card_id_key,
    sha256_hex,
)
from .transaction import standalone_output_write_guard


POOL_SCHEMA_VERSION = 1


@dataclass(frozen=True, slots=True)
class PoolArtifacts:
    runtime_registered: Mapping[str, Any]
    validated_playable: Mapping[str, Any]
    registered_vs_validated: Mapping[str, Any]


def _load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ValidationError(
            "failed to read card-pool generation input",
            path=path,
            details={"error": str(exc)},
        ) from exc


def _load_json_with_bytes(path: Path) -> tuple[Any, bytes]:
    try:
        raw = path.read_bytes()
        return json.loads(raw.decode("utf-8")), raw
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ValidationError(
            "failed to read card-pool generation input",
            path=path,
            details={"error": str(exc)},
        ) from exc


def _load_id_snapshot(path: Path, *, payload: Any | None = None) -> tuple[str, ...]:
    payload = _load_json(path) if payload is None else payload
    if isinstance(payload, list):
        card_ids = payload
    elif isinstance(payload, Mapping):
        card_ids = payload.get("card_ids")
    else:
        raise ValidationError("registry snapshot must contain a card_ids array", path=path)
    if not isinstance(card_ids, list) or not all(isinstance(item, str) for item in card_ids):
        raise ValidationError("registry snapshot must contain a card_ids array", path=path)
    if len(card_ids) != len(set(card_ids)):
        raise ValidationError("registry snapshot contains duplicate card IDs", path=path)
    return tuple(sorted(card_ids, key=natural_card_id_key))


def _manifest_revision(payload: Mapping[str, Any]) -> str:
    declared = payload.get("manifest_revision")
    if not isinstance(declared, str) or len(declared) != 64:
        raise ValidationError("campaign manifest lacks a valid manifest_revision")
    unhashed = {key: value for key, value in payload.items() if key != "manifest_revision"}
    actual = content_digest(unhashed)
    if declared != actual:
        raise ValidationError(
            "campaign manifest revision does not match its content",
            details={"declared": declared, "actual": actual},
        )
    return declared


def _input_revisions(
    manifest: Mapping[str, Any],
    *,
    manifest_revision: str,
) -> dict[str, str]:
    revision = manifest.get("input_revision")
    if not isinstance(revision, Mapping):
        raise ValidationError("campaign manifest lacks an input_revision block")
    required = (
        "canonical_revision",
        "source_manifest_digest",
        "yaml_tree_digest",
        "runtime_registry_digest",
        "verdict_digest",
        "behavioral_evidence_digest",
        "ignored_test_digest",
        "blocker_owner_digest",
        "deck_usage_digest",
        "engine_revision",
        "test_tree_digest",
    )
    values: dict[str, str] = {"campaign_manifest_revision": manifest_revision}
    for key in required:
        value = revision.get(key)
        if not isinstance(value, str) or len(value) != 64:
            raise ValidationError(
                "campaign input revision is missing a required digest",
                field_path=f"input_revision.{key}",
            )
        values[key] = value
    return values


def _starter_usage(path: Path, card_ids: set[str]) -> dict[str, list[dict[str, Any]]]:
    usage = {card_id: [] for card_id in card_ids}
    if not path.exists():
        return usage
    payload = _load_json(path)
    decks = payload.get("starter_decks", [])
    if not isinstance(decks, list):
        raise ValidationError("starter_decks must be an array", path=path)
    for deck in decks:
        if not isinstance(deck, Mapping):
            raise ValidationError("starter deck entry must be an object", path=path)
        cards = list(deck.get("egg_deck", [])) + list(deck.get("main_deck", []))
        if not all(isinstance(card_id, str) for card_id in cards):
            raise ValidationError("starter deck cards must be strings", path=path)
        counts = Counter(cards)
        for card_id in sorted(card_ids & counts.keys(), key=natural_card_id_key):
            usage[card_id].append(
                {
                    "surface": "starter_decks",
                    "deck_id": str(deck.get("id", "")),
                    "name": str(deck.get("name", "")),
                    "copies": counts[card_id],
                }
            )
    return usage


def _saved_usage(paths: Iterable[Path], card_ids: set[str]) -> dict[str, list[dict[str, Any]]]:
    usage = {card_id: [] for card_id in card_ids}
    for path in paths:
        payload = _load_json(path)
        decks = payload.get("decks", payload) if isinstance(payload, Mapping) else payload
        if not isinstance(decks, list):
            raise ValidationError(
                "saved-deck artifact must be an array or contain decks",
                path=path,
            )
        for index, deck in enumerate(decks):
            if not isinstance(deck, Mapping):
                raise ValidationError("saved deck entry must be an object", path=path)
            raw_decklist = deck.get("decklist")
            if isinstance(raw_decklist, str):
                try:
                    raw_decklist = json.loads(raw_decklist)
                except json.JSONDecodeError as exc:
                    raise ValidationError(
                        "saved deck contains invalid serialized decklist JSON",
                        path=path,
                        details={"deck_index": index, "error": str(exc)},
                    ) from exc
            cards = (
                list(raw_decklist)
                if isinstance(raw_decklist, list)
                else list(deck.get("egg_deck", [])) + list(deck.get("main_deck", []))
            )
            if not all(isinstance(card_id, str) for card_id in cards):
                raise ValidationError("saved deck cards must be strings", path=path)
            counts = Counter(cards)
            for card_id in sorted(card_ids & counts.keys(), key=natural_card_id_key):
                usage[card_id].append(
                    {
                        "surface": "saved_decks",
                        "path": path.as_posix(),
                        "deck_id": str(deck.get("id", deck.get("deck_id", index))),
                        "name": str(deck.get("name", "")),
                        "copies": counts[card_id],
                    }
                )
    return usage


def repository_deck_usage(
    card_ids: set[str],
    *,
    deck_library_path: Path,
    starter_decks_path: Path,
    saved_deck_paths: Iterable[Path] = (),
) -> tuple[dict[str, list[dict[str, Any]]], dict[str, str | None]]:
    usage = {card_id: [] for card_id in card_ids}
    if deck_library_path.exists():
        library = _load_json(deck_library_path)
        for card_id, rows in find_deck_usage(library, card_ids).items():
            usage[card_id].extend({"surface": "deck_library", **row} for row in rows)
    starter = _starter_usage(starter_decks_path, card_ids)
    saved_paths = tuple(saved_deck_paths)
    saved = _saved_usage(saved_paths, card_ids)
    for card_id in card_ids:
        usage[card_id].extend(starter[card_id])
        usage[card_id].extend(saved[card_id])
        usage[card_id].sort(
            key=lambda row: (
                str(row.get("surface", "")),
                str(row.get("deck_id", "")),
                str(row.get("path", "")),
            )
        )
    digests: dict[str, str | None] = {
        "deck_library": (
            sha256_hex(deck_library_path.read_bytes()) if deck_library_path.exists() else None
        ),
        "starter_decks": (
            sha256_hex(starter_decks_path.read_bytes()) if starter_decks_path.exists() else None
        ),
    }
    for path in saved_paths:
        digests[f"saved:{path.as_posix()}"] = sha256_hex(path.read_bytes())
    return dict(sorted(usage.items())), dict(sorted(digests.items()))


def _missing_done_requirements(record: Mapping[str, Any]) -> list[str]:
    inputs = record.get("inputs", {})
    missing: list[str] = []
    if not inputs.get("production_yaml_paths"):
        missing.append("production_yaml")
    if not inputs.get("runtime_registered"):
        missing.append("runtime_registration")
    if not inputs.get("verdict_current") or inputs.get("verdict_status") not in {
        "IMPLEMENTED",
        "AUDITED-OK",
    }:
        missing.append("accepted_current_verdict")
    evidence = inputs.get("behavioral_evidence", [])
    if not any(
        row.get("result") == "passed" and row.get("current") is True
        for row in evidence
        if isinstance(row, Mapping)
    ):
        missing.append("current_passing_behavioral_evidence")
    if inputs.get("ignored_tests"):
        missing.append("no_ignored_tests")
    if inputs.get("blocker_ids"):
        missing.append("no_active_blockers")
    return missing


def build_pool_artifacts(
    *,
    campaign_manifest_path: Path,
    canonical_path: Path,
    generated_cards_path: Path,
    cards_root: Path,
    source_lanes_path: Path,
    production_registry_path: Path,
    verdict_path: Path,
    evidence_path: Path,
    blockers_path: Path,
    deck_library_path: Path,
    test_root: Path,
    starter_decks_path: Path,
    saved_deck_paths: Iterable[Path] = (),
    repo_root: Path | None = None,
) -> PoolArtifacts:
    manifest, manifest_raw = _load_json_with_bytes(campaign_manifest_path)
    if not isinstance(manifest, Mapping):
        raise ValidationError("campaign manifest root must be an object")
    manifest_revision = _manifest_revision(manifest)
    typed_manifest = campaign_manifest_from_payload(manifest)
    revisions = _input_revisions(manifest, manifest_revision=manifest_revision)
    repository_root = (repo_root or canonical_path.parent.parent).resolve()
    canonical_loaded = load_json_artifact(
        canonical_path,
        repository_root=repository_root,
        marker_path=repository_root / "data/card_data_revision.json",
        journal_path=repository_root / "data/.card_data_transaction.json",
    )
    canonical = load_canonical_real_cards(
        canonical_path,
        payload=canonical_loaded.value,
    )
    if revisions["canonical_revision"] != sha256_hex(canonical_loaded.raw_bytes):
        raise ValidationError("campaign canonical revision is stale")
    registry_payload, registry_raw = _load_json_with_bytes(production_registry_path)
    if revisions["runtime_registry_digest"] != sha256_hex(registry_raw):
        raise ValidationError("campaign runtime-registry revision is stale")

    live_manifest = build_campaign_manifest(
        CampaignSources(
            repo_root=repository_root,
            canonical_path=canonical_path,
            generated_cards_path=generated_cards_path,
            cards_root=cards_root,
            source_lanes_path=source_lanes_path,
            runtime_registry_path=production_registry_path,
            verdict_path=verdict_path,
            evidence_path=evidence_path,
            blockers_path=blockers_path,
            deck_library_path=deck_library_path,
            test_root=test_root,
        )
    )
    live_payload = manifest_payload(live_manifest)
    if dict(manifest) != live_payload:
        declared_inputs = manifest.get("input_revision")
        live_inputs = live_payload["input_revision"]
        mismatched_inputs = sorted(
            key
            for key in set(live_inputs)
            | (set(declared_inputs) if isinstance(declared_inputs, Mapping) else set())
            if not isinstance(declared_inputs, Mapping)
            or declared_inputs.get(key) != live_inputs.get(key)
        )
        raise ValidationError(
            "campaign manifest is stale for current pool inputs",
            path=campaign_manifest_path,
            details={
                "mismatched_input_revisions": mismatched_inputs,
                "declared_manifest_revision": manifest_revision,
                "current_manifest_revision": live_payload["manifest_revision"],
            },
        )

    records = manifest.get("records")
    if not isinstance(records, Mapping) or set(records) != set(canonical):
        raise ValidationError(
            "campaign records do not exactly reconcile to the canonical universe"
        )
    for card_id, record in records.items():
        if not isinstance(record, Mapping):
            raise ValidationError(
                "campaign record must be an object",
                card_id=str(card_id),
            )
    raw_registry = set(
        _load_id_snapshot(production_registry_path, payload=registry_payload)
    )
    registered_ids = tuple(
        sorted(raw_registry & set(canonical), key=natural_card_id_key)
    )
    for card_id, record in typed_manifest.records.items():
        declared = record.inputs.runtime_registered
        if declared != (card_id in registered_ids):
            raise ValidationError(
                "campaign runtime registration flag is stale",
                card_id=card_id,
            )
    validated_ids = tuple(
        sorted(
            (
                card_id
                for card_id, record in typed_manifest.records.items()
                if record.state is CampaignState.DONE
            ),
            key=natural_card_id_key,
        )
    )
    if not set(validated_ids) <= set(registered_ids):
        raise ValidationError("DONE candidate pool is not a subset of production registration")

    runtime_payload = {
        "schema_version": POOL_SCHEMA_VERSION,
        "pool_kind": "runtime_registered",
        "semantics": "mechanical production registry intersected with canonical real cards",
        "input_revisions": revisions,
        "card_count": len(registered_ids),
        "card_ids": registered_ids,
    }
    validated_payload = {
        "schema_version": POOL_SCHEMA_VERSION,
        "pool_kind": "validated_playable",
        "semantics": "candidate future pool containing campaign DONE cards only",
        "consumer_enforced": False,
        "input_revisions": revisions,
        "card_count": len(validated_ids),
        "card_ids": validated_ids,
    }

    affected = set(registered_ids) - set(validated_ids)
    usage, deck_digests = repository_deck_usage(
        affected,
        deck_library_path=deck_library_path,
        starter_decks_path=starter_decks_path,
        saved_deck_paths=saved_deck_paths,
    )
    rows = []
    for card_id in sorted(affected, key=natural_card_id_key):
        record = records[card_id]
        rows.append(
            {
                "card_id": card_id,
                "campaign_state": record.get("state"),
                "discrepancies": record.get("discrepancies", []),
                "missing_done_requirements": _missing_done_requirements(record),
                "deck_count": len(usage[card_id]),
                "decks": usage[card_id],
            }
        )
    comparison = {
        "schema_version": POOL_SCHEMA_VERSION,
        "comparison": "runtime-registered-vs-validated-playable",
        "consumer_cutover_performed": False,
        "input_revisions": revisions,
        "input_digests": {
            "campaign_manifest": sha256_hex(manifest_raw),
            "canonical": sha256_hex(canonical_loaded.raw_bytes),
            "production_registry": sha256_hex(registry_raw),
            **deck_digests,
        },
        "counts": {
            "runtime_registered": len(registered_ids),
            "validated_playable": len(validated_ids),
            "registered_not_validated": len(affected),
            "validated_not_registered": 0,
            "affected_repository_decks": sum(len(rows) for rows in usage.values()),
        },
        "registered_not_validated": rows,
        "validated_not_registered": [],
    }
    return PoolArtifacts(runtime_payload, validated_payload, comparison)


def _rendered_outputs(
    artifacts: PoolArtifacts,
    *,
    runtime_output: Path,
    validated_output: Path,
    impact_output: Path,
) -> dict[Path, str]:
    return {
        runtime_output: canonical_json_text(artifacts.runtime_registered),
        validated_output: canonical_json_text(artifacts.validated_playable),
        impact_output: canonical_json_text(artifacts.registered_vs_validated),
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument(
        "--campaign",
        type=Path,
        default=Path("qa/qa-reports/card_implementation_manifest.json"),
    )
    parser.add_argument("--canonical", type=Path, default=Path("data/cards_canonical.json"))
    parser.add_argument("--generated-cards", type=Path, default=Path("data/cards.json"))
    parser.add_argument("--cards-root", type=Path, default=Path("code/digimon-engine/cards"))
    parser.add_argument(
        "--source-lanes",
        type=Path,
        default=Path("code/digimon-engine/cards/source_lanes.json"),
    )
    parser.add_argument(
        "--production-registry",
        type=Path,
        default=Path("data/production_runtime_registry.json"),
    )
    parser.add_argument(
        "--verdicts",
        type=Path,
        default=Path("qa/qa-reports/validated_cards_dsl.json"),
    )
    parser.add_argument(
        "--evidence",
        type=Path,
        default=Path("qa/qa-reports/card_behavioral_evidence.json"),
    )
    parser.add_argument("--blockers", type=Path, default=Path("data/card_blockers.json"))
    parser.add_argument("--deck-library", type=Path, default=Path("data/deck_library.json"))
    parser.add_argument("--test-root", type=Path, default=Path("code/digimon-engine/tests"))
    parser.add_argument("--starter-decks", type=Path, default=Path("data/starter_decks.json"))
    parser.add_argument("--saved-decks", type=Path, action="append", default=[])
    parser.add_argument(
        "--runtime-output",
        type=Path,
        default=Path("data/runtime_registered_cards.json"),
    )
    parser.add_argument(
        "--validated-output",
        type=Path,
        default=Path("data/validated_playable_cards.json"),
    )
    parser.add_argument(
        "--impact-output",
        type=Path,
        default=Path("qa/qa-reports/registered_vs_validated_pool_impact.json"),
    )
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)
    if args.write == args.check:
        parser.error("choose exactly one of --write or --check")
    root = args.repo_root.resolve()

    def resolved(path: Path) -> Path:
        return path if path.is_absolute() else root / path

    artifacts = build_pool_artifacts(
        campaign_manifest_path=resolved(args.campaign),
        canonical_path=resolved(args.canonical),
        generated_cards_path=resolved(args.generated_cards),
        cards_root=resolved(args.cards_root),
        source_lanes_path=resolved(args.source_lanes),
        production_registry_path=resolved(args.production_registry),
        verdict_path=resolved(args.verdicts),
        evidence_path=resolved(args.evidence),
        blockers_path=resolved(args.blockers),
        deck_library_path=resolved(args.deck_library),
        test_root=resolved(args.test_root),
        starter_decks_path=resolved(args.starter_decks),
        saved_deck_paths=(resolved(path) for path in args.saved_decks),
        repo_root=root,
    )
    outputs = _rendered_outputs(
        artifacts,
        runtime_output=resolved(args.runtime_output),
        validated_output=resolved(args.validated_output),
        impact_output=resolved(args.impact_output),
    )
    if args.check:
        stale = [
            path for path, text in outputs.items()
            if not path.exists() or path.read_text(encoding="utf-8") != text
        ]
        if stale:
            raise ValidationError(
                "generated pool artifact is stale",
                details={"paths": [path.as_posix() for path in stale]},
            )
        return 0
    try:
        with standalone_output_write_guard(
            root,
            tuple(outputs),
            marker_path=root / "data/card_data_revision.json",
            journal_path=root / "data/.card_data_transaction.json",
        ):
            for path, text in outputs.items():
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(text, encoding="utf-8", newline="\n")
    except OSError as exc:
        raise WriteError("could not write generated pool artifacts") from exc
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
