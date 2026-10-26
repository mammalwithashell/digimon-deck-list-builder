"""Start the prepared October 26 fixed-engine continuation on the existing host.

The preparation is a separate full-budget validation step. No old service,
checkpoint, replay directory or credential file is modified by this launcher.
"""
import json
from pathlib import Path
import shlex
import subprocess
import time


def main():
    base = Path('/opt/digimon-search')
    root = base / 'fixed-continuation-20261026'
    name = 'digimon-fixed-continuation'
    run_id = 'english-meta-fixed-4x1100-20261026'
    image = 'ghcr.io/mammalwithashell/digimon-search-cpu@sha256:705f2c4b86c5458a2df7f09ded1e1d9cc49160f65bca442cd187b0b1480f0711'
    commit = '6627540bab607109b9eb8d5216ff647b8e4b9ae8'
    plan = json.loads((root / 'campaign-plan.json').read_text())
    metadata = json.loads(subprocess.check_output(['docker', 'image', 'inspect', image], text=True))[0]
    if metadata['Config']['Labels']['org.opencontainers.image.revision'] != commit:
        raise ValueError('Native image source mismatch')
    if plan['image'] != image or plan['controller_commit'] != commit or not plan['fresh_replay']:
        raise ValueError('Unexpected prepared campaign')
    if (root / 'generation/gen0/shards').exists() or (root / 'started.json').exists():
        raise ValueError('Refusing old replay or an already-started campaign')
    if subprocess.check_output(['docker', 'ps', '-q'], text=True).strip():
        raise RuntimeError('Inspect existing containers before starting this campaign')
    units = Path('/etc/systemd/system')
    for suffix in ('', '-monitor'):
        if (units / f'{name}{suffix}.service').exists():
            raise ValueError('Refusing to overwrite an existing service')
    command = ['/usr/bin/docker', 'run', '--name', name, '--rm', '--init',
        '--network=none', '--memory=28g', '--cpus=8', '--stop-timeout=120',
        '-e', f'DIGIMON_DEPLOYED_IMAGE={image}', '-e', 'OMP_NUM_THREADS=1',
        '-e', 'MKL_NUM_THREADS=1', '-e', 'OPENBLAS_NUM_THREADS=1',
        '-e', 'DIGIMON_SEARCH_EVAL_CACHE_ENTRIES=0', '-e', 'DIGIMON_ONNX_OPPONENT=0',
        '--mount', f'type=bind,src={root},dst=/experiment', image,
        'python', '-m', 'tools.cloud_search.run_small_generations',
        '--evaluation-root', '/experiment/evaluations']
    subprocess.run([*command, '--validate-only'], check=True)
    monitor = root / 'monitors/pilot'
    monitor.mkdir(parents=True, exist_ok=False)
    logging = ['/usr/bin/docker', 'run', '--name', name + '-monitor', '--rm', '--init',
        '--memory=4g', '--cpus=.25', '--stop-timeout=120',
        '--env-file', str(base / 'monitor/credentials.env'),
        '--mount', f'type=bind,src={root},dst=/experiment,readonly',
        '--mount', f'type=bind,src={monitor},dst=/monitor',
        '-e', f'DIGIMON_DEPLOYED_IMAGE={image}', image,
        'python', '-m', 'tools.cloud_search.wandb_monitor',
        '--training-log', '/experiment/training.log', '--card-eval-root', '/experiment/evaluations',
        '--run-id', run_id, '--entity', 'james-peters97-mammal', '--project', 'digimon-rl', '--online']
    # Arguments are fixed strings/paths with no systemd specifier expansions.
    def service(container, args, logfile):
        return ('[Unit]\nDescription=Digimon fixed-engine continuation\n'
                'After=docker.service\nRequires=docker.service\n[Service]\n'
                'Type=simple\nRestart=no\nRuntimeMaxSec=24h\nTimeoutStopSec=150\n'
                f'ExecStart={shlex.join(args)}\n'
                f'ExecStop=-/usr/bin/docker stop -t 120 {container}\n'
                f'StandardOutput=append:{root / logfile}\n'
                f'StandardError=append:{root / logfile}\n')
    for suffix, cmd, logfile in (('', command, 'training.log'), ('-monitor', logging, 'monitor.log')):
        content = service(name + suffix, cmd, logfile)
        (units / f'{name}{suffix}.service').write_text(content)
        (root / f'{name}{suffix}.service').write_text(content)
    (root / 'started.json').write_text(json.dumps(dict(time_unix=time.time(), run_id=run_id)))
    subprocess.run(['systemctl', 'daemon-reload'], check=True)
    subprocess.run(['systemctl', 'start', name + '.service'], check=True)
    subprocess.run(['systemctl', 'start', name + '-monitor.service'], check=True)
    print(json.dumps(dict(status='launched', root=str(root), run_id=run_id, image=image)))


if __name__ == '__main__':
    main()
