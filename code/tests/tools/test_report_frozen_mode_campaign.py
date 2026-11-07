import copy
import pytest
from tools.cloud_search import report_frozen_mode_campaign as r
from tools.cloud_search.prepare_frozen_mode_campaign import blocks


def scores(count=13):
    decks=[dict(name=f'arch{i:02d}',cards=[str(i)],source_deck_id=str(i)) for i in range(count)]
    requests=blocks(decks,count,100,200)
    return [dict(request=q['id'],block=q['block'],game_seed=q['game_seed'],search_seed=q['search_seed'],
        deck_indices=q['deck_indices'],candidate_seat=seat,archetype=decks[q['deck_indices'][seat]]['name'],
        first=seat==q['expected_first_player'],score=float(q['block']%2)) for q in requests for seat in (0,1)]


@pytest.mark.parametrize('count,expected',[(13,520),(12,480)])
def test_complete_panel_split_denominators(count,expected):
    result=r.analyze(scores(count),expected,draws=80)
    assert result['games']==expected and result['blocks']==expected//4
    assert len(result['by_archetype'])==count
    assert all(v['games']==40 for v in result['by_archetype'].values())
    assert all(v['games']==expected//2 for v in result['by_seat'].values())
    assert all(v['games']==expected//2 for v in result['by_opening_order'].values())


@pytest.mark.parametrize('mutation',[lambda rows:rows.pop(),lambda rows:rows.__setitem__(0,rows[1]),
    lambda rows:rows[0].update(first=not rows[0]['first']),lambda rows:rows[0].update(archetype='missing')])
def test_incomplete_duplicate_or_unbalanced_panels_fail(mutation):
    rows=scores();mutation(rows)
    with pytest.raises(ValueError):r.analyze(rows,520,draws=20)


def test_four_game_block_never_split_by_bootstrap():
    result=r.block_estimate([dict(block=0,score=x) for x in [1,0,1,0]],draws=100)
    assert result['ci95']==[.5,.5] and result['blocks']==1


def test_draws_count_half_and_keep_separate_denominator():
    result=r.block_estimate([dict(block=0,score=x) for x in [1,.5,0,.5]],draws=20)
    assert result['score']==.5 and result['draws']==2 and result['wins']==result['losses']==1


def test_paired_effect_uses_block_differences():
    closed=scores();opened=copy.deepcopy(closed)
    for index,(a,b) in enumerate(zip(closed,opened)):
        if index%4==0:a['score']=0;b['score']=1
    result=r.paired_effect(closed,opened,draws=100)
    assert result['paired_blocks']==130 and result['paired_games']==520
    assert result['mean']==.25 and result['ci95']==[.25,.25]


@pytest.mark.parametrize('key',['game_seed','search_seed','block','archetype','deck_indices'])
def test_paired_comparison_rejects_schedule_drift(key):
    a=scores();b=copy.deepcopy(a);b[0][key]='different'
    with pytest.raises(ValueError,match='schedule mismatch'):r.paired_effect(a,b,draws=20)


def test_behavior_retains_distinct_routes_and_belief_denominators(monkeypatch):
    monkeypatch.setattr(r,'opportunity_summary',lambda rows:dict(counts={'searched_roots':len(rows)},means={}))
    info=dict(regime='closed-list',compatible_candidates=0,fallback_worlds=4,sampled_worlds=4,sampling_seconds=.1,
              fallback_mass=1.,mixture_component_ess=1.)
    game=dict(candidate_seat=0,search_diagnostics=[dict(decision=0,opportunity={},diagnostics=dict(viewer=0,seconds=.2,information=info))],
        card_metrics=dict(steps=[dict(selection_seconds=.3,events=[dict(type='Digivolve',player=0,field_index=14),
            dict(type='Digivolve',player=0,field_index=2),dict(type='Digivolve',player=0,field_index=2,was_dna=True),
            dict(type='BreedingMove',player=0)])]))
    behavior=r.Behavior();behavior.add(game);result=behavior.report()['roles']
    candidate=result['candidate'];assert candidate['resolved_routes']==dict(digivolve_raising=1,digivolve_battle=1,digivolve_dna=1,promotion=1)
    assert candidate['closed_roots']==candidate['fallback_roots']==1 and candidate['sampled_worlds']==4
    assert candidate['searched_root_selection_seconds']['mean']==.3
    assert result['opponent']['games']==1 and result['opponent']['closed_roots']==0
    assert result['opponent']['belief_sampling_seconds']['mean'] is None


def test_no_artifact_on_incomplete_panel(tmp_path,monkeypatch):
    def failed(*a):raise ValueError('frozen complete-panel verification failed')
    monkeypatch.setattr(r,'load_panel',failed)
    out=tmp_path/'report'
    with pytest.raises(ValueError):r.report(tmp_path,out)
    assert not out.exists()
