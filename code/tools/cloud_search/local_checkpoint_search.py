"""Frozen local search-versus-policy experiment; never trains or contacts a host.

The native search_quality executable owns gameplay. This controller freezes the
schedule, validates completed seat pairs, and reports actual-game telemetry.
"""
import argparse
from collections import Counter, defaultdict
import html
import itertools
import json
import math
import os
from pathlib import Path
import random
import shutil
import subprocess
import time
from types import SimpleNamespace

from tools.selfplay_scheduler import atomic_json, controller_lock, sha256
from tools.search_quality_study import Jobs, score_interval
from tools.search_budget_confirmation import normalized_event, role_ui


SEARCH = dict(id='search-320-4', kind='search', simulations=320, worlds=4)
POLICY = dict(id='policy-only', kind='policy')


def read(path):
    return json.loads(Path(path).read_text(encoding='utf-8-sig'))


def make_schedule(decks, seed=8326102900):
    """130 four-game blocks: roles and openers balanced within every block.

Ten cyclic rounds give each archetype twenty block endpoints. Each endpoint
plays twice (one first, one second), including both endpoints of exact mirrors.
All unordered archetype matchups occur; this is not a uniform 13x13 matrix.
"""
    groups = defaultdict(list)
    for index, deck in enumerate(decks):
        groups[deck['name']].append(index)
    names = sorted(groups)
    if len(names) != 13 or len({d['source_deck_id'] for d in decks}) != len(decks):
        raise ValueError('Expected thirteen archetypes and unique exact lists')
    if seed % 2:
        raise ValueError('Schedule base seed must be even')
    seen = Counter()
    result = []
    for round_index in range(10):
        for index, a in enumerate(names):
            b = names[(index + round_index) % len(names)]
            low, high = sorted((a, b))
            variants = ([(d, d) for d in groups[a]] if a == b else
                        list(itertools.product(groups[low], groups[high])))
            x, y = variants[seen[low, high] % len(variants)]
            seen[low, high] += 1
            indices = [x, y] if a == low else [y, x]
            block = len(result) // 2
            for opener in (0, 1):
                result.append(dict(id=f'match-{block:03d}-{opener}', block=block,
                    deck_indices=indices, game_seed=seed + block * 10 + opener,
                    search_seed=seed + 1000000 + block * 10 + opener,
                    candidate=SEARCH, opponent=POLICY, card_metrics=True))
    return result


def validate_record(plan, request, record, telemetry=True):
    for key in ('candidate', 'opponent', 'game_seed', 'deck_indices'):
        if record[key] != request[key]:
            raise ValueError(f'Result changed request field {key}')
    if 'model' in plan and any(record.get(k)!=plan[k] for k in ('model','opponent_model')):
        raise ValueError('Native actor model identities differ from frozen plan')
    games = record['games']
    if len(games) != 2 or {g['candidate_seat'] for g in games} != {0, 1}:
        raise ValueError('Missing candidate seat')
    for game in games:
        if game['truncated'] or any(x in game['terminal_reason'] for x in ('Crash', 'StepLimit', 'None')):
            raise ValueError('Incomplete game preserved; never scored as a draw')
        if game['first_player'] != request['game_seed'] % 2:
            raise ValueError('Opening order differs')
        winner = game['winner']
        expected = .5 if winner is None else float(winner == game['candidate_seat'])
        if winner not in (None, 0, 1) or game['score'] != expected:
            raise ValueError('Wrong outcome perspective')
        if len(game['actions']) != game['decisions']:
            raise ValueError('Incomplete executed action trace')
        if telemetry:
            steps = game['card_metrics']['steps']
            if len(steps) != game['decisions']:
                raise ValueError('Missing actual-game telemetry')
            for action, step in zip(game['actions'], steps, strict=True):
                if action != step['action'] or action not in step['legal_actions']:
                    raise ValueError('Executed action differs from telemetry or legal mask')
                if not math.isfinite(step['selection_seconds']) or step['selection_seconds'] < 0:
                    raise ValueError('Invalid timing')
    return games


def runtime_sources():
    import tools.search_quality_study as jobs
    import tools.search_budget_confirmation as adapter
    import tools.selfplay_scheduler as scheduler
    import digimon_gym.agents.card_eval as collector
    return [Path(__file__).resolve(), Path(jobs.__file__), Path(adapter.__file__),
            Path(scheduler.__file__), Path(collector.__file__)]


