from collections import Counter
from copy import deepcopy
from types import SimpleNamespace

import pytest

from tools.cloud_search.head_to_head import make_schedule, summarize


def pool(count=13):
    return SimpleNamespace(archetypes={str(i): [SimpleNamespace(
        archetype_name=str(i), deck_id=f"deck-{i}")] for i in range(count)})


def games(schedule, mixed_winner=1, development_winner=1):
    return {arm: [dict(seed=row["seed"], context={k:v for k,v in row.items() if k not in ("seed","index")},
                      terminal_reason="security_attack", winner_id=winner) for row in schedule]
            for arm,winner in [("mixed",mixed_winner),("development",development_winner)]}


def test_full_grid_balances_all_deck_cells_and_openers():
    rows=make_schedule(pool(),10,91)
    assert len(rows)==3380 and len({r['seed'] for r in rows})==3380
    counts=Counter((r['agent_archetype'],r['opponent_archetype'],r['seed']%2) for r in rows)
    assert len(counts)==13*13*2 and set(counts.values())=={10}
    for a,b in zip(rows[::2],rows[1::2]):
        assert a['agent_deck_id']==b['agent_deck_id'] and a['opponent_deck_id']==b['opponent_deck_id']
    result=summarize(games(rows),rows,samples=20)
    assert result['games']==6760 and result['score']==.5
    assert len(result['matrix'])==169 and {c['games'] for c in result['matrix']}=={40}


def test_mixed_perspective_reverses_second_player_results():
    rows=make_schedule(pool(2),1,19)
    result=summarize(games(rows,1,2),rows,samples=20)
    assert result['score']==1 and result['wins']==16 and result['ci95']==[1,1]
    result=summarize(games(rows,0,None),rows,samples=20)
    assert result['score']==.5 and result['draws']==16


@pytest.mark.parametrize('damage',['missing','duplicate','truncated','deck','winner'])
def test_incomplete_or_changed_cohort_is_rejected(damage):
    rows=make_schedule(pool(2),1,19)
    reports=games(rows)
    if damage=='missing': reports['mixed'].pop()
    if damage=='duplicate': reports['mixed'].append(deepcopy(reports['mixed'][0]))
    if damage=='truncated': reports['mixed'][0]['terminal_reason']='step_limit'
    if damage=='deck': reports['mixed'][0]['context']['agent_deck_id']='different'
    if damage=='winner': reports['mixed'][0]['winner_id']=8
    with pytest.raises(ValueError):
        summarize(reports,rows,samples=20)
