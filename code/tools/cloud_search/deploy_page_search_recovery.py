"""Launch the prepared CP2 continuation once; keep the failed source untouched."""
import argparse
import json
from pathlib import Path
import shlex
import subprocess
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--image', required=True)
    parser.add_argument('--commit', required=True)
    args = parser.parse_args()
    base = Path('/opt/digimon-search')
    root = base / 'ex13-page-search-20261027'
    source = base / 'ex13-medusamon-dp-20261027'
    name = 'digimon-ex13-page-search-20261027'
    run_id = 'ex13-page-search-gen3to4-20261027'
    plan = json.loads((root / 'campaign-plan.json').read_text())
    metadata = json.loads(subprocess.check_output(['docker', 'image', 'inspect', args.image], text=True))[0]
    if (metadata['Config']['Labels']['org.opencontainers.image.revision'] != args.commit
            or plan['image'] != args.image or plan['controller_commit'] != args.commit
            or plan['first_generation'] != 3 or plan['generations'] != 4):
        raise ValueError('Prepared campaign and immutable image differ')
    if (root / 'started.json').exists():
        raise ValueError('Continuation already started; no automatic restart')
    for unit in ('digimon-ex13-medusamon-dp-20261027.service', name + '.service'):
        state = subprocess.run(['systemctl', 'is-active', unit], capture_output=True, text=True).stdout.strip()
        if state not in ('inactive', 'failed', 'unknown'):
            raise ValueError(f'Conflicting active training unit: {unit}')
    command = ['/usr/bin/docker', 'run', '--rm', '--init', '--name', name,
        '--memory=24g', '--cpus=8', '--stop-timeout=120', '--network=none',
        '-e', f'DIGIMON_DEPLOYED_IMAGE={args.image}', '-e', 'DIGIMON_ONNX_OPPONENT=0',
        '-e', 'DIGIMON_SEARCH_EVAL_CACHE_ENTRIES=0', '-e', 'OMP_NUM_THREADS=1',
        '-e', 'MKL_NUM_THREADS=1', '-e', 'OPENBLAS_NUM_THREADS=1',
        '--mount', f'type=bind,src={source},dst=/archive,readonly',
        '--mount', f'type=bind,src={root},dst=/experiment', args.image,
        'python', '-m', 'tools.cloud_search.run_small_generations',
        '--evaluation-root', '/experiment/evaluations']
    subprocess.run([*command, '--validate-only'], check=True)
    monitor = root / 'monitors/pilot'
    monitor.mkdir(parents=True, exist_ok=False)
    logger = ['/usr/bin/docker', 'run', '--rm', '--init', '--name', name + '-monitor',
        '--memory=4g', '--cpus=.25', '--stop-timeout=120',
        '--env-file', str(base / 'monitor/credentials.env'),
        '--mount', f'type=bind,src={root},dst=/experiment,readonly',
        '--mount', f'type=bind,src={monitor},dst=/monitor',
        '-e', f'DIGIMON_DEPLOYED_IMAGE={args.image}', args.image,
        'python', '-m', 'tools.cloud_search.wandb_monitor',
        '--training-log', '/experiment/training.log', '--card-eval-root', '/experiment/evaluations',
        '--run-id', run_id, '--entity', 'james-peters97-mammal', '--project', 'digimon-rl', '--online']

    def install(container, argv, log):
        path = Path('/etc/systemd/system') / (container + '.service')
        text = ('[Unit]\nDescription=Digimon page-search repair continuation\nAfter=docker.service\nRequires=docker.service\n'
            '[Service]\nType=simple\nRestart=no\nRuntimeMaxSec=24h\nTimeoutStopSec=150\n'
            f'ExecStart={shlex.join(argv)}\nExecStop=-/usr/bin/docker stop -t 120 {container}\n'
            f'StandardOutput=append:{root / log}\nStandardError=append:{root / log}\n'
            '[Install]\nWantedBy=multi-user.target\n')
        with path.open('x') as f:
            f.write(text)
        (root / path.name).write_text(text)

    install(name, command, 'training.log')
    install(name + '-monitor', logger, 'monitor.log')
    (root / 'started.json').write_text(json.dumps(dict(time_unix=time.time(), image=args.image,
        source_commit=args.commit, run_id=run_id, parent_checkpoint=2,
        first_generation=3, final_generation=4), indent=2))
    subprocess.run(['systemctl', 'daemon-reload'], check=True)
    subprocess.run(['systemctl', 'start', name + '.service'], check=True)
    subprocess.run(['systemctl', 'start', name + '-monitor.service'], check=True)
    print(json.dumps(dict(status='launched', root=str(root), run_id=run_id, image=args.image)))


if __name__ == '__main__':
    main()
