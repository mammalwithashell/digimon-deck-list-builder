"""Prepare a fresh engine-repair run from fitted CP1 weights, excluding old replay.

Mount the failed campaign read-only at /archive. Preparation never starts training
or promotes a model; a failed preparation is preserved and cannot be adopted.
"""
from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor
import copy
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import subprocess
import time

from tools.cloud_search.prepare_checkpoint_continuation import (
    PROFILE, PROTOCOL, POOL_FILES, RUNTIME_FILES as SHARED_RUNTIME_FILES,
    copy_verified, read, summarize_pool, validate_runtime,
)
from tools.selfplay_scheduler import atomic_json, sha256

PANELS = [['combined_diagnostic', 'combined_diagnostic_snapshot.json'],
          ['original_pet_regression', 'pet_evaluation_snapshot.json']]
ANCHOR_FILES = {'parent-checkpoint4': 'seed.zip', 'development-fixed': 'development.zip'}
RUNTIME_FILES = (*SHARED_RUNTIME_FILES, 'data_paths.py',
                 'tools/cloud_search/prepare_payment_recovery.py',
                 'tools/cloud_search/resume_payment_recovery.py')
SEARCH = dict(simulations=320, worlds=4, evaluation_batch_size=4, decision_cap=1200)
FIT = dict(epochs=2, batch_size=512, learning_rate=0.0001, replay_window=3)


def pinned_path(root, name):
    relative = PurePosixPath(name)
    path = (root / name).resolve()
    if relative.is_absolute() or '\\' in name or '..' in relative.parts or not path.is_relative_to(root.resolve()):
        raise ValueError(f'Artifact path escapes its root: {name}')
    return path


def anchors_for_root(original, root):
    """Relocate paths only; keep original anchor names, metadata and weights."""
    anchors = copy.deepcopy(original)
    champions = anchors.get('champions', [])
    if len(champions) != 2 or {c['name'] for c in champions} != set(ANCHOR_FILES):
        raise ValueError('Original frozen anchors must be parent-checkpoint4 and development-fixed')
    for champion in champions:
        filename = ANCHOR_FILES[champion['name']]
        if PurePosixPath(champion['weights_path']).name != filename:
            raise ValueError('Original anchor path substitutes another checkpoint')
        champion['weights_path'] = str(root / 'inputs' / filename)
    return anchors


def validate_models(parent, inputs, cards_json):
    runtime = validate_runtime(parent, inputs / 'development.zip', inputs, cards_json)
    from model_contract import validate_checkpoint
    from sb3_contrib import MaskablePPO
    if validate_checkpoint(MaskablePPO.load(inputs / 'seed.zip', device='cpu'), PROFILE) != runtime['contract']:
        raise ValueError('Original parent-checkpoint4 anchor contract mismatch')
    return runtime


def run_health(args, root):
    """Eight native games from fitted CP1 at the unchanged production budget."""
    validation = root / 'validation'
    validation.mkdir(parents=True, exist_ok=True)
    def health(start):
        out = validation / f'health-{start}'
        command = [str(args.cli_bin), '--pool', 'implemented', '--cards-json', str(args.cards_json),
                   'selfplay', '--model', str(root / 'generation/gen1/model.onnx'),
                   '--decks', str(root / 'inputs/decks.json'), '--games', '4',
                   '--game-start', str(start), '--seed', str(20261027 + 10000000),
                   '--sims', '320', '--worlds', '4', '--eval-batch-size', '4',
                   '--temperature-decisions', '16', '--decision-cap', '1200',
                   '--tensor-profile', PROFILE, '--out', str(out)]
        with (validation / f'health-{start}.log').open('x', encoding='utf-8') as log:
            subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=1800)
        manifest = read(out / 'manifest.json')
        if manifest['games'] != 4 or manifest['truncated_games'] or manifest['rows'] <= 0:
            raise ValueError('Native full-budget health check failed')
        return dict(start=start, games=4, rows=manifest['rows'], truncated_games=0)
    with ThreadPoolExecutor(max_workers=2) as executor:
        return list(executor.map(health, (0, 4)))


