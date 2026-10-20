"""Characterize compatibility-pool leakage from non-production DSL lanes.

This report is intentionally read-only.  It captures the migration baseline
needed before source-lane enforcement can change pack construction or any
consumer-facing pool.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from collections import Counter
from pathlib import Path
from typing import Iterable, Mapping

from .errors import ValidationError, WriteError
from .serialization import natural_card_id_key
from .source_lanes import SourceLane, load_source_lane_manifest


SCHEMA_VERSION = 1
CARD_DECLARATION_RE = re.compile(
    r"^\s*card:\s*[\"']?(?P<card_id>[A-Z0-9-]+)",
    re.MULTILINE,
)
REAL_CARD_ID_RE = re.compile(r"^(?!TEST-|TST-)[A-Z]+\d*-\d+[A-Z]?$")


def _sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _file_digest(path: Path) -> str:
    return _sha256(path.read_bytes())


def _tree_digest(paths: Iterable[Path], *, root: Path) -> str:
    digest = hashlib.sha256()
    for path in sorted(paths, key=lambda item: item.as_posix().casefold()):
        relative = path.relative_to(root).as_posix()
        digest.update(relative.encode("utf-8"))
        digest.update(b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return digest.hexdigest()


def _display_path(path: Path, *, display_root: Path | None) -> str:
    """Render repository inputs without leaking an absolute staging path."""

    if display_root is None:
        return path.as_posix()
    try:
        return path.resolve().relative_to(display_root.resolve()).as_posix()
    except ValueError:
        return path.as_posix()


def load_yaml_card_ids(
    paths: Iterable[Path],
    *,
    display_root: Path | None = None,
) -> dict[str, list[str]]:
    """Return card IDs mapped to every declaring YAML path."""

    declarations: dict[str, list[str]] = {}
    for path in sorted(paths, key=lambda item: item.as_posix().casefold()):
        match = CARD_DECLARATION_RE.search(path.read_text(encoding="utf-8"))
        if match is None:
            raise ValidationError(
                "YAML card file has no top-level card declaration",
                path=path,
            )
        declarations.setdefault(match.group("card_id"), []).append(
            _display_path(path, display_root=display_root)
        )
    return declarations


def _load_deck_cards(raw_decklist: object, *, deck_id: str) -> list[str]:
    if isinstance(raw_decklist, str):
        try:
            raw_decklist = json.loads(raw_decklist)
        except json.JSONDecodeError as exc:
            raise ValidationError(
                "deck_library decklist is not valid JSON",
                card_id=None,
                details={"deck_id": deck_id},
            ) from exc
    if not isinstance(raw_decklist, list) or not all(
        isinstance(card_id, str) for card_id in raw_decklist
    ):
        raise ValidationError(
            "deck_library decklist must be a list of card IDs",
            details={"deck_id": deck_id},
        )
    return raw_decklist


def find_deck_usage(
    deck_library: Mapping[str, object], card_ids: Iterable[str]
) -> dict[str, list[dict[str, object]]]:
    """Find repository-visible library decks containing each requested ID."""

    wanted = set(card_ids)
    usage: dict[str, list[dict[str, object]]] = {card_id: [] for card_id in wanted}
    archetypes = deck_library.get("archetypes", {})
    if not isinstance(archetypes, Mapping):
        raise ValidationError("deck_library archetypes must be an object")

    for archetype_key, raw_archetype in archetypes.items():
        if not isinstance(raw_archetype, Mapping):
            raise ValidationError(
                "deck_library archetype entry must be an object",
                details={"archetype": str(archetype_key)},
            )
        archetype_name = str(raw_archetype.get("archetype_name", archetype_key))
        decklists = raw_archetype.get("decklists", [])
        if not isinstance(decklists, list):
            raise ValidationError(
                "deck_library decklists must be a list",
                details={"archetype": archetype_name},
            )
        for raw_deck in decklists:
            if not isinstance(raw_deck, Mapping):
                raise ValidationError(
                    "deck_library deck entry must be an object",
                    details={"archetype": archetype_name},
                )
            deck_id = str(raw_deck.get("deck_id", ""))
            cards = _load_deck_cards(raw_deck.get("decklist"), deck_id=deck_id)
            counts = Counter(cards)
            for card_id in sorted(wanted & counts.keys()):
                usage[card_id].append(
                    {
                        "deck_id": deck_id,
                        "archetype": archetype_name,
                        "source": raw_deck.get("source"),
                        "copies": counts[card_id],
                    }
                )

    for rows in usage.values():
        rows.sort(key=lambda row: (str(row["archetype"]).casefold(), str(row["deck_id"])))
    return dict(sorted(usage.items()))


def build_pool_impact_report(
    *,
    tested_cards_path: Path,
    cards_root: Path,
    deck_library_path: Path,
) -> dict[str, object]:
    tested_payload = json.loads(tested_cards_path.read_text(encoding="utf-8"))
    tested_ids = set(tested_payload.get("card_ids", []))
    if not all(isinstance(card_id, str) for card_id in tested_ids):
        raise ValidationError("tested_cards card_ids must contain only strings")

    all_yaml = list(cards_root.rglob("*.yaml"))
    example_root = cards_root / "_examples"
    example_yaml = [path for path in all_yaml if example_root in path.parents]
    production_yaml = [path for path in all_yaml if example_root not in path.parents]
    production = load_yaml_card_ids(production_yaml)
    examples = load_yaml_card_ids(example_yaml)

    production_ids = {
        card_id for card_id in production if REAL_CARD_ID_RE.fullmatch(card_id)
    }
    example_real_ids = {
        card_id for card_id in examples if REAL_CARD_ID_RE.fullmatch(card_id)
    }
    example_only_real_ids = sorted((example_real_ids - production_ids) & tested_ids)
    old_only_ids = sorted(tested_ids - production_ids)
    production_only_ids = sorted(production_ids - tested_ids)

    deck_library = json.loads(deck_library_path.read_text(encoding="utf-8"))
    usage = find_deck_usage(deck_library, example_only_real_ids)

    return {
        "schema_version": SCHEMA_VERSION,
        "baseline": "old-tested-cards-vs-production-without-examples",
        "inputs": {
            "tested_cards": {
                "path": tested_cards_path.as_posix(),
                "sha256": _file_digest(tested_cards_path),
            },
            "production_yaml": {
                "root": cards_root.as_posix(),
                "sha256": _tree_digest(production_yaml, root=cards_root),
            },
            "example_yaml": {
                "root": example_root.as_posix(),
                "sha256": _tree_digest(example_yaml, root=cards_root),
            },
            "deck_library": {
                "path": deck_library_path.as_posix(),
                "sha256": _file_digest(deck_library_path),
            },
        },
        "counts": {
            "old_tested_cards": len(tested_ids),
            "production_yaml_ids": len(production_ids),
            "example_yaml_ids": len(examples),
            "example_real_yaml_ids": len(example_real_ids),
            "old_tested_minus_production": len(old_only_ids),
            "production_minus_old_tested": len(production_only_ids),
            "example_only_real_cards_in_old_pool": len(example_only_real_ids),
            "affected_repository_decks": sum(len(rows) for rows in usage.values()),
        },
        "old_tested_minus_production": old_only_ids,
        "production_minus_old_tested": production_only_ids,
        "example_only_real_cards_in_old_pool": [
            {
                "card_id": card_id,
                "example_paths": examples[card_id],
                "deck_count": len(usage[card_id]),
                "decks": usage[card_id],
            }
            for card_id in example_only_real_ids
        ],
    }


def _simple_deck_usage(
    payload: object,
    card_ids: set[str],
    *,
    surface: str,
    source_path: Path,
    display_root: Path | None = None,
) -> dict[str, list[dict[str, object]]]:
    usage: dict[str, list[dict[str, object]]] = {card_id: [] for card_id in card_ids}
    if isinstance(payload, Mapping):
        decks = payload.get("starter_decks", payload.get("decks", []))
    else:
        decks = payload
    if not isinstance(decks, list):
        raise ValidationError("repository deck artifact must contain an array", path=source_path)
    for index, deck in enumerate(decks):
        if not isinstance(deck, Mapping):
            raise ValidationError("repository deck entry must be an object", path=source_path)
        decklist = deck.get("decklist")
        if isinstance(decklist, str):
            decklist = json.loads(decklist)
        cards = (
            list(decklist)
            if isinstance(decklist, list)
            else list(deck.get("egg_deck", [])) + list(deck.get("main_deck", []))
        )
        if not all(isinstance(card_id, str) for card_id in cards):
            raise ValidationError("repository deck cards must be strings", path=source_path)
        counts = Counter(cards)
        for card_id in sorted(card_ids & counts.keys()):
            usage[card_id].append(
                {
                    "surface": surface,
                    "path": _display_path(source_path, display_root=display_root),
                    "deck_id": str(deck.get("id", deck.get("deck_id", index))),
                    "name": str(deck.get("name", "")),
                    "copies": counts[card_id],
                }
            )
    return usage


def build_current_production_lane_report(
    *,
    tested_cards_path: Path,
    cards_root: Path,
    source_lanes_path: Path,
    deck_library_path: Path,
    starter_decks_path: Path,
    saved_deck_paths: Iterable[Path] = (),
    display_root: Path | None = None,
) -> dict[str, object]:
    """Compare the unchanged compatibility pool with the production YAML lane."""

    saved_paths = tuple(saved_deck_paths)
    tested_payload = json.loads(tested_cards_path.read_text(encoding="utf-8"))
    tested_ids = set(tested_payload.get("card_ids", []))
    if not all(isinstance(card_id, str) for card_id in tested_ids):
        raise ValidationError("tested_cards card_ids must contain only strings")
    lane_manifest = load_source_lane_manifest(source_lanes_path)
    lane_manifest.validate_tree(cards_root)
    production_paths = lane_manifest.yaml_paths(cards_root, SourceLane.PRODUCTION)
    production = load_yaml_card_ids(
        production_paths,
        display_root=display_root,
    )
    production_ids = {
        card_id for card_id in production if REAL_CARD_ID_RE.fullmatch(card_id)
    }
    removed = sorted(tested_ids - production_ids, key=natural_card_id_key)
    added = sorted(production_ids - tested_ids, key=natural_card_id_key)

    deck_library = json.loads(deck_library_path.read_text(encoding="utf-8"))
    library_usage = find_deck_usage(deck_library, removed)
    other_usage = {card_id: [] for card_id in removed}
    repository_paths = [starter_decks_path, *saved_paths]
    for index, path in enumerate(repository_paths):
        if not path.exists():
            continue
        surface = "starter_decks" if index == 0 else "saved_decks"
        found = _simple_deck_usage(
            json.loads(path.read_text(encoding="utf-8")),
            set(removed),
            surface=surface,
            source_path=path,
            display_root=display_root,
        )
        for card_id, rows in found.items():
            other_usage[card_id].extend(rows)

    removal_rows = []
    for card_id in removed:
        decks = [
            {"surface": "deck_library", **row}
            for row in library_usage[card_id]
        ] + other_usage[card_id]
        decks.sort(key=lambda row: (str(row["surface"]), str(row.get("deck_id", ""))))
        removal_rows.append(
            {
                "card_id": card_id,
                "production_paths": production.get(card_id, []),
                "deck_count": len(decks),
                "decks": decks,
            }
        )

    return {
        "schema_version": SCHEMA_VERSION,
        "comparison": "tested-cards-vs-current-production-source-lane",
        "consumer_pool_changed": False,
        "inputs": {
            "tested_cards": _file_digest(tested_cards_path),
            "source_lanes": _file_digest(source_lanes_path),
            "production_yaml": _tree_digest(production_paths, root=cards_root),
            "deck_library": _file_digest(deck_library_path),
            "starter_decks": (
                _file_digest(starter_decks_path) if starter_decks_path.exists() else None
            ),
            "saved_decks": {
                _display_path(path, display_root=display_root): _file_digest(path)
                for path in saved_paths
            },
        },
        "counts": {
            "tested_cards": len(tested_ids),
            "production_lane_cards": len(production_ids),
            "tested_not_production": len(removed),
            "production_not_tested": len(added),
            "affected_repository_decks": sum(
                row["deck_count"] for row in removal_rows
            ),
        },
        "tested_not_production": removal_rows,
        "production_not_tested": [
            {"card_id": card_id, "production_paths": production[card_id]}
            for card_id in added
        ],
    }
def render_report(report: Mapping[str, object]) -> str:
    return json.dumps(report, ensure_ascii=False, indent=2, sort_keys=True) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tested-cards", type=Path, default=Path("data/tested_cards.json"))
    parser.add_argument(
        "--cards-root", type=Path, default=Path("code/digimon-engine/cards")
    )
    parser.add_argument(
        "--deck-library", type=Path, default=Path("data/deck_library.json")
    )
    parser.add_argument(
        "--source-lanes",
        type=Path,
        default=Path("code/digimon-engine/cards/source_lanes.json"),
    )
    parser.add_argument(
        "--starter-decks", type=Path, default=Path("data/starter_decks.json")
    )
    parser.add_argument("--saved-decks", type=Path, action="append", default=[])
    parser.add_argument("--current-production-lane", action="store_true")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)

    if args.current_production_lane:
        report = build_current_production_lane_report(
            tested_cards_path=args.tested_cards,
            cards_root=args.cards_root,
            source_lanes_path=args.source_lanes,
            deck_library_path=args.deck_library,
            starter_decks_path=args.starter_decks,
            saved_deck_paths=args.saved_decks,
        )
    else:
        report = build_pool_impact_report(
            tested_cards_path=args.tested_cards,
            cards_root=args.cards_root,
            deck_library_path=args.deck_library,
        )
    rendered = render_report(report)
    if args.output is None:
        print(rendered, end="")
        return 0
    if args.check:
        if not args.output.exists() or args.output.read_text(encoding="utf-8") != rendered:
            raise ValidationError("pool impact baseline is stale", path=args.output)
        return 0
    try:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    except OSError as exc:
        raise WriteError("could not write pool impact baseline", path=args.output) from exc
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
