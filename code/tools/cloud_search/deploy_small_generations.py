"""Prepare the approved Hetzner continuation without modifying the old run.

Run on the Linux host after uploading the reviewed source bundle to
deployment-small/. Every mounted source and input is pinned by SHA-256.
"""
import json
import hashlib
from pathlib import Path
import shutil
import subprocess
import time

def sha256(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


OVERLAYS = {
    'run_small_generations.py': '/app/code/tools/cloud_search/run_small_generations.py',
    'replay_window.py': '/app/code/tools/cloud_search/replay_window.py',
    'card_eval_worker.py': '/app/code/tools/cloud_search/card_eval_worker.py',
    'run_selfplay_generation.py': '/app/code/tools/run_selfplay_generation.py',
    'azero_trainer.py': '/app/code/digimon_gym/agents/azero_trainer.py',
    'anchored_eval.py': '/app/code/digimon_gym/agents/anchored_eval.py',
    'anchored_eval_cli.py': '/app/code/tools/anchored_eval_cli.py',
    'card_eval.py': '/app/code/digimon_gym/agents/card_eval.py',
    'wandb_monitor.py': '/app/code/tools/cloud_search/wandb_monitor.py',
}


def main():
    base = Path('/opt/digimon-search')
    old, root = base / 'experiment', base / 'small-generations'
    deployment, reports, monitor = base / 'deployment-small', base / 'small-card-evaluation', base / 'small-monitor'
    active = subprocess.run(['systemctl', 'is-active', 'digimon-search.service'], capture_output=True, text=True).stdout.strip()
    if active not in ('inactive', 'failed'):
        raise RuntimeError('Original training must already be stopped')
    image = (base / 'image.env').read_text().split('=', 1)[1].strip().strip('"')
    source = json.loads((deployment / 'source.json').read_text())
    for name, digest in source['files'].items():
        if sha256(deployment / name) != digest:
            raise ValueError(f'Upload hash mismatch: {name}')
    root.mkdir(exist_ok=False)
    reports.mkdir(exist_ok=False)
    monitor.mkdir(exist_ok=False)
    shutil.copytree(old / 'inputs', root / 'inputs')  # preserves frozen-bootstrap anchor
    shutil.copytree(old / 'validation', root / 'validation')
    parent = root / 'generation/gen0'
    parent.mkdir(parents=True)
    for path in (old / 'generation/gen0').glob('model.*'):
        shutil.copy2(path, parent / path.name)
    (parent / 'shards').symlink_to('/archive/generation/gen0/shards', target_is_directory=True)
    (root / 'generation/state.json').write_text(json.dumps(dict(next_gen=1, prev=dict(
        model_zip='/experiment/generation/gen0/model.zip', model_onnx='/experiment/generation/gen0/model.onnx'))))
    inputs = {p.relative_to(root).as_posix(): sha256(p)
              for directory in (root / 'inputs', root / 'validation', parent)
              for p in directory.iterdir() if p.is_file()}
    plan = dict(generations=4, games=1000, workers=8, seed=30261025, image=image,
        files=inputs, runtime_files={target: sha256(deployment / name) for name, target in OVERLAYS.items()},
        controller_commit=source['commit'], parent_run='english-meta-pets-20261024',
        excluded_collection='/archive/generation/gen1', exclusion_reason='two unresolved capped games; not fitted',
        replay_window=3, replay_cap_games=1000, evaluation_games_per_archetype_per_anchor=200)
    (root / 'campaign-plan.json').write_text(json.dumps(plan, indent=2))
    legacy = json.loads((old / 'deployment-plan.json').read_text())
    legacy.update(image=image, controller_commit=source['commit'], inputs=inputs,
        generation=dict(games=1000, generations=4, simulations=320, worlds=4, evaluation_batch_size=4,
            workers=8, replay_window=3, replay_cap_games=1000, archetype_count=12),
        evaluation=dict(games_per_archetype_per_anchor=200, checkpoint_only=True, automatic_promotion=False),
        parent_run=plan['parent_run'])
    (root / 'deployment-plan.json').write_text(json.dumps(legacy, indent=2))
    (root / 'experiment_state.json').write_text(json.dumps(dict(stage='prepared', generation_index=0)))
    mounts = ['--mount', f'type=bind,src={root},dst=/experiment',
              '--mount', f'type=bind,src={old},dst=/archive,readonly',
              '--mount', f'type=bind,src={reports},dst=/card-evaluation']
    for name, target in OVERLAYS.items():
        mounts += ['--mount', f'type=bind,src={deployment / name},dst={target},readonly']
    environment = ['-e', f'DIGIMON_DEPLOYED_IMAGE={image}', '-e', 'OMP_NUM_THREADS=1',
        '-e', 'MKL_NUM_THREADS=1', '-e', 'OPENBLAS_NUM_THREADS=1',
        '-e', 'DIGIMON_SEARCH_EVAL_CACHE_ENTRIES=0', '-e', 'DIGIMON_EVAL_FAILURE_DIR=/experiment/eval-failures']
    command = ['/usr/bin/docker', 'run', '--name', 'digimon-search-experiment', '--rm', '--init',
        '--network=none', '--memory=28g', '--cpus=8', '--stop-timeout=120', *environment, *mounts,
        image, 'python', '-m', 'tools.cloud_search.run_small_generations']
    subprocess.run([*command, '--validate-only'], check=True)
    backup = deployment / 'previous-units'
    backup.mkdir()
    units = Path('/etc/systemd/system')
    for name in ['digimon-search.service', 'digimon-search-monitor.service', 'digimon-search-journal.service']:
        shutil.copy2(units / name, backup / name)
    subprocess.run(['systemctl', 'stop', 'digimon-search-monitor.service', 'digimon-search-card-eval.service',
                    'digimon-search-journal.service'], check=True)
    training = (backup / 'digimon-search.service').read_text()
    training = '\n'.join('ExecStart=' + ' '.join(command) if line.startswith('ExecStart=') else line
                         for line in training.splitlines()) + '\n'
    (units / 'digimon-search.service').write_text(training)
    logging = (backup / 'digimon-search-monitor.service').read_text()
    logging = logging.replace(f'src={old},dst=/experiment', f'src={root},dst=/experiment')
    logging = logging.replace(f'src={base / "monitor"},dst=/monitor', f'src={monitor},dst=/monitor')
    logging = logging.replace(f'src={base / "card-evaluation"},dst=/card-evaluation', f'src={reports},dst=/card-evaluation')
    logging = logging.replace(f'src={base / "deployment"},dst=/deployment', f'src={deployment},dst=/deployment')
    logging = logging.replace(f'src={base / "deployment/card-eval/card_eval.py"}', f'src={deployment / "card_eval.py"}')
    logging = logging.replace('python /deployment/wandb_monitor.py',
                              'python /deployment/wandb_monitor.py --run-id english-meta-pets-4x1000-20261025')
    (units / 'digimon-search-monitor.service').write_text(logging)
    journal = (backup / 'digimon-search-journal.service').read_text()
    journal = journal.replace('--lines=all', '--lines=0')
    journal = journal.replace(f'{base / "monitor"}/training.log', f'{monitor}/training.log')
    (units / 'digimon-search-journal.service').write_text(journal)
    (root / 'started.json').write_text(json.dumps(dict(time_unix=time.time())))
    subprocess.run(['systemctl', 'daemon-reload'], check=True)
    subprocess.run(['systemctl', 'start', 'digimon-search-journal.service'], check=True)
    subprocess.run(['systemctl', 'start', 'digimon-search.service'], check=True)
    subprocess.run(['systemctl', 'start', 'digimon-search-monitor.service'], check=True)
    print(json.dumps(dict(status='launched', root=str(root), image=image, plan=plan)), flush=True)


if __name__ == '__main__':
    main()