def prepare(root, binary, workers):
    if (root/'plan.json').exists():
        raise ValueError('Preserving existing frozen plan')
    if workers not in range(1, 5):
        raise ValueError('Use one to four local workers')
    inputs=root/'inputs'; protocol=read(inputs/'protocol.json')
    source=read(inputs/'source-campaign-plan.json')
    for name,digest in protocol['expected_files'].items():
        if sha256(inputs/name)!=digest: raise ValueError(f'Frozen cloud input differs: {name}')
    candidate=protocol['candidate']; opponent=protocol['opponent']
    metadata=read(inputs/(candidate['model']+'.meta.json'))
    other_metadata=read(inputs/(opponent['model']+'.meta.json'))
    if metadata != other_metadata:
        raise ValueError('Candidate/opponent observation and value contracts differ')
    if metadata['value_semantics']!='terminal_outcome' or metadata['value_head']!='inline':
        raise ValueError('Expected inline terminal-outcome model')
    snapshot=read(inputs/'train_snapshot.json')
    decks=[dict(name=d['archetype'],cards=d['card_ids'],source_deck_id=d['deck_id'])
           for d in sorted(snapshot['decks'],key=lambda d:(d['archetype'],d['deck_id']))]
    if len(decks)!=21: raise ValueError('Expected frozen 21-list pool')
    scheduled=make_schedule(decks,seed=protocol['seed'])
    for row in scheduled:
        row['candidate']=candidate['settings']; row['opponent']=opponent['settings']
    if {i for r in scheduled for i in r['deck_indices']}!=set(range(21)):
        raise ValueError('Schedule misses a list')
    runtime=root/'runtime';runtime.mkdir(exist_ok=False)
    engine=runtime/binary.name;shutil.copy2(binary,engine)
    for folder in (binary.parent,binary.parent.parent):
        for dll in folder.glob('*.dll'):
            if not (runtime/dll.name).exists():shutil.copy2(dll,runtime/dll.name)
    atomic_json(root/'schedule.json',scheduled)
    files={p.relative_to(root).as_posix():sha256(p) for folder in (inputs,runtime)
           for p in folder.iterdir() if p.is_file()}
    files['schedule.json']=sha256(root/'schedule.json')
    repo=Path(__file__).resolve().parents[3]
    native=repo/'code/digimon-engine/examples/checkpoint_search_eval.rs'
    plan=dict(schema='local-checkpoint-search-v1',checkpoint=candidate['checkpoint'],
        title=protocol['title'],roles={'candidate':candidate,'opponent':opponent},
        model=str(inputs/candidate['model']),opponent_model=str(inputs/opponent['model']),
        cards_json=str(inputs/'cards.json'),engine=str(engine),decks=decks,
        decision_cap=1200,total_games=520,seat_pairs=260,four_game_blocks=130,
        games_per_archetype=40,workers=workers,source_image=source['image'],
        source_commit=source['controller_commit'],model_contract=metadata,
        native_harness_sha256=sha256(native),files=files,
        python_sources={str(p):sha256(p) for p in runtime_sources()},
        regime='open_decklist_exact_composition',evaluation_root_noise=0,
        full_search_fraction=1,batch_size=4,candidate=candidate['settings'],opponent=opponent['settings'],
        training=False,automatic_promotion=False,cloud_mutations=False,
        cohort='Same 21 training lists; 40 candidate appearances per archetype, 20 per opening order',
        scope='Fixed-pool direct duel, not held-out-list or human strength evidence.',
        effect_sequence_opportunities='unmeasured; resolved DNA events measured')
    atomic_json(root/'plan.json',plan)
    atomic_json(root/'state.json',dict(stage='prepared',completed_games=0,total_games=520))
    print(json.dumps(dict(status='prepared',title=plan['title'],games=520)),flush=True)


def validate(root):
    plan = read(root/'plan.json')
    for name, digest in plan['files'].items():
        if sha256(root/name) != digest:
            raise ValueError(f'Frozen asset changed: {name}')
    for path, digest in plan['python_sources'].items():
        if sha256(Path(path)) != digest:
            raise ValueError(f'Python observer/controller changed: {path}')
    return plan


