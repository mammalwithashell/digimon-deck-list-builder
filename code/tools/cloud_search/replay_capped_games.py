"""Read-only replay audit of two committed cloud games that reached the cap.

Trace the temperature-sampled prefix, then reconstruct the deterministic suffix
from committed visit targets, verifying every saved observation before acting.
Production data/model mounts must be read-only; all output goes to /diagnostic.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

import numpy as np

from digimon_gym.digimon_gym import DigimonEnv

ROOT = Path('/experiment')
OUT = Path('/diagnostic/truncations')


def audit(gid, chunk, generation, model):
    out = OUT / str(gid)
    out.mkdir(parents=True, exist_ok=False)
    source = ROOT / 'generation' / f'gen{generation}' / 'shards' / f'proc{chunk:08d}'
    manifest = json.loads((source / 'manifest.json').read_text())
    from tools.selfplay_scheduler import _read_commit
    _read_commit(source, manifest['game_start'], manifest['games'], manifest['seed'])
    cmd = ['digimon-engine-cli', '--cards-json', '/app/data/cards.json', '--pool', 'implemented',
           'selfplay', '--model', str(model), '--decks', str(ROOT / 'inputs/decks.json'),
           '--games', '1', '--game-start', str(gid), '--seed', str(manifest['seed']),
           '--sims', '320', '--worlds', '4', '--eval-batch-size', '4', '--temperature-decisions', '16',
           '--decision-cap', '40', '--tensor-profile', 'standard_lite_v2', '--out', str(out / 'prefix')]
    with (out / 'prefix.log').open('w') as log:
        subprocess.run(cmd, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=900,
                       env={**os.environ, 'DIGIMON_SELFPLAY_TRACE_DECISIONS': '1'})
    text = (out / 'prefix.log').read_text()
    _, seed, di, dj = map(int, re.search(r'game (\d+) start: seed (\d+), decks (\d+) vs (\d+)', text).groups())
    prefix = [(int(d), int(a)) for d, a in re.findall(r'decision (\d+): (?:forced|search) action (\d+)', text)]
    assert [d for d, _ in prefix] == list(range(len(prefix)))
    obs, targets = [], []
    for shard in manifest['shards']:
        p = source / shard['dir']
        ids = np.load(p / 'game_id.npy')
        indices = np.flatnonzero(ids == gid)
        obs.extend(np.load(p / 'obs.npy', mmap_mode='r')[indices])
        offsets = np.load(p / 'policy_offsets.npy')
        actions, visits = np.load(p / 'policy_actions.npy'), np.load(p / 'policy_visits.npy')
        for i in indices:
            targets.append((actions[offsets[i]:offsets[i+1]], visits[offsets[i]:offsets[i+1]]))
    decks = json.loads((ROOT / 'inputs/decks.json').read_text())
    env = DigimonEnv(deck1=decks[di]['cards'], deck2=decks[dj]['cards'],
                    tensor_profile='standard_lite_v2', max_turns=1200)
    env.reset(seed=seed)
    row, states, steps = 0, {}, []
    for decision in range(manifest['decision_cap']):
        if env.is_game_over:
            raise RuntimeError('Truncated source replay ended early')
        mask = env.action_mask()
        legal = np.flatnonzero(mask)
        if decision < len(prefix):
            action = prefix[decision][1]
        elif len(legal) == 1:
            action = int(legal[0])
        else:
            assert row >= manifest['temperature_decisions']
            actions, visits = targets[row]
            assert visits.sum() > 0
            action = int(actions[np.argmax(visits)])
        assert mask[action]
        ui = env.runner.to_ui_json()
        digest = hashlib.sha256(json.dumps(ui, sort_keys=True).encode()).hexdigest()
        if digest not in states:
            states[digest] = dict(first_decision=decision, count=0, ui=ui, legal=legal.tolist())
        states[digest]['count'] += 1
        if len(legal) > 1:
            actual = env.runner.get_board_tensor(env.current_player_id)
            if not np.array_equal(actual, obs[row]):
                raise RuntimeError(f'Observation mismatch at decision {decision}, saved row {row}')
            row += 1
        steps.append(dict(decision=decision, action=action, ui_hash=digest, saved_rows_seen=row,
                          player=env.current_player_id, phase=ui['currentPhase'], turn=ui['turnCount']))
        # Reproduce the native self-play runner directly. DigimonEnv.step has
        # an additional Python no-op tripwire; stopping there would prevent us
        # from auditing the rest of a native game's committed trajectory.
        env.runner.step(action)
    assert row == len(obs)
    report = dict(game=gid, decks=[manifest['deck_names'][di], manifest['deck_names'][dj]],
                  decisions=len(steps), saved_positions=row, original_observations_match=True,
                  source_generation=generation, model_sha256=hashlib.sha256(model.read_bytes()).hexdigest(),
                  unique_ui_states=len(states), repeated_states=sorted(
                      [dict(hash=k, count=v['count'], first_decision=v['first_decision']) for k,v in states.items()],
                      key=lambda s: -s['count'])[:10],
                  prefix_decisions=len(prefix), states=states, steps=steps, final=env.runner.to_ui_json())
    (out / 'replay.json').write_text(json.dumps(report, indent=2))
    print(json.dumps({k:v for k,v in report.items() if k not in ('states','steps','final')}), flush=True)
    env.close()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path('/experiment'))
    parser.add_argument('--out', type=Path, default=Path('/diagnostic/truncations'))
    parser.add_argument('--generation', type=int, required=True)
    parser.add_argument('--model', type=Path, required=True)
    parser.add_argument('--games', type=int, nargs='+', required=True)
    args = parser.parse_args()
    ROOT, OUT = args.root, args.out
    chunks = [json.loads(p.read_text()) for p in
              (ROOT / 'generation' / f'gen{args.generation}' / 'shards').glob('proc*/manifest.json')]
    for game in args.games:
        matches = [m for m in chunks if m['game_start'] <= game < m['game_start'] + m['games']]
        if len(matches) != 1:
            raise ValueError(f'Expected exactly one committed chunk for game {game}')
        audit(game, matches[0]['game_start'], args.generation, args.model)
