"""Frozen CP12/CP8 policy duel and paired analysis of their recorded anchors.

Uses production masked SB3 evaluation. Four-game blocks swap the two models
between fixed decks/seats and use both opening orders. No training or promotion.
"""
import argparse
from collections import Counter, defaultdict
import gzip
import html
import json
import math
import os
from pathlib import Path
import subprocess
import sys
import time

from tools.selfplay_scheduler import atomic_json, sha256, controller_lock
from tools.cloud_search.local_policy_matrix import runtime_files, reference_differences

HASHES = {8: 'ea7d8447434003d9a969bc612087b54248204528f378893b19f3ff70a7bc6d5c',
          12: '4836d024b87806400287c8c09c0486243df0a5dc8d458ef73e5aef76e5517f1f'}
BAD_REASONS = {'evaluation_end', 'reset_before_terminal', 'step_limit', 'truncated', 'crash'}


def read(path):
    return json.loads(Path(path).read_text(encoding='utf-8'))


def runtime():
    files = runtime_files()
    for p in [Path(__file__), Path(__file__).with_name('analyze_page_search_results.py')]:
        files[str(p.resolve())] = sha256(p)
    return files


def make_schedule(pool, seed=94261029):
    from digimon_gym.agents.anchored_eval import balanced_deck_pairs, _seat_balanced_seed
    pairs = list(balanced_deck_pairs(pool, 100, seed))
    blocks = []
    for i in range(0, len(pairs), 2):
        a, b = pairs[i]
        assert pairs[i] == pairs[i + 1]
        rows = []
        for seat in (1, 2):
            for opener in (0, 1):
                rows.append(dict(id=f'{i//2:04d}-{seat}-{opener}', block=i//2,
                    seed=_seat_balanced_seed(seed, i + opener), cp12_seat=seat,
                    deck1_id=a.deck_id, deck2_id=b.deck_id,
                    deck1_archetype=a.archetype_name, deck2_archetype=b.archetype_name))
        blocks.append(dict(id=i//2, rows=rows))
    return blocks


def checked_outcome(row, game):
    if game['seed'] != row['seed'] or game.get('context') != row:
        raise ValueError('Recorded game differs from frozen schedule')
    if not game.get('terminal_reason') or game['terminal_reason'] in BAD_REASONS:
        raise ValueError('Incomplete game is not a draw')
    winner = game['winner_id']
    if winner not in (None, 0, 1, 2):
        raise ValueError('Invalid winner')
    seat = row['cp12_seat']
    return dict(row, winner=winner, terminal_reason=game['terminal_reason'],
        score=.5 if winner in (None, 0) else float(winner == seat),
        first=(seat == (1 if row['seed'] % 2 == 0 else 2)),
        cp12_archetype=row[f'deck{seat}_archetype'],
        cp8_archetype=row[f'deck{3-seat}_archetype'])


def interval(rows, samples=10000):
    import numpy as np
    groups = defaultdict(list)
    for r in rows:
        groups[r['block']].append(r['score'])
    sums = np.array([sum(v) for v in groups.values()])
    sizes = np.array([len(v) for v in groups.values()])
    rng = np.random.default_rng(4729)
    means = []
    for start in range(0, samples, 200):
        ix = rng.integers(len(sums), size=(min(200, samples-start), len(sums)))
        means.extend(sums[ix].sum(axis=1) / sizes[ix].sum(axis=1))
    lo, hi = np.quantile(means, [.025, .975])
    n = len(rows); p = sum(r['score'] for r in rows) / n
    # Avoid degenerate bounds for tiny/all-one-outcome subsets.
    z = 1.95996398454; den = 1 + z*z/n
    center = (p + z*z/(2*n))/den
    radius = z*math.sqrt(p*(1-p)/n+z*z/(4*n*n))/den
    return [float(min(lo, center-radius)), float(max(hi, center+radius))]


def counts(rows, confidence=True):
    n = len(rows)
    result = dict(games=n, wins=sum(r['score'] == 1 for r in rows),
                  draws=sum(r['score'] == .5 for r in rows),
                  losses=sum(r['score'] == 0 for r in rows),
                  score=sum(r['score'] for r in rows)/n)
    if confidence:
        result['ci95'] = interval(rows)
    return result


def summarize(blocks, receipts):
    expected = {b['id']: b for b in blocks}
    if len(receipts) != len(expected) or {r['block'] for r in receipts} != set(expected):
        raise ValueError('Missing or duplicate block receipts')
    rows = []
    for receipt in receipts:
        block = expected[receipt['block']]
        if len(receipt['games']) != 4:
            raise ValueError('Incomplete four-game block')
        for want, got in zip(block['rows'], receipt['games']):
            for k, v in want.items():
                if got[k] != v:
                    raise ValueError('Result changed frozen request')
            rebuilt = checked_outcome(want, dict(seed=got['seed'], context=want,
                winner_id=got['winner'], terminal_reason=got['terminal_reason']))
            if rebuilt != got:
                raise ValueError('Wrong model, seat or outcome perspective')
            rows.append(got)
    names = sorted({r['cp12_archetype'] for r in rows})
    return dict(status='complete', perspective='checkpoint12', **counts(rows),
        first=counts([r for r in rows if r['first']]),
        second=counts([r for r in rows if not r['first']]),
        engine_player1=counts([r for r in rows if r['cp12_seat'] == 1]),
        engine_player2=counts([r for r in rows if r['cp12_seat'] == 2]),
        archetypes={n: counts([r for r in rows if r['cp12_archetype'] == n]) for n in names},
        matrix=[dict(cp12_deck=a, cp8_deck=b,
            **counts([r for r in rows if r['cp12_archetype'] == a and r['cp8_archetype'] == b], False))
            for a in names for b in names
            if any(r['cp12_archetype'] == a and r['cp8_archetype'] == b for r in rows)],
        confidence_method='10000 bootstrap resamples of four-game role/opener blocks; Wilson interval floor',
        archetype_caveat='Exploratory 200-game subsets, unadjusted for multiple comparisons',
        search=False, training=False, automatic_promotion=False)


def matched_analysis(root, evidence):
    from tools.cloud_search.analyze_page_search_results import paired, score
    groups = {(r['checkpoint'], r['panel']): r for r in evidence['reports']}
    if len(groups) != 4:
        raise ValueError('Expected exactly two complete panels for each checkpoint')
    out = []
    for panel in ('combined_diagnostic', 'original_pet_regression'):
        a, b = groups[8, panel], groups[12, panel]
        for k in ('engine_image', 'runtime_cards_sha256', 'observation_profile', 'tensor_layout_hash',
                  'base_seed', 'cohort_scheme', 'snapshot_sha256', 'anchors_sha256', 'anchor_model_hashes'):
            if a['provenance'][k] != b['provenance'][k]:
                raise ValueError(f'Anchor comparison changed {k}')
        for r in (a, b):
            assert r['provenance']['status'] == 'complete'
            assert r['provenance']['candidate_sha256'] == HASHES[r['checkpoint']]
            assert all(g['terminal_reason'] and g['terminal_reason'] not in BAD_REASONS for g in r['games'])
        for anchor in sorted({g['context']['anchor'] for g in a['games']}):
            aa = [g for g in a['games'] if g['context']['anchor'] == anchor]
            bb = [g for g in b['games'] if g['context']['anchor'] == anchor]
            for arch in ['ALL'] + sorted({g['context']['agent_archetype'] for g in aa}):
                left = [g for g in aa if arch == 'ALL' or g['context']['agent_archetype'] == arch]
                right = [g for g in bb if arch == 'ALL' or g['context']['agent_archetype'] == arch]
                out.append(dict(panel=panel, anchor=anchor, archetype=arch,
                    cp8_score=sum(map(score, left))/len(left), cp12_score=sum(map(score, right))/len(right),
                    **paired(left, right)))
    result = dict(comparisons=out, source_reports=[{k:r[k] for k in ('checkpoint','panel','source','sha256')} for r in evidence['reports']],
                  method='Matched seeds, decks and fixed anchors; opening-order-pair bootstrap, 10000 resamples',
                  inference='Changes against fixed anchors, not a direct CP12 vs CP8 win rate',
                  caveat='Archetype intervals are exploratory and not adjusted for multiple comparisons')
    atomic_json(root/'matched-analysis.json', result)
    return result


def prepare(root, workers):
    import torch
    torch.set_num_threads(1)
    import digimon_engine
    from sb3_contrib import MaskablePPO
    from model_contract import validate_checkpoint
    from digimon_gym.agents.gauntlet import GeneralistDeckPool
    from digimon_gym.agents.anchored_eval import play_one
    from digimon_gym.digimon_gym import greedy_policy
    from digimon_gym.agents.card_eval import CardEvalCollector, save_report
    if (root/'plan.json').exists() or (root/'started.json').exists():
        raise ValueError('Refusing to overwrite or restart an evaluation')
    if not 1 <= workers <= 8:
        raise ValueError('Use 1-8 local workers')
    inputs = root/'inputs'
    for cp, digest in HASHES.items():
        assert sha256(inputs/f'checkpoint{cp}.zip') == digest
    evidence = json.loads(gzip.decompress((inputs/'matched-evaluations.json.gz').read_bytes()))
    assert sha256(inputs/'cards.json') == evidence['reports'][0]['provenance']['runtime_cards_sha256']
    assert sha256(inputs/'train_snapshot.json') == '13d6fd44a64c9725be77321015204862e34aafe77d63abc76eda34db6320b571'
    os.environ['DIGIMON_CARDS_JSON'] = str(inputs/'cards.json')
    os.environ['DIGIMON_ONNX_OPPONENT'] = '0'
    models = {cp: MaskablePPO.load(inputs/f'checkpoint{cp}.zip', device='cpu') for cp in HASHES}
    contracts = {cp: validate_checkpoint(m, 'standard_lite_v2') for cp, m in models.items()}
    assert contracts[8] == contracts[12]
    pool = GeneralistDeckPool.from_snapshot(inputs/'train_snapshot.json',
        implemented_card_ids=digimon_engine.load_implemented_card_ids())
    assert pool.archetype_count == 13 and pool.deck_count == 21
    checks = []
    for index, ref in enumerate(evidence['references']):
        folder = root/'preflight'/f'reference-{index:02d}'
        os.environ['DIGIMON_EVAL_FAILURE_DIR'] = str(folder/'failures')
        c = CardEvalCollector(ref['checkpoint'], index, digimon_engine.CardDatabase().get_card)
        c.context = ref['game']['context']
        play_one(models[ref['checkpoint']], greedy_policy, ref['deck1']['card_ids'], ref['deck2']['card_ids'],
                 ref['game']['seed'], contracts[12]['observation_profile'], collector=c)
        differences = reference_differences(ref['game'], c.games[-1])
        save_report(c.report(), folder)
        checks.append(dict(checkpoint=ref['checkpoint'], seed=ref['game']['seed'], differences=differences))
    atomic_json(root/'preflight/references.json', dict(checks=checks, passed=not any(c['differences'] for c in checks)))
    if len(checks) != 16 or any(c['differences'] for c in checks):
        raise ValueError('Local/cloud reference mismatch; evidence preserved')
    blocks = make_schedule(pool)
    assert len(blocks) == 650
    old_seeds = {g['seed'] for r in evidence['reports'] for g in r['games']}
    assert not old_seeds.intersection(r['seed'] for b in blocks for r in b['rows'])
    atomic_json(root/'schedule.json', blocks)
    matched_analysis(root, evidence)
    plan = dict(schema='local-checkpoint-duel-v1', workers=workers, games=2600, blocks=650,
        archetypes=pool.archetype_names, lists=21, games_per_archetype=200,
        contract=contracts[12], model_sha256=HASHES, source_image=evidence['reports'][0]['provenance']['engine_image'],
        reference_checks=checks, runtime=runtime(),
        files={p.relative_to(root).as_posix():sha256(p) for p in inputs.iterdir() if p.is_file()},
        schedule_sha256=sha256(root/'schedule.json'), search=False, training=False, automatic_promotion=False,
        cohort='Frozen 21 training lists, equal archetype weight; 650 role/opener blocks, not uniform matchup cells')
    atomic_json(root/'plan.json', plan)
    atomic_json(root/'state.json', dict(stage='prepared', games_completed=0, total_games=2600))
    print(json.dumps(dict(stage='prepared', games=2600, reference_games=16, workers=workers)), flush=True)


def validate(root):
    plan = read(root/'plan.json')
    assert runtime() == plan['runtime'], 'Runtime changed since preflight'
    assert sha256(root/'schedule.json') == plan['schedule_sha256']
    for name, digest in plan['files'].items():
        assert sha256(root/name) == digest, f'Frozen file changed: {name}'
    os.environ['DIGIMON_CARDS_JSON'] = str(root/'inputs/cards.json')
    os.environ['DIGIMON_ONNX_OPPONENT'] = '0'
    return plan


def worker(root, shard):
    import torch
    torch.set_num_threads(1)
    import digimon_engine
    from sb3_contrib import MaskablePPO
    from digimon_gym.agents.pilot_training import make_agent_opponent_fn
    from digimon_gym.agents.anchored_eval import play_one
    from digimon_gym.agents.card_eval import CardEvalCollector, save_report
    from tools.cloud_search.eval_progress import EvaluationProgress
    plan = validate(root)
    models = {cp:MaskablePPO.load(root/f'inputs/checkpoint{cp}.zip', device='cpu') for cp in HASHES}
    opponents = {cp:make_agent_opponent_fn(str(root/f'inputs/checkpoint{cp}.zip'), algorithm='mlp') for cp in HASHES}
    decks = {d['deck_id']:d for d in read(root/'inputs/train_snapshot.json')['decks']}
    blocks = [b for b in read(root/'schedule.json') if b['id'] % plan['workers'] == shard]
    progress = EvaluationProgress(root/f'worker-{shard}.json', total=len(blocks)*4, panel='cp12_vs_cp8', checkpoint=12)
    try:
        for block in blocks:
            folder = root/'blocks'/f"block-{block['id']:04d}"
            folder.mkdir(parents=True, exist_ok=False)
            os.environ['DIGIMON_EVAL_FAILURE_DIR'] = str(folder/'failures')
            collector = CardEvalCollector(12, block['id'], digimon_engine.CardDatabase().get_card)
            collector.provenance = dict(status='running', evaluation_mode='policy_only', model_sha256=plan['model_sha256'])
            results = []
            try:
                for row in block['rows']:
                    atomic_json(folder/'current-game.json', row)
                    collector.context = row
                    p1, p2 = (12,8) if row['cp12_seat'] == 1 else (8,12)
                    event = dict(anchor=f'checkpoint{p2}', archetype=row['deck1_archetype'],
                        opponent_archetype=row['deck2_archetype'], index=row['id'])
                    progress.event(dict(event, event='game_started'))
                    play_one(models[p1], opponents[p2], decks[row['deck1_id']]['card_ids'], decks[row['deck2_id']]['card_ids'],
                             row['seed'], plan['contract']['observation_profile'], collector=collector)
                    results.append(checked_outcome(row, collector.games[-1]))
                    progress.event(dict(event, event='game_completed'))
                collector.provenance['status'] = 'complete'
                save_report(collector.report(), folder/'reports')
                atomic_json(folder/'done.json', dict(block=block['id'], games=results, committed_unix=time.time()))
            except BaseException as error:
                collector.provenance.update(status='failed', error=str(error))
                save_report(collector.report(), folder/'reports')
                atomic_json(folder/'failed.json', dict(error=str(error), completed_games=results))
                raise
        validate(root)
        progress.finish('complete')
    except BaseException as error:
        progress.finish('failed', error)
        raise


def report(root):
    plan = read(root/'plan.json'); state = read(root/'state.json')
    result = read(root/'summary.json') if (root/'summary.json').exists() else None
    matched = read(root/'matched-analysis.json')['comparisons']
    body = f'<h1>Checkpoint 12 versus checkpoint 8</h1><p>{html.escape(state["stage"])}: {state["games_completed"]:,} / 2,600 policy-only games.</p>'
    if result:
        body += f'<p><b>CP12 score: {result["score"]:.2%}</b> (95% interval {result["ci95"][0]:.2%}–{result["ci95"][1]:.2%}); {result["wins"]} wins, {result["losses"]} losses, {result["draws"]} draws.</p>'
        body += '<p>Same frozen engine, cards and 21 training lists. Four-game blocks swap models between fixed deck seats and balance opening order. No search, training or model promotion.</p><table><tr><th>CP12 deck</th><th>W/L/D</th><th>Score</th><th>95% interval</th></tr>'
        for name, r in sorted(result['archetypes'].items(), key=lambda kv:kv[1]['score'], reverse=True):
            body += f'<tr><td>{html.escape(name)}</td><td>{r["wins"]}/{r["losses"]}/{r["draws"]}</td><td>{r["score"]:.1%}</td><td>{r["ci95"][0]:.1%}–{r["ci95"][1]:.1%}</td></tr>'
        body += '</table><p>200 games per archetype; these subsets are exploratory, with no multiple-comparison adjustment. Matchup-cell quotas differ slightly. Deck percentages also reflect matchup strength; use the matched fixed-anchor deltas to assess progress on a particular deck. This is model performance on known training lists, not a human strength rating.</p>'
        body += f'<p>CP12 going first: {result["first"]["score"]:.2%}; going second: {result["second"]["score"]:.2%}. Confidence: four-game-block bootstrap (10,000 resamples), with a Wilson interval floor.</p>'
    body += '<h2>Existing matched fixed-anchor evaluations</h2><p>These measure change against unchanged anchors. They are separate from the direct duel above.</p><table><tr><th>Panel</th><th>Anchor</th><th>CP8</th><th>CP12</th><th>Change, pp (95% interval)</th></tr>'
    for r in matched:
        if r['archetype'] == 'ALL':
            body += f'<tr><td>{r["panel"]}</td><td>{r["anchor"]}</td><td>{r["cp8_score"]:.2%}</td><td>{r["cp12_score"]:.2%}</td><td>{r["delta_pp"]:+.2f} ({r["ci95_low_pp"]:+.2f}, {r["ci95_high_pp"]:+.2f})</td></tr>'
    body += '</table><p>Matched game seeds and exact decks, grouped by opening-order pairs. Full per-archetype comparisons and input hashes are in matched-analysis.json and plan.json.</p>'
    (root/'index.html').write_text('<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>CP12 vs CP8 evaluation</title><style>body{font:16px system-ui;max-width:1000px;margin:24px auto;padding:0 16px;color:#19304c;background:#f6f8fb}table{border-collapse:collapse;width:100%;font-size:14px;background:white}th,td{border:1px solid #ced7e3;padding:8px;text-align:left}p{line-height:1.5}</style>'+body, encoding='utf-8')


def run(root):
    plan = validate(root)
    with (root/'started.json').open('x') as stream:
        json.dump(dict(pid=os.getpid(), time_unix=time.time()), stream)
    processes, logs = [], []
    started = time.time()
    try:
        for shard in range(plan['workers']):
            log = (root/f'worker-{shard}.log').open('x'); logs.append(log)
            processes.append(subprocess.Popen([sys.executable,'-m','tools.cloud_search.local_checkpoint_duel','worker','--root',str(root),'--shard',str(shard)], stdout=log, stderr=subprocess.STDOUT))
        atomic_json(root/'processes.json', dict(controller=os.getpid(), workers=[p.pid for p in processes]))
        while True:
            receipts = list((root/'blocks').glob('block-*/done.json'))
            if any(p.poll() not in (None, 0) for p in processes):
                raise RuntimeError('Worker failed; exact request, partial report and recording preserved under blocks')
            done = all(p.poll() == 0 for p in processes)
            if done and len(receipts) != plan['blocks']:
                raise ValueError('Missing committed results')
            state = dict(stage='finalizing' if done else 'running', games_completed=4*len(receipts),
                total_games=plan['games'], elapsed_seconds=time.time()-started, time_unix=time.time())
            atomic_json(root/'state.json', state)
            if done:
                validate(root)
                summary = summarize(read(root/'schedule.json'), [read(p) for p in receipts])
                summary['elapsed_seconds'] = time.time()-started
                atomic_json(root/'summary.json', summary)
                atomic_json(root/'state.json', dict(state, stage='complete', time_unix=time.time()))
            report(root)
            if done:
                print(json.dumps({k:summary[k] for k in ('status','games','wins','losses','draws','score','ci95','elapsed_seconds')}), flush=True)
                break
            time.sleep(5)
    except BaseException as error:
        previous = read(root/'state.json')
        atomic_json(root/'state.json', dict(previous,stage='failed',error=str(error),time_unix=time.time()))
        report(root)
        raise
    finally:
        for proc in processes:
            if proc.poll() is None:
                proc.terminate()
                try: proc.wait(timeout=15)
                except subprocess.TimeoutExpired: proc.kill(); proc.wait()
        for log in logs: log.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['prepare','run','worker','report'])
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--workers', type=int, default=4)
    parser.add_argument('--shard', type=int)
    args = parser.parse_args(); root = args.root.resolve()
    if args.command == 'worker': worker(root, args.shard)
    elif args.command == 'report': report(root)
    else:
        with controller_lock(root/'controller.lock'):
            if args.command == 'prepare': prepare(root, args.workers)
            else: run(root)


if __name__ == '__main__':
    main()
