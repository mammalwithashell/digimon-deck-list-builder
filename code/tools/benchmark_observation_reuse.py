"""Paired native observation benchmark. Outputs are diagnostics, never training data."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

import numpy as np


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline', type=Path, required=True)
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--model', type=Path, required=True)
    parser.add_argument('--decks', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--seeds', type=int, nargs='+', default=[24261024, 24261025])
    parser.add_argument('--repeats', type=int, default=2)
    parser.add_argument('--games', type=int, default=1)
    parser.add_argument('--decision-cap', type=int, default=40)
    parser.add_argument('--hash-leaves', action='store_true')
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=False)
    report = dict(diagnostic_only=True, simulations=320, worlds=4, batch_size=4,
                  cache_entries=0, settings={k: str(v) if isinstance(v, Path) else v
                                           for k, v in vars(args).items()},
                  hashes={k: sha(getattr(args, k)) for k in ['baseline', 'candidate', 'model', 'decks']},
                  cards_sha256=sha(Path('data/cards.json')), cases=[], comparisons=[])

    def save() -> None:
        temp = args.out / 'benchmark.tmp'
        temp.write_text(json.dumps(report, indent=2))
        temp.replace(args.out / 'benchmark.json')

    for seed in args.seeds:
        for repeat in range(args.repeats):
            cases = {}
            order = ['baseline', 'candidate'] if repeat % 2 == 0 else ['candidate', 'baseline']
            for kind in order:
                name = f'{seed}-{repeat}-{kind}'
                target = args.out / name
                env = dict(os.environ, OMP_NUM_THREADS='1', MKL_NUM_THREADS='1',
                           OPENBLAS_NUM_THREADS='1', DIGIMON_SEARCH_PROFILE='1')
                env.pop('DIGIMON_OBSERVATION_PARITY', None)
                if args.hash_leaves:
                    env['DIGIMON_OBSERVATION_PARITY'] = '1'
                command = [str(getattr(args, kind).resolve()), str(args.model), str(args.decks),
                           str(target), str(seed), str(args.games), str(args.decision_cap), '0', '4']
                started = time.monotonic()
                with (args.out / f'{name}.log').open('w') as log:
                    subprocess.run(command, env=env, stdout=log, stderr=subprocess.STDOUT,
                                   check=True, timeout=3600)
                case = json.loads((target / 'performance.json').read_text())
                manifest = json.loads((target / 'manifest.json').read_text())
                case.update(kind=kind, repeat=repeat, directory=str(target),
                            process_wall_seconds=time.monotonic() - started,
                            profiling=manifest.get('profiling'))
                cases[kind] = case
                report['cases'].append(case)
                save()
                print(json.dumps({k: case[k] for k in ['kind', 'seed', 'repeat', 'rows',
                                                       'elapsed_seconds_including_registry']}), flush=True)
            left, right = (Path(cases[k]['directory']) for k in ['baseline', 'candidate'])
            names = {p.relative_to(left) for p in left.rglob('*.npy')}
            assert names and names == {p.relative_to(right) for p in right.rglob('*.npy')}
            checks = {}
            for name in sorted(names):
                a, b = np.load(left / name), np.load(right / name)
                checks[str(name)] = a.shape == b.shape and a.dtype == b.dtype and a.tobytes() == b.tobytes()
            leaf_equal = (cases['baseline']['leaf_input_sha256'] == cases['candidate']['leaf_input_sha256']) if args.hash_leaves else None
            result = dict(seed=seed, repeat=repeat, arrays_equal=checks, leaf_inputs_equal=leaf_equal,
                          throughput_ratio=cases['baseline']['elapsed_seconds_including_registry'] /
                                           cases['candidate']['elapsed_seconds_including_registry'])
            report['comparisons'].append(result)
            save()
            if not all(checks.values()) or leaf_equal is False:
                raise RuntimeError('Observation optimization changed search data')
    totals = {k: sum(c['elapsed_seconds_including_registry'] for c in report['cases'] if c['kind'] == k)
              for k in ['baseline', 'candidate']}
    report.update(status='complete', total_native_seconds=totals,
                  aggregate_throughput_ratio=totals['baseline'] / totals['candidate'])
    save()


if __name__ == '__main__':
    main()
