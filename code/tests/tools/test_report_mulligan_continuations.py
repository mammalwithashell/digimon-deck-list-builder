import copy
import pytest
from tools.cloud_search import report_mulligan_continuations as r


def nodes(count=2):
    return [dict(id=f'root{i}', namespace='primary' if i%2 else 'confirmation', block=i//2,
                 first=bool(i%2), mover=i%2, archetype=f'deck{i//4}') for i in range(count)]


def games(ns, cp=8):
    return [dict(root=n['id'], repeat=repeat, branch=branch, checkpoint=cp,
                 host='windows' if repeat==0 else 'linux', outcome_seed=100+i*2+repeat,
                 search_seed=200+i*2+repeat, score=float(branch==0))
            for i,n in enumerate(ns) for repeat in (0,1) for branch in (0,1)]


def test_root_average_paired_difference_and_branch_draw_scores():
    ns=nodes(1); rows=games(ns)
    rows[0]['score']=1; rows[1]['score']=0
    rows[2]['score']=0; rows[3]['score']=.5
    out=r.paired_summary(ns,rows,100)
    assert out['paired_outcomes']==2 and out['unique_roots']==1 and out['games']==4
    assert out['keep_minus_redraw']['mean']==.25
    assert out['keep_minus_redraw']['ci95']==[.25,.25]
    assert out['branches']['redraw']['draws']==1
    assert out['branches']['keep']['wins']==1


def test_source_block_is_indivisible():
    ns=nodes(2)
    for n in ns: n.update(namespace='primary',block=5)
    out=r.interval(ns,[1,-1],100)
    assert out['source_blocks']==1 and out['roots']==2
    assert out['ci95']==[0,0]


def test_namespaces_with_same_block_number_are_separate_strata():
    out=r.interval(nodes(2),[1,-1],100)
    assert out['source_blocks']==2 and out['ci95']==[0,0]


@pytest.mark.parametrize('mutation', ['missing','duplicate','host','outcome_seed','search_seed','score'])
def test_pair_integrity_fails(mutation):
    ns=nodes(); rows=games(ns)
    if mutation=='missing': rows.pop()
    elif mutation=='duplicate': rows.append(copy.deepcopy(rows[0]))
    elif mutation=='host': rows[0]['host']='other'
    elif mutation in ('outcome_seed','search_seed'): rows[0][mutation]+=1
    else: rows[0]['score']=.3
    with pytest.raises(ValueError): r.paired_summary(ns,rows,20)


def test_832_games_are_four_correlated_model_reports():
    ns=nodes(52); rows=sum((games(ns,cp) for cp in r.CHECKPOINTS),[])
    report=r.analyze(ns,rows,20)
    assert set(report)=={'8','12','16','20'}
    for value in report.values():
        assert value['games']==208 and value['unique_roots']==52 and value['paired_outcomes']==104
        assert value['branches']['keep']['games']==104
        assert all(a['unique_roots']==4 and a['games']==16 for a in value['archetypes'].values())
        assert value['keep_minus_redraw']['mean']==1
    with pytest.raises(ValueError): r.analyze(ns,rows[:-1],20)


def test_event_routes_and_forced_intervention_timing_stay_distinct():
    game=dict(candidate_seat=0,search_diagnostics=[],card_metrics={'steps':[
        dict(mover=1,legal_actions=[0,1],forced_intervention=True,selection_seconds=99,events=[]),
        dict(mover=1,legal_actions=[2,3],selection_seconds=.3,events=[
            dict(type='EvalDigivolve',player=0,field_index=14,was_dna=False),
            dict(type='Digivolve',player=0,field_index=0,was_dna=False),
            dict(type='Digivolve',player=0,field_index=0,was_dna=True),
            dict(type='BreedingMove',player=0),dict(type='Hatch',player=1)]),
        dict(mover=1,legal_actions=[5],selection_seconds=.2,events=[]) ]})
    b=r.Behavior();b.add(8,'linux',0,game);out=b.report()['groups']
    candidate=next(x for x in out if x['role']=='candidate')
    assert candidate['resolved_routes']==dict(digivolve_raising=1,digivolve_battle=1,digivolve_dna=1,promotion=1)
    assert candidate['timing']['nonforced_actions']==1 and candidate['timing']['total_seconds']==.3
    assert next(x for x in out if x['role']=='opponent')['resolved_routes']=={'hatch':1}


def test_missing_frozen_receipts_cannot_emit_report(tmp_path,monkeypatch):
    def incomplete(*args): raise ValueError('incomplete receipts')
    monkeypatch.setattr(r,'load_verified',incomplete)
    out=tmp_path/'report'
    with pytest.raises(ValueError,match='incomplete'): r.report(tmp_path/'w',tmp_path/'l',out)
    assert not out.exists()


def test_existing_report_never_overwritten(tmp_path):
    with pytest.raises(ValueError,match='preserve'):r.report(tmp_path/'w',tmp_path/'l',tmp_path)
