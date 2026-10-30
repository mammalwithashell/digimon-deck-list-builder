"""Reuse a frozen cloud cohort without mixing its existing evaluation partitions."""

from __future__ import annotations
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
ROOT = Path(__file__).resolve().parents[2]
SOURCES = {
    "train": "train_snapshot.json",
    "validation": "supplemental_snapshot.json",
    "test": "heldout_snapshot.json",
    "pet_regression": "pet_regression_snapshot.json",
}


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def read(path):
    return json.loads(Path(path).read_bytes())


def content(cards):
    return tuple(sorted(Counter(cards).items()))


def validate_roles(partitions):
    sets = {
        name: {content(row["cards"]) for row in rows}
        for name, rows in partitions.items()
    }
    for name in ("train", "validation", "test"):
        if not sets[name] or len(sets[name]) != len(partitions[name]):
            raise ValueError(f"{name}: missing or duplicate distinct lists")
    for a, b in (("train", "validation"), ("train", "test"), ("validation", "test")):
        if sets[a] & sets[b]:
            raise ValueError(f"{a}/{b}: evaluation content leakage")
    # Original pet regressions may intentionally overlap training. Never call
    # them an unseen-list panel or add them to the held-out test population.
    return {name: len(values) for name, values in sets.items()}


def prepare(source, out, cli):
    if out.exists():
        raise FileExistsError("refusing to replace a frozen cloud-belief pool")
    import digimon_engine
    from digimon_gym.agents.gauntlet import GeneralistDeckPool
    from digimon_gym.agents.belief_runtime import native_binding_fingerprints

    playable = set(digimon_engine.load_implemented_card_ids()) & set(
        read(ROOT / "data/cards.json")
    )
    partitions = {}
    sources = {}
    for role, filename in SOURCES.items():
        path = source / filename
        pool = GeneralistDeckPool.from_snapshot(path, implemented_card_ids=playable)
        rows = []
        for archetype, entries in sorted(pool.archetypes.items()):
            for entry in sorted(entries, key=lambda e: e.deck_id):
                legal = digimon_engine.validate_deck_for_game_mode(
                    entry.card_ids, "standard"
                )
                if not legal.is_valid:
                    raise ValueError(f"{role}/{entry.deck_id}: {legal.errors}")
                rows.append(
                    {
                        "name": f"{archetype}:{entry.deck_id}",
                        "archetype": archetype,
                        "deck_id": entry.deck_id,
                        "cards": list(entry.card_ids),
                    }
                )
        partitions[role] = rows
        sources[role] = {
            "file": str(path),
            "sha256": sha(path),
            "snapshot_hash": read(path)["snapshot_hash"],
        }
    distinct = validate_roles(partitions)
    runtime = read(source / "decks.json")
    if {content(r["cards"]) for r in runtime} != {
        content(r["cards"]) for r in partitions["train"]
    }:
        raise ValueError("native sampling slots differ from frozen training contents")
    out.mkdir(parents=True)
    for role, rows in partitions.items():
        (out / (role + ".json")).write_text(
            json.dumps(runtime if role == "train" else rows, indent=2), encoding="utf-8"
        )
    provenance = {
        "schema": "cloud-belief-cohort-v1",
        "source": str(source),
        "sources": sources,
        "runtime_slots_sha256": sha(source / "decks.json"),
        "source_policy": "exact frozen cloud partitions; no card substitutions or resplitting",
        "prior_weighting": "equal per distinct training list; repeated native slots are sampling weights, not extra candidates",
        "population_fallback": "globally admitted IDs, independent of held-out list contents",
        "native_engine_sha256": sha(cli),
        "native_binding_sha256": native_binding_fingerprints(),
        "gates": {
            name: sha(ROOT / "data" / name)
            for name in (
                "cards.json",
                "card_admissibility.json",
                "mechanics_readiness.json",
                "deck_formats.json",
            )
        },
        "coverage": {
            role: {
                "distinct_lists": distinct[role],
                "runtime_slots": len(runtime) if role == "train" else len(rows),
                "archetypes": dict(
                    sorted(Counter(row["archetype"] for row in rows).items())
                ),
            }
            for role, rows in partitions.items()
        },
        "admission_is_closed_search_coverage": False,
        "heldout_scope": (
            f"{distinct['test']} source-held-out lists in {len({r['archetype'] for r in partitions['test']})} archetypes; "
            + (
                ", ".join(
                    sorted(
                        {r["archetype"] for r in partitions["train"]}
                        - {r["archetype"] for r in partitions["test"]}
                    )
                )
                + " has same-list pet regression only"
                if {r["archetype"] for r in partitions["train"]}
                - {r["archetype"] for r in partitions["test"]}
                else "pet regressions remain separate"
            )
        ),
        "supplemental_scope": "validation only; source evidence is weaker/mixed; not new competitive qualification",
        "promotion_eligible": False,
    }
    (out / "provenance.json").write_text(
        json.dumps(provenance, indent=2), encoding="utf-8"
    )
    subprocess.run(
        [
            str(cli),
            "--cards-json",
            str(ROOT / "data/cards.json"),
            "prepare-belief-prior",
            "--decks",
            str(out / "train.json"),
            "--out",
            str(out / "prior.json"),
            "--provenance",
            json.dumps(
                {"cohort_sha256": sha(out / "provenance.json"), "split": "train"},
                sort_keys=True,
            ),
        ],
        check=True,
        cwd=ROOT,
    )
    # Zero-game calls validate the whole native admission surface without
    # pretending an admission check is a played game or a search qualification.
    for role in partitions:
        subprocess.run(
            [
                str(cli),
                "--cards-json",
                str(ROOT / "data/cards.json"),
                "selfplay",
                "--uniform",
                "--decks",
                str(out / (role + ".json")),
                "--games",
                "0",
                "--out",
                str(out / (role + "-admission")),
            ],
            check=True,
            cwd=ROOT,
            stdout=subprocess.DEVNULL,
        )
    provenance["prior_sha256"] = sha(out / "prior.json")
    provenance["native_all_partitions_admitted"] = True
    # Keep the pre-prior provenance bytes intact: the prior fingerprints them.
    (out / "admission-report.json").write_text(
        json.dumps(provenance, indent=2), encoding="utf-8"
    )
    return provenance


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument(
        "--source", type=Path, default=ROOT / "data/training_pools/ex13-20261026"
    )
    p.add_argument("--out", type=Path, required=True)
    p.add_argument("--cli", type=Path, required=True)
    a = p.parse_args()
    result = prepare(a.source.resolve(), a.out.resolve(), a.cli.resolve())
    print(json.dumps(result["coverage"], indent=2))


if __name__ == "__main__":
    main()
