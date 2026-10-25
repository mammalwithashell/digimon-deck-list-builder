"""Explicit one-generation continuation after hash-bound validation.

Uses the existing generation state and replay window; never overwrites the
original checkpoint or silently resumes partially collected games.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import time

from tools.selfplay_scheduler import atomic_json, sha256


def validate(root, plan):
    index = plan['generation_index']
    if not isinstance(index, int) or index < 1:
        raise ValueError('Continuation requires a positive generation index')
    state = json.loads((root / 'generation/state.json').read_text())
    previous = root / 'generation' / f'gen{index - 1}'
    if state != dict(next_gen=index, prev=dict(model_zip=str(previous / 'model.zip'),
                                              model_onnx=str(previous / 'model.onnx'))):
        raise ValueError('Generation state does not match approved parent')
    if (root / 'generation' / f'gen{index}').exists():
        raise ValueError('Next generation already exists; explicit recovery required')
    for name, digest in plan['files'].items():
        path = root / name
        if not path.resolve().is_relative_to(root.resolve()) or sha256(path) != digest:
            raise ValueError(f'Continuation hash mismatch: {name}')
    for name, digest in plan.get('runtime_files', {}).items():
        if sha256(Path(name)) != digest:
            raise ValueError(f'Runtime source hash mismatch: {name}')
    required = {f'generation/gen{index-1}/model.zip', f'generation/gen{index-1}/model.onnx',
                'inputs/decks.json', 'inputs/heldout_snapshot.json',
                'inputs/pet_evaluation_snapshot.json', 'inputs/anchors.json',
                plan['validation_report']}
    if not required <= set(plan['files']):
        raise ValueError('Continuation plan is missing required file hashes')
    report = json.loads((root / plan['validation_report']).read_text())
    if report.get('status') != 'passed' or report.get('image') != plan['image']:
        raise ValueError('Validation did not pass for this image')
    if report.get('checkpoint_sha256') != sha256(previous / 'model.zip'):
        raise ValueError('Validation belongs to a different checkpoint')
    if os.environ.get('DIGIMON_DEPLOYED_IMAGE') != plan['image']:
        raise ValueError('Runtime image differs from validated image')
    if plan['games'] != 3300 or plan['workers'] not in range(1, 9):
        raise ValueError('Continuation must retain approved sampling and CPU scope')
    return index


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path('/experiment'))
    parser.add_argument('--plan', type=Path, required=True)
    args = parser.parse_args()
    root, plan = args.root, json.loads(args.plan.read_text())
    index = validate(root, plan)
    marker = root / f'continuation-gen{index}-started.json'
    with marker.open('x') as out:
        json.dump(dict(time_unix=time.time(), plan=plan), out, indent=2)

    def stage(name, **extra):
        atomic_json(root / 'experiment_state.json', dict(stage=name, generation_index=index,
                    time_unix=time.time(), automatic_restart=False, image=plan['image'], **extra))

    inputs = root / 'inputs'
    env = dict(os.environ, DIGIMON_EVAL_FAILURE_DIR=str(root / f'eval-failures-gen{index}'))
    try:
        stage('generation', workers=plan['workers'], games=plan['games'])
        print(f'=== GENERATION_MONITOR_INDEX={index} ===', flush=True)
        command = [sys.executable, '-m', 'tools.run_selfplay_generation',
            '--run-dir', str(root / 'generation'), '--seed-model', str(inputs / 'seed.zip'),
            '--seed-model-onnx', str(inputs / 'seed.onnx'), '--decks', str(inputs / 'decks.json'),
            '--cli-bin', '/usr/local/bin/digimon-engine-cli', '--cards-json', '/app/data/cards.json',
            '--generations', '1', '--games', str(plan['games']), '--procs', str(plan['workers']),
            '--chunk-games', '4', '--worker-timeout-seconds', '7200',
            '--sims', '320', '--worlds', '4', '--eval-batch-size', '4', '--decision-cap', '1200',
            '--fail-on-truncation',
            '--window', '3', '--epochs', '2', '--batch-size', '512', '--lr', '0.0001',
            '--train-device', 'cpu', '--eval-n', '300', '--seed', '26261024',
            '--deck-pool-snapshot', str(inputs / 'heldout_snapshot.json'),
            '--champions', str(inputs / 'anchors.json')]
        subprocess.run(command, check=True, env=env)
        stage('pet_deck_evaluation')
        directory = root / 'generation' / f'gen{index}'
        with (directory / 'pet_anchored_eval.txt').open('w') as log:
            subprocess.run([sys.executable, '-m', 'tools.anchored_eval_cli',
                '--candidate', str(directory / 'model.zip'), '--champions', str(inputs / 'anchors.json'),
                '--n', '300', '--seed', '27261024', '--device', 'cpu', '--profile', 'standard_lite_v2',
                '--deck-pool-snapshot', str(inputs / 'pet_evaluation_snapshot.json')],
                stdout=log, stderr=subprocess.STDOUT, check=True, env=env)
        stage('complete', automatic_promotion=False)
    except BaseException as error:
        stage('failed', error=str(error))
        raise


if __name__ == '__main__':
    main()
