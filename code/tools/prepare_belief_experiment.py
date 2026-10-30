"""Freeze disjoint legal/admitted deck splits before any robustness training."""

from __future__ import annotations
import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from digimon_gym.agents.belief_evaluation import fingerprint


def split_lists(rows: list[dict], seed: int) -> dict[str, list[dict]]:
    unique = {}
    for row in rows:
        cards = sorted(row["cards"])
        key = hashlib.sha256(
            json.dumps(cards, separators=(",", ":")).encode()
        ).hexdigest()
        unique.setdefault(
            key,
            {
                "id": key,
                "cards": cards,
                "archetype": row.get("archetype", "unspecified"),
            },
        )
    ordered = sorted(
        unique.values(),
        key=lambda r: hashlib.sha256(f"{seed}:{r['id']}".encode()).hexdigest(),
    )
    if len(ordered) < 3:
        raise ValueError("need at least three distinct legal lists for disjoint splits")
    heldout = max(1, len(ordered) // 10)
    return {
        "train": ordered[2 * heldout :],
        "validation": ordered[:heldout],
        "test": ordered[heldout : 2 * heldout],
    }


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--source", type=Path, required=True)
    p.add_argument("--out", type=Path, required=True)
    p.add_argument("--cli", type=Path, required=True)
    p.add_argument("--format", default="standard")
    p.add_argument("--seed", type=int, default=1729)
    a = p.parse_args(argv)
    if a.out.exists():
        raise FileExistsError("refusing to overwrite a frozen experiment")
    import digimon_engine

    raw = json.loads(a.source.read_text(encoding="utf-8"))
    if isinstance(raw, dict) and "archetypes" in raw:
        rows = []
        for archetype, value in raw["archetypes"].items():
            for entry in value.get("decklists", []):
                cards = entry["decklist"]
                if isinstance(cards, str):
                    cards = json.loads(cards)
                rows.append({"archetype": archetype, "cards": cards})
    elif isinstance(raw, list):
        rows = [r if isinstance(r, dict) else {"cards": r} for r in raw]
    else:
        raise ValueError("source must be deck_library or a list of card arrays/objects")
    root = Path(__file__).resolve().parents[2]
    admission = json.loads((root / "data/card_admissibility.json").read_text())["cards"]
    registered = set(digimon_engine.load_implemented_card_ids())
    input_count = len(rows)
    accepted = []
    excluded = []
    for i, row in enumerate(rows):
        cards = row["cards"]
        result = digimon_engine.validate_deck_for_game_mode(cards, a.format)
        unsupported = sorted(
            {
                c
                for c in cards
                if c not in registered
                or admission.get(c, {}).get("admissible") is not True
            }
        )
        if result.is_valid and not unsupported:
            accepted.append(row)
        else:
            excluded.append(
                {"index": i, "errors": list(result.errors), "unadmitted": unsupported}
            )
    splits = split_lists(accepted, a.seed)
    a.out.mkdir(parents=True)
    for name, rows in splits.items():
        (a.out / f"{name}.json").write_text(
            json.dumps(rows, indent=2), encoding="utf-8"
        )
    source_hash = hashlib.sha256(a.source.read_bytes()).hexdigest()
    inventory = {
        "schema": "belief-experiment-population-v1",
        "source_sha256": source_hash,
        "format": a.format,
        "region": "ENG",
        "restriction_policy": "repository snapshot; not a claim of live tournament legality",
        "split_seed": a.seed,
        "input_lists": input_count,
        "format_policy_sha256": hashlib.sha256(
            (root / "data/deck_formats.json").read_bytes()
        ).hexdigest(),
        "accepted_lists": len(accepted),
        "excluded": excluded,
        "splits": {
            k: {
                "unique_lists": len(v),
                "archetypes": sorted({r["archetype"] for r in v}),
                "sha256": hashlib.sha256(
                    (a.out / f"{k}.json").read_bytes()
                ).hexdigest(),
            }
            for k, v in splits.items()
        },
        "candidate_weighting": "one equal weight per distinct training list; duplicate source sightings add no mass",
    }
    inventory["hash"] = fingerprint(inventory)
    (a.out / "population.json").write_text(
        json.dumps(inventory, indent=2), encoding="utf-8"
    )
    subprocess.run(
        [
            str(a.cli),
            "prepare-belief-prior",
            "--decks",
            str(a.out / "train.json"),
            "--out",
            str(a.out / "prior.json"),
            "--format-id",
            a.format,
            "--provenance",
            json.dumps(
                {
                    "population": inventory["hash"],
                    "split": "train",
                    "source_sha256": source_hash,
                },
                sort_keys=True,
            ),
        ],
        check=True,
    )
    prior_hash = hashlib.sha256((a.out / "prior.json").read_bytes()).hexdigest()
    protocol = {
        "schema": "fair-belief-protocol-v1",
        "population_hash": inventory["hash"],
        "prior_file_sha256": prior_hash,
        "seeds": [11, 22, 33],
        "budgets": {
            "exploiter_steps": 2048,
            "training_games_per_seed": 32,
            "search_simulations": 32,
            "worlds": 4,
            "generations": 2,
        },
        "confidence_method": "difference of 97.5% Wilson component intervals; nominal 95% pairwise coverage; checked per seed",
        "scope": "full comparison within the first frozen admitted cohort; no cross-archetype claim; inconclusive evidence blocks promotion",
        "panels": {
            "heldout_policy": {
                "direction": "higher",
                "margin": 0.02,
                "minimum_games_per_seed": 256,
                "evaluation_mode": "policy_only",
            },
            "anchors": {
                "direction": "higher",
                "margin": -0.03,
                "minimum_games_per_seed": 256,
                "evaluation_mode": "policy_only",
            },
            "exploiter": {
                "direction": "lower",
                "margin": 0.03,
                "minimum_games_per_seed": 256,
                "evaluation_mode": "policy_only",
            },
            "heldout_search": {
                "direction": "higher",
                "margin": -0.03,
                "minimum_games_per_seed": 256,
                "evaluation_mode": "search_assisted",
            },
        },
    }
    (a.out / "protocol.json").write_text(
        json.dumps(protocol, indent=2), encoding="utf-8"
    )
    print(
        json.dumps(
            {
                "population_hash": inventory["hash"],
                "protocol_hash": fingerprint(protocol),
                "splits": inventory["splits"],
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
