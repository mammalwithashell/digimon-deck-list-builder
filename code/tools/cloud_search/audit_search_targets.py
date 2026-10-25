"""Count normal evolution visit targets in frozen, non-quarantined search rows.

These are decision opportunities and target mass, not completed game events.
Phase and breeding-slot meanings are pinned to standard_lite_v2.3.
"""
import argparse
import hashlib
import json
from collections import Counter
from pathlib import Path

import numpy as np


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--shards', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    counts = Counter()
    routes = {k: Counter() for k in ('raising', 'battle', 'dna', 'promotion')}
    for folder in sorted(args.shards.glob('proc*')):
        manifest = json.loads((folder / 'manifest.json').read_text())
        if manifest.get('model_contract', {}).get('feature_schema_version') != 'standard_lite_v2.3':
            raise ValueError('Expected standard_lite_v2.3 model contract')
        exclusions = folder / 'training_exclusions.json'
        excluded = set()
        if exclusions.exists():
            data = json.loads(exclusions.read_text())
            if data['manifest_sha256'] != hashlib.sha256((folder / 'manifest.json').read_bytes()).hexdigest():
                raise ValueError('Exclusions belong to a different manifest')
            excluded = {r['game_id'] for r in data['games']}
        for shard in manifest['shards']:
            p = folder / shard['dir']
            obs = np.load(p / 'obs.npy', mmap_mode='r')
            ids = np.load(p / 'game_id.npy')
            offsets = np.load(p / 'policy_offsets.npy')
            actions = np.load(p / 'policy_actions.npy')
            visits = np.load(p / 'policy_visits.npy')
            for i, row in enumerate(obs):
                if int(ids[i]) in excluded:
                    counts['excluded_rows'] += 1
                    continue
                counts['rows'] += 1
                phase = int(np.argmax(row[8:40]))
                if phase not in (2, 3):
                    continue
                a, v = actions[offsets[i]:offsets[i+1]], visits[offsets[i]:offsets[i+1]]
                total = float(v.sum())
                if total == 0:
                    counts['zero_visit_rows'] += 1
                    continue
                evolution = (a >= 400) & (a < 1000) & (phase == 3)
                masks = dict(raising=evolution & ((a - 400) % 15 == 14),
                    battle=evolution & ((a - 400) % 15 != 14),
                    dna=(a >= 63) & (a < 93) & (phase == 3),
                    promotion=(a == 61) & (phase == 2))
                for name, mask in masks.items():
                    if not mask.any():
                        continue
                    r = routes[name]
                    mass = float(v[mask].sum()) / total
                    r['decision_opportunities'] += 1
                    r['positive_target_decisions'] += int(mass > 0)
                    r['target_probability_sum'] += mass
                    r['argmax_decisions'] += int(mask[np.argmax(v)])
    args.out.write_text(json.dumps(dict(counts=counts, routes=routes,
        scope='non-quarantined rows; normal Main/Breeding actions only; effect evolutions excluded'), indent=2))
    print(args.out.read_text(), flush=True)


if __name__ == '__main__':
    main()
