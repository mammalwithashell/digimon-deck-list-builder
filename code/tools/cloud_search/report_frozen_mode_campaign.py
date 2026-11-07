"""Complete-panel scores and paired information-regime effects, never partial scores."""
import argparse
from collections import Counter, defaultdict
import html
import json
from pathlib import Path
import statistics
import subprocess
import sys
import time
import numpy as np
from tools.cloud_search.frozen_mode_campaign import read,write,digest,require
from tools.cloud_search.search_opportunity_report import summarize as opportunity_summary


def block_estimate(rows,draws=4000):
    require(bool(rows),'empty score cohort')
    require(all(row['score'] in (0,.5,1) for row in rows),'invalid terminal score')
    groups=defaultdict(list)
    for row in rows:groups[row['block']].append(row['score'])
    sums=np.asarray([sum(v) for _,v in sorted(groups.items())],dtype=float)
    counts=np.asarray([len(v) for _,v in sorted(groups.items())],dtype=float)
    rng=np.random.default_rng(11826103000)
    picks=rng.integers(0,len(sums),size=(draws,len(sums)))
    estimates=sums[picks].sum(axis=1)/counts[picks].sum(axis=1)
    return dict(games=len(rows),blocks=len(groups),score=float(sums.sum()/counts.sum()),
        wins=sum(r['score']==1 for r in rows),losses=sum(r['score']==0 for r in rows),draws=sum(r['score']==.5 for r in rows),
        ci95=[float(x) for x in np.quantile(estimates,[.025,.975])],method='four-game source-block percentile bootstrap; subset blocks retained')


