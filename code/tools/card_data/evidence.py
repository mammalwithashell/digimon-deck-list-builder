"""Migrate legacy verdict rows into honest structured behavioral evidence."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, Mapping

from card_data_reader import load_cards_json

from .campaign import BehavioralEvidence
from .campaign_adapters import (
    behavioral_test_fingerprint,
    behavioral_test_support_revision,
    engine_behavior_revision,
)
from .errors import ValidationError, WriteError
from .serialization import canonical_json_text, canonicalize, content_digest, sha256_hex


SCHEMA_VERSION = 1
DEFAULT_COMMAND = (
    "cargo test --manifest-path code/digimon-engine/Cargo.toml "
    "--test cards_behavioral"
)


def _file_digest(path: Path) -> str:
    return sha256_hex(path.read_bytes())


def migrate_verdict_evidence(
    repo_root: Path,
    *,
    verdict_path: Path | None = None,
) -> dict[str, Any]:
    """Represent verdict claims as ``not-run`` until a real result is attached.

    Legacy verdict rows describe intended/previous validation but do not bind a
    command, run artifact, and all current fingerprints.  The migration keeps
    that information visible without incorrectly manufacturing passing evidence.
    """

    verdict_path = verdict_path or repo_root / "qa/qa-reports/validated_cards_dsl.json"
    payload = json.loads(verdict_path.read_text(encoding="utf-8"))
    cards = payload["cards"]
    cards_json = load_cards_json(
        repository_root=repo_root,
        path=repo_root / "data/cards.json",
        marker_path=repo_root / "data/card_data_revision.json",
        journal_path=repo_root / "data/.card_data_transaction.json",
    ).value
    engine_revision = engine_behavior_revision(repo_root)
    source_revision = _file_digest(repo_root / "data/cards.json")
    test_root = repo_root / "code/digimon-engine/tests"
    support_revision = behavioral_test_support_revision(repo_root, test_root)

    grouped: dict[str, list[tuple[str, Mapping[str, Any]]]] = {}
    for card_id, row in cards.items():
        test_path = row.get("test_path")
        group_key = str(test_path) if test_path else f"unmapped:{card_id}"
        grouped.setdefault(group_key, []).append((card_id, row))

    evidence_rows: list[dict[str, Any]] = []
    mapped_card_ids: set[str] = set()
    for group_key in sorted(grouped):
        rows = sorted(grouped[group_key])
        card_ids = tuple(card_id for card_id, _row in rows)
        overlap = mapped_card_ids & set(card_ids)
        if overlap:
            raise ValidationError(
                "legacy verdict card maps to more than one migrated evidence record",
                details={"card_ids": sorted(overlap)},
            )
        mapped_card_ids.update(card_ids)

        test_path = None if group_key.startswith("unmapped:") else repo_root / group_key
        test_exists = test_path is not None and test_path.is_file()
        test_fingerprint, test_inputs_complete = behavioral_test_fingerprint(
            (() if test_path is None else (group_key,)),
            repo_root=repo_root,
            test_root=test_root,
            support_revision=support_revision,
        )
        canonical_fingerprint = content_digest(
            {card_id: cards_json.get(card_id) for card_id in card_ids}
        )
        yaml_rows: dict[str, str | None] = {}
        for card_id, row in rows:
            raw_yaml_path = row.get("yaml_path")
            yaml_path = repo_root / raw_yaml_path if raw_yaml_path else None
            yaml_rows[card_id] = (
                _file_digest(yaml_path) if yaml_path is not None and yaml_path.is_file() else None
            )
        yaml_fingerprint = content_digest(yaml_rows)
        migration_artifact_digest = content_digest(
            {card_id: dict(row) for card_id, row in rows}
        )
        binding = {
            "card_ids": card_ids,
            "test_target": group_key,
            "engine_revision": engine_revision,
            "source_revision": source_revision,
            "canonical_data_fingerprint": canonical_fingerprint,
            "yaml_fingerprint": yaml_fingerprint,
            "test_fingerprint": test_fingerprint,
            "legacy_verdict_digest": migration_artifact_digest,
        }
        evidence = BehavioralEvidence(
            evidence_id=content_digest(binding),
            card_ids=card_ids,
            test_targets=(group_key,),
            command=DEFAULT_COMMAND,
            profile="production-card-behavioral",
            engine_revision=engine_revision,
            source_revision=source_revision,
            result="not-run",
            artifact_digest=migration_artifact_digest,
            canonical_data_fingerprint=canonical_fingerprint,
            yaml_fingerprint=yaml_fingerprint,
            test_fingerprint=test_fingerprint,
            current=bool(
                test_exists
                and test_inputs_complete
                and all(value is not None for value in yaml_rows.values())
            ),
        )
        row_payload = canonicalize(evidence)
        assert isinstance(row_payload, dict)
        row_payload.update(
            {
                "migration_kind": "legacy-verdict-without-bound-run-result",
                "legacy_verdict_statuses": {
                    card_id: row.get("status") for card_id, row in rows
                },
            }
        )
        evidence_rows.append(row_payload)

    if mapped_card_ids != set(cards):
        raise ValidationError("migrated evidence does not cover every legacy verdict card")
    return {
        "schema_version": SCHEMA_VERSION,
        "source": verdict_path.relative_to(repo_root).as_posix(),
        "input_revision": {
            "verdict_digest": _file_digest(verdict_path),
            "engine_revision": engine_revision,
            "source_revision": source_revision,
        },
        "evidence_count": len(evidence_rows),
        "mapped_card_count": len(mapped_card_ids),
        "result_counts": {"passed": 0, "failed": 0, "not-run": len(evidence_rows)},
        "evidence": evidence_rows,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("qa/qa-reports/card_behavioral_evidence.json"),
    )
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)
    repo_root = args.repo_root.resolve()
    output = args.output if args.output.is_absolute() else repo_root / args.output
    rendered = canonical_json_text(migrate_verdict_evidence(repo_root))
    if args.check:
        if not output.exists() or output.read_text(encoding="utf-8") != rendered:
            raise ValidationError("structured behavioral evidence is stale", path=output)
        return 0
    try:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(rendered, encoding="utf-8", newline="\n")
    except OSError as exc:
        raise WriteError("could not write structured behavioral evidence", path=output) from exc
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
