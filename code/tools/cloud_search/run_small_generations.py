"""Four bounded search updates with one fixed-cohort evaluation per model.

Starts from an explicitly prepared, hash-bound checkpoint and archive. No
automatic restart, promotion, partial-run adoption or source-data mutation.
"""
import argparse
from collections import Counter
import json
import os
from pathlib import Path
import subprocess
import sys
import time
from types import SimpleNamespace

from tools.cloud_search.card_eval_worker import evaluate_window
from tools.selfplay_scheduler import atomic_json, controller_lock, sha256


def validate(root, plan):
    if plan['generations'] != 4 or plan['games'] < 1000 or plan['workers'] not in range(1, 9):
        raise ValueError('Unexpected campaign scope')
    if os.environ.get('DIGIMON_DEPLOYED_IMAGE') != plan['image']:
        raise ValueError('Image differs from approved engine')
    required = {'inputs/decks.json', 'inputs/seed.zip', 'inputs/seed.onnx',
                'inputs/heldout_snapshot.json', 'inputs/pet_evaluation_snapshot.json',
                'inputs/anchors.json', 'generation/gen0/model.zip', 'generation/gen0/model.onnx',
                'validation/generation0-recovery.json'}
    if not required <= set(plan['files']):
        raise ValueError('Missing required input hashes')
    for name, digest in plan['files'].items():
        path = root / name
        if not path.resolve().is_relative_to(root.resolve()) or sha256(path) != digest:
            raise ValueError(f'Input hash mismatch: {name}')
    for name, digest in plan['runtime_files'].items():
        if sha256(Path(name)) != digest:
            raise ValueError(f'Runtime hash mismatch: {name}')
    report = json.loads((root / 'validation/generation0-recovery.json').read_text())
    if (report.get('status') != 'passed' or report.get('image') != plan['image']
            or report.get('checkpoint_sha256') != sha256(root / 'generation/gen0/model.zip')):
        raise ValueError('Parent checkpoint validation differs')
    slots = Counter(d['name'].split(':sha256:', 1)[0] for d in json.loads((root / 'inputs/decks.json').read_text()))
    if len(set(slots.values())) != 1 or plan['games'] * 2 / len(slots) < 150:
        raise ValueError('Pool must have equal archetype weight and >=150 expected appearances')
    state = json.loads((root / 'generation/state.json').read_text())
    if state != dict(next_gen=1, prev=dict(model_zip=str(root / 'generation/gen0/model.zip'),
                                          model_onnx=str(root / 'generation/gen0/model.onnx'))):
        raise ValueError('Expected prepared parent state')
    if any((root / 'generation' / f'gen{i}').exists() for i in range(1, 5)):
        raise ValueError('Existing campaign output; explicit recovery required')
    return slots


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path('/experiment'))
    parser.add_argument('--evaluation-root', type=Path, default=Path('/card-evaluation'))
    parser.add_argument('--validate-only', action='store_true')
    args = parser.parse_args()
    root, out = args.root, args.evaluation_root
    plan = json.loads((root / 'campaign-plan.json').read_text())
    validate(root, plan)
    if args.validate_only:
        print('Campaign inputs and runtime hashes verified', flush=True)
        return
    with controller_lock(root / 'campaign.lock'):
        with (root / 'campaign-started.json').open('x') as f:
            json.dump(dict(time_unix=time.time(), plan=plan), f, indent=2)
        out.mkdir(parents=True, exist_ok=True)

        def stage(name, index, **extra):
            atomic_json(root / 'experiment_state.json', dict(stage=name, generation_index=index,
                time_unix=time.time(), image=plan['image'], automatic_restart=False,
                automatic_promotion=False, **extra))

        def evaluate(index):
            model = root / 'generation' / f'gen{index}' / 'model.zip'
            checkpoint = dict(path=str(model), sha256=sha256(model), step=index)
            atomic_json(out / 'status.json', dict(status='running', checkpoint=checkpoint, window=index))
            options = SimpleNamespace(panels=[('meta_validation', 'heldout_snapshot.json'),
                ('pet_same_list', 'pet_evaluation_snapshot.json')], games=200,
                games_per_archetype=200, seed=28261025, profile='standard_lite_v2', timeout=10800)
            result = evaluate_window(root, out, checkpoint, index, 'new_checkpoint', options)
            directory = root / 'generation' / f'gen{index}'
            # Keep monitor-compatible scalar summaries alongside full card reports.
            import shutil
            for panel, target in [('meta_validation', 'anchored_eval.txt'), ('pet_same_list', 'pet_anchored_eval.txt')]:
                shutil.copyfile(out / f'window-{index:06d}' / f'{panel}.log', directory / target)
            atomic_json(out / 'status.json', dict(result, worker_status='waiting'))
            if result['status'] != 'complete':
                raise RuntimeError(f'Checkpoint evaluation failed: {result}')

        index = 0
        try:
            stage('checkpoint_evaluation', 0)
            evaluate(0)  # once on the same fixed cohort as all subsequent models
            for index in range(1, plan['generations'] + 1):
                stage('generation', index, games=plan['games'], workers=plan['workers'])
                print(f'=== GENERATION_MONITOR_INDEX={index} ===', flush=True)
                subprocess.run([sys.executable, '-m', 'tools.run_selfplay_generation',
                    '--run-dir', str(root / 'generation'), '--seed-model', str(root / 'inputs/seed.zip'),
                    '--seed-model-onnx', str(root / 'inputs/seed.onnx'), '--decks', str(root / 'inputs/decks.json'),
                    '--cli-bin', '/usr/local/bin/digimon-engine-cli', '--cards-json', '/app/data/cards.json',
                    '--generations', '1', '--games', str(plan['games']), '--procs', str(plan['workers']),
                    '--chunk-games', '4', '--worker-timeout-seconds', '7200', '--sims', '320',
                    '--worlds', '4', '--eval-batch-size', '4', '--decision-cap', '1200',
                    '--fail-on-truncation', '--window', '3', '--replay-games-per-generation', str(plan['games']),
                    '--epochs', '2', '--batch-size', '512', '--lr', '0.0001', '--train-device', 'cpu',
                    '--seed', str(plan['seed']), '--skip-eval'], check=True)
                stage('checkpoint_evaluation', index)
                evaluate(index)
            stage('complete', index)
            atomic_json(out / 'status.json', dict(status='stopped', pipeline='complete'))
        except BaseException as error:
            stage('failed', index, error=str(error))
            atomic_json(out / 'status.json', dict(status='stopped', pipeline='failed',
                        last_result=dict(status='failed', error=str(error))))
            raise


if __name__ == '__main__':
    main()
