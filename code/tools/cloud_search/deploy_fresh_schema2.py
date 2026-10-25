"""Prepare/start the approved fresh schema-2, 13-archetype search campaign.

Run on the Hetzner host. The newly bootstrapped checkpoint and frozen inputs are copied; prior-run
weights and replay are never mounted. Validation must already have passed on this digest.
Preparation is separate from --start so the complete plan can be inspected.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import time


def sha256(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--image', required=True)
    parser.add_argument('--commit', required=True)
    parser.add_argument('--ci-run', required=True)
    parser.add_argument('--validation', type=Path, required=True)
    parser.add_argument('--start', action='store_true')
    args = parser.parse_args()
    if '@sha256:' not in args.image:
        raise ValueError('Use an immutable image digest')
    base = Path('/opt/digimon-search')
    old = base / 'small-generations'
    source = base / 'fresh-schema2-preflight-20261025'
    root = base / 'fresh-schema2-generations-20261025'
    reports = base / 'fresh-schema2-card-evaluation-20261025'
    monitor = base / 'fresh-schema2-monitor-20261025'
    run_id = 'english-meta-pets-light-fresh-2x1100-20261025'
    parent = source / 'generation/gen0'
    report = json.loads(args.validation.read_text())
    initialization = json.loads((source / 'inputs/initialization.json').read_text())
    if (report.get('status') != 'passed' or report.get('image') != args.image
            or report.get('checkpoint_sha256') != sha256(parent / 'model.zip')
            or report.get('source_commit') != args.commit
            or report.get('validated_deck_slots') != 26
            or report.get('full_games', 0) < 31
            or report.get('contract', {}).get('action_schema_version') != 2
            or not any(c['label'] == 'light-hybrid' and c['games'] >= 4
                       for c in report.get('native', []))
            or initialization.get('inherited_weights') is not False
            or initialization.get('image') != args.image
            or initialization.get('seed_sha256') != sha256(parent / 'model.zip')
            or sha256(source / 'inputs/seed.zip') != sha256(parent / 'model.zip')):
        raise ValueError('Validation does not bind this image and checkpoint')
    inspect = json.loads(subprocess.check_output(['docker', 'image', 'inspect', args.image], text=True))[0]
    if inspect['Config']['Labels']['org.opencontainers.image.revision'] != args.commit:
        raise ValueError('Image source revision mismatch')
    state = subprocess.run(['systemctl', 'is-active', 'digimon-search.service'],
                           capture_output=True, text=True).stdout.strip()
    if state not in ('inactive', 'failed'):
        raise RuntimeError('Training must be stopped before preparing or starting')
    runtime_paths = ['code/tools/cloud_search/run_small_generations.py',
        'code/tools/cloud_search/replay_window.py', 'code/tools/cloud_search/card_eval_worker.py',
        'code/tools/run_selfplay_generation.py', 'code/digimon_gym/agents/azero_trainer.py',
        'code/digimon_gym/agents/anchored_eval.py', 'code/tools/anchored_eval_cli.py',
        'code/digimon_gym/agents/card_eval.py', 'code/tools/cloud_search/wandb_monitor.py']
    if not args.start:
        root.mkdir(exist_ok=False)
        reports.mkdir(exist_ok=False)
        monitor.mkdir(exist_ok=False)
        shutil.copytree(source / 'inputs', root / 'inputs')
        (root / 'validation').mkdir()
        shutil.copy2(args.validation, root / 'validation/generation0-recovery.json')
        target = root / 'generation/gen0'
        target.mkdir(parents=True)
        for path in parent.glob('model.*'):
            shutil.copy2(path, target / path.name)
        # Intentionally no gen0/shards: fresh seed weights carry no prior-run replay.
        (root / 'generation/state.json').write_text(json.dumps(dict(next_gen=1, prev=dict(
            model_zip='/experiment/generation/gen0/model.zip',
            model_onnx='/experiment/generation/gen0/model.onnx'))))
        script = 'import hashlib,json,pathlib; print(json.dumps({"/app/"+p:hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest() for p in ' + repr(runtime_paths) + '}))'
        runtime = json.loads(subprocess.check_output(['docker', 'run', '--rm', '--network=none',
            args.image, 'python', '-c', script], text=True))
        inputs = {p.relative_to(root).as_posix(): sha256(p)
            for directory in (root / 'inputs', root / 'validation', target)
            for p in directory.iterdir() if p.is_file()}
        plan = dict(generations=2, games=1100, workers=8, seed=40261025, image=args.image,
            files=inputs, runtime_files=runtime, controller_commit=args.commit,
            deployment_script_sha256=sha256(Path(__file__)),
            initialization='fresh greedy imitation and terminal value fit on schema 2',
            previous_run_reference='english-meta-pets-4x1000-20261025', inherited_weights=False,
            replay_window=3, replay_cap_games=1100, fresh_replay=True,
            evaluation_games_per_archetype_per_anchor=200)
        (root / 'campaign-plan.json').write_text(json.dumps(plan, indent=2))
        legacy = json.loads((old / 'deployment-plan.json').read_text())
        # Old host identity is useful; old source/archive evidence is not the
        # provenance of a newly initialized model and must not survive by accident.
        for key in ('source_archive_sha256', 'controller_sha256', 'preflight',
                    'parent_run', 'parent_checkpoint', 'source_commit'):
            legacy.pop(key, None)
        legacy.update(image=args.image, image_source_commit=args.commit, controller_commit=args.commit,
            ci_run=args.ci_run, initialization=plan['initialization'], inherited_weights=False, inputs=inputs,
            generation=dict(games=1100, generations=2, simulations=320, worlds=4,
                evaluation_batch_size=4, workers=8, replay_window=3, replay_cap_games=1100,
                archetype_count=13, distinct_training_lists=21, fresh_replay=True,
                inherited_weights=False, initialization=plan['initialization'],
                seed_checkpoint_sha256=report['checkpoint_sha256']))
        legacy['evaluation'] = dict(games_per_archetype_per_anchor=200, total_games_per_checkpoint=5200,
            anchors=['greedy', 'frozen-bootstrap'], automatic_promotion=False,
            meta_pool='8 held-out lists', pet_pool='5 exact same-list pets')
        legacy['status'] = 'prepared_fresh_schema2'
        (root / 'deployment-plan.json').write_text(json.dumps(legacy, indent=2))
        (root / 'experiment_state.json').write_text(json.dumps(dict(stage='prepared', generation_index=0)))
    plan = json.loads((root / 'campaign-plan.json').read_text())
    if plan['image'] != args.image or plan['controller_commit'] != args.commit:
        raise ValueError('Prepared plan differs')
    command = ['/usr/bin/docker', 'run', '--name', 'digimon-search-experiment', '--rm', '--init',
        '--network=none', '--memory=28g', '--cpus=8', '--stop-timeout=120',
        '-e', f'DIGIMON_DEPLOYED_IMAGE={args.image}', '-e', 'OMP_NUM_THREADS=1',
        '-e', 'MKL_NUM_THREADS=1', '-e', 'OPENBLAS_NUM_THREADS=1',
        '-e', 'DIGIMON_SEARCH_EVAL_CACHE_ENTRIES=0',
        '-e', 'DIGIMON_EVAL_FAILURE_DIR=/experiment/eval-failures',
        '--mount', f'type=bind,src={root},dst=/experiment',
        '--mount', f'type=bind,src={reports},dst=/card-evaluation',
        args.image, 'python', '-m', 'tools.cloud_search.run_small_generations']
    subprocess.run([*command, '--validate-only'], check=True)
    if not args.start:
        print(json.dumps(dict(status='prepared', root=str(root), run_id=run_id, plan=plan)))
        return
    if (root / 'generation/gen0/shards').exists():
        raise ValueError('Replay must not enter the fresh initialization')
    units = Path('/etc/systemd/system')
    backup = root / 'previous-units'
    backup.mkdir(exist_ok=False)
    names = ['digimon-search.service', 'digimon-search-monitor.service', 'digimon-search-journal.service']
    for name in names:
        shutil.copy2(units / name, backup / name)
    subprocess.run(['systemctl', 'stop', *names[1:], 'digimon-search-card-eval.service'], check=True)
    def replace_start(name, cmd):
        source = (backup / name).read_text()
        return '\n'.join('ExecStart=' + ' '.join(cmd) if line.startswith('ExecStart=') else line
                         for line in source.splitlines()) + '\n'
    (units / names[0]).write_text(replace_start(names[0], command))
    logging = ['/usr/bin/docker', 'run', '--name', 'digimon-search-monitor', '--rm', '--init',
        '--memory=1g', '--cpus=0.25', '--cpu-shares=128',
        '--env-file', str(base / 'monitor/credentials.env'),
        '--mount', f'type=bind,src={root},dst=/experiment,readonly',
        '--mount', f'type=bind,src={monitor},dst=/monitor',
        '--mount', f'type=bind,src={reports},dst=/card-evaluation,readonly',
        '-e', f'DIGIMON_DEPLOYED_IMAGE={args.image}', args.image,
        'python', '-m', 'tools.cloud_search.wandb_monitor', '--run-id', run_id]
    (units / names[1]).write_text(replace_start(names[1], logging))
    journal = (backup / names[2]).read_text().replace(
        str(base / 'small-monitor/training.log'), str(monitor / 'training.log'))
    (units / names[2]).write_text(journal)
    (root / 'started.json').write_text(json.dumps(dict(time_unix=time.time())))
    subprocess.run(['systemctl', 'daemon-reload'], check=True)
    for name in [names[2], names[0], names[1]]:
        subprocess.run(['systemctl', 'start', name], check=True)
    print(json.dumps(dict(status='launched', root=str(root), run_id=run_id, image=args.image)))


if __name__ == '__main__':
    main()
