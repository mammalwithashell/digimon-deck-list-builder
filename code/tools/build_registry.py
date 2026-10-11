#!/usr/bin/env python3
"""Manage the append-only global card-index registry.

This compatibility entry point is intentionally network-free. Card discovery,
printed metadata normalization, and fixed-path publication belong to the
canonical transaction. This tool checks stable global indices and derived
``index``, ``norm_id``, and real-card ``card_index`` values, or writes an
explicit pair of non-fixed scratch paths.

Usage::

    python code/tools/build_registry.py --check
    python code/tools/build_registry.py --dry-run
    python code/tools/build_registry.py --cards-json <scratch> --registry <scratch>
    python code/tools/build_registry.py --check --offline  # deprecated alias
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
import uuid
from pathlib import Path
from typing import Any, Iterable, Mapping


PROJECT_ROOT = Path(__file__).resolve().parents[1]
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))

from data_paths import CARD_DATA_REVISION, CARD_REGISTRY, CARDS_JSON  # noqa: E402
from tools.card_data.errors import CardDataError, WriteError  # noqa: E402
from tools.card_data.legacy_commands import (  # noqa: E402
    CANONICAL_SYNC_GUIDANCE,
    emit_deprecation_warning,
)
from tools.card_data.registry import (  # noqa: E402
    REGISTRY_CAPACITY,
    RegistryValidationError,
    StableCardRegistry,
    allocate_append_only,
    apply_compatibility_values,
    read_registry,
    strict_json_object,
    validate_append_only,
    write_registry,
)
from tools.card_data.serialization import canonical_json_bytes  # noqa: E402


def card_sort_key(card_id: str) -> tuple[str, int, int]:
    """Preserve the legacy natural-sort helper's exact return shape."""

    prefix, num_str = card_id.rsplit("-", 1)
    match = re.match(r"^([A-Za-z]+)(\d*)$", prefix)
    letters = match.group(1) if match else prefix
    set_num = int(match.group(2)) if match and match.group(2) else 0
    return (letters, set_num, int(num_str))


def load_existing_mappings(path: str | os.PathLike[str]) -> tuple[dict[str, int], dict[str, dict]]:
    """Compatibility reader for the old ``(mappings, card_data)`` return shape."""

    source = Path(path)
    if not source.exists():
        return {}, {}
    payload = json.loads(
        source.read_text(encoding="utf-8"), object_pairs_hook=strict_json_object
    )
    if not isinstance(payload, dict):
        return {}, {}
    mappings = {
        card_id: row["index"]
        for card_id, row in payload.items()
        if isinstance(row, dict) and isinstance(row.get("index"), int)
    }
    return mappings, payload


def build_registry(
    card_ids: Iterable[str],
    existing_mappings: Mapping[str, int],
    capacity: int = REGISTRY_CAPACITY,
) -> dict[str, int]:
    """Compatibility wrapper around append-only allocation.

    Unlike the old implementation, this function performs no acquisition or
    card conversion.  ``capacity`` is retained in the signature but cannot
    change the fixed compatibility contract.
    """

    previous = StableCardRegistry(
        schema_version=1,
        registry_version=1,
        capacity=capacity,
        padding_index=0,
        assignments=existing_mappings,
    )
    return dict(allocate_append_only(previous, card_ids).assignments)


def _load_cards(path: Path) -> dict[str, dict[str, Any]]:
    try:
        payload = json.loads(
            path.read_text(encoding="utf-8"), object_pairs_hook=strict_json_object
        )
    except (OSError, json.JSONDecodeError) as exc:
        raise RegistryValidationError(
            "cards.json cannot be read",
            path=path,
            details={"error": str(exc)},
        ) from exc
    if not isinstance(payload, dict) or not all(
        isinstance(card_id, str) and isinstance(row, dict)
        for card_id, row in payload.items()
    ):
        raise RegistryValidationError("cards.json must map card IDs to objects", path=path)
    return payload


def _atomic_write_cards(path: Path, cards: Mapping[str, Mapping[str, Any]]) -> None:
    temporary = path.with_name(f".{path.name}.{os.getpid()}.{uuid.uuid4().hex}.tmp")
    try:
        temporary.write_bytes(canonical_json_bytes(cards))
        os.replace(temporary, path)
    except OSError as exc:
        try:
            temporary.unlink(missing_ok=True)
        except OSError:
            pass
        raise WriteError(
            "failed to write derived cards.json registry values",
            path=path,
            details={"error": str(exc)},
        ) from exc


def _registry_is_canonical(path: Path, registry: StableCardRegistry) -> bool:
    try:
        return path.read_bytes() == canonical_json_bytes(registry.to_dict())
    except OSError:
        return False


def _is_fixed_artifact(path: Path, fixed_path: Path) -> bool:
    return path.resolve() == fixed_path.resolve()


