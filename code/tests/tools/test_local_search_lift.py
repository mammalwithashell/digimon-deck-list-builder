from collections import Counter
import copy

import pytest

from tools.cloud_search.local_search_lift import make_schedule, validate_record, SEARCH, POLICY


def decks():
    return [dict(name=f'arch{i:02d}',source_deck_id=f'{i}-{j}',cards=[])
            for i in range(13) for j in range(2 if i<8 else 1)]


def test_520_games_balance_every_archetype_role_opener_and_cover_all_lists():
    pool=decks(); rows=make_schedule(pool)
    assert len(rows)==260
    counts=Counter(); pairs=set(); used=set()
    for offset in range(0,len(rows),2):
        block=rows[offset:offset+2]
        assert block[0]['deck_indices']==block[1]['deck_indices']
        assert {r['game_seed']%2 for r in block}=={0,1}
        assert block[0]['block']==block[1]['block']
    for row in rows:
        used.update(row['deck_indices'])
        a,b=[pool[i]['name'] for i in row['deck_indices']]
        pairs.add(tuple(sorted((a,b))))
        for seat in range(2):
            counts[pool[row['deck_indices'][seat]]['name'],seat,row['game_seed']%2]+=1
    assert set(counts.values())=={10}
    assert len(counts)==52 and len(pairs)==91 and used==set(range(21))
    assert len({r['game_seed'] for r in rows})==260


def record():
    request=dict(candidate=SEARCH,opponent=POLICY,game_seed=2,deck_indices=[0,1])
    games=[dict(candidate_seat=s,first_player=0,winner=0,score=float(s==0),
                truncated=False,terminal_reason='Some(SecurityAttack)',actions=[62],decisions=1,
                card_metrics=dict(steps=[dict(action=62,legal_actions=[62],selection_seconds=.1)])) for s in range(2)]
    return request,dict(request,games=games)


def test_caps_and_crashes_never_become_draws():
    request,result=record()
    assert len(validate_record({},request,result))==2
    for key,value in [('truncated',True),('terminal_reason','Some(Crash)'),('terminal_reason','Some(StepLimit)')]:
        bad=copy.deepcopy(result); bad['games'][0][key]=value
        with pytest.raises(ValueError,match='Incomplete game'): validate_record({},request,bad)


def test_rejects_wrong_perspective_and_invalid_executed_actions():
    request,result=record()
    bad=copy.deepcopy(result); bad['games'][1]['score']=1
    with pytest.raises(ValueError,match='perspective'): validate_record({},request,bad)
    bad=copy.deepcopy(result); bad['games'][0]['card_metrics']['steps'][0]['legal_actions']=[0]
    with pytest.raises(ValueError,match='legal mask'): validate_record({},request,bad)
