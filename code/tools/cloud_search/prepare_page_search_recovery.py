"""Prepare CP2 -> generations 3--4 after the presentation-page search repair.

The stopped experiment is mounted read-only at /archive. Preserve its shards;
only weights, frozen inputs and provenance enter the new root. The identical
policy-only baseline is reused after checking its full cohort and runtime.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import shutil
import subprocess
import time

from tools.cloud_search.prepare_checkpoint_continuation import RUNTIME_FILES
from tools.cloud_search.prepare_payment_recovery import anchors_for_root, read, validate_models
from tools.cloud_search.resume_payment_recovery import validate_evaluation
from tools.cloud_search.run_small_generations import validate
from tools.selfplay_scheduler import atomic_json, sha256


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path('/experiment'))
    parser.add_argument('--source', type=Path, default=Path('/archive'))
    parser.add_argument('--source-commit', required=True)
    parser.add_argument('--ci-run', required=True)
    parser.add_argument('--parent-sha256', required=True)
    parser.add_argument('--source-plan-sha256', required=True)
    parser.add_argument('--replay-proof', type=Path, required=True)
    args = parser.parse_args()
    root, source = args.root, args.source
    image = os.environ['DIGIMON_DEPLOYED_IMAGE']
    parent = source / 'generation/gen2'
    if '@sha256:' not in image or sha256(parent / 'model.zip') != args.parent_sha256:
        raise ValueError('Immutable image and exact CP2 weights required')
    if sha256(source / 'campaign-plan.json') != args.source_plan_sha256:
        raise ValueError('Stopped campaign plan changed')
    old = read(source / 'campaign-plan.json')
    state = read(source / 'experiment_state.json')
    if state['stage'] != 'failed' or state['generation_index'] != 3 or (source / 'generation/gen3/model.zip').exists():
        raise ValueError('Expected the unfitted failed third generation')
    if (root / 'preparation-started.json').exists() or (root / 'generation').exists():
        raise ValueError('Refusing to adopt a partial preparation')
    proof = read(args.replay_proof)
    if (proof.get('status') != 'passed' or proof.get('image') != image
            or proof.get('checkpoint_sha256') != args.parent_sha256
            or proof.get('source_commit') != args.source_commit):
        raise ValueError('Require exact-image, exact-checkpoint page-loop replay proof')
    root.mkdir(parents=True, exist_ok=True)
    atomic_json(root / 'preparation-started.json', dict(time_unix=time.time(), image=image))
    runtime = validate_models(parent, source / 'inputs', Path('/app/data/cards.json'))
    if runtime['contract'] != old['model_contract']:
        raise ValueError('Observation/action contract changed')
    if sha256(Path('/app/data/cards.json')) != old['runtime_cards_sha256']:
        raise ValueError('Card data changed; cannot reuse policy-only baseline')
    # Search-only repair: the saved policy evaluation code must remain exact.
    evaluation_files = ('digimon_gym/agents/anchored_eval.py', 'digimon_gym/agents/card_eval.py',
        'tools/cloud_search/card_eval_worker.py', 'tools/anchored_eval_cli.py', 'model_contract.py')
    for name in evaluation_files:
        path = Path('/app/code') / name
        if old['runtime_files'].get(str(path)) != sha256(path):
            raise ValueError(f'Policy evaluation runtime changed: {name}')
    baseline = source / 'evaluations/window-000002'
    validate_evaluation(baseline, source, old, 2)
    shutil.copytree(source / 'inputs', root / 'inputs')
    atomic_json(root / 'inputs/anchors.json', anchors_for_root(read(source / 'inputs/anchors.json'), root))
    target = root / 'generation/gen2'
    target.mkdir(parents=True)
    for path in parent.glob('model.*'):
        shutil.copy2(path, target / path.name)
    for name in ('campaign-plan.json', 'experiment_state.json', 'started.json', 'generation/state.json'):
        dest = root / 'lineage/source' / name
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source / name, dest)
    provenance = root / 'corpora/pilot/provenance.json'
    provenance.parent.mkdir(parents=True)
    shutil.copy2(source / 'corpora/pilot/provenance.json', provenance)
    validation = root / 'validation'
    validation.mkdir()
    shutil.copy2(args.replay_proof, validation / 'page-loop-replay.json')
    receipt = dict(status='complete', checkpoint_sha256=args.parent_sha256,
        source_image=old['image'], source_directory=str(baseline),
        evaluation_panels=old['evaluation_panels'], evaluation_seed=old['evaluation_seed'],
        games_per_archetype=old['evaluation_games_per_archetype_per_anchor'],
        policy_evaluation_unchanged=True,
        report_hashes={p.name: sha256(p) for p in (baseline / 'reports').glob('checkpoint_*.json')},
        evaluation_runtime={name: sha256(Path('/app/code') / name) for name in evaluation_files})
    atomic_json(validation / 'parent-evaluation.json', receipt)

    def health(start_count):
        start, count = start_count
        out = validation / f'health-{start}'
        command = ['digimon-engine-cli', '--cards-json', '/app/data/cards.json', '--pool', 'implemented',
            'selfplay', '--model', str(target / 'model.onnx'), '--decks', str(root / 'inputs/decks.json'),
            '--games', str(count), '--game-start', str(start), '--seed', '20561027',
            '--sims', '320', '--worlds', '4', '--eval-batch-size', '4', '--decision-cap', '1200',
            '--temperature-decisions', '16', '--tensor-profile', 'standard_lite_v2', '--out', str(out)]
        with (validation / f'health-{start}.log').open('w') as log:
            subprocess.run(command, check=True, stdout=log, stderr=subprocess.STDOUT, timeout=1800)
        manifest = read(out / 'manifest.json')
        if manifest['games'] != count or manifest['truncated_games'] or manifest['rows'] <= 0:
            raise ValueError(f'Full-budget health check failed at {start}')
        return dict(start=start, games=count, rows=manifest['rows'], truncated_games=0,
                    manifest_sha256=sha256(out / 'manifest.json'))
    with ThreadPoolExecutor(max_workers=3) as executor:
        health_checks = list(executor.map(health, [(0, 4), (4, 4), (398, 1)]))
    atomic_json(validation / 'generation0-recovery.json', dict(status='passed', image=image,
        checkpoint_sha256=args.parent_sha256, source_commit=args.source_commit,
        model_contract=runtime['contract'], native_health=health_checks))
    atomic_json(root / 'generation/state.json', dict(next_gen=3, prev=dict(
        model_zip=str(target / 'model.zip'), model_onnx=str(target / 'model.onnx'))))
    files = {p.relative_to(root).as_posix(): sha256(p)
        for folder in ('inputs', 'generation/gen2', 'validation', 'lineage', 'corpora')
        for p in (root / folder).rglob('*') if p.is_file() and 'health-' not in str(p)}
    runtime_files = {str(Path('/app/code') / name): sha256(Path('/app/code') / name)
        for name in (*RUNTIME_FILES, 'tools/cloud_search/prepare_page_search_recovery.py')}
    runtime_files['/usr/local/bin/digimon-engine-cli'] = sha256(Path('/usr/local/bin/digimon-engine-cli'))
    plan = dict(old, schema='page-search-recovery-v1', first_generation=3, generations=4,
        baseline_checkpoint=2, remaining_generations=2,
        image=image, controller_commit=args.source_commit, ci_run=args.ci_run,
        parent_checkpoint=2, parent_checkpoint_sha256=args.parent_sha256,
        source_image=old['image'], source_plan_sha256=args.source_plan_sha256,
        fresh_replay=True, parent_replay_games=0, reuse_parent_evaluation=True,
        files=files, runtime_files=runtime_files, model_contract=runtime['contract'])
    plan['parent_run'] = read(source / 'started.json')['run_id']
    plan['source_files'] = {name: sha256(source / name) for name in (
        'campaign-plan.json', 'experiment_state.json', 'generation/gen2/model.zip',
        'generation/gen2/model.onnx', 'generation/gen2/model.onnx.meta.json')}
    for name in ('campaign-plan.json', 'deployment-plan.json'):
        atomic_json(root / name, plan)
    validate(root, plan)
    atomic_json(root / 'experiment_state.json', dict(stage='prepared', generation_index=2,
        image=image, time_unix=time.time(), automatic_restart=False, automatic_promotion=False))
    print(json.dumps(dict(status='prepared', image=image, first_generation=3, final_generation=4,
        games_per_generation=plan['games'], health=health_checks)), flush=True)


if __name__ == '__main__':
    main()
