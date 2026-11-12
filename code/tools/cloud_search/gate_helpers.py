"""Run specialist-league promotion gates on helper hosts (2026-11-09).

Runs on the league host, outside the container (stdlib only). Each helper is
an SSH-reachable machine with Docker and the league image loaded under the
same image id. The coordinator claims a gate for a helper
(``specialist_league.claim``), copies exactly the files that gate reads,
runs ``specialist_league gate-one`` there, and copies the gate folder and
then the record back (record last, so the controller never sees a record
without its games). A failed or timed-out gate releases its claim; the
controller then runs it itself (``settle_gates``).

A gate is eligible once its specialist has trained that generation
(``state.json`` ``next_gen`` past it) and, after generation 1, its previous
gate is decided (the champion it plays is known). Helpers take the lowest
generation first and the archetype list from the end, the controller from
the start.

  python3 gate_helpers.py --root /opt/digimon-search/specialist-league-20261108 \\
      --helpers helpers.json --image sha256:... --ssh-key /root/.ssh/gate_helpers
"""
from __future__ import annotations

import argparse
import json
import shlex
import subprocess
import threading
import time
from pathlib import Path

from tools.cloud_search.specialist_league import claim, claim_path, gate_path, slug, specialist_dir

HELPER_ROOT = '/opt/league'
CONTAINER_ENV = {'DIGIMON_ONNX_OPPONENT': '0', 'DIGIMON_SELFPLAY_TRACE_DECISIONS': '1',
                 'DIGIMON_SEARCH_EVAL_CACHE_ENTRIES': '0', 'OMP_NUM_THREADS': '1',
                 'MKL_NUM_THREADS': '1', 'OPENBLAS_NUM_THREADS': '1'}


def log(message: str):
    print(time.strftime('%Y-%m-%d %H:%M:%S ') + message, flush=True)


def owner(name: str) -> str:
    return f'helper:{name}'


def eligible(root: Path, arch: str, gen: int) -> bool:
    if gate_path(root, arch, gen).exists() or claim_path(root, arch, gen).exists():
        return False
    state = json.loads((specialist_dir(root, arch) / 'state.json').read_text())
    if state['next_gen'] <= gen:
        return False
    return gen == 1 or gate_path(root, arch, gen - 1).exists()


def next_task(root: Path, plan: dict, name: str):
    for gen in range(1, plan['generations'] + 1):
        for arch in reversed(plan['archetypes']):
            if eligible(root, arch, gen) and claim(root, arch, gen, owner(name)):
                return gen, arch
    return None


def champion_generation(root: Path, arch: str, gen: int) -> int:
    if gen == 0:
        return 0
    return json.loads(gate_path(root, arch, gen).read_text())['champion_after']['generation']


def inputs_for(root: Path, arch: str, gen: int) -> list[str]:
    """Paths (relative to the league root) a gate reads."""
    rels = ['league-plan.json', 'inputs/decks.json']
    rels += [p.relative_to(root).as_posix() for p in (root / 'inputs/opponents').iterdir() if p.is_file()]
    run = specialist_dir(root, arch).relative_to(root).as_posix()
    for g in {gen, champion_generation(root, arch, gen - 1)}:
        rels += [f'{run}/gen{g}/{f.name}' for f in (root / run / f'gen{g}').iterdir()
                 if f.is_file() and f.name.startswith('model.')]
    if gen > 1:
        rels.append(gate_path(root, arch, gen - 1).relative_to(root).as_posix())
    return rels


