"""Frozen policy-only cross-deck, adjacent-checkpoint, and true-mirror evaluation.

Uses the production masked evaluator, records both players, and stops on any
incomplete game. Four-game blocks balance model roles and opening order;
true mirrors use two-game opening-order blocks with checkpoint 4 on both sides.
"""
import argparse
from collections import Counter, defaultdict
import itertools
import json
import os
from pathlib import Path
import subprocess
import sys
import time

from tools.selfplay_scheduler import atomic_json, sha256


def schedule(names, panel, seed, repetitions=10):
    if panel not in {'self', 'adjacent', 'mirror'} or repetitions < 1:
        raise ValueError('Invalid panel or repetitions')
    names = sorted(names)
    if len(set(names)) != len(names):
        raise ValueError('Duplicate archetypes')
    if panel == 'mirror':
        return [dict(index=2*block+opening, block=block,
                     seed=2*(seed+2*block+opening)+opening,
                     deck1=name, deck2=name, model1=4, model2=4)
                for block, name in enumerate(n for n in names for _ in range(repetitions))
                for opening in range(2)]
    pairs = (itertools.combinations(names, 2) if panel == 'self'
             else itertools.product(names, repeat=2))
    rows = []
    block = 0
    for a, b in pairs:
        for _ in range(repetitions):
            for swap in range(2):
                for opening in range(2):
                    rows.append(dict(index=len(rows), block=block,
                        seed=2*(seed+2*block+opening)+opening,
                        deck1=b if panel == 'self' and swap else a,
                        deck2=a if panel == 'self' and swap else b,
                        model1=3 if panel == 'adjacent' and swap else 4,
                        model2=3 if panel == 'adjacent' and not swap else 4))
            block += 1
    return rows


def summarize(rows, games, panel):
    import numpy as np
    if len(games) != len(rows) or len({g['index'] for g in games}) != len(rows):
        raise ValueError('Missing or duplicate games')
    expected = {r['index']: r for r in rows}
    matrix = defaultdict(Counter)
    blocks = defaultdict(list)
    for g in games:
        r = expected[g['index']]
        if any(g.get(k) != v for k, v in r.items()):
            raise ValueError('Game differs from frozen schedule')
        if g['winner'] not in (None, 0, 1, 2) or g['terminal_reason'] in {
                None, '', 'step_limit', 'truncated', 'crash', 'evaluation_end', 'reset_before_terminal'}:
            raise ValueError('Incomplete or invalid outcome')
        winner = g['winner']
        if panel == 'mirror':
            first = 1 if r['seed'] % 2 == 0 else 2
            matrix[r['deck1'], r['deck2']]['draws' if not winner else 'wins' if winner == first else 'losses'] += 1
            blocks[r['block']].append(.5 if not winner else float(winner == first))
        elif panel == 'self':
            for pid, a, b in [(1, r['deck1'], r['deck2']), (2, r['deck2'], r['deck1'])]:
                matrix[a, b]['draws' if not winner else 'wins' if winner == pid else 'losses'] += 1
        else:
            pid = 1 if r['model1'] == 4 else 2
            a, b = (r['deck1'], r['deck2']) if pid == 1 else (r['deck2'], r['deck1'])
            matrix[a, b]['draws' if not winner else 'wins' if winner == pid else 'losses'] += 1
            blocks[r['block']].append(.5 if not winner else float(winner == pid))
    cells, decks = [], defaultdict(Counter)
    for (a, b), c in sorted(matrix.items()):
        cells.append(dict(deck=a, opponent_deck=b, games=sum(c.values()),
                          **{k: c[k] for k in ['wins', 'losses', 'draws']}))
        decks[a].update(c)
    result = dict(status='complete', panel=panel, games=len(games), matrix=cells,
                  decks=[dict(deck=a, games=sum(c.values()),
                              score=(c['wins']+.5*c['draws'])/sum(c.values()), **dict(c))
                         for a, c in sorted(decks.items())], automatic_promotion=False)
    if panel in {'adjacent', 'mirror'}:
        block_size = 2 if panel == 'mirror' else 4
        if any(len(b) != block_size for b in blocks.values()):
            raise ValueError('Incomplete paired block')
        means = np.array([sum(b)/block_size for b in blocks.values()])
        rng = np.random.default_rng(10262026)
        samples = np.concatenate([means[rng.integers(len(means), size=(100, len(means)))].mean(axis=1)
                                  for _ in range(100)])
        totals = sum(decks.values(), Counter())
        result.update(perspective='first_player' if panel == 'mirror' else 'checkpoint4', score=float(means.mean()),
                      ci95=list(np.quantile(samples, [.025, .975])),
                      **{k: totals[k] for k in ['wins', 'losses', 'draws']},
                      bootstrap_unit=('two-game opening-order pair' if panel == 'mirror' else
                                      'four-game deck/seed block; model roles and opening order balanced'))
    return result