def preflight(root):
    plan=validate(root);directory=root/'preflight';directory.mkdir(exist_ok=False)
    atomic_json(directory/'plan.json',plan)
    job=Jobs(plan,directory,1,3600)
    base=dict(read(root/'schedule.json')[28]);base.pop('id');base.pop('block')
    base.update(game_seed=9326102900,search_seed=9327102900)
    policy_request=dict(base,candidate=POLICY,opponent=POLICY)
    plain=job.one('policy-pair','match',policy_request)
    validate_record(plan,policy_request,plain)
    # Swap both model identities and candidate seats. Physical players must
    # execute exactly the same policy game, even when the models differ.
    swapped_plan=dict(plan,model=plan['opponent_model'],opponent_model=plan['model'])
    swapped_dir=directory/'swapped';swapped_dir.mkdir()
    atomic_json(swapped_dir/'plan.json',swapped_plan)
    swapped=Jobs(swapped_plan,swapped_dir,1,3600).one('policy-pair','match',policy_request)
    validate_record(swapped_plan,policy_request,swapped)
    for index,a in enumerate(plain['games']):
        b=swapped['games'][1-index]
        if any(a[k]!=b[k] for k in ('actions','winner','decisions','terminal_reason')) or a['score']+b['score']!=1:
            raise ValueError('Model routing failed physical-seat swap equivalence')
    searched=job.one('search-observed','match',base)
    validate_record(plan,base,searched)
    bare_request=dict(base,card_metrics=False)
    bare=job.one('search-unobserved','match',bare_request)
    validate_record(plan,bare_request,bare,telemetry=False)
    for a,b in zip(searched['games'],bare['games'],strict=True):
        if any(a[k]!=b[k] for k in ('actions','winner','decisions','searched','score','terminal_reason')):
            raise ValueError('Observer changed gameplay or search results')
    collect_behavior(directory,[dict(base,id='search-observed',block=-1)])
    atomic_json(directory/'validation.json',dict(passed=True,diagnostic_games=8,
        plan_sha256=sha256(root/'plan.json'),observer_action_parity=True,
        model_swap_seat_equivalence=True,search_pair_seconds=searched['seconds'],
        evidence={str(p.relative_to(directory)):sha256(p) for p in directory.rglob('*.done.json')}))
    print(json.dumps(dict(status='preflight-passed',games=8,search_pair_seconds=searched['seconds'])),flush=True)


def summary(root):
    plan=read(root/'plan.json'); scheduled=read(root/'schedule.json')
    games=[]; requests=[]
    for request in scheduled:
        name=request['id']; done=root/'results'/f'{name}.done.json'
        if not done.exists(): continue
        path=root/'results'/f'{name}.json'
        if read(done)!={'request_sha256':sha256(root/'requests'/f'{name}.json'),'output_sha256':sha256(path)}:
            raise ValueError('Completed result hashes changed')
        record=read(path)
        for game in validate_record(plan,request,record):
            seat=game['candidate_seat']
            games.append(dict(block=request['block'],archetype=plan['decks'][request['deck_indices'][seat]]['name'],
                opponent=plan['decks'][request['deck_indices'][1-seat]]['name'],
                score=game['score'],first=seat==game['first_player'],seconds=game['seconds'],
                decisions=game['decisions'],searched=game['searched']))
        requests.append(request)
    def totals(rows):
        return dict(games=len(rows),wins=sum(g['score']==1 for g in rows),
            losses=sum(g['score']==0 for g in rows),draws=sum(g['score']==.5 for g in rows),
            score=sum(g['score'] for g in rows)/len(rows) if rows else None,
            first_games=sum(g['first'] for g in rows))
    result=dict(totals(games),total_games=plan['total_games'],complete=len(games)==plan['total_games'],
        generated_at=time.time(),by_archetype={name:totals([g for g in games if g['archetype']==name])
            for name in sorted({d['name'] for d in plan['decks']})},
        by_opening_order={order:totals([g for g in games if g['first']==first])
            for order,first in [('first',True),('second',False)]},failures_or_truncations=0)
    if result['complete']:
        if any(r['games']!=40 or r['first_games']!=20 for r in result['by_archetype'].values()):
            raise ValueError('Completed coverage is not balanced')
        result['score_ci95']=score_interval([g['score'] for g in games],[g['block'] for g in games])
        result['interval_method']='four-game block bootstrap with a conservative Wilson floor; conditional on this fixed pool'
    atomic_json(root/'summary.json',result)
    render(root,result)
    return result, requests