def analyze(rows,expected_games,draws=4000):
    require(len(rows)==expected_games and expected_games in (480,520),'complete panel denominator')
    require(len({(r['request'],r['candidate_seat']) for r in rows})==expected_games,'duplicate game identity')
    groups=Counter(r['block'] for r in rows)
    require(len(groups)==expected_games//4 and set(groups.values())=={4},'four-game blocks')
    result=block_estimate(rows,draws)
    for label,key in (('by_archetype','archetype'),('by_seat','candidate_seat'),('by_opening_order','first')):
        result[label]={str(v):block_estimate([r for r in rows if r[key]==v],draws) for v in sorted({r[key] for r in rows})}
    require(all(v['games']==40 for v in result['by_archetype'].values()),'archetype denominator')
    for arch in result['by_archetype']:
        selected=[r for r in rows if r['archetype']==arch]
        require(sum(r['first'] for r in selected)==20 and Counter(r['candidate_seat'] for r in selected)=={0:20,1:20},'seat/opener balance')
    return result


def paired_effect(closed,opened,draws=4000):
    require(len(closed)==len(opened)==520,'complete matched primary panels required')
    require(all(row['score'] in (0,.5,1) for row in closed+opened),'invalid terminal score')
    key=lambda row:(row['request'],row['candidate_seat'])
    left={key(r):r for r in closed};right={key(r):r for r in opened}
    require(len(left)==len(right)==520 and left.keys()==right.keys(),'paired game identity')
    groups=defaultdict(list)
    for k,a in left.items():
        b=right[k]
        for field in ('block','game_seed','search_seed','deck_indices','archetype','first'):
            require(a[field]==b[field],'paired schedule mismatch: '+field)
        groups[a['block']].append(b['score']-a['score'])
    require(len(groups)==130 and all(len(v)==4 for v in groups.values()),'paired block denominator')
    values=np.asarray([statistics.mean(v) for _,v in sorted(groups.items())])
    rng=np.random.default_rng(11926103000)
    sampled=values[rng.integers(0,130,size=(draws,130))].mean(axis=1)
    return dict(estimand='candidate open/open score minus closed/closed score',paired_games=520,paired_blocks=130,
        mean=float(values.mean()),ci95=[float(x) for x in np.quantile(sampled,[.025,.975])],
        method='paired four-game-block percentile bootstrap; not subtraction of independent confidence intervals',
        caveat='Shared seeds align starting states; subsequent trajectories may differ. Exploratory pairwise comparison.')


class Behavior:
    def __init__(self):
        self.groups=defaultdict(lambda:dict(games=0,events=Counter(),opportunity=Counter(),means={},selection=[],search=[],belief=[],
            closed_roots=0,fallback_roots=0,fallback_worlds=0,sampled_worlds=0,candidate_support=[],fallback_mass=[],mixture_ess=[]))

    def add(self,game):
        for role,seat in (('candidate',game['candidate_seat']),('opponent',1-game['candidate_seat'])):
            g=self.groups[role];g['games']+=1
            searches=[s for s in game['search_diagnostics'] if s['diagnostics']['viewer']==seat]
            summary=opportunity_summary([s['opportunity'] for s in searches]);g['opportunity'].update(summary['counts'])
            for key,estimate in summary['means'].items():
                total,count=g['means'].get(key,(0.,0));n=estimate['measured_roots']
                g['means'][key]=(total+estimate['value']*n,count+n)
            for search in searches:
                diag=search['diagnostics'];info=diag['information'];g['search'].append(diag['seconds'])
                g['selection'].append(game['card_metrics']['steps'][search['decision']]['selection_seconds'])
                if info['regime']=='closed-list':
                    g['closed_roots']+=1;g['fallback_roots']+=int(info['fallback_worlds']>0)
                    g['fallback_worlds']+=info['fallback_worlds'];g['sampled_worlds']+=info['sampled_worlds']
                    g['candidate_support'].append(info['compatible_candidates']);g['belief'].append(info['sampling_seconds'])
                    g['fallback_mass'].append(info['fallback_mass']);g['mixture_ess'].append(info['mixture_component_ess'])
            for step in game['card_metrics']['steps']:
                for event in step['events']:
                    if event.get('player')!=seat:continue
                    kind=event['type']
                    if kind in ('Digivolve','EvalDigivolve'):
                        route='dna' if event.get('was_dna',False) else ('raising' if event['field_index']==14 else 'battle')
                        g['events']['digivolve_'+route]+=1
                    elif kind=='BreedingMove':g['events']['promotion']+=1
                    elif kind in ('Play','Hatch'):g['events'][kind.lower()]+=1

    def report(self):
        def stats(v):
            return dict(count=len(v),mean=float(np.mean(v)) if v else None,p50=float(np.median(v)) if v else None,
                        p95=float(np.quantile(v,.95)) if v else None,total=float(sum(v)))
        result={}
        for role,g in self.groups.items():
            result[role]=dict(games=g['games'],resolved_routes=dict(g['events']),opportunity_counts=dict(g['opportunity']),
                opportunity_means={k:dict(value=total/n,measured_roots=n) for k,(total,n) in g['means'].items()},
                searched_root_selection_seconds=stats(g['selection']),search_seconds=stats(g['search']),
                belief_sampling_seconds=stats(g['belief']),closed_roots=g['closed_roots'],
                fallback_roots=g['fallback_roots'],fallback_worlds=g['fallback_worlds'],sampled_worlds=g['sampled_worlds'],
                compatible_candidate_count=stats(g['candidate_support']),fallback_mass=stats(g['fallback_mass']),
                mixture_component_ess=stats(g['mixture_ess']),calibrated_game_win_probabilities='unmeasured',
                offline_hidden_card_prediction_scores='unmeasured')
        return dict(roles=result,note='Both roles describe the same games. Searched-root denominators exclude forced roots. Selection time includes search and diagnostic work. Resolved routes are event counts; pending effect-sequence opportunities unmeasured. Tau0/noise0 evaluation is not training exploration.')


def load_panel(root):
    manifest=read(root/'manifest.json')['files']
    require(digest(root/'controller.py')==manifest['controller.py'],'frozen controller changed')
    run=subprocess.run([sys.executable,str(root/'controller.py'),'verify','--root',str(root.resolve())],capture_output=True,text=True)
    require(run.returncode==0,'frozen complete-panel verification failed: '+run.stderr[-1200:])
    protocol=read(root/'protocol.json');panel=read(root/'inputs/panel.json');plan=read(root/'plan.json')
    rows=[];behavior=Behavior()
    for job in read(root/'inputs/jobs.json'):
        output=read(root/f"results/{job['id']}.json")
        for game in output['games']:
            seat=game['candidate_seat'];deck=plan['decks'][job['deck_indices'][seat]]
            rows.append(dict(request=job['id'],block=job['block'],game_seed=job['game_seed'],search_seed=job['search_seed'],
                deck_indices=job['deck_indices'],candidate_seat=seat,archetype=deck['name'],first=game['first_player']==seat,
                score=game['score'],seconds=game['seconds']))
            behavior.add(game)
    return panel,protocol,plan,rows,behavior.report()


def report(root,out):
    require(not out.exists(),'preserve prior report')
    panel,protocol,plan,rows,behavior=load_panel(root)
    summary=dict(schema='frozen-mode-panel-report-v1',panel=panel,platform=protocol['platform'],
        results=analyze(rows,protocol['games']),information_regimes=[panel['candidate_mode'],panel['opponent_mode']],
        scope='Closed-list inference on known training population' if panel['partition']=='train' else 'Source-held-out lists; prior evaluation exposure disclosed in provenance',
        caveats=['Four-game blocks are the resampling units; panels with shared seeds are correlated.',
            'Forty appearances per archetype imply wide uncertainty; this is not human-strength evidence.',
            'Multiple pairwise comparisons are exploratory, not automatic promotion gates.'])
    out.mkdir(parents=True);write(out/'summary.json',summary);write(out/'behavior.json',behavior);write(out/'game-scores.json',rows)
    r=summary['results'];table=''.join(f'<tr><td>{html.escape(k)}</td><td>{v["wins"]}/{v["losses"]}/{v["draws"]}</td><td>{100*v["score"]:.1f}%</td><td>{100*v["ci95"][0]:.1f}–{100*v["ci95"][1]:.1f}%</td></tr>' for k,v in r['by_archetype'].items())
    (out/'index.html').write_text('<!doctype html><meta charset="utf-8"><title>Frozen model results</title><style>body{font:17px system-ui;max-width:1100px;margin:40px auto}td,th{padding:10px;text-align:left}</style><h1>'+html.escape(panel['id'])+f'</h1><p>{r["games"]} completed games; W/L/D {r["wins"]}/{r["losses"]}/{r["draws"]}. Candidate score {100*r["score"]:.2f}%, block95% interval {100*r["ci95"][0]:.2f}–{100*r["ci95"][1]:.2f}%.</p><p>Each archetype:40 appearances; uncertainty remains wide.</p><table><tr><th>Archetype</th><th>W/L/D</th><th>Score</th><th>Block95% interval</th></tr>'+table+'</table><p><a href="summary.json">Seats and opening-order results</a> · <a href="behavior.json">Timing, belief support and evolution routes</a> · <a href="game-scores.json">Game scores</a></p><ul>'+''.join('<li>'+html.escape(c)+'</li>' for c in summary['caveats'])+'</ul>',encoding='utf-8')
    write(out/'completion-verified.json',dict(at=time.time(),input_root=str(root.resolve()),manifest_sha256=digest(root/'manifest.json'),
        panel_completion_sha256=digest(root/'completion-verified.json'),reporter_sha256=digest(Path(__file__)),
        opportunity_reporter_sha256=digest(Path(__file__).with_name('search_opportunity_report.py')),
        outputs={p.name:digest(p) for p in out.iterdir() if p.is_file()}))
    return dict(panel=panel['id'],games=r['games'],out=str(out))


def compare(closed,opened,out):
    require(not out.exists(),'preserve paired comparison')
    a,ap,am,ar,_=load_panel(closed);b,bp,bm,br,_=load_panel(opened)
    require(a['stage']=='A' and b['stage']=='B' and a['paired_control']==b['id'],'matched A/B panels required')
    require(ap['schedule_manifest_sha256']==bp['schedule_manifest_sha256'],'same frozen schedule')
    for key in ('model_sha256','opponent_model_sha256','cards_sha256','prior_file_sha256','prior_snapshot_hash','decks'):
        require(am[key]==bm[key],'same runtime inputs: '+key)
    builds=[read(r/'inputs/build-provenance.json') for r in (closed,opened)]
    require(builds[0]['source_manifest_sha256']==builds[1]['source_manifest_sha256']
            and builds[0]['dependency_lock_sha256']==builds[1]['dependency_lock_sha256']
            and builds[0]['features']==builds[1]['features'],'same qualified source/dependencies/math')
    result=paired_effect(ar,br);result.update(closed_panel=a['id'],open_panel=b['id'],same_host_platform=ap['platform']==bp['platform'])
    out.mkdir(parents=True);write(out/'summary.json',result)
    (out/'index.html').write_text('<!doctype html><meta charset="utf-8"><title>Paired information-regime effect</title><style>body{font:17px system-ui;max-width:1000px;margin:40px auto}</style><h1>Open versus closed search</h1><p>'+html.escape(b['id'])+' compared with '+html.escape(a['id'])+f'</p><p>Candidate score difference: {100*result["mean"]:.2f} percentage points; paired-block95% interval {100*result["ci95"][0]:.2f} to {100*result["ci95"][1]:.2f} points. Positive values favor open/open search for this candidate matchup.</p><p>520 aligned games in130 four-game blocks. Subsequent trajectories may differ. Same host platform: '+str(result['same_host_platform'])+'. This is an exploratory paired comparison, not independent-panel evidence or a promotion gate.</p><p><a href="summary.json">Method and exact result</a></p>',encoding='utf-8')
    write(out/'completion-verified.json',dict(at=time.time(),inputs={name:dict(root=str(r.resolve()),manifest_sha256=digest(r/'manifest.json'),completion_sha256=digest(r/'completion-verified.json')) for name,r in [('closed',closed),('open',opened)]},reporter_sha256=digest(Path(__file__)),outputs={p.name:digest(p) for p in out.iterdir() if p.is_file()}))
    return result


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('command',choices=('report','compare'))
    for name in ('root','closed','opened','out'):parser.add_argument('--'+name,type=Path)
    a=parser.parse_args();print(json.dumps(report(a.root,a.out) if a.command=='report' else compare(a.closed,a.opened,a.out)))
