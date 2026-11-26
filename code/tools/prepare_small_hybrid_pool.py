"""Slice and verify the frozen 3-archetype EX13 hybrid training pool (`ex13-small3`).

Slices exact admitted tournament lists from `data/training_pools/ex13-20261026/`
with zero card substitutions, verifying:
  - Global mechanics readiness (`require_mechanics_ready()`)
  - Per-card admissibility (`validate_admissible_deck`)
  - Standard deck legality (`digimon_engine.validate_deck`)
  - Exact 2 training + 2 held-out distinct lists per archetype
  - Disjoint card-count fingerprints between training and held-out panels
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from collections import Counter
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
if str(REPO_ROOT / "code") not in sys.path:
    sys.path.insert(0, str(REPO_ROOT / "code"))

DEFAULT_SOURCE_DIR = REPO_ROOT / "data" / "training_pools" / "ex13-20261026"
DEFAULT_OUT_DIR = REPO_ROOT / "data" / "training_pools" / "ex13-small3"
DEFAULT_ARCHETYPES = ("Examon", "TS Jupitermon", "Three Musketeers")


def _sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _sha256_file(path: Path) -> str:
    return _sha256_bytes(path.read_bytes())


def _fingerprint(cards: list[str]) -> tuple[tuple[str, int], ...]:
    return tuple(sorted(Counter(cards).items()))


def prepare_pool(
    source_dir: Path = DEFAULT_SOURCE_DIR,
    out_dir: Path = DEFAULT_OUT_DIR,
    archetypes: tuple[str, ...] = DEFAULT_ARCHETYPES,
) -> dict:
    import digimon_engine
    from digimon_gym.agents.gauntlet import (
        GeneralistDeckPool,
        require_mechanics_ready,
        validate_admissible_deck,
    )

    require_mechanics_ready()
    implemented = set(digimon_engine.load_implemented_card_ids())

    src_train_path = source_dir / "train_snapshot.json"
    src_heldout_path = source_dir / "heldout_snapshot.json"
    src_decks_path = source_dir / "decks.json"

    src_train = GeneralistDeckPool.from_snapshot(src_train_path, implemented_card_ids=implemented)
    src_heldout = GeneralistDeckPool.from_snapshot(src_heldout_path, implemented_card_ids=implemented)
    raw_decks = json.loads(src_decks_path.read_text(encoding="utf-8"))

    train_subset = {}
    heldout_subset = {}
    train_fps: set[tuple[tuple[str, int], ...]] = set()
    heldout_fps: set[tuple[tuple[str, int], ...]] = set()

    for arch in archetypes:
        if arch not in src_train.archetypes:
            raise ValueError(f"Archetype {arch!r} missing from {src_train_path}")
        if arch not in src_heldout.archetypes:
            raise ValueError(f"Archetype {arch!r} missing from {src_heldout_path}")

        t_entries = sorted(src_train.archetypes[arch], key=lambda d: d.deck_id)
        h_entries = sorted(src_heldout.archetypes[arch], key=lambda d: d.deck_id)
        if len(t_entries) != 2:
            raise ValueError(f"{arch}: expected 2 training lists, found {len(t_entries)}")
        if len(h_entries) != 2:
            raise ValueError(f"{arch}: expected 2 held-out lists, found {len(h_entries)}")

        for panel_name, entries, fp_set in (
            ("train", t_entries, train_fps),
            ("heldout", h_entries, heldout_fps),
        ):
            for entry in entries:
                validate_admissible_deck(entry.card_ids, label=f"{arch}:{panel_name}:{entry.deck_id}")
                if not set(entry.card_ids) <= implemented:
                    raise ValueError(f"{arch}:{entry.deck_id} has unimplemented cards")
                legality = digimon_engine.validate_deck(entry.card_ids)
                if not legality.is_valid:
                    raise ValueError(f"{arch}:{entry.deck_id} failed deck legality: {legality.errors}")
                fp = _fingerprint(entry.card_ids)
                if fp in fp_set:
                    raise ValueError(f"{arch}:{entry.deck_id} duplicate card-count fingerprint in {panel_name}")
                fp_set.add(fp)

        train_subset[arch] = t_entries
        heldout_subset[arch] = h_entries

    overlap = train_fps & heldout_fps
    if overlap:
        raise ValueError("Training and held-out panels share identical card-count fingerprints")

    # Slice exact native decks.json slots preserving source order for the chosen archetypes.
    wanted_set = set(archetypes)
    sliced_decks = [
        slot
        for slot in raw_decks
        if slot["name"].split(":sha256:", 1)[0] in wanted_set
    ]
    if len(sliced_decks) != 2 * len(archetypes):
        raise ValueError(
            f"Expected {2 * len(archetypes)} native deck slots, found {len(sliced_decks)}"
        )

    heldout_decks = []
    for arch in sorted(heldout_subset):
        for idx, entry in enumerate(heldout_subset[arch], start=1):
            heldout_decks.append(
                {
                    "name": f"{arch}:{entry.deck_id}:heldout{idx}",
                    "cards": list(entry.card_ids),
                }
            )

    out_dir.mkdir(parents=True, exist_ok=True)
    train_pool = GeneralistDeckPool(train_subset)
    heldout_pool = GeneralistDeckPool(heldout_subset)
    train_snap_out = out_dir / "train_snapshot.json"
    heldout_snap_out = out_dir / "heldout_snapshot.json"
    decks_out = out_dir / "decks.json"
    heldout_decks_out = out_dir / "heldout_decks.json"

    train_pool.write_snapshot(train_snap_out)
    heldout_pool.write_snapshot(heldout_snap_out)
    decks_out.write_text(json.dumps(sliced_decks, indent=2) + "\n", encoding="utf-8")
    heldout_decks_out.write_text(json.dumps(heldout_decks, indent=2) + "\n", encoding="utf-8")

    manifest = {
        "schema": "hybrid-small-pool-v1",
        "pool_id": out_dir.name,
        "source_pool": str(source_dir.relative_to(REPO_ROOT)),
        "archetypes": list(archetypes),
        "train_decks_per_archetype": 2,
        "heldout_decks_per_archetype": 2,
        "total_train_slots": len(sliced_decks),
        "total_heldout_slots": len(heldout_decks),
        "source_hashes": {
            "train_snapshot.json": _sha256_file(src_train_path),
            "heldout_snapshot.json": _sha256_file(src_heldout_path),
            "decks.json": _sha256_file(src_decks_path),
            "cards.json": _sha256_file(REPO_ROOT / "data" / "cards.json"),
            "card_admissibility.json": _sha256_file(REPO_ROOT / "data" / "card_admissibility.json"),
            "mechanics_readiness.json": _sha256_file(REPO_ROOT / "data" / "mechanics_readiness.json"),
        },
        "output_hashes": {
            "train_snapshot.json": _sha256_file(train_snap_out),
            "heldout_snapshot.json": _sha256_file(heldout_snap_out),
            "decks.json": _sha256_file(decks_out),
            "heldout_decks.json": _sha256_file(heldout_decks_out),
        },
        "lists": {
            arch: {
                "training": [
                    {
                        "deck_id": e.deck_id,
                        "source": e.source,
                        "source_deck_id": e.source_deck_id,
                        "source_url": e.source_url,
                    }
                    for e in train_subset[arch]
                ],
                "heldout": [
                    {
                        "deck_id": e.deck_id,
                        "source": e.source,
                        "source_deck_id": e.source_deck_id,
                        "source_url": e.source_url,
                    }
                    for e in heldout_subset[arch]
                ],
            }
            for arch in archetypes
        },
    }
    (out_dir / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")

    readme_lines = [
        f"# Frozen Small Hybrid Pool (`{out_dir.name}`)",
        "",
        f"Exact zero-substitution 3-archetype slice of `{source_dir.relative_to(REPO_ROOT)}`.",
        "",
        "| Archetype | Panel | Deck ID | Source ID |",
        "| --- | --- | --- | --- |",
    ]
    for arch in archetypes:
        for panel, entries in (("training", train_subset[arch]), ("heldout", heldout_subset[arch])):
            for e in entries:
                readme_lines.append(f"| {arch} | {panel} | `{e.deck_id[:19]}...` | `{e.source_deck_id}` |")
    (out_dir / "README.md").write_text("\n".join(readme_lines) + "\n", encoding="utf-8")
    return manifest


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    p.add_argument("--source-dir", type=Path, default=DEFAULT_SOURCE_DIR)
    p.add_argument("--out-dir", type=Path, default=DEFAULT_OUT_DIR)
    p.add_argument(
        "--archetypes",
        type=str,
        default=",".join(DEFAULT_ARCHETYPES),
        help="Comma-separated archetype names to slice.",
    )
    args = p.parse_args(argv)
    archs = tuple(a.strip() for a in args.archetypes.split(",") if a.strip())
    manifest = prepare_pool(args.source_dir, args.out_dir, archs)
    print(
        f"Prepared {manifest['pool_id']}: {len(archs)} archetypes, "
        f"{manifest['total_train_slots']} train slots, "
        f"{manifest['total_heldout_slots']} held-out slots -> {args.out_dir}",
        flush=True,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