def render(root,result):
    plan=read(root/'plan.json'); title=html.escape(plan['title'])
    rows=''.join(f'<tr><td>{html.escape(name)}</td><td>{r["wins"]}–{r["losses"]}–{r["draws"]}</td><td>{100*r["score"]:.1f}%</td></tr>'
        for name,r in result['by_archetype'].items() if r['games'])
    rate=f'{100*result["score"]:.1f}%' if result['games'] else 'Pending'
    interval=(' · 95% interval '+ '–'.join(f'{100*x:.1f}%' for x in result['score_ci95'])) if result.get('score_ci95') else ''
    text=f'''<!doctype html><html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>{title}</title><style>body{{font:17px system-ui;max-width:850px;margin:35px auto;padding:0 18px;color:#152337}}table{{border-collapse:collapse;width:100%}}td,th{{text-align:left;padding:10px;border-bottom:1px solid #ddd}}small{{color:#526275}}</style>
<h1>{title}</h1><p><b>{result['games']} / 520 games</b> · {'Complete' if result['complete'] else 'In progress; provisional results'}</p>
<p>Candidate score: <b>{rate}</b>{interval}. {result['wins']} wins, {result['losses']} losses, {result['draws']} draws.</p>
<p>Candidate: {html.escape(plan['roles']['candidate']['label'])}. Opponent: {html.escape(plan['roles']['opponent']['label'])}. Search uses four worlds and batch four on every non-forced root, with zero root noise. Exact deck compositions are known; hidden placements are sampled.</p>
<table><tr><th>Candidate archetype</th><th>W–L–D</th><th>Score</th></tr>{rows}</table>
<p>Each archetype receives 40 appearances (20 first, 20 second); all 21 training lists are included. The opponent mix is balanced overall but this is not a uniform matchup matrix. Forty games per deck provide preliminary evidence.</p>
<p><a href="summary.json">Summary</a> · <a href="plan.json">Frozen protocol</a> · <a href="behavior.json">Card behavior (on completion)</a></p>
<small>Preflight games are excluded. Failed or capped games stop the experiment and are never counted as draws. Training is unchanged. Nyaromon-specific effect-sequence opportunities are unmeasured by this observer; resolved DNA events are collected.</small></html>'''
    (root/'index.html').write_text(text,encoding='utf-8')