class Helper:
    def __init__(self, name: str, host: str, args):
        self.name, self.host, self.args = name, host, args
        self.busy = False
        self.cooldown_until = 0.0

    def ssh(self, command: str, timeout: float | None = None):
        return subprocess.run(['ssh', '-i', self.args.ssh_key, '-o', 'StrictHostKeyChecking=accept-new',
                               '-o', 'ServerAliveInterval=60', f'root@{self.host}', command],
                              check=True, timeout=timeout)

    def rsync(self, *paths: str):
        subprocess.run(['rsync', '-a', '-e', f'ssh -i {self.args.ssh_key} -o StrictHostKeyChecking=accept-new',
                        *paths], check=True, timeout=1800)

    def run(self, root: Path, gen: int, arch: str):
        name = f'gate-{slug(arch)}-gen{gen}'
        try:
            started = time.monotonic()
            log(f'{self.name}: gate gen {gen} {arch} starting')
            self.ssh(f'mkdir -p {HELPER_ROOT}')
            self.rsync('-R', *[f'{root}/./{rel}' for rel in inputs_for(root, arch, gen)],
                       f'root@{self.host}:{HELPER_ROOT}/')
            env = ' '.join(f'-e {k}={v}' for k, v in {**CONTAINER_ENV, 'DIGIMON_DEPLOYED_IMAGE': self.args.image}.items())
            command = (f'docker rm -f {name} >/dev/null 2>&1; docker run --rm --init --name {name} --network=none '
                       f'--cpus={self.args.cpus} --memory={self.args.memory} {env} '
                       f'--mount type=bind,src={HELPER_ROOT},dst=/experiment {self.args.image} '
                       f'python -m tools.cloud_search.specialist_league gate-one '
                       f'--arch {shlex.quote(arch)} --gen {gen}')
            try:
                self.ssh(command, timeout=self.args.timeout_seconds)
            except subprocess.TimeoutExpired:
                self.ssh(f'docker rm -f {name}', timeout=120)
                raise
            folder = gate_path(root, arch, gen).parent
            folder.mkdir(parents=True, exist_ok=True)
            remote = f'root@{self.host}:{HELPER_ROOT}/gates/gen{gen}/{slug(arch)}'
            self.rsync(f'{remote}/', str(folder / slug(arch)) + '/')
            self.rsync(f'{remote}.json', str(folder) + '/')
            record = json.loads(gate_path(root, arch, gen).read_text())
            log(f'{self.name}: gate gen {gen} {arch}: challenger {record["challenger_score"]:.3f} vs champion '
                f'gen{record["champion_before"]["generation"]} {record["champion_score"]:.3f} -> '
                f'{"PROMOTED" if record["promoted"] else "kept champion"} '
                f'({time.monotonic() - started:.0f}s)')
        except Exception as error:  # release the claim; the controller runs it
            log(f'{self.name}: gate gen {gen} {arch} FAILED: {error!r}; releasing claim')
            if not gate_path(root, arch, gen).exists():
                claim_path(root, arch, gen).unlink(missing_ok=True)
            self.cooldown_until = time.monotonic() + self.args.cooldown_seconds
        finally:
            self.busy = False


def release_stale_claims(root: Path, names: set[str]):
    """Claims a previous coordinator left behind for gates nobody is running."""
    for path in (root / 'gates' / 'claims').glob('gen*/*'):
        holder = path.read_text().strip()
        gen = int(path.parent.name[3:])
        if holder.startswith('helper:'):
            records = list((root / 'gates' / f'gen{gen}').glob(f'{path.name}.json'))
            if not records:
                log(f'releasing stale claim {path} ({holder})')
                path.unlink()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--helpers', type=Path, required=True, help='JSON list of {name, host}')
    parser.add_argument('--image', required=True)
    parser.add_argument('--ssh-key', required=True)
    parser.add_argument('--cpus', default='8')
    parser.add_argument('--memory', default='14g')
    parser.add_argument('--timeout-seconds', type=float, default=3 * 3600)
    parser.add_argument('--cooldown-seconds', type=float, default=600)
    parser.add_argument('--poll-seconds', type=float, default=30)
    args = parser.parse_args()
    root = args.root
    helpers = [Helper(h['name'], h['host'], args) for h in json.loads(args.helpers.read_text())]
    release_stale_claims(root, {h.name for h in helpers})
    log(f'coordinating {len(helpers)} helper(s): {", ".join(h.name for h in helpers)}')
    while True:
        plan = json.loads((root / 'league-plan.json').read_text())
        if plan['image'] != args.image:
            raise SystemExit(f'league image {plan["image"]} differs from {args.image}')
        state = json.loads((root / 'league_state.json').read_text())
        if state.get('stage') == 'complete' and not any(h.busy for h in helpers):
            log('league complete')
            return
        for helper in helpers:
            if helper.busy or time.monotonic() < helper.cooldown_until:
                continue
            task = next_task(root, plan, helper.name)
            if task is None:
                continue
            helper.busy = True
            threading.Thread(target=helper.run, args=(root, *task), daemon=True).start()
        time.sleep(args.poll_seconds)


if __name__ == '__main__':
    main()
