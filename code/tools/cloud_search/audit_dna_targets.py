"""Attribute existing DNA-cohort audit rows to the target card in hand.

No game reconstruction: these counts do not establish which material recipe
made an action legal or whether a DNA action was ultimately selected/resolved.
"""
import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path

import numpy as np


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--root", type=Path, required=True)
    p.add_argument("--audit", type=Path, required=True)
    p.add_argument("--cards", type=Path, required=True)
    p.add_argument("--out", type=Path, required=True)
    args = p.parse_args()
    if args.out.exists():
        raise FileExistsError(args.out)
    import digimon_engine
    from tools.selfplay_scheduler import _read_commit
    source = json.loads(args.audit.read_text())
    assert hashlib.sha256(args.cards.read_bytes()).hexdigest() == source["cards_sha256"]
    cards = json.loads(args.cards.read_text())
    ids = {c["index"]: cid for cid, c in cards.items()}
    layout = digimon_engine.get_observation_layout("standard_lite_v2")
    assert layout["feature_schema_version"] == "standard_lite_v2.3"
    assert layout["layout_hash"] == source["layout_hash"]
    hand = next(s for s in layout["sections"] if s["name"] == "own_hand")
    positions = sorted(i for i in layout["card_id_positions"]
                       if hand["offset"] <= i < hand["offset"] + hand["size"])
    assert len(positions) == 30
    result = {"source_audit_sha256": hashlib.sha256(args.audit.read_bytes()).hexdigest(),
              "scope": __doc__, "generations": {}}
    checked = set()
    for g, gen in source["generations"].items():
        counts = defaultdict(Counter)
        for ref in gen["opportunity_refs"]:
            s = args.root / ref["shard"]
            if s.parent not in checked:
                m = json.loads((s.parent / "manifest.json").read_text())
                _read_commit(s.parent, m["game_start"], m["games"], m["seed"])
                checked.add(s.parent)
            j = ref["row"]
            obs = np.load(s / "obs.npy", mmap_mode="r")[j]
            off = np.load(s / "policy_offsets.npy", mmap_mode="r")
            a = np.load(s / "policy_actions.npy", mmap_mode="r")[off[j]:off[j+1]]
            v = np.load(s / "policy_visits.npy", mmap_mode="r")[off[j]:off[j+1]]
            best = int(a[np.argmax(v)])
            target_actions = defaultdict(list)
            for k, action in enumerate(a):
                if 63 <= action < 93:
                    cid = ids[int(obs[positions[int(action)-63]])]
                    assert cid in source["lists"][ref["list"]]["dna_targets"]
                    target_actions[cid].append(k)
            for cid, indices in target_actions.items():
                counts[cid]["opportunities"] += 1
                counts[cid]["teacher_argmax"] += int(best in a[indices])
                counts[cid]["teacher_mass_sum"] += float(v[indices].sum() / v.sum())
        result["generations"][g] = counts
    args.out.write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