def collect_behavior(root,requests):
    from digimon_gym.agents.card_eval import CardEvalCollector, save_report, CARD_COUNTERS
    plan=read(root/'plan.json'); cards=read(plan['cards_json'])
    lookup=lambda cid:SimpleNamespace(card_name=cards.get(cid,{}).get('card_name_eng',cid))
    results={}
    for role in ('candidate','opponent'):
        collector=CardEvalCollector(plan['roles'][role]['checkpoint'],0,lookup)
        collector.identity['window_id']=f'cp{plan["checkpoint"]}-local-search-{role}-{sha256(root/"plan.json")[:12]}'
        collector.provenance=dict(evaluation_mode=role,plan_sha256=sha256(root/'plan.json'),
            model_sha256=plan['files']['inputs/'+plan['roles'][role]['model']],information_regime=plan['regime'],
            role_mapping='Player 1 is the named role; both role reports describe the same games',
            effect_sequence_opportunities='unmeasured',status='complete')
        timings=[]; decisions=0; nonforced=0; games_count=0
        for request in requests:
            record=read(root/'results'/f'{request["id"]}.json')
            for game in validate_record(plan,request,record):
                games_count+=1
                seat=game['candidate_seat'] if role=='candidate' else 1-game['candidate_seat']
                flip=seat==1; telemetry=game['card_metrics']
                decks=[plan['decks'][i] for i in request['deck_indices']]
                collector.context=dict(agent_archetype=decks[seat]['name'],opponent_archetype=decks[1-seat]['name'],
                    agent_deck_id=decks[seat]['source_deck_id'],opponent_deck_id=decks[1-seat]['source_deck_id'],
                    evaluation_mode=role,native_agent_seat=seat,block=request['block'])
                ui=role_ui(telemetry['initial_ui'],flip)
                collector.start_game(ui,(decks[seat]['cards'],decks[1-seat]['cards']),request['game_seed'])
                for step in telemetry['steps']:
                    mask=[False]*plan['model_contract']['action_space_size']
                    for a in step['legal_actions']: mask[a]=True
                    pid=3-step['mover'] if flip else step['mover']
                    collector.before_step(dict(current_player_id=pid,phase=step['phase']),ui,mask,step['action'])
                    ui=role_ui(step['after_ui'],flip)
                    events=[e for event in step['events'] if (e:=normalized_event(event,flip))]
                    collector.after_step(dict(turn_count=step['turn_count']),ui,events,False,False)
                    if pid==1:
                        decisions+=1
                        if len(step['legal_actions'])>1:
                            nonforced+=1; timings.append(step['selection_seconds'])
                winner=game['winner']; winner=None if winner is None else (1-winner if flip else winner)+1
                collector.finish_game(winner,game['terminal_reason'])
        report=collector.report(); save_report(report,root/'eval_cards'/role)
        counters=Counter()
        for row in report['cards']:
            if row['player']==1:
                for key in CARD_COUNTERS: counters[key]+=row[key]
        route_counts=defaultdict(Counter)
        for row in report['route_decisions']:
            if row['player']==1:
                route_counts[row['route']].update({k:row[k] for k in ('opportunities','choices')})
        ordered=sorted(timings)
        results[role]=dict(resolved_card_events=dict(counters),route_decisions=dict(route_counts),
            decisions=decisions,nonforced_decisions=nonforced,selection_seconds=sum(timings),
            mean_seconds_per_nonforced_decision=sum(timings)/len(timings) if timings else None,
            median_seconds_per_nonforced_decision=ordered[len(ordered)//2] if ordered else None,
            p95_seconds_per_nonforced_decision=ordered[min(len(ordered)-1,int(.95*len(ordered)))] if ordered else None)
    atomic_json(root/'behavior.json',dict(games=games_count,roles=results,
        note='The two role reports describe the same games. Pending effect-sequence opportunities are unmeasured.'))


class CheckedJobs(Jobs):
    def one(self,name,mode,request):
        result=super().one(name,mode,request)
        try:
            validate_record(self.plan,request,result)
        except BaseException:
            receipt=self.directory/'results'/f'{name}.done.json'
            if receipt.exists():receipt.rename(receipt.with_name(f'{name}.rejected-receipt.json'))
            raise
        return result


def run(root):
    plan=validate(root)
    pre=read(root/'preflight/validation.json')
    if not pre['passed'] or pre['plan_sha256']!=sha256(root/'plan.json'):
        raise ValueError('Missing matching preflight evidence')
    if (root/'started.json').exists():
        raise ValueError('No automatic resume; existing run preserved')
    with controller_lock(root/'run.lock'):
        atomic_json(root/'started.json',dict(time_unix=time.time(),pid=os.getpid(),plan_sha256=sha256(root/'plan.json')))
        atomic_json(root/'state.json',dict(stage='running',started_at=time.time(),total_games=520))
        summary(root)
        schedule=read(root/'schedule.json')
        # Randomize the frozen block order, preserving neighboring openers.
        blocks=[schedule[i:i+2] for i in range(0,len(schedule),2)]
        random.Random(918273).shuffle(blocks)
        jobs=[(r['id'],'match',{k:v for k,v in r.items() if k not in ('id','block')}) for block in blocks for r in block]
        try:
            CheckedJobs(plan,root,plan['workers'],3600).run(jobs)
            result,requests=summary(root)
            if not result['complete']: raise ValueError('Missing completed games')
            collect_behavior(root,requests)
            atomic_json(root/'state.json',dict(stage='complete',completed_at=time.time(),completed_games=520,total_games=520))
            print(json.dumps({k:result[k] for k in ('games','wins','losses','draws','score','score_ci95')}),flush=True)
        except BaseException as error:
            atomic_json(root/'state.json',dict(stage='failed',time_unix=time.time(),error=repr(error)))
            raise


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode',choices=('prepare','preflight','run','report'))
    parser.add_argument('--root',type=Path,required=True)
    parser.add_argument('--engine',type=Path)
    parser.add_argument('--workers',type=int,default=4)
    args=parser.parse_args(); root=args.root.resolve()
    if args.mode=='prepare': prepare(root,args.engine.resolve(),args.workers)
    elif args.mode=='preflight': preflight(root)
    elif args.mode=='run': run(root)
    else:
        result,_=summary(root)
        print(json.dumps({k:v for k,v in result.items() if k!='by_archetype'}))


if __name__=='__main__':
    main()
