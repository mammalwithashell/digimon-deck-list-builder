"""Host-side paired search benchmark of pinned Linux images, with exact shards.

Requires Docker. Uses read-only experiment inputs and isolated diagnostic
outputs. Baseline and candidate execute sequentially with the same CPU quota.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline-image', required=True)
    parser.add_argument('--candidate-image', required=True)
    parser.add_argument('--experiment', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    if any('@sha256:' not in value for value in (args.baseline_image, args.candidate_image)):
        raise ValueError('Use digest-pinned images')
    args.out.mkdir(parents=True, exist_ok=False)
    report = dict(status='running', simulations=320, worlds=4, batch_size=4,
                  decision_cap=40, cpu_quota=1, cases=[], comparisons=[],
                  baseline_image=args.baseline_image, candidate_image=args.candidate_image,
                  model_sha256=hashlib.sha256((args.experiment/'generation/gen0/model.onnx').read_bytes()).hexdigest())

    def save():
        temp = args.out / 'benchmark.tmp'
        temp.write_text(json.dumps(report, indent=2))
        temp.replace(args.out / 'benchmark.json')

    for seed in (24261024, 24261025):
        for repeat in range(2):
            cases = {}
            for kind in (['baseline', 'candidate'] if repeat == 0 else ['candidate', 'baseline']):
                name = f'{seed}-{repeat}-{kind}'
                start = time.monotonic()
                command = ['docker', 'run', '--rm', '--network=none', '--cpus=1', '--memory=4g',
                    '-v', f'{args.experiment.resolve()}:/experiment:ro', '-v', f'{args.out.resolve()}:/benchmark',
                    '-e', 'DIGIMON_SEARCH_PROFILE=1', '-e', 'DIGIMON_SEARCH_EVAL_CACHE_ENTRIES=0',
                    getattr(args, f'{kind}_image'), 'digimon-engine-cli', '--cards-json', '/app/data/cards.json',
                    '--pool', 'implemented', 'selfplay', '--model', '/experiment/generation/gen0/model.onnx',
                    '--decks', '/experiment/inputs/decks.json', '--games', '1', '--seed', str(seed),
                    '--sims', '320', '--worlds', '4', '--eval-batch-size', '4', '--temperature-decisions', '16',
                    '--decision-cap', '40', '--tensor-profile', 'standard_lite_v2', '--out', f'/benchmark/{name}']
                with (args.out / f'{name}.log').open('w') as log:
                    subprocess.run(command, check=True, timeout=1800, stdout=log, stderr=subprocess.STDOUT)
                directory = args.out / name
                manifest = json.loads((directory / 'manifest.json').read_text())
                progress = json.loads((directory / 'progress.json').read_text())
                hashes = {str(p.relative_to(directory)): hashlib.sha256(p.read_bytes()).hexdigest()
                          for p in sorted(directory.glob('shard-*/*.npy'))}
                if len(hashes) != 8:
                    raise ValueError('Expected all eight arrays for this bounded single shard')
                case = dict(kind=kind, seed=seed, repeat=repeat, rows=manifest['rows'],
                            native_seconds=progress['elapsed_seconds'], wall_seconds=time.monotonic()-start,
                            array_hashes=hashes, profiling=manifest.get('profiling'))
                cases[kind] = case
                report['cases'].append(case)
                save()
            parity = cases['baseline']['array_hashes'] == cases['candidate']['array_hashes']
            report['comparisons'].append(dict(seed=seed, repeat=repeat, exact_array_parity=parity,
                speedup=cases['baseline']['native_seconds']/cases['candidate']['native_seconds']))
            save()
            if not parity:
                raise RuntimeError('Image behavior differs in this sample; investigate before attributing speedup')
    totals = {kind:sum(c['native_seconds'] for c in report['cases'] if c['kind']==kind)
              for kind in ['baseline','candidate']}
    report.update(status='passed', exact_array_parity=True, aggregate_speedup=totals['baseline']/totals['candidate'])
    save()
    print(json.dumps({k:v for k,v in report.items() if k not in ['cases','comparisons']}), flush=True)


if __name__ == '__main__':
    main()
