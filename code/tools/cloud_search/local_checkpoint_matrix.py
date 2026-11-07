"""Frozen local same-model matchup grid, with exact-list mirrors and telemetry.

Runs production policy-only play_one; never trains, searches, or changes cloud jobs.
The native binding and Python evaluator are hash-bound, and reference cloud games
must reproduce before the grid can start. Incomplete games fail the evaluation.
"""
import argparse
from collections import Counter, defaultdict
import csv
import gc
import html
import itertools
import json
import os
from pathlib import Path
import subprocess
import sys
import time

from tools.cloud_search.final_policy_eval import schedule, summarize
from tools.selfplay_scheduler import atomic_json, sha256


def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def make_cells(decks, games=100, seed=91261028):
    if games < 4 or games % 4 or not decks or any(not ids for ids in decks.values()):
        raise ValueError('Use nonempty lists and games divisible by four')
    if any(len(ids) != len(set(ids)) for ids in decks.values()):
        raise ValueError('Duplicate exact lists within an archetype')
    cells = {}
    for panel in ['self', 'mirror']:
        rows = schedule(decks, panel, seed + (1000000 if panel == 'mirror' else 0),
                        games // (4 if panel == 'self' else 2))
        for row in rows:
            a, b = sorted([row['deck1'], row['deck2']])
            key = (panel, a, b)
            cell = cells.setdefault(key, dict(id=len(cells), panel=panel, deck=a, opponent=b, rows=[]))
            variants = ([(d, d) for d in sorted(decks[a])] if panel == 'mirror' else
                        list(itertools.product(sorted(decks[a]), sorted(decks[b]))))
            # A complete four-game block keeps the same exact lists while roles
            # and opening order change. Mirrors use identical lists on both sides.
            da, db = variants[(len(cell['rows']) // (4 if panel == 'self' else 2)) % len(variants)]
            ids = (da, db) if row['deck1'] == a else (db, da)
            cell['rows'].append(dict(row, deck1_id=ids[0], deck2_id=ids[1]))
    return list(cells.values())


def runtime_files():
    import digimon_engine
    import digimon_gym
    import model_contract
    import tools.cloud_search.final_policy_eval as scoring
    import tools.cloud_search.eval_progress as progress
    import tools.selfplay_scheduler as scheduler
    binding = next(p for p in Path(digimon_engine.__file__).parent.iterdir()
                   if p.suffix in {'.pyd', '.so'})
    paths = [binding, Path(__file__), Path(model_contract.__file__), Path(scoring.__file__),
             Path(progress.__file__), Path(scheduler.__file__),
             *Path(digimon_gym.__file__).parent.rglob('*.py')]
    return {str(p.resolve()): sha256(p) for p in paths}


def reference_differences(original, actual):
    """Telemetry table ordering is not a game decision; compare all row values."""
    keys = ['winner_id', 'terminal_reason', 'steps', 'cards', 'routes', 'route_decisions', 'effect_sequences']
    def comparable(value):
        if isinstance(value, list):
            return sorted(json.dumps(row, sort_keys=True) for row in value)
        return value
    return [key for key in keys if comparable(original[key]) != comparable(actual[key])]


def summarize_cell(cell, outcomes):
    result = summarize(cell['rows'], outcomes, cell['panel'])
    if cell['panel'] == 'self':
        for row in result['matrix']:
            counts = Counter()
            for game in outcomes:
                pid = 1 if game['deck1'] == row['deck'] else 2
                first = 1 if game['seed'] % 2 == 0 else 2
                order = 'first' if pid == first else 'second'
                outcome = 'draws' if not game['winner'] else 'wins' if game['winner'] == pid else 'losses'
                counts[f'{outcome}_{order}'] += 1
                counts[f'games_{order}'] += 1
            row.update({f'{stat}_{order}': counts[f'{stat}_{order}']
                        for order in ['first', 'second'] for stat in ['wins', 'losses', 'draws', 'games']})
    return result


def prepare(root, expected_hash, games, workers, checkpoint):
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
        raise ValueError('Refusing to overwrite an existing evaluation')
    if not 1 <= workers <= 8 or digimon_engine.CARD_EVAL_EVENT_VERSION < 2:
        raise ValueError('Use 1â€“8 workers and current decision telemetry')
    source = read(root/'inputs/source-campaign-plan.json')
    model_path = root/f'inputs/checkpoint{checkpoint}.zip'
    if sha256(model_path) != expected_hash:
        raise ValueError('Checkpoint identity differs')
    if sha256(root/'inputs/train_snapshot.json') != source['files']['inputs/train_snapshot.json']:
        raise ValueError('Training deck snapshot differs')
    reference = read(root/'inputs/reference_games.json')
    if reference['source_checkpoint_sha256'] != expected_hash:
        raise ValueError('Reference games use a different checkpoint')
    cards = root/'inputs/cards.json'
    if sha256(cards) != reference['provenance']['runtime_cards_sha256']:
        raise ValueError('Local card database differs from the cloud evaluation')
    os.environ['DIGIMON_CARDS_JSON'] = str(cards)
    pool = GeneralistDeckPool.from_snapshot(root/'inputs/train_snapshot.json',
        implemented_card_ids=digimon_engine.load_implemented_card_ids())
    if pool.archetype_count != 13 or pool.deck_count != 21:
        raise ValueError('Expected the exact 13-archetype, 21-list training pool')
    model = MaskablePPO.load(model_path, device='cpu')
    contract = validate_checkpoint(model, 'standard_lite_v2')
    if contract != (source.get('model_contract') or source['initialization']['contract']):
        raise ValueError('Model contract differs from the source campaign')
    validation = root/'validation'
    validation.mkdir(exist_ok=True)
    os.environ['DIGIMON_EVAL_FAILURE_DIR'] = str(validation/'failures')
    checks = []
    for index, item in enumerate(reference['references']):
        original = item['game']
        collector = CardEvalCollector(checkpoint, index, digimon_engine.CardDatabase().get_card)
        collector.context = original['context']
        play_one(model, greedy_policy, item['deck1'], item['deck2'], original['seed'],
                 contract['observation_profile'], collector=collector)
        actual = collector.games[-1]
        differences = reference_differences(original, actual)
        checks.append(dict(seed=original['seed'], archetype=original['context']['agent_archetype'],
                           matches=not differences, differences=differences))
        save_report(collector.report(), validation/f'reference-{index}')
    atomic_json(validation/'reference-reproduction.json', dict(checks=checks, passed=all(c['matches'] for c in checks)))
    if len(checks) != 8 or not all(c['matches'] for c in checks):
        raise ValueError('Local/cloud reference games differ; inspect validation before evaluating')
    cells = make_cells({name:[d.deck_id for d in ds] for name,ds in pool.archetypes.items()}, games)
    # Display the requested deck's matchups early; seeds, quotas and cells stay fixed.
    cells.sort(key=lambda c: ('Light Hybrid' not in (c['deck'],c['opponent']), c['id']))
    atomic_json(root/'schedule.json', cells)
    inputs = [p for p in (root/'inputs').iterdir() if p.is_file()]
    files = {str(p.relative_to(root)):sha256(p) for p in inputs}
    files['schedule.json'] = sha256(root/'schedule.json')
    plan = dict(schema='local-policy-matrix-v1', checkpoint=checkpoint, model_sha256=expected_hash,
        source_image=source['image'], source_commit=source['controller_commit'],
        platform=sys.platform, runtime=runtime_files(), cards_path=str(cards), cards_sha256=sha256(cards),
        contract=contract, files=files, workers=workers, games_per_pair=games,
        total_games=sum(len(c['rows']) for c in cells), cells=len(cells), archetypes=pool.archetype_names,
        lists=pool.deck_count, search=False, training=False, automatic_promotion=False,
        reference_games=checks, cohort='Exact training lists; all variants cycled within each matchup',
        execution_priority='Light Hybrid cells first; all frozen matchups still run',
        diagonal='First-player win rate in same-model, same-exact-list mirrors')
    atomic_json(root/'plan.json', plan)
    atomic_json(root/'state.json', dict(stage='prepared', games_completed=0, total_games=plan['total_games']))
    render(root)
    print(json.dumps(dict(status='prepared', games=plan['total_games'], cells=plan['cells'], references=len(checks))), flush=True)


def validate(root):
    plan = read(root/'plan.json')
    if runtime_files() != plan['runtime']:
        raise ValueError('Runtime changed since preflight')
    if sha256(Path(plan['cards_path'])) != plan['cards_sha256']:
        raise ValueError('Card data changed')
    os.environ['DIGIMON_CARDS_JSON'] = plan['cards_path']
    for name, digest in plan['files'].items():
        if sha256(root/name) != digest:
            raise ValueError(f'Frozen input changed: {name}')
    return plan


def worker(root, shard):
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
    checkpoint = plan['checkpoint']
    cells = [c for c in read(root/'schedule.json') if c['id'] % plan['workers'] == shard]
    pool = GeneralistDeckPool.from_snapshot(root/'inputs/train_snapshot.json',
        implemented_card_ids=digimon_engine.load_implemented_card_ids())
    decks = {d.deck_id:d for ds in pool.archetypes.values() for d in ds}
    model = MaskablePPO.load(root/f'inputs/checkpoint{checkpoint}.zip', device='cpu')
    opponent = make_agent_opponent_fn(str(root/f'inputs/checkpoint{checkpoint}.zip'), algorithm='mlp')
    progress = EvaluationProgress(root/f'worker-{shard}.json', total=sum(len(c['rows']) for c in cells),
                                  panel='local_matrix', checkpoint=checkpoint)
    try:
        for cell in cells:
            folder = root/'cells'/f"cell-{cell['id']:03d}"
            folder.mkdir(parents=True, exist_ok=False)
            os.environ['DIGIMON_EVAL_FAILURE_DIR'] = str(folder/'failures')
            collector = CardEvalCollector(checkpoint, cell['id'], digimon_engine.CardDatabase().get_card)
            collector.provenance = dict(checkpoint_sha256=plan['model_sha256'], panel=cell['panel'],
                evaluation_mode='policy_only', deterministic=True, status='running',
                source_image=plan['source_image'], local_runtime=plan['runtime'])
            outcomes = []
            try:
                with (folder/'outcomes.jsonl').open('x', encoding='utf-8') as stream:
                    for row in cell['rows']:
                        a,b = decks[row['deck1_id']],decks[row['deck2_id']]
                        collector.context = dict(row, panel=cell['panel'], anchor=f'checkpoint{checkpoint}',
                            agent_archetype=a.archetype_name, opponent_archetype=b.archetype_name,
                            agent_deck_id=a.deck_id, opponent_deck_id=b.deck_id, evaluation_mode='policy_only')
                        event = dict(anchor=f'checkpoint{checkpoint}', archetype=a.archetype_name,
                                     opponent_archetype=b.archetype_name,index=row['index'])
                        progress.event(dict(event,event='game_started'))
                        winner = play_one(model,opponent,a.card_ids,b.card_ids,row['seed'],
                                          plan['contract']['observation_profile'],collector=collector)
                        result = dict(row,winner=winner,terminal_reason=collector.games[-1]['terminal_reason'])
                        outcomes.append(result)
                        stream.write(json.dumps(result)+'\n')
                        stream.flush()
                        progress.event(dict(event,event='game_completed'))
                result = summarize_cell(cell, outcomes)
                collector.provenance['status'] = 'complete'
                atomic_json(folder/'result.json',dict(result,cell_id=cell['id']))
            except BaseException as error:
                collector.provenance.update(status='failed',error=str(error))
                raise
            finally:
                save_report(collector.report(),folder/'reports')
            del collector,outcomes
            gc.collect()
        progress.finish('complete')
    except BaseException as error:
        progress.finish('failed',error)
        raise


def render(root):
    plan,state = read(root/'plan.json'),read(root/'state.json')
    results = [read(p) for p in sorted((root/'cells').glob('cell-*/result.json'))]
    matrix = {(c['deck'],c['opponent_deck']):dict(c,panel=r['panel']) for r in results for c in r['matrix']}
    out = root/'report'
    out.mkdir(exist_ok=True)
    with (out/'matrix.csv').open('w',newline='',encoding='utf-8') as f:
        writer=csv.DictWriter(f,fieldnames=['deck','opponent_deck','games','wins','losses','draws','panel']+
            [f'{stat}_{order}' for order in ['first','second'] for stat in ['games','wins','losses','draws']])
        writer.writeheader(); writer.writerows(matrix.values())
    names=plan['archetypes']
    table='<tr><th>Row deck â†“ / opponent â†’</th>'+''.join(f'<th>{html.escape(n)}</th>' for n in names)+'</tr>'
    totals=defaultdict(Counter)
    for a in names:
        table+='<tr><th>'+html.escape(a)+'</th>'
        for b in names:
            cell=matrix.get((a,b))
            if cell is None:
                table+='<td class="pending">pending</td>'; continue
            rate=cell['wins']/cell['games']
            score=(cell['wins']+.5*cell['draws'])/cell['games']
            title=f"{cell['wins']}W / {cell['losses']}L / {cell['draws']}D; n={cell['games']}"
            label='1st: ' if a==b else ''
            color=f'hsl({round(120*score)},45%,87%)'
            data=''
            if a!=b:
                data=f' data-overall="{rate}" data-n-overall="{cell["games"]}"'
                for order in ['first','second']:
                    data+=f' data-{order}="{cell[f"wins_{order}"]/cell[f"games_{order}"]}" data-n-{order}="{cell[f"games_{order}"]}"'
                    title+=f'; going {order}: {cell[f"wins_{order}"]}/{cell[f"games_{order}"]} wins'
            table+=f'<td{data} style="background:{color}" title="{title}">{label}{rate:.0%}<small>n={cell["games"]}</small></td>'
            if a!=b:
                totals[a].update({k:cell[k] for k in ['wins','losses','draws','games']})
        table+='</tr>'
    complete=state['stage']=='complete'
    refresh='' if complete or state['stage']=='failed' else '<meta http-equiv="refresh" content="30">'
    text=f'''<!doctype html><html lang="en"><meta charset="utf-8">{refresh}
<title>Checkpoint {plan["checkpoint"]} â€” local 13Ã—13 matchup grid</title>
<style>body{{font:15px system-ui;margin:28px;color:#172b44;background:#f5f7fb}}table{{border-collapse:collapse;background:white}}th,td{{border:1px solid #ccd5df;padding:10px;text-align:center;min-width:75px}}th{{font-size:12px}}small{{display:block;font-size:10px}}.scroll{{overflow:auto}}.pending{{color:#8898aa}}p{{max-width:1050px;line-height:1.5}}h1{{font-size:26px}}</style>
<h1>Checkpoint {plan["checkpoint"]} Â· local 13Ã—13 matchup grid</h1>
<p><strong>Status: {html.escape(state['stage'])} Â· {state.get('games_completed',0):,} / {plan['total_games']:,} games</strong> Â· {len(results)} / {plan['cells']} matchups complete.</p>
<p>Same checkpoint on both sides, without search. Rows show wins against the column deck. Both engine roles and opening order are balanced.
<strong>Diagonal cells show first-player win rate in exact-list mirrors</strong>, a different metric from off-diagonal cells.</p>
<p>All 21 training lists are cycled within their 13 archetypes. This measures matchups under this policy, not intrinsic deck strength or isolated piloting skill.
Each cell has only {plan['games_per_pair']} games: small differences are exploratory. Hover for wins, losses, draws and sample counts.
The two directions of each matchup reuse the same games.</p>
<p><label>Row deck opening order: <select id="opening"><option value="overall">Balanced overall</option><option value="first">Going first only</option><option value="second">Going second only</option></select></label>
 Â· Diagonals always show first-player mirror win rate.</p><div class="scroll"><table>{table}</table></div>
<script>document.getElementById('opening').addEventListener('change',event=>{{const mode=event.target.value;document.querySelectorAll('td[data-overall]').forEach(cell=>{{const rate=Number(cell.getAttribute('data-'+mode));const n=cell.getAttribute('data-n-'+mode);cell.innerHTML=(100*rate).toFixed(1)+'%<small>n='+n+'</small>';cell.style.background='hsl('+Math.round(120*rate)+',45%,87%)';}});}});</script>'''
    if totals:
        text+='<h2>Performance against the other 12 archetypes</h2><p>Excludes mirrors. Compare rankings only after the grid is complete.</p><table><tr><th>Deck</th><th>Score (draw = Â½)</th><th>Games</th></tr>'
        for name,c in sorted(totals.items(),key=lambda item:(item[1]['wins']+.5*item[1]['draws'])/item[1]['games'],reverse=True):
            text+=f'<tr><td>{html.escape(name)}</td><td>{(c["wins"]+.5*c["draws"])/c["games"]:.1%}</td><td>{c["games"]}</td></tr>'
        text+='</table>'
    text+='<details><summary>Provenance and frozen configuration</summary><pre>'+html.escape(json.dumps(plan,indent=2))+'</pre></details></html>'
    temp=out/'index.html.tmp';temp.write_text(text,encoding='utf-8');temp.replace(out/'index.html')
    if complete:
        atomic_json(root/'result.json',dict(status='complete',games=plan['total_games'],matrix=list(matrix.values()),
            decks=[dict(deck=k,**v) for k,v in totals.items()],automatic_promotion=False))


def run(root):
    plan=validate(root)
    with (root/'started.json').open('x') as f:
        json.dump(dict(time_unix=time.time(),pid=os.getpid()),f)
    processes,logs=[],[]
    started=time.time()
    try:
        for shard in range(plan['workers']):
            log=(root/f'worker-{shard}.log').open('x');logs.append(log)
            processes.append(subprocess.Popen([sys.executable,str(Path(__file__).resolve()),'--root',str(root),'--worker',str(shard),'--checkpoint',str(plan['checkpoint'])],stdout=log,stderr=subprocess.STDOUT))
        while True:
            progress=[read(p) for p in root.glob('worker-*.json')]
            count=sum(p['games_completed'] for p in progress)
            if any(p.poll() not in (None,0) for p in processes):
                raise RuntimeError('Local evaluator failed; preserve worker logs and failure recordings')
            done=all(p.poll()==0 for p in processes)
            if done and count!=plan['total_games']:
                raise RuntimeError('Missing completed games')
            atomic_json(root/'state.json',dict(stage='complete' if done else 'running',games_completed=count,
                total_games=plan['total_games'],time_unix=time.time(),elapsed_seconds=time.time()-started,
                automatic_promotion=False))
            render(root)
            if done: break
            time.sleep(5)
    except BaseException as error:
        previous=read(root/'state.json')
        atomic_json(root/'state.json',dict(previous,stage='failed',error=str(error),time_unix=time.time()))
        render(root)
        raise
    finally:
        for p in processes:
            if p.poll() is None:
                p.terminate()
                try: p.wait(timeout=15)
                except subprocess.TimeoutExpired: p.kill();p.wait()
        for log in logs: log.close()


if __name__=='__main__':
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--root',type=Path,required=True)
    ap.add_argument('--prepare',action='store_true')
    ap.add_argument('--checkpoint-sha256')
    ap.add_argument('--checkpoint',type=int,required=True)
    ap.add_argument('--games-per-pair',type=int,default=100)
    ap.add_argument('--workers',type=int,default=6)
    ap.add_argument('--worker',type=int)
    args=ap.parse_args()
    root=args.root.resolve()
    if args.prepare: prepare(root,args.checkpoint_sha256,args.games_per_pair,args.workers,args.checkpoint)
    elif args.worker is not None: worker(root,args.worker)
    else: run(root)