def inspect_source(args):
    source = args.source.resolve()
    original = read(source / 'campaign-plan.json')
    for name in ('campaign-plan.json', 'deployment-plan.json'):
        if sha256(source / name) != args.source_plan_sha256:
            raise ValueError('Archived source-plan hash differs')
    state = read(source / 'experiment_state.json')
    if state.get('stage') != 'failed' or state.get('generation_index') != 1:
        raise ValueError('Recovery requires the stopped, failed CP1 evaluation')
    expected = dict(generations=4, games=1100, workers=8, seed=20261027,
                    evaluation_seed=22261027, evaluation_protocol=PROTOCOL,
                    evaluation_panels=PANELS, evaluation_games_per_archetype_per_anchor=200,
                    search=SEARCH, replay_window=3, replay_cap_games=1100)
    if any(original.get(k) != v for k, v in expected.items()):
        raise ValueError('Source campaign scope or fixed evaluation cohort differs')
    generation_state = read(source / 'generation/state.json')
    if generation_state.get('next_gen') != 2 or any(
        not generation_state.get('prev', {}).get(key, '').endswith(f'/generation/gen1/model{suffix}')
        for key, suffix in (('model_zip', '.zip'), ('model_onnx', '.onnx'))
    ) or any((source / f'generation/gen{i}').exists() for i in range(2, 5)):
        raise ValueError('Expected fitted CP1 and no later generation')
    if read(source / 'evaluations/window-000001/result.json').get('status') != 'failed':
        raise ValueError('Expected archived failed CP1 evaluation evidence')
    parent = source / 'generation/gen1'
    for suffix, digest in (('.zip', args.checkpoint_sha256), ('.onnx', args.onnx_sha256),
                           ('.onnx.meta.json', args.onnx_metadata_sha256)):
        if not re.fullmatch('[0-9a-f]{64}', digest) or sha256(parent / f'model{suffix}') != digest:
            raise ValueError(f'Approved checkpoint artifact differs: {suffix}')
    source_hashes = {}
    for name, digest in original['files'].items():
        if name.startswith('inputs/') or name == 'corpora/pilot/provenance.json':
            if sha256(pinned_path(source, name)) != digest:
                raise ValueError(f'Frozen source input differs: {name}')
            source_hashes[name] = digest
    required = {f'inputs/{name}' for name in (*POOL_FILES, 'anchors.json', 'seed.zip', 'seed.onnx',
                'seed.onnx.meta.json', 'development.zip', 'pet_evaluation_snapshot.json')}
    if not required <= source_hashes.keys():
        raise ValueError('Missing hash-bound original pool or anchor inputs')
    manifests = sorted(parent.glob('shards/proc*/manifest.json'))
    counts = [read(path) for path in manifests]
    collected = dict(games=sum(m['games'] for m in counts), rows=sum(m['rows'] for m in counts),
                     truncated_games=sum(m['truncated_games'] for m in counts))
    if collected != dict(games=1100, rows=105563, truncated_games=0):
        raise ValueError('Saved CP1 collection evidence differs from the investigated run')
    names = ['campaign-plan.json', 'deployment-plan.json', 'experiment_state.json', 'started.json',
             'generation/state.json', 'generation/gen1/summary.json', 'generation/gen1/scheduler_progress.json',
             'evaluations/window-000001/result.json', 'corpora/pilot/provenance.json',
             *[f'generation/gen1/model{s}' for s in ('.zip', '.onnx', '.onnx.meta.json')],
             *[p.relative_to(source).as_posix() for p in manifests]]
    source_hashes.update({name: sha256(pinned_path(source, name)) for name in names})
    anchors_for_root(read(source / 'inputs/anchors.json'), args.root.resolve())
    return original, state, source_hashes, collected