def prepare(source, root, mirrors=False):
    import digimon_engine
    from sb3_contrib import MaskablePPO
    from model_contract import validate_checkpoint
    from digimon_gym.agents.gauntlet import GeneralistDeckPool
    state = json.loads((source/'experiment_state.json').read_text())
    if state['stage'] != 'complete' or state['generation_index'] != 4:
        raise ValueError('Source four-generation run must be complete')
    if state['image'] != os.environ['DIGIMON_DEPLOYED_IMAGE']:
        raise ValueError('Native engine differs from completed evaluation')
    if (root/'plan.json').exists() or (root/'started.json').exists():
        raise ValueError('Refusing to overwrite evaluation')
    pools = [GeneralistDeckPool.from_snapshot(source/'inputs'/name,
              implemented_card_ids=digimon_engine.load_implemented_card_ids())
             for name in ['heldout_snapshot.json', 'pet_evaluation_snapshot.json']]
    if set(pools[0].archetypes) & set(pools[1].archetypes):
        raise ValueError('Cohorts overlap')
    pool = GeneralistDeckPool({**pools[0].archetypes, **pools[1].archetypes})
    if len(pool.archetypes) != 13 or any(len(v) != 1 for v in pool.archetypes.values()):
        raise ValueError('Expected 13 frozen evaluation lists')
    root.mkdir(parents=True, exist_ok=True)
    pool.write_snapshot(root/'pool.json')
    hashes, contracts = {}, {}
    for i in [3, 4]:
        model = source/'generation'/f'gen{i}'/'model.zip'
        context = json.loads((source/'evaluations'/f'window-{1000+i:06d}'/'context.json').read_text())
        if context['engine_image'] != state['image']:
            raise ValueError('Checkpoint evaluated on a different engine')
        contracts[str(i)] = validate_checkpoint(MaskablePPO.load(model, device='cpu'))
        hashes[str(i)] = sha256(model)
        import shutil
        shutil.copy2(model, root/f'checkpoint{i}.zip')
        if sha256(root/f'checkpoint{i}.zip') != hashes[str(i)]:
            raise ValueError('Checkpoint changed during copy')
    if contracts['3'] != contracts['4']:
        raise ValueError('Model contracts differ')
    panels = [('mirror', 23261026)] if mirrors else [('self', 21261026), ('adjacent', 22261026)]
    for panel, seed in panels:
        atomic_json(root/f'{panel}-schedule.json', schedule(pool.archetype_names, panel, seed, 100 if mirrors else 10))
    files = ['pool.json', 'checkpoint3.zip', 'checkpoint4.zip']+[f'{p}-schedule.json' for p, _ in panels]
    plan = dict(image=state['image'], models=hashes, contract=contracts['4'],
                files={f: sha256(root/f) for f in files}, controller_sha256=sha256(Path(__file__)),
                cards_sha256=sha256(Path('/app/data/cards.json')), search=False, training=False,
                automatic_promotion=False, panels=[p for p, _ in panels],
                mirror_games=2600 if mirrors else 0, self_games=0 if mirrors else 3120,
                adjacent_games=0 if mirrors else 6760, workers_per_panel=4)
    atomic_json(root/'plan.json', plan)
    print(json.dumps(plan), flush=True)


def validate(root):
    plan = json.loads((root/'plan.json').read_text())
    if plan['image'] != os.environ['DIGIMON_DEPLOYED_IMAGE'] or plan['controller_sha256'] != sha256(Path(__file__)):
        raise ValueError('Runtime differs from prepared evaluation')
    for f, digest in plan['files'].items():
        if sha256(root/f) != digest:
            raise ValueError(f'Frozen input changed: {f}')
    if sha256(Path('/app/data/cards.json')) != plan['cards_sha256']:
        raise ValueError('Card database changed')
    return plan