def _guard_transactionally_owned_writes(
    cards_path: Path,
    registry_path: Path,
    *,
    check: bool,
    dry_run: bool,
) -> None:
    """Refuse direct writes to marker-owned fixed paths."""

    if check or dry_run or not CARD_DATA_REVISION.exists():
        return
    fixed_targets = [
        path
        for path, fixed_path in (
            (cards_path, CARDS_JSON),
            (registry_path, CARD_REGISTRY),
        )
        if _is_fixed_artifact(path, fixed_path)
    ]
    if not fixed_targets:
        return
    raise WriteError(
        "build_registry.py cannot directly mutate transactionally owned "
        f"card-data artifacts after producer cutover; {CANONICAL_SYNC_GUIDANCE}",
        path=fixed_targets[0],
        details={
            "revision_marker": str(CARD_DATA_REVISION),
            "fixed_targets": [str(path) for path in fixed_targets],
        },
    )


def synchronize_registry(
    cards_path: Path,
    registry_path: Path,
    *,
    check: bool = False,
    dry_run: bool = False,
) -> tuple[StableCardRegistry, bool, bool]:
    """Synchronize local IDs and derived values without touching printed fields.

    Returns ``(candidate, registry_changed, cards_changed)``.
    """

    _guard_transactionally_owned_writes(
        cards_path,
        registry_path,
        check=check,
        dry_run=dry_run,
    )
    cards = _load_cards(cards_path)
    if registry_path.exists():
        previous = read_registry(registry_path)
        candidate = allocate_append_only(previous, cards.keys())
        validate_append_only(previous, candidate)
    else:
        if check:
            raise RegistryValidationError(
                "stable card registry is missing; run build_registry.py",
                path=registry_path,
            )
        previous = None
        candidate = StableCardRegistry.from_cards_json(cards)

    derived_cards = apply_compatibility_values(cards, candidate)
    registry_changed = previous is None or candidate != previous or not _registry_is_canonical(
        registry_path, candidate
    )
    cards_changed = derived_cards != cards

    if check:
        if registry_changed:
            raise RegistryValidationError(
                "stable card registry is stale or non-canonical",
                path=registry_path,
            )
        if cards_changed:
            raise RegistryValidationError(
                "cards.json registry-derived values are stale",
                path=cards_path,
            )
    elif not dry_run:
        if registry_changed:
            write_registry(registry_path, candidate)
        if cards_changed:
            _atomic_write_cards(cards_path, derived_cards)

    return candidate, registry_changed, cards_changed


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail on committed registry or derived-value drift without writing",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="report append-only candidate changes without writing",
    )
    parser.add_argument(
        "--offline",
        action="store_true",
        help="deprecated compatibility flag; registry management is always offline",
    )
    parser.add_argument(
        "--capacity",
        type=int,
        default=REGISTRY_CAPACITY,
        help=f"fixed compatibility capacity (must remain {REGISTRY_CAPACITY})",
    )
    parser.add_argument(
        "--sets",
        nargs="+",
        default=None,
        help="retired acquisition flag; ingest source data before allocating indices",
    )
    parser.add_argument(
        "--force",
        action="store_true",
        help="retired unsafe flag; stable indices can never be reassigned",
    )
    parser.add_argument(
        "--cards-json",
        type=Path,
        default=CARDS_JSON,
        help=(
            "compatibility cards input/output; post-cutover writes require an "
            "explicit non-fixed scratch path"
        ),
    )
    parser.add_argument(
        "--registry",
        type=Path,
        default=CARD_REGISTRY,
        help=(
            "stable registry input/output; post-cutover writes require an "
            "explicit non-fixed scratch path"
        ),
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    mode = (
        "--check"
        if args.check
        else "--dry-run"
        if args.dry_run
        else "append-only compatibility sync"
    )
    emit_deprecation_warning("build_registry.py", mode)
    if args.capacity != REGISTRY_CAPACITY:
        print(
            f"error: registry capacity is fixed at {REGISTRY_CAPACITY}; "
            "changing it requires a separate tensor/model compatibility change",
            file=sys.stderr,
        )
        return 2
    if args.sets:
        print(
            "error: --sets no longer fetches or normalizes card data; run the "
            "source ingest/sync command first, then run build_registry.py",
            file=sys.stderr,
        )
        return 2
    if args.force:
        print("error: --force is retired; stable indices cannot be reassigned", file=sys.stderr)
        return 2
    if args.check and args.dry_run:
        print("error: --check and --dry-run are mutually exclusive", file=sys.stderr)
        return 2
    if args.offline:
        print(
            "warning: --offline is deprecated; registry management is always network-free",
            file=sys.stderr,
        )

    try:
        registry, registry_changed, cards_changed = synchronize_registry(
            args.cards_json,
            args.registry,
            check=args.check,
            dry_run=args.dry_run,
        )
    except (CardDataError, ValueError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    action = "checked" if args.check else "would manage" if args.dry_run else "managed"
    print(
        f"{action} {len(registry.assignments)} stable card indices "
        f"(max={registry.maximum_index}, capacity={registry.capacity})"
    )
    if args.dry_run:
        print(f"registry_changed={registry_changed} cards_changed={cards_changed}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
