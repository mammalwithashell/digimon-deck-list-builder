"""Read-only DNA opportunity audit, restricted to the acting player's deck.

Uses printed DNA targets present in the frozen pool, never archetype names or
the word DNA in a card name. Main-phase decisions only: effect/counter DNA is
not inferred from reused selection IDs. Reports every eligible list, even zeroes.
"""
from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path

import numpy as np


def capable_lists(decks, cards):
    return {
        i: {cid: n for cid, n in Counter(d["cards"]).items()
            if cards[cid].get("dna_costs")}
        for i, d in enumerate(decks)
        if any(cards[cid].get("dna_costs") for cid in d["cards"])
    }


def acting_deck(manifest, game_id, mover):
    if int(mover) not in (0, 1):
        raise ValueError("Invalid mover")
    offset = int(game_id) - manifest["game_start"]
    if not 0 <= offset < manifest["games"]:
        raise ValueError("Game outside chunk")
    return manifest["game_decks"][offset][int(mover)]


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--root", type=Path, required=True)
    p.add_argument("--cards", type=Path, required=True)
    p.add_argument("--out", type=Path, required=True)
    args = p.parse_args()
    if args.out.exists():
        raise FileExistsError(args.out)
    import digimon_engine
    import torch
    from sb3_contrib import MaskablePPO
    from tools.selfplay_scheduler import _read_commit

    torch.set_num_threads(1)
    cards = json.loads(args.cards.read_text())
    pool_path = args.root / "inputs/decks.json"
    decks = json.loads(pool_path.read_text())
    eligible = capable_lists(decks, cards)
    layout = digimon_engine.get_observation_layout("standard_lite_v2")
    assert layout["feature_schema_version"] == "standard_lite_v2.3"
    phase_start = next(s["offset"] for s in layout["sections"]
                       if s["name"] == "global_features") + 8
    sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
    report = {"scope": "Acting-seat DNA-capable list cohorts; Main-phase non-forced rows only",
              "pool_sha256": sha(pool_path), "cards_sha256": sha(args.cards),
              "layout_hash": layout["layout_hash"], "lists": [],
              "model_sha256": {}, "generations": {}}
    for i, d in enumerate(decks):
        report["lists"].append({"index": i, "name": d["name"],
                                "dna_targets": eligible.get(i, {}),
                                "target_names": {c: cards[c]["card_name_eng"]
                                                 for c in eligible.get(i, {})}})
    models = {}
    for g in range(3):
        path = args.root / f"generation/gen{g}/model.zip"
        report["model_sha256"][g] = sha(path)
        models[g] = MaskablePPO.load(path, device="cpu")
        models[g].policy.set_training_mode(False)

    for generation in (1, 2):
        stats = {i: Counter(appearances=0, rows=0, main_rows=0,
                            opportunities=0, teacher_argmax=0, teacher_mass_sum=0)
                 for i in eligible}
        appearance_sets = {i: set() for i in eligible}
        positive_sets = {i: set() for i in eligible}
        rows, masks, targets, refs = [], [], [], []
        totals = Counter()
        for folder in sorted((args.root / f"generation/gen{generation}/shards").glob("proc*")):
            m = json.loads((folder / "manifest.json").read_text())
            _read_commit(folder, m["game_start"], m["games"], m["seed"])
            assert m["truncated_games"] == 0
            assert m["deck_names"] == [d["name"] for d in decks]
            assert m["model_contract"]["tensor_layout_hash"] == layout["layout_hash"]
            assert m["model_contract"]["action_schema_version"] == 1
            for pair in m["game_decks"]:
                totals["games"] += 1
                totals["games_with_capable_seat"] += int(any(i in eligible for i in pair))
                for i in pair:
                    if i in eligible:
                        stats[i]["appearances"] += 1
            for shard in m["shards"]:
                s = folder / shard["dir"]
                obs = np.load(s / "obs.npy", mmap_mode="r")
                gids, movers = np.load(s / "game_id.npy"), np.load(s / "mover.npy")
                offsets = np.load(s / "policy_offsets.npy")
                actions, visits = np.load(s / "policy_actions.npy"), np.load(s / "policy_visits.npy")
                assert np.load(s / "terminated.npy").all()
                for j, (gid, mover) in enumerate(zip(gids, movers)):
                    totals["all_rows"] += 1
                    di = acting_deck(m, gid, mover)
                    phase = int(np.argmax(obs[j, phase_start:phase_start+32]))
                    a = actions[offsets[j]:offsets[j+1]]
                    dna = (a >= 63) & (a < 93) & (phase == 3)
                    if di not in eligible:
                        assert not dna.any(), f"Unclassified legal DNA in list {di}"
                        continue
                    stats[di]["rows"] += 1
                    stats[di]["main_rows"] += int(phase == 3)
                    if not dna.any():
                        continue
                    v = visits[offsets[j]:offsets[j+1]]
                    assert v.sum() > 0
                    pi = v / v.sum()
                    preferred = bool(dna[np.argmax(v)])
                    stats[di]["opportunities"] += 1
                    stats[di]["teacher_argmax"] += int(preferred)
                    stats[di]["teacher_mass_sum"] += float(pi[dna].sum())
                    appearance_sets[di].add((int(gid), int(mover)))
                    if preferred:
                        positive_sets[di].add((int(gid), int(mover)))
                    rows.append(np.array(obs[j], copy=True))
                    mask = np.zeros(digimon_engine.ACTION_SPACE_SIZE, dtype=bool)
                    mask[a] = True
                    masks.append(mask)
                    targets.append((a.copy(), dna, preferred))
                    refs.append({"list": di, "game": int(gid), "mover": int(mover),
                                 "shard": str(s.relative_to(args.root)), "row": j})
        model_stats = {i: {g: Counter() for g in models} for i in eligible}
        for g, model in models.items():
            with torch.inference_mode():
                for start in range(0, len(rows), 64):
                    probs = model.policy.get_distribution(
                        torch.as_tensor(np.asarray(rows[start:start+64])),
                        action_masks=np.asarray(masks[start:start+64])
                    ).distribution.probs.cpu().numpy()
                    for j, pr in enumerate(probs):
                        a, dna, preferred = targets[start+j]
                        st = model_stats[refs[start+j]["list"]][g]
                        chose = int(int(pr.argmax()) in a[dna])
                        st["route_argmax"] += chose
                        st["route_mass_sum"] += float(pr[a[dna]].sum())
                        if preferred:
                            st["teacher_positive_route_argmax"] += chose
                            st["teacher_positive_route_mass_sum"] += float(pr[a[dna]].sum())
        for i in eligible:
            stats[i]["appearances_with_opportunity"] = len(appearance_sets[i])
            stats[i]["appearances_with_teacher_preference"] = len(positive_sets[i])
        report["generations"][generation] = {
            "totals": totals, "lists": stats, "models_by_list": model_stats,
            "opportunity_refs": refs}
        print(json.dumps({"generation": generation, "totals": totals, "lists": stats}), flush=True)
    args.out.write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
