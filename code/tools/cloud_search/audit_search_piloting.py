"""Bounded, seat-paired mirror screen: frozen policy with/without search.

Diagnostic only. No training inputs are written. Incomplete pairs never count
as draws; per-archetype samples are exploratory and cannot certify proficiency.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path, value):
    temporary = path.with_suffix('.tmp')
    temporary.write_text(json.dumps(value, indent=2), encoding='utf-8')
    temporary.replace(path)


def run_pair(engine, request, target, timeout=1800):
    """Keep wall time, including failed attempts, separate from engine timing."""
    started = time.perf_counter()
    try:
        with target.with_suffix('.log').open('w') as log:
            subprocess.run([str(engine), 'match', str(request), str(target)],
                stdout=log, stderr=subprocess.STDOUT, check=True, timeout=timeout,
                env={**os.environ, 'OMP_NUM_THREADS': '1', 'MKL_NUM_THREADS': '1',
                     'OPENBLAS_NUM_THREADS': '1', 'DIGIMON_SEARCH_EVAL_CACHE_ENTRIES': '0'})
        result = json.loads(target.read_text())
        games = result['games']
        if (len(games) != 2 or {g['candidate_seat'] for g in games} != {0, 1}
                or sum(g['first_player'] == g['candidate_seat'] for g in games) != 1):
            raise ValueError('Pair is not seat/opening balanced')
        result = dict(result, status='incomplete' if any(g['truncated'] for g in games) else 'complete')
    except (subprocess.SubprocessError, ValueError, OSError, KeyError, TypeError) as error:
        result = dict(status='failed', error=str(error), games=[])
    return dict(result, process_wall_seconds=time.perf_counter() - started)


def summarize(rows, names, expected_pairs_per_archetype=None):
    def group(items, expected):
        complete = [r for r in items if r['status'] == 'complete']
        games = [g for r in complete for g in r['games']]
        wins = sum(g['winner'] == g['candidate_seat'] for g in games)
        draws = sum(g['winner'] is None for g in games)
        return dict(pairs=len(complete), valid_pairs=len(complete), attempted_pairs=len(items),
            expected_pairs=expected,
            coverage_complete=len(complete) == expected if expected is not None else None,
            games=len(games), wins=wins, draws=draws,
            losses=len(games)-wins-draws, score=(wins + .5*draws)/len(games) if games else None,
            incomplete_pairs=sum(r['status'] != 'complete' for r in items),
            failed_pairs=sum(r['status'] == 'failed' for r in items),
            truncated_pairs=sum(r['status'] == 'incomplete' for r in items),
            paired_scores=[sum(g['score'] for g in r['games'])/2 for r in complete],
            searched_decisions=sum(g['searched'] for g in games),
            decisions=sum(g['decisions'] for g in games),
            engine_gameplay_seconds=sum(r.get('seconds', 0) for r in items),
            process_wall_seconds=sum(r.get('process_wall_seconds', 0) for r in items))
    total = expected_pairs_per_archetype * len(names) if expected_pairs_per_archetype is not None else None
    return dict(overall=group(rows, total), archetypes={n:group(
        [r for r in rows if r['archetype']==n], expected_pairs_per_archetype) for n in names})


def completion_status(summary):
    return 'complete' if summary['overall']['coverage_complete'] else 'finished_with_incomplete'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--engine', type=Path, required=True)
    parser.add_argument('--model', type=Path, required=True)
    parser.add_argument('--snapshot', type=Path, required=True)
    parser.add_argument('--cards', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--pairs', type=int, default=4)
    parser.add_argument('--workers', type=int, default=6)
    args = parser.parse_args()
    if args.pairs < 1 or not 1 <= args.workers <= 8:
        parser.error('Positive pair count and 1..8 workers required')
    root = args.out.resolve()
    root.mkdir(parents=True, exist_ok=False)
    (root/'requests').mkdir()
    (root/'results').mkdir()
    snapshot = json.loads(args.snapshot.read_text(encoding='utf-8'))
    decks = sorted(snapshot['decks'], key=lambda d:d['archetype'])
    names = [d['archetype'] for d in decks]
    if len(names) != len(set(names)):
        raise ValueError('Screen expects one frozen evaluation list per archetype')
    assets = [p.resolve() for p in (args.engine,args.model,args.snapshot,args.cards,Path(str(args.model)+'.meta.json'))]
    hashes = {str(p):digest(p) for p in assets}
    source = subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()
    plan = dict(schema='archetype-policy-search-screen-v1', diagnostic_only=True, source_commit=source,
        driver_sha256=digest(Path(__file__)),
        engine=str(args.engine.resolve()), model=str(args.model.resolve()), cards_json=str(args.cards.resolve()),
        decision_cap=1200, pairs_per_archetype=args.pairs, games=2*args.pairs*len(names), workers=args.workers,
        simulations=320, worlds=4, evaluation_batch=4, noise=0, seed=33261026, hashes=hashes,
        decks=[dict(name=d['archetype'],cards=d['card_ids']) for d in decks],
        limitations=['Confirmed End of Attack timing defects remain in this diagnostic engine; no rules-faithful strength or promotion claim.',
            'Mirrors isolate own-list piloting; no cross-deck or real-meta strength claim.',
            'Four paired seeds per archetype are an exploratory screen, not a precise estimate.',
            'Exact-composition open-decklist PIMC; not a closed-decklist benchmark.',
            'Incomplete pairs are excluded from strength; search visits and policy probabilities use first-max ties.'])
    write(root/'plan.json',plan)
    jobs = [(rep,index,name) for rep in range(args.pairs) for index,name in enumerate(names)]
    started = time.time()
    completed = []
    def worker(job):
        rep,index,name=job
        key=f'{index:02d}-{rep:02d}'
        request=dict(plan=str(root/'plan.json'),candidate=dict(id='s320-w4',simulations=320,worlds=4),
            opponent=dict(id='policy',kind='policy'),deck_indices=[index,index],
            game_seed=plan['seed']+10000*index+rep, search_seed=plan['seed']+1000000+10000*index+rep,
            card_metrics=False)
        write(root/'requests'/f'{key}.json',request)
        target=root/'results'/f'{key}.json'
        return dict(run_pair(args.engine.resolve(), root/'requests'/f'{key}.json', target),
                    archetype=name, repetition=rep)
    write(root/'progress.json',dict(status='running',attempted_pairs=0,valid_pairs=0,
        total_pairs=len(jobs),started_unix=started))
    with ThreadPoolExecutor(max_workers=args.workers) as executor:
        futures=[executor.submit(worker,job) for job in jobs]
        for future in as_completed(futures):
            row=future.result()
            completed.append(row)
            measured = summarize(completed, names, args.pairs)
            progress=dict(status='running',attempted_pairs=len(completed),
                valid_pairs=measured['overall']['valid_pairs'],total_pairs=len(jobs),
                started_unix=started,heartbeat_unix=time.time(),elapsed_seconds=time.time()-started,
                last_archetype=row['archetype'],**measured)
            write(root/'progress.json',progress)
            print(json.dumps({k:progress[k] for k in ['attempted_pairs','valid_pairs','total_pairs','elapsed_seconds','last_archetype']}),flush=True)
    if {str(p):digest(p) for p in assets} != hashes:
        raise ValueError('Diagnostic asset changed during execution')
    measured = summarize(completed, names, args.pairs)
    summary=dict(status=completion_status(measured),started_unix=started,finished_unix=time.time(),
        **measured, limitations=plan['limitations'])
    write(root/'summary.json',summary)
    write(root/'progress.json',summary)
    print(json.dumps(summary),flush=True)


if __name__ == '__main__':
    main()
