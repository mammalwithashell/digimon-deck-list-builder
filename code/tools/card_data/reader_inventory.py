"""Machine-readable inventory and guard for fixed-path card-data readers."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from .errors import ValidationError, WriteError
from .serialization import canonical_json_text, sha256_hex


SCHEMA_VERSION = 1

PYTHON_CONSUMERS = (
    "code/digimon_gym/agents/mulligan_log.py",
    "code/server/ai/dispatcher.py",
    "code/server/ai/retrieval.py",
    "code/server/routers/deck_tools.py",
    "code/tools/author_set/lexicons.py",
    "code/tools/author_set/report_set.py",
    "code/tools/author_set/set_resolver.py",
    "code/tools/build_review_batches.py",
    "code/tools/card_data/campaign_generate.py",
    "code/tools/card_data/evidence.py",
    "code/tools/gen_starter_decks.py",
    "code/tools/keyword_audit.py",
    "code/tools/keyword_parse_diff.py",
    "code/tools/resolve_deck.py",
    "code/tools/split_cards_json.py",
)

RUST_EMBEDDED_CONSUMERS = (
    "code/digimon-engine/src/deck_tools.rs",
    "code/digimon-engine/build.rs",
)

LEGACY_COMPATIBILITY_WRAPPERS = (
    "code/tools/build_card_bundles.py",
    "code/tools/build_registry.py",
    "code/tools/ingest_cards.py",
    "code/tools/scrape_official_evo_costs.py",
)


def build_reader_inventory(repo_root: Path) -> dict[str, object]:
    root = repo_root.resolve()
    python_rows = []
    for relative in PYTHON_CONSUMERS:
        path = root / relative
        text = path.read_text(encoding="utf-8")
        if "card_data_reader" not in text:
            raise ValidationError(
                "fixed-path Python card-data reader bypasses the consistent protocol",
                path=path,
            )
        python_rows.append(
            {
                "path": relative,
                "protocol": "card_data_reader",
                "sha256": sha256_hex(path.read_bytes()),
            }
        )

    rust_rows = []
    for relative in RUST_EMBEDDED_CONSUMERS:
        path = root / relative
        rust_rows.append(
            {
                "path": relative,
                "protocol": "build-time-consistent-revision",
                "sha256": sha256_hex(path.read_bytes()),
            }
        )
    build_text = (root / "code/digimon-engine/build.rs").read_text(encoding="utf-8")
    for required in ("card_data_revision.json", ".card_data_transaction.json"):
        if required not in build_text:
            raise ValidationError(
                "Rust embedded reader build guard omits transaction metadata",
                path=root / "code/digimon-engine/build.rs",
                details={"missing": required},
            )

    wrapper_rows = []
    for relative in LEGACY_COMPATIBILITY_WRAPPERS:
        path = root / relative
        text = path.read_text(encoding="utf-8")
        if "deprecated" not in text.lower() and "compatibility" not in text.lower():
            raise ValidationError(
                "legacy reader/writer lacks compatibility or deprecation guidance",
                path=path,
            )
        wrapper_rows.append(
            {
                "path": relative,
                "protocol": "deprecated-compatibility-wrapper",
                "sha256": sha256_hex(path.read_bytes()),
            }
        )

    return {
        "schema_version": SCHEMA_VERSION,
        "policy": {
            "pre_cutover": "legacy-unversioned when metadata is absent or a terminal rolled_back journal restores the empty base revision without a marker",
            "post_cutover": "reject incomplete journal, marker/hash mismatch, and concurrent revision change",
            "runtime_network": "forbidden",
        },
        "python_consistent_readers": python_rows,
        "rust_embedded_readers": rust_rows,
        "legacy_compatibility_wrappers": wrapper_rows,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("qa/qa-reports/card_data_reader_inventory.json"),
    )
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true")
    mode.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)
    root = args.repo_root.resolve()
    output = args.output if args.output.is_absolute() else root / args.output
    rendered = canonical_json_text(build_reader_inventory(root))
    if args.check:
        if not output.exists() or output.read_text(encoding="utf-8") != rendered:
            raise ValidationError("card-data reader inventory is stale", path=output)
        return 0
    try:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(rendered, encoding="utf-8", newline="\n")
    except OSError as exc:
        raise WriteError("failed to write card-data reader inventory", path=output) from exc
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
