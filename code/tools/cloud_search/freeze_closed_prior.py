"""Audit and copy an existing training-only prior; native qualification stays separate."""
from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import shutil
import time

SOURCES = dict(train="train_snapshot.json", validation="supplemental_snapshot.json",
               test="heldout_snapshot.json", pet_regression="pet_regression_snapshot.json")
GATES = ("cards.json", "card_admissibility.json", "mechanics_readiness.json", "deck_formats.json")


def read(path):
    return json.loads(Path(path).read_bytes())


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def require(value, message):
    if not value:
        raise ValueError(message)


def contents(cards):
    return tuple(sorted(Counter(cards).items()))


def audit(prior, snapshots, slots, admission, mechanics, cards):
    partitions = {role: {contents(row["card_ids"]) for row in doc["decks"]}
                  for role, doc in snapshots.items()}
    for role, expected in (("train", 21), ("validation", 36), ("test", 23)):
        require(len(partitions[role]) == len(snapshots[role]["decks"]) == expected,
                f"{role}: distinct-list coverage changed")
    overlaps = {f"{a}/{b}": len(partitions[a] & partitions[b])
                for a, b in (("train", "validation"), ("train", "test"), ("validation", "test"))}
    require(not any(overlaps.values()), "partition content leakage")
    require(len(slots) == 26 and {contents(row["cards"]) for row in slots} == partitions["train"],
            "runtime slots differ from training partition")
    candidates = prior["candidates"]
    require(len(candidates) == 21 and {contents(row["cards"]) for row in candidates} == partitions["train"],
            "prior is not exactly the 21 training lists")
    require(all(row["weight"] == 1.0 for row in candidates), "unequal candidate weights")
    require(prior["schema_version"] == 1 and prior["format_id"] == "standard", "prior schema/format")
    require(json.loads(prior["provenance"])["split"] == "train", "prior split")
    require(prior["unseen_mass"] == 0.1 and prior["fallback_attempts"] == 128, "fallback settings changed")
    population = set(prior["population"])
    require(len(population) == len(prior["population"]), "duplicate population IDs")
    require(mechanics["ready"] is True, "mechanics admission blocked")
    require(all(c in cards and admission["cards"].get(c, {}).get("admissible") is True
                for c in population), "fallback population no longer admitted")
    require(all(set(row["cards"]) <= population for row in candidates), "candidate outside population")
    return dict(distinct_lists={r: len(s) for r, s in partitions.items()}, runtime_slots=len(slots),
                candidate_weights="equal per distinct list", fallback_population=len(population),
                unseen_mass=prior["unseen_mass"], fallback_attempts=prior["fallback_attempts"],
                partition_overlaps=overlaps,
                pet_training_overlap=len(partitions["train"] & partitions["pet_regression"]),
                heldout_archetypes=len({r["archetype"] for r in snapshots["test"]["decks"]}),
                heldout_light_hybrid_measured=False)


def freeze(prepared, source, data, out):
    require(not out.exists(), "refusing to replace existing prior freeze")
    provenance = read(prepared / "provenance.json")
    for role, name in SOURCES.items():
        require(digest(source / name) == provenance["sources"][role]["sha256"], f"changed {role} source")
    require(digest(source / "decks.json") == provenance["runtime_slots_sha256"], "changed slots")
    # Data and deck rules must match the native preparation that admitted this prior.
    for name in ("cards.json", "deck_formats.json"):
        require(digest(data / name) == provenance["gates"][name], f"changed {name}")
    require(digest(prepared / "provenance.json") == json.loads(read(prepared / "prior.json")["provenance"])["cohort_sha256"],
            "prior does not bind preparation provenance")
    report = audit(read(prepared / "prior.json"), {r: read(source / n) for r, n in SOURCES.items()},
                   read(source / "decks.json"), read(data / "card_admissibility.json"),
                   read(data / "mechanics_readiness.json"), read(data / "cards.json"))
    copies = {"prior.json": prepared / "prior.json", "original-provenance.json": prepared / "provenance.json",
              "original-admission-report.json": prepared / "admission-report.json",
              "original-prior-admission.json": prepared / "prior.admission.json",
              "sources/decks.json": source / "decks.json", "freeze_closed_prior.py": Path(__file__)}
    copies.update({"sources/" + n: source / n for n in SOURCES.values()})
    copies.update({"data/" + n: data / n for n in GATES})
    expected = {n: digest(p) for n, p in copies.items()}
    out.mkdir(parents=True)
    for name, src in copies.items():
        target = out / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(src, target)
        require(digest(target) == expected[name], f"copy/source race: {name}")
    report.update(schema="closed-prior-freeze-v1", created_at=time.time(), files=expected,
                  native_runtime_admission="pending_new_frozen_runtime",
                  full_search_qualification=False, scored_launch_authorized=False,
                  weight_training_heldout_provenance="pending_separate_audit",
                  gate_hash_changes={n: dict(original=provenance["gates"][n], current=expected["data/" + n])
                                     for n in GATES if provenance["gates"][n] != expected["data/" + n]})
    (out / "freeze-audit.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return report


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("prepared", "source", "data", "out"):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    result = freeze(args.prepared, args.source, args.data, args.out)
    print(json.dumps({k: result[k] for k in ("distinct_lists", "fallback_population", "native_runtime_admission")}))
