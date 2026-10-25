"""Read-only parent/image validation before the generation-three restart.

Mount the stopped campaign at /experiment (read-only), this script and recorded
regressions at /diagnostic (writable). Never writes training inputs or weights.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

import numpy as np


def sha256(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def replay_checks(fixture):
    from digimon_gym.digimon_gym import DigimonEnv
    checks = []
    for case in json.loads(fixture.read_text())['games']:
        gid = case['game']
        if gid == 340:
            continue  # Native full-game check below; exact frozen-search regression is in Rust.
        env = DigimonEnv(deck1=case['deck1'], deck2=case['deck2'], tensor_profile='standard_lite_v2')
        env.reset(seed=case['seed'])
        count = 19 if gid == 521 else 45
        for index, step in enumerate(case['prefix'][:count]):
            assert env.current_player_id == step['player'] + 1, (gid, index, 'actor')
            assert env.action_mask()[step['action']], (gid, index, 'illegal replay action')
            env.runner.step(step['action'])
        if gid == 858:
            # Physical card IDs are public in the local headless debugging view.
            ui = env.runner.to_ui_json()
            assert 'EX12-004' not in ui['player1']['handIds']
            assert ui['pendingSelection'], 'Darumamon still offers its independent play choice'
        else:
            for _ in range(40):
                ui = env.runner.to_ui_json()
                if ui.get('pendingSelection') is None:
                    break
                legal = np.flatnonzero(env.action_mask())
                action = 62 if 62 in legal else int(legal[0])
                env.runner.step(action)
            ui = env.runner.to_ui_json()
            assert ui['pendingSelection'] is None
            assert ui['currentPhase'] == 3 and np.count_nonzero(env.action_mask()) > 1
        checks.append(dict(game=gid, prefix_decisions=count, status='passed'))
        env.close()
    return checks


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-commit', required=True)
    parser.add_argument('--root', type=Path, default=Path('/experiment'))
    parser.add_argument('--out', type=Path, default=Path('/diagnostic'))
    parser.add_argument('--fixture', type=Path, default=Path('/diagnostic/regressions.json'))
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    report = dict(status='running', image=os.environ['DIGIMON_DEPLOYED_IMAGE'],
        source_commit=args.source_commit,
        checkpoint_sha256=sha256(args.root / 'generation/gen2/model.zip'),
        validator_sha256=sha256(Path(__file__)), started_unix=time.time(),
        search=dict(simulations=320, worlds=4, batch=4, decision_cap=1200))
    path = args.out / 'validation.json'
    path.write_text(json.dumps(report, indent=2))
    try:
        import digimon_engine
        from sb3_contrib import MaskablePPO
        from model_contract import validate_checkpoint, validate_contract
        report['contract'] = validate_checkpoint(MaskablePPO.load(
            args.root / 'generation/gen2/model.zip', device='cpu'), 'standard_lite_v2')
        validate_contract(json.loads((args.root / 'generation/gen2/model.onnx.meta.json').read_text()),
                          'standard_lite_v2')
        report['card_eval_event_version'] = digimon_engine.CARD_EVAL_EVENT_VERSION
        report['replays'] = replay_checks(args.fixture)

        def native(label, seed, start, games):
            out = args.out / label
            cmd = ['digimon-engine-cli', '--pool', 'implemented', '--cards-json', '/app/data/cards.json',
                'selfplay', '--model', str(args.root / 'generation/gen2/model.onnx'),
                '--decks', str(args.root / 'inputs/decks.json'), '--games', str(games),
                '--game-start', str(start), '--seed', str(seed), '--sims', '320', '--worlds', '4',
                '--eval-batch-size', '4', '--temperature-decisions', '16', '--decision-cap', '1200',
                '--tensor-profile', 'standard_lite_v2', '--out', str(out)]
            with (args.out / f'{label}.log').open('w') as log:
                subprocess.run(cmd, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=1800)
            manifest = json.loads((out / 'manifest.json').read_text())
            assert manifest['games'] == games and manifest['truncated_games'] == 0, label
            result = dict(label=label, games=games, rows=manifest['rows'],
                          truncated_games=manifest['truncated_games'], seed=seed, game_start=start)
            print(json.dumps(result), flush=True)
            return result

        cases = [(f'regression-{gid}', 30561025, gid, 1) for gid in (340, 521, 858)]
        cases += [(f'health-{i}', 40361025, i, 4) for i in range(0, 24, 4)]
        with ThreadPoolExecutor(max_workers=4) as pool:
            report['native'] = list(pool.map(lambda values: native(*values), cases))
        report.update(status='passed', full_games=sum(c['games'] for c in report['native']))
    except BaseException as error:
        report.update(status='failed', error=f'{type(error).__name__}: {error}')
        raise
    finally:
        report['finished_unix'] = time.time()
        path.write_text(json.dumps(report, indent=2))
        print(json.dumps(report), flush=True)


if __name__ == '__main__':
    main()
