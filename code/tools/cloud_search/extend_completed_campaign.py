"""Prepare a same-image continuation with verified replay, then explicitly start it.

Run prepare inside the pinned image with the source mounted read-only. The host
start subcommand installs new bounded services; it never restarts old services.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import time


def read(path):
    return json.loads(path.read_text())


def sha256(path):
    with path.open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()


def prepare(args):
    from tools.cloud_search.prepare_payment_recovery import validate_models
    from tools.cloud_search.resume_payment_recovery import validate_evaluation
    from tools.cloud_search.run_small_generations import validate
    from tools.selfplay_scheduler import _read_commit, atomic_json

    source, root = args.source.resolve(), args.root.resolve()
    if source == root or source in root.parents or root in source.parents:
        raise ValueError('Source and destination overlap')
    if root.exists() and any(root.iterdir()):
        raise ValueError('Destination must be empty')
    old = read(source / 'campaign-plan.json')
    state = read(source / 'experiment_state.json')
    parent = state['generation_index']
    if state['stage'] != 'complete' or parent != old['generations']:
        raise ValueError('Only a verified completed campaign can be extended')
    if args.updates not in range(1, 5):
        raise ValueError('Use one to four bounded updates')
    image = os.environ['DIGIMON_DEPLOYED_IMAGE']
    if image != old['image'] or '@sha256:' not in image:
        raise ValueError('Continuation requires the same immutable engine image')
    if sha256(source / f'generation/gen{parent}/model.zip') != args.parent_sha256:
        raise ValueError('Unexpected parent checkpoint')
    saved = read(source / 'generation/state.json')
    if saved != dict(next_gen=parent + 1, prev=dict(
            model_zip=f'/experiment/generation/gen{parent}/model.zip',
            model_onnx=f'/experiment/generation/gen{parent}/model.onnx')):
        raise ValueError('Unexpected completed generation state')
    for name, digest in old['files'].items():
        path = source / name
        if not path.resolve().is_relative_to(source) or sha256(path) != digest:
            raise ValueError(f'Frozen source file changed: {name}')
    controller = '/app/code/tools/cloud_search/run_small_generations.py'
    runtime_files = dict(old['runtime_files'])
    for name, digest in runtime_files.items():
        if name != controller and sha256(Path(name)) != digest:
            raise ValueError(f'Runtime changed: {name}')
    original = runtime_files[controller]
    runtime_files[controller] = sha256(Path(controller))
    if sha256(Path('/app/data/cards.json')) != old['runtime_cards_sha256']:
        raise ValueError('Runtime cards changed')
    runtime = validate_models(source / f'generation/gen{parent}', source / 'inputs', Path('/app/data/cards.json'))
    if runtime['contract'] != old['model_contract']:
        raise ValueError('Model contract changed')
    baseline = source / 'evaluations' / f'window-{parent:06d}'
    validate_evaluation(baseline, source, old, parent)
    print('Parent model, frozen inputs, runtime and complete evaluation verified', flush=True)

    root.mkdir(parents=True, exist_ok=True)
    atomic_json(root / 'preparation-started.json', dict(time_unix=time.time(), source=str(source)))
    shutil.copytree(source / 'inputs', root / 'inputs')
    shutil.copytree(source / 'corpora', root / 'corpora')
    lineage = root / 'lineage/source'
    lineage.mkdir(parents=True)
    for name in ('campaign-plan.json', 'experiment_state.json', 'started.json'):
        shutil.copy2(source / name, lineage / name)
    replay = []
    # Window 3 on the next update needs exactly the two most recent generations.
    for index in range(max(0, parent - 1), parent + 1):
        before = source / f'generation/gen{index}'
        progress = read(before / 'scheduler_progress.json')
        if (progress['status'] != 'complete' or progress['committed_games'] != old['games']
                or progress['truncated_games'] or progress.get('error')):
            raise ValueError('Only complete, untruncated generations may enter replay')
        schedule = read(before / 'schedule.json')
        target = root / f'generation/gen{index}'
        target.mkdir(parents=True)
        for file in before.iterdir():
            if file.is_file() and file.name != 'scheduler.lock':
                shutil.copy2(file, target / file.name)
        total = 0
        chunks = []
        for start in range(0, old['games'], schedule['chunk_games']):
            count = min(schedule['chunk_games'], old['games'] - start)
            chunk = before / 'shards' / f'proc{start:08d}'
            commit = _read_commit(chunk, start, count, schedule['seed'])
            if commit['truncated_games']:
                raise ValueError('Truncated replay chunk')
            copied = target / 'shards' / chunk.name
            shutil.copytree(chunk, copied)
            if _read_commit(copied, start, count, schedule['seed']) != commit:
                raise ValueError('Replay copy changed')
            total += count
            chunks.append(dict(directory=copied.relative_to(root).as_posix(),
                               commit_sha256=sha256(copied / 'chunk_commit.json')))
        replay.append(dict(generation=index, games=total, chunks=chunks))
        print(f'Copied and verified generation {index}: {total} replay games', flush=True)
    validation = root / 'validation'
    validation.mkdir()
    atomic_json(validation / 'replay-lineage.json', dict(source=str(source), generations=replay))
    atomic_json(validation / 'parent-evaluation.json', dict(status='complete',
        checkpoint_sha256=args.parent_sha256, source_directory=str(baseline),
        source_image=image, evaluation_panels=old['evaluation_panels'],
        evaluation_seed=old['evaluation_seed'],
        games_per_archetype=old['evaluation_games_per_archetype_per_anchor'],
        policy_evaluation_unchanged=True,
        report_hashes={p.name: sha256(p) for p in (baseline / 'reports').glob('checkpoint_*.json')}))
    # Actual native inference from the new parent at the unchanged search budget.
    health = validation / 'health'
    with (validation / 'health.log').open('x') as log:
        subprocess.run(['digimon-engine-cli', '--cards-json', '/app/data/cards.json', '--pool', 'implemented',
            'selfplay', '--model', str(root / f'generation/gen{parent}/model.onnx'),
            '--decks', str(root / 'inputs/decks.json'), '--games', '4', '--seed', '30261027',
            '--sims', '320', '--worlds', '4', '--eval-batch-size', '4', '--decision-cap', '1200',
            '--temperature-decisions', '16', '--tensor-profile', 'standard_lite_v2', '--out', str(health)],
            check=True, stdout=log, stderr=subprocess.STDOUT, timeout=1800)
    manifest = read(health / 'manifest.json')
    if manifest['games'] != 4 or manifest['truncated_games'] or manifest['rows'] <= 0:
        raise ValueError('Parent native health check failed')
    atomic_json(validation / 'generation0-recovery.json', dict(status='passed', image=image,
        checkpoint_sha256=args.parent_sha256, model_contract=runtime['contract'],
        native_health=dict(games=4, rows=manifest['rows'], truncated_games=0)))
    atomic_json(root / 'generation/state.json', dict(next_gen=parent + 1, prev=dict(
        model_zip=str(root / f'generation/gen{parent}/model.zip'),
        model_onnx=str(root / f'generation/gen{parent}/model.onnx'))))
    files = {p.relative_to(root).as_posix(): sha256(p)
             for folder in ('inputs', 'corpora', 'lineage', 'validation', 'generation')
             for p in (root / folder).rglob('*')
             if p.is_file() and p != root / 'generation/state.json' and 'health' not in p.parts}
    plan = dict(old, schema='completed-campaign-extension-v1',
        first_generation=parent + 1, generations=parent + args.updates,
        final_generation=parent + args.updates, remaining_generations=args.updates,
        baseline_checkpoint=parent, parent_checkpoint=parent,
        parent_checkpoint_sha256=args.parent_sha256, parent_run=read(source / 'started.json')['run_id'],
        parent_source_state=state, source_image=image, source_plan_sha256=sha256(source / 'campaign-plan.json'),
        source_files={}, fresh_replay=False, parent_replay_games=sum(r['games'] for r in replay),
        inherited_collection=dict(games=sum(r['games'] for r in replay)),
        reuse_parent_evaluation=True, files=files, runtime_files=runtime_files,
        preparation_sha256=sha256(Path(__file__)),
        controller_overlay=dict(path=controller, original_sha256=original,
                                replacement_sha256=runtime_files[controller],
                                purpose='Allow up to four updates at continuing logical checkpoint indices'))
    for name in ('campaign-plan.json', 'deployment-plan.json'):
        atomic_json(root / name, plan)
    validate(root, plan)
    atomic_json(root / 'experiment_state.json', dict(stage='prepared', generation_index=parent,
        image=image, time_unix=time.time(), automatic_restart=False, automatic_promotion=False))
    print(json.dumps(dict(status='prepared', first=parent + 1, last=parent + args.updates,
                          replay_games=plan['parent_replay_games'], health_games=4)), flush=True)


def start(args):
    source, root = args.source.resolve(), args.root.resolve()
    plan = read(root / 'campaign-plan.json')
    image, name = plan['image'], args.name
    if not name or any(c not in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in name):
        raise ValueError('Use a simple service name')
    if (root / 'started.json').exists() or read(root / 'experiment_state.json')['stage'] != 'prepared':
        raise ValueError('Only a prepared, never-started continuation may launch')
    if subprocess.check_output(['docker', 'ps', '-q'], text=True).strip():
        raise ValueError('Inspect active containers before launching')
    metadata = json.loads(subprocess.check_output(['docker', 'image', 'inspect', image], text=True))[0]
    if metadata['Config']['Labels']['org.opencontainers.image.revision'] != plan['controller_commit']:
        raise ValueError('Image revision differs')
    overlay = args.controller_overlay.resolve()
    if sha256(overlay) != plan['controller_overlay']['replacement_sha256']:
        raise ValueError('Controller overlay differs')
    common = ['--mount', f'type=bind,src={root},dst=/experiment',
              '--mount', f'type=bind,src={overlay},dst={plan["controller_overlay"]["path"]},readonly']
    command = ['/usr/bin/docker', 'run', '--rm', '--init', '--name', name,
        '--memory=24g', '--cpus=8', '--stop-timeout=120', '--network=none',
        '-e', f'DIGIMON_DEPLOYED_IMAGE={image}', '-e', 'DIGIMON_ONNX_OPPONENT=0',
        '-e', 'DIGIMON_SELFPLAY_TRACE_DECISIONS=1', '-e', 'DIGIMON_SEARCH_EVAL_CACHE_ENTRIES=0',
        '-e', 'OMP_NUM_THREADS=1', '-e', 'MKL_NUM_THREADS=1', '-e', 'OPENBLAS_NUM_THREADS=1',
        *common, image, 'python', '-m', 'tools.cloud_search.run_small_generations',
        '--evaluation-root', '/experiment/evaluations']
    subprocess.run([*command, '--validate-only'], check=True)
    monitor = root / 'monitors/pilot'
    monitor.mkdir(parents=True, exist_ok=False)
    logger = ['/usr/bin/docker', 'run', '--rm', '--init', '--name', name + '-monitor',
        '--memory=4g', '--cpus=.25', '--stop-timeout=120',
        '--env-file', '/opt/digimon-search/monitor/credentials.env',
        '--mount', f'type=bind,src={root},dst=/experiment,readonly',
        '--mount', f'type=bind,src={monitor},dst=/monitor',
        '-e', f'DIGIMON_DEPLOYED_IMAGE={image}', image,
        'python', '-m', 'tools.cloud_search.wandb_monitor', '--training-log', '/experiment/training.log',
        '--card-eval-root', '/experiment/evaluations', '--run-id', args.run_id,
        '--entity', 'james-peters97-mammal', '--project', 'digimon-rl', '--online']
    units = Path('/etc/systemd/system')
    if any((units / f'{name}{suffix}.service').exists() for suffix in ('', '-monitor')):
        raise ValueError('Refusing to overwrite an existing service')
    for suffix, argv, logfile in (('', command, 'training.log'), ('-monitor', logger, 'monitor.log')):
        unit = name + suffix
        content = ('[Unit]\nDescription=Digimon bounded continued training\nAfter=docker.service\nRequires=docker.service\n'
            '[Service]\nType=simple\nRestart=no\nRuntimeMaxSec=24h\nTimeoutStopSec=150\n'
            f'ExecStart={shlex.join(argv)}\nExecStop=-/usr/bin/docker stop -t 120 {unit}\n'
            f'StandardOutput=append:{root / logfile}\nStandardError=append:{root / logfile}\n')
        with (units / f'{unit}.service').open('x') as f:
            f.write(content)
        (root / f'{unit}.service').write_text(content)
    (root / 'started.json').write_text(json.dumps(dict(time_unix=time.time(), run_id=args.run_id,
        image=image, source_commit=plan['controller_commit'], parent_checkpoint=plan['parent_checkpoint'],
        first_generation=plan['first_generation'], final_generation=plan['generations'],
        deployment_script_sha256=sha256(Path(__file__))), indent=2))
    subprocess.run(['systemctl', 'daemon-reload'], check=True)
    subprocess.run(['systemctl', 'start', name + '.service'], check=True)
    subprocess.run(['systemctl', 'start', name + '-monitor.service'], check=True)
    print(json.dumps(dict(status='launched', root=str(root), run_id=args.run_id, image=image)))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='mode', required=True)
    for mode in ('prepare', 'start'):
        p = sub.add_parser(mode)
        p.add_argument('--root', type=Path, required=True)
        p.add_argument('--source', type=Path, required=True)
        if mode == 'prepare':
            p.add_argument('--updates', type=int, default=4)
            p.add_argument('--parent-sha256', required=True)
        else:
            p.add_argument('--name', required=True)
            p.add_argument('--run-id', required=True)
            p.add_argument('--controller-overlay', type=Path, required=True)
    args = parser.parse_args()
    (prepare if args.mode == 'prepare' else start)(args)
