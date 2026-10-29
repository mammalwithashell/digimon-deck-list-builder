"""Read-only search-target / frozen-policy comparison on identical positions.

Main/Breeding action routes only; these are decision targets, not game events.
Opportunity cohorts are fixed-seed reservoir samples. Teacher-argmax cohorts
are separately labeled oversamples, never estimates of overall frequency.
No fitting, live-run writes, policy changes, or hidden-state reconstruction.
"""
from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path

import numpy as np


def route_masks(actions, phase):
    """Pinned to action schema 1; IDs are only decoded in their proper phase."""
    a = np.asarray(actions)
    evo = (phase == 3) & (a >= 400) & (a < 1000)
    return {
        "raising": evo & ((a - 400) % 15 == 14),
        "battle": evo & ((a - 400) % 15 != 14),
        "dna": (phase == 3) & (a >= 63) & (a < 93),
        "promotion": (phase == 2) & (a == 61),
    }


def offer(reservoir, seen, key, item, capacity, rng):
    seen[key] += 1
    bucket = reservoir.setdefault(key, [])
    if len(bucket) < capacity:
        bucket.append(item)
    else:
        index = int(rng.integers(seen[key]))
        if index < capacity:
            bucket[index] = item


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--generations", type=int, nargs="+", default=[1, 2])
    parser.add_argument("--sample", type=int, default=512)
    args = parser.parse_args()
    if args.sample < 1 or any(g < 1 for g in args.generations):
        parser.error("sample and generation numbers must be positive")
    if args.out.exists():
        raise FileExistsError(args.out)
    import digimon_engine
    import torch
    from sb3_contrib import MaskablePPO
    from tools.selfplay_scheduler import _read_commit

    torch.set_num_threads(1)
    layout = digimon_engine.get_observation_layout("standard_lite_v2")
    assert layout["feature_schema_version"] == "standard_lite_v2.3"
    global_offset = next(s["offset"] for s in layout["sections"] if s["name"] == "global_features")
    # Global phase one-hot is slots 8..40 in this explicitly pinned schema.
    phase_slice = slice(global_offset + 8, global_offset + 40)
    result = {"schema": "evolution-imitation-v1", "sample_seed": 20261025,
              "scope": "completed non-forced training positions; Main/Breeding routes; in-sample imitation diagnostic, not a strength evaluation",
              "cohorts": "opportunity reservoir and separately oversampled teacher-argmax rows; overlaps allowed",
              "layout_hash": layout["layout_hash"], "generations": {}, "model_hashes": {}}
    selected = {}
    for generation in args.generations:
        totals, routes, reservoir, seen = Counter(), {}, {}, Counter()
        rng = np.random.default_rng(20261025 + generation)
        for folder in sorted((args.root / f"generation/gen{generation}/shards").glob("proc*")):
            manifest = json.loads((folder / "manifest.json").read_text())
            _read_commit(folder, manifest["game_start"], manifest["games"], manifest["seed"])
            assert manifest["truncated_games"] == 0, "Use only clean completed generations"
            assert manifest["value_target"] == "terminal_outcome"
            assert manifest["model_contract"]["tensor_layout_hash"] == layout["layout_hash"]
            assert manifest["model_contract"]["action_schema_version"] == 1
            assert manifest["model_contract"]["action_space_size"] == digimon_engine.ACTION_SPACE_SIZE
            totals["games"] += manifest["games"]
            for shard in manifest["shards"]:
                p = folder / shard["dir"]
                obs = np.load(p / "obs.npy", mmap_mode="r")
                acts, visits = np.load(p / "policy_actions.npy"), np.load(p / "policy_visits.npy")
                offsets, games = np.load(p / "policy_offsets.npy"), np.load(p / "game_id.npy")
                assert np.load(p / "terminated.npy").all()
                for i, row in enumerate(obs):
                    totals["rows"] += 1
                    phase = int(np.argmax(row[phase_slice]))
                    if phase not in (2, 3):
                        continue
                    a, v = acts[offsets[i]:offsets[i+1]], visits[offsets[i]:offsets[i+1]]
                    assert v.sum() > 0 and np.isfinite(row).all()
                    pi, best = v / v.sum(), int(np.argmax(v))
                    item = (str(p.relative_to(args.root)), i, int(games[i]), phase)
                    for route, mask in route_masks(a, phase).items():
                        if not mask.any():
                            continue
                        r = routes.setdefault(route, Counter())
                        r["opportunities"] += 1
                        r["positive_mass"] += int(pi[mask].sum() > 0)
                        r["mass_sum"] += float(pi[mask].sum())
                        r["argmax"] += int(mask[best])
                        offer(reservoir, seen, route + "/opportunity", item, args.sample, rng)
                        if mask[best]:
                            offer(reservoir, seen, route + "/teacher_argmax", item, args.sample, rng)
        assert totals["rows"] > 0, f"No completed rows found for generation {generation}"
        result["generations"][generation] = {"totals": totals, "all_rows": routes,
                                               "cohort_population": seen, "samples": {}}
        selected[generation] = reservoir
        print(json.dumps({"generation": generation, "totals": totals, "routes": routes}), flush=True)

    model_generations = sorted({g for g in args.generations} | {g-1 for g in args.generations})
    models = {}
    for generation in model_generations:
        path = args.root / f"generation/gen{generation}/model.zip"
        result["model_hashes"][generation] = digest(path)
        model = MaskablePPO.load(path, device="cpu")
        model.policy.set_training_mode(False)
        models[generation] = model
    for generation, cohorts in selected.items():
        for cohort, refs in cohorts.items():
            rows, legal, targets = [], [], []
            route = cohort.split("/")[0]
            for relative, i, gid, phase in refs:
                p = args.root / relative
                obs = np.load(p / "obs.npy", mmap_mode="r")
                offsets = np.load(p / "policy_offsets.npy", mmap_mode="r")
                a = np.load(p / "policy_actions.npy", mmap_mode="r")[offsets[i]:offsets[i+1]]
                v = np.load(p / "policy_visits.npy", mmap_mode="r")[offsets[i]:offsets[i+1]]
                rows.append(np.array(obs[i], copy=True))
                mask = np.zeros(digimon_engine.ACTION_SPACE_SIZE, dtype=bool); mask[a] = True
                legal.append(mask)
                rm = route_masks(a, phase)[route]
                targets.append((a.copy(), v / v.sum(), rm))
            obs, masks = np.asarray(rows), np.asarray(legal)
            out = {"n": len(refs), "rows": refs, "models": {},
                   "teacher_mass": sum(float(pi[m].sum()) for a, pi, m in targets)/len(refs),
                   "teacher_argmax": sum(int(m[np.argmax(pi)]) for a, pi, m in targets)}
            for mg, model in models.items():
                stats = Counter()
                with torch.inference_mode():
                    for start in range(0, len(rows), 64):
                        probs = model.policy.get_distribution(torch.as_tensor(obs[start:start+64]),
                            action_masks=masks[start:start+64]).distribution.probs.cpu().numpy()
                        for j, p in enumerate(probs):
                            a, pi, rm = targets[start+j]
                            chosen, teacher = int(p.argmax()), int(a[np.argmax(pi)])
                            stats["route_argmax"] += int(chosen in a[rm])
                            stats["route_mass_sum"] += float(p[a[rm]].sum())
                            stats["exact_teacher_argmax"] += int(chosen == teacher)
                            stats["cross_entropy_sum"] += float(-(pi * np.log(np.maximum(p[a], 1e-30))).sum())
                out["models"][mg] = dict(stats)
            result["generations"][generation]["samples"][cohort] = out
            print(json.dumps({"generation": generation, "cohort": cohort,
                              **{k:v for k,v in out.items() if k != "rows"}}), flush=True)
    args.out.write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