def prepare(args):
    source, root = args.source.resolve(), args.root.resolve()
    if root == source or root.is_relative_to(source) or source.is_relative_to(root):
        raise ValueError('Source and destination must not overlap')
    if root.exists() and (not root.is_dir() or any(root.iterdir())):
        raise ValueError('Preparation destination must be empty')
    image = os.environ.get('DIGIMON_DEPLOYED_IMAGE', '')
    if not re.fullmatch(r'.+@sha256:[0-9a-f]{64}', image):
        raise ValueError('An immutable repaired image digest is required')
    if not re.fullmatch('[0-9a-f]{40}', args.source_commit):
        raise ValueError('An exact image source commit is required')
    original, state, source_hashes, collected = inspect_source(args)
    if image == original['image']:
        raise ValueError('Recovery requires a new repaired engine image')
    provenance = read(source / 'corpora/pilot/provenance.json')
    if sha256(args.cards_json) != provenance['cards_sha256']:
        raise ValueError('Runtime card data changed across recovery')
    denominators = summarize_pool(source / 'inputs', source / 'inputs/pet_evaluation_snapshot.json')
    if denominators != original['pool_denominators']:
        raise ValueError('Original EX13 pool denominators changed')
    runtime = validate_models(source / 'generation/gen1', source / 'inputs', args.cards_json)
    cli_bin = Path(shutil.which(str(args.cli_bin)) or str(args.cli_bin)).resolve()
    runtime_hashes = {str((args.code_root / name).resolve()): sha256(args.code_root / name)
                      for name in RUNTIME_FILES}
    runtime_hashes.update({str(args.cards_json.resolve()): sha256(args.cards_json), str(cli_bin): sha256(cli_bin)})
    sources = {name: source / name for name in source_hashes if name.startswith('inputs/')}
    sources.update({f'generation/gen1/model{s}': source / f'generation/gen1/model{s}'
                    for s in ('.zip', '.onnx', '.onnx.meta.json')})
    for name in ('campaign-plan.json', 'experiment_state.json', 'started.json', 'generation/state.json',
                 'generation/gen1/summary.json', 'generation/gen1/scheduler_progress.json',
                 'evaluations/window-000001/result.json', 'corpora/pilot/provenance.json'):
        sources[f'lineage/source/{name}'] = source / name
    sources['lineage/source/inputs/anchors.json'] = source / 'inputs/anchors.json'
    root.mkdir(parents=True, exist_ok=True)
    with (root / 'preparation-started.json').open('x', encoding='utf-8') as marker:
        json.dump(dict(time_unix=time.time(), image=image, source_plan_sha256=args.source_plan_sha256), marker)
    for name, path in sources.items():
        copy_verified(path, root / name, source_hashes[path.relative_to(source).as_posix()])
    anchors = anchors_for_root(read(source / 'inputs/anchors.json'), root)
    if read(root / 'inputs/anchors.json') != anchors:
        atomic_json(root / 'inputs/anchors.json', anchors)
    gen1 = root / 'generation/gen1'
    atomic_json(root / 'generation/state.json', dict(next_gen=2, prev=dict(
        model_zip=str(gen1 / 'model.zip'), model_onnx=str(gen1 / 'model.onnx'))))
    (root / 'corpora/pilot').mkdir(parents=True)
    (root / 'validation').mkdir()
    atomic_json(root / 'corpora/pilot/provenance.json', dict(cards_sha256=sha256(args.cards_json),
        source='fitted CP1 recovery after deletion-payment engine repair',
        parent_provenance=provenance, parent_replay_games=0, fresh_replay_first_generation=2))
    evidence = dict(status='running', image=image, source_commit=args.source_commit,
        checkpoint_sha256=args.checkpoint_sha256, **runtime, cards_sha256=sha256(args.cards_json),
        fresh_replay=True, parent_replay_games=0, started_unix=time.time())
    try:
        evidence['native_health'] = run_health(args, root)
        if [h['start'] for h in evidence['native_health']] != [0, 4] or any(
            h['games'] != 4 or h['truncated_games'] or h['rows'] <= 0 for h in evidence['native_health']
        ):
            raise ValueError('Eight complete native health games are required')
        for name, digest in source_hashes.items():
            if sha256(pinned_path(source, name)) != digest:
                raise ValueError(f'Frozen source changed during preparation: {name}')
        evidence['status'] = 'passed'
    except BaseException as error:
        evidence.update(status='failed', error=str(error))
        raise
    finally:
        evidence['finished_unix'] = time.time()
        atomic_json(root / 'validation/checkpoint1-recovery.json', evidence)
    files = {p.relative_to(root).as_posix(): sha256(p)
             for directory in ('inputs', 'generation/gen1', 'validation', 'lineage', 'corpora')
             for p in (root / directory).rglob('*') if p.is_file()}
    plan = dict(schema='fitted-checkpoint-payment-recovery-v1', image=image,
        source_image=original['image'], source_plan_sha256=args.source_plan_sha256,
        source_files=source_hashes, files=files, runtime_files=runtime_hashes,
        controller_commit=args.source_commit, ci_run=args.ci_run, preparation_sha256=sha256(Path(__file__)),
        parent_run=read(source / 'started.json')['run_id'], parent_checkpoint=1,
        parent_checkpoint_sha256=args.checkpoint_sha256, parent_source_state=state,
        inherited_collection=collected, parent_replay_games=0, fresh_replay=True,
        baseline_checkpoint=1, first_generation=2, final_generation=4, remaining_generations=3,
        generations=4, games=1100, workers=8, seed=20261027, replay_window=3, replay_cap_games=1100,
        expected_appearances_per_archetype=2200 / denominators['training']['archetypes'],
        pool_denominators=denominators, pool_source_commit=original.get('pool_source_commit'),
        evaluation_protocol=PROTOCOL, evaluation_panels=PANELS, evaluation_seed=22261027,
        evaluation_games_per_archetype_per_anchor=200, games_per_evaluation=10800,
        anchor_model_hashes={name: files[f'inputs/{filename}'] for name, filename in ANCHOR_FILES.items()},
        automatic_restart=False, automatic_promotion=False, search=SEARCH, fit=FIT,
        meta_archetypes=original['meta_archetypes'], pet_archetypes=original['pet_archetypes'],
        runtime_cards_sha256=sha256(args.cards_json), model_contract=runtime['contract'])
    from tools.cloud_search.resume_payment_recovery import validate
    validate(root, plan, source)
    for name in ('campaign-plan.json', 'deployment-plan.json'):
        atomic_json(root / name, plan)
    atomic_json(root / 'experiment_state.json', dict(stage='prepared', generation_index=1, image=image))
    return plan


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, default=Path('/archive'))
    parser.add_argument('--root', type=Path, default=Path('/experiment'))
    for flag in ('source-plan-sha256', 'checkpoint-sha256', 'onnx-sha256', 'onnx-metadata-sha256',
                 'source-commit', 'ci-run'):
        parser.add_argument('--' + flag, required=True)
    parser.add_argument('--cards-json', type=Path, default=Path('/app/data/cards.json'))
    parser.add_argument('--code-root', type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument('--cli-bin', default='/usr/local/bin/digimon-engine-cli')
    plan = prepare(parser.parse_args())
    print(json.dumps(dict(status='prepared', image=plan['image'], baseline_checkpoint=1,
                          remaining_generations=[2, 3, 4], games_per_evaluation=10800)), flush=True)


if __name__ == '__main__':
    main()
