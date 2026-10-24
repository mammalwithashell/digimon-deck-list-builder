"""Paired cache diagnostic on the frozen cloud engine; never training data."""
import hashlib
import json
import os
from pathlib import Path
import resource
import subprocess
import time

import numpy as np

ROOT = Path('/experiment')
OUT = Path('/diagnostic/eval-cache')


def main():
    OUT.mkdir(parents=True, exist_ok=False)
    report = dict(diagnostic_only=True, simulations=320, worlds=4, batch_size=4,
                  decision_cap=40, cases=[], comparisons=[],
                  model_sha256=hashlib.sha256((ROOT / 'inputs/seed.onnx').read_bytes()).hexdigest())
    def save():
        tmp = OUT / 'benchmark.tmp'
        tmp.write_text(json.dumps(report, indent=2))
        tmp.replace(OUT / 'benchmark.json')
    for seed in [24261024, 24261025]:
        for repeat in range(2):
            cases = {}
            capacities = [0, 1024, 8192] if repeat == 0 else [8192, 1024, 0]
            for capacity in capacities:
                name = f'seed-{seed}-repeat-{repeat}-cache-{capacity}'
                target = OUT / name
                command = ['digimon-engine-cli', '--cards-json', '/app/data/cards.json', '--pool', 'implemented',
                    'selfplay', '--model', str(ROOT / 'inputs/seed.onnx'), '--decks', str(ROOT / 'inputs/decks.json'),
                    '--games', '1', '--seed', str(seed), '--sims', '320', '--worlds', '4',
                    '--eval-batch-size', '4', '--temperature-decisions', '16', '--decision-cap', '40',
                    '--tensor-profile', 'standard_lite_v2', '--out', str(target)]
                env = dict(os.environ, DIGIMON_SEARCH_EVAL_CACHE_ENTRIES=str(capacity))
                env.pop('DIGIMON_SELFPLAY_TRACE_DECISIONS', None)
                start = time.monotonic()
                before = resource.getrusage(resource.RUSAGE_CHILDREN)
                with (OUT / (name + '.log')).open('w') as log:
                    subprocess.run(command, env=env, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=1200)
                after = resource.getrusage(resource.RUSAGE_CHILDREN)
                manifest = json.loads((target / 'manifest.json').read_text())
                progress = json.loads((target / 'progress.json').read_text())
                case = dict(seed=seed, repeat=repeat, capacity=capacity, directory=str(target),
                    rows=manifest['rows'], process_wall_seconds=time.monotonic()-start,
                    cpu_seconds=after.ru_utime+after.ru_stime-before.ru_utime-before.ru_stime,
                    native_seconds=progress['elapsed_seconds'], game_decks=manifest['game_decks'])
                cases[capacity] = case
                report['cases'].append(case)
                save()
                print(json.dumps(case), flush=True)
            for capacity in [1024, 8192]:
                base, cached = cases[0], cases[capacity]
                left, right = Path(base['directory']) / 'shard-000', Path(cached['directory']) / 'shard-000'
                names = {p.name for p in left.glob('*.npy')}
                assert names == {p.name for p in right.glob('*.npy')}
                checks = {name: bool(np.array_equal(np.load(left/name), np.load(right/name))) for name in sorted(names)}
                comparison = dict(seed=seed, repeat=repeat, capacity=capacity, arrays_equal=checks,
                    exact_parity=bool(checks) and all(checks.values()),
                    cpu_speedup=base['cpu_seconds']/cached['cpu_seconds'],
                    wall_speedup=base['native_seconds']/cached['native_seconds'])
                report['comparisons'].append(comparison)
                save()
                print(json.dumps(comparison), flush=True)
                if not comparison['exact_parity']:
                    raise RuntimeError('Cache changed saved targets; do not enable in production')
    report['status'] = 'complete'
    save()


if __name__ == '__main__':
    main()