def worker(root, panel, shard):
    import torch
    torch.set_num_threads(1)
    import digimon_engine
    from sb3_contrib import MaskablePPO
    from digimon_gym.agents.anchored_eval import play_one
    from digimon_gym.agents.pilot_training import make_agent_opponent_fn
    from digimon_gym.agents.card_eval import CardEvalCollector, save_report
    from digimon_gym.agents.gauntlet import GeneralistDeckPool
    from tools.cloud_search.eval_progress import EvaluationProgress
    plan = validate(root)
    rows = [r for r in json.loads((root/f'{panel}-schedule.json').read_text())
            if r['block'] % plan['workers_per_panel'] == shard]
    folder = root/panel/f'shard{shard}'
    folder.mkdir(parents=True, exist_ok=False)
    os.environ['DIGIMON_EVAL_FAILURE_DIR'] = str(folder/'failures')
    pool = GeneralistDeckPool.from_snapshot(root/'pool.json', implemented_card_ids=digimon_engine.load_implemented_card_ids())
    decks = {k: v[0] for k, v in pool.archetypes.items()}
    indices = [3, 4] if panel == 'adjacent' else [4]
    models = {i: MaskablePPO.load(root/f'checkpoint{i}.zip', device='cpu') for i in indices}
    opponents = {i: make_agent_opponent_fn(str(root/f'checkpoint{i}.zip'), algorithm='mlp') for i in indices}
    collector = CardEvalCollector(4, 2000+(0 if panel == 'self' else 10)+shard, digimon_engine.CardDatabase().get_card)
    collector.provenance = dict(plan, panel=panel, shard=shard, evaluation_mode='policy_only',
                                deterministic=True, status='running')
    progress = EvaluationProgress(folder/'progress.json', total=len(rows), panel=panel, checkpoint=4)
    outcomes = []
    try:
        for row in rows:
            a, b = decks[row['deck1']], decks[row['deck2']]
            collector.context = dict(row, panel=panel, anchor=f"checkpoint{row['model2']}",
                agent_archetype=a.archetype_name, opponent_archetype=b.archetype_name,
                agent_deck_id=a.deck_id, opponent_deck_id=b.deck_id, evaluation_mode='policy_only')
            event = dict(anchor=collector.context['anchor'], archetype=row['deck1'], opponent_archetype=row['deck2'], index=row['index'])
            progress.event(dict(event, event='game_started'))
            winner = play_one(models[row['model1']], opponents[row['model2']], a.card_ids, b.card_ids,
                              row['seed'], plan['contract']['observation_profile'], collector=collector)
            outcomes.append(dict(row, winner=winner, terminal_reason=collector.games[-1]['terminal_reason']))
            progress.event(dict(event, event='game_completed'))
        collector.provenance['status'] = 'complete'
        atomic_json(folder/'outcomes.json', outcomes)
        progress.finish('complete')
    except BaseException as error:
        collector.provenance.update(status='failed', error=str(error))
        progress.finish('failed', error)
        raise
    finally:
        save_report(collector.report(), folder/'reports')


def run(root):
    validate(root)
    with (root/'started.json').open('x') as f:
        json.dump(dict(time_unix=time.time()), f)
    atomic_json(root/'state.json', dict(stage='running', time_unix=time.time()))
    processes, logs = [], []
    try:
        for panel in validate(root).get('panels', ['self', 'adjacent']):
            for shard in range(4):
                log = (root/f'{panel}-{shard}.log').open('w')
                logs.append(log)
                processes.append(subprocess.Popen([sys.executable, __file__, '--root', str(root),
                    '--worker', '--panel', panel, '--shard', str(shard)], stdout=log, stderr=subprocess.STDOUT))
        while any(p.poll() is None for p in processes):
            if any(p.poll() not in (None, 0) for p in processes):
                raise RuntimeError('Evaluator failed; preserve shard logs and recordings')
            time.sleep(2)
        if any(p.returncode for p in processes):
            raise RuntimeError('Evaluator failed')
        for panel in validate(root).get('panels', ['self', 'adjacent']):
            rows = json.loads((root/f'{panel}-schedule.json').read_text())
            games = [g for shard in range(4) for g in json.loads((root/panel/f'shard{shard}'/'outcomes.json').read_text())]
            atomic_json(root/f'{panel}-result.json', summarize(rows, games, panel))
        atomic_json(root/'state.json', dict(stage='complete', time_unix=time.time(), automatic_promotion=False))
    except BaseException as error:
        atomic_json(root/'state.json', dict(stage='failed', error=str(error), time_unix=time.time()))
        raise
    finally:
        for p in processes:
            if p.poll() is None:
                p.terminate()
                try:
                    p.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    p.kill()
                    p.wait()
        for log in logs:
            log.close()


if __name__ == '__main__':
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--root', type=Path, default=Path('/evaluation'))
    ap.add_argument('--source', type=Path, default=Path('/source'))
    ap.add_argument('--prepare', action='store_true')
    ap.add_argument('--mirrors', action='store_true')
    ap.add_argument('--worker', action='store_true')
    ap.add_argument('--panel', choices=['self', 'adjacent', 'mirror'])
    ap.add_argument('--shard', type=int)
    args = ap.parse_args()
    if args.prepare:
        prepare(args.source, args.root, args.mirrors)
    elif args.worker:
        worker(args.root, args.panel, args.shard)
    else:
        run(args.root)
