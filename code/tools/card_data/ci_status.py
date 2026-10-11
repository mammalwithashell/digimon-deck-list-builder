"""Report which offline card-data freshness checks are currently materialized.

The canonical/campaign migration is intentionally incremental.  This helper
does not turn a missing artifact into a successful freshness claim: it records
the exact missing prerequisites so CI can distinguish a pre-cutover checkout
from a stale, fully materialized stack.
"""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any, Iterable

from card_data_reader import load_cards_json

from .serialization import canonical_json_text, sha256_hex


SCHEMA_VERSION = 1

CAMPAIGN_INPUTS = (
    "data/cards_canonical.json",
    "data/cards.json",
    "data/production_runtime_registry.json",
    "qa/qa-reports/validated_cards_dsl.json",
    "qa/qa-reports/card_behavioral_evidence.json",
    "data/card_blockers.json",
    "data/deck_library.json",
    "code/digimon-engine/cards/source_lanes.json",
)
CAMPAIGN_OUTPUTS = (
    "qa/qa-reports/card_implementation_manifest.json",
    "data/runtime_registered_cards.json",
    "data/validated_playable_cards.json",
    "qa/qa-reports/registered_vs_validated_pool_impact.json",
)


def _missing(repo_root: Path, paths: Iterable[str]) -> list[str]:
    return [path for path in paths if not (repo_root / path).is_file()]


def build_ci_status(repo_root: Path) -> dict[str, Any]:
    """Build a deterministic status report without contacting the network."""

    root = repo_root.resolve()
    cards_path = root / "data/cards.json"
    marker_path = root / "data/card_data_revision.json"
    journal_path = root / "data/.card_data_transaction.json"
    loaded = load_cards_json(
        repository_root=root,
        path=cards_path,
        marker_path=marker_path,
        journal_path=journal_path,
        require_committed_revision=marker_path.exists(),
    )

    input_missing = _missing(root, CAMPAIGN_INPUTS)
    output_missing = _missing(root, CAMPAIGN_OUTPUTS)
    inputs_ready = not input_missing
    outputs_ready = not output_missing
    campaign_state = (
        "ready_for_exact_check"
        if inputs_ready and outputs_ready
        else "stale_or_incomplete"
        if inputs_ready
        else "not_materialized"
    )
    tracked = sorted(
        {
            "data/cards.json",
            "data/card_compatibility_projection.json",
            "data/card_registry.json",
            "data/card_corrections.json",
            "data/tested_cards.json",
            *CAMPAIGN_INPUTS,
            *CAMPAIGN_OUTPUTS,
        }
    )
    digests = {
        path: sha256_hex((root / path).read_bytes())
        for path in tracked
        if (root / path).is_file()
    }
    return {
        "schema_version": SCHEMA_VERSION,
        "network_accessed": False,
        "transaction": {
            "state": (
                "committed_revision" if marker_path.exists() else "legacy_unversioned"
            ),
            "revision": loaded.revision,
            "journal_present": journal_path.exists(),
        },
        "campaign_and_pools": {
            "state": campaign_state,
            "exact_check_required": inputs_ready,
            "input_missing": input_missing,
            "output_missing": output_missing,
        },
        "input_digests": digests,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args(argv)
    output = args.output if args.output.is_absolute() else args.repo_root / args.output
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        canonical_json_text(build_ci_status(args.repo_root)),
        encoding="utf-8",
        newline="\n",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
