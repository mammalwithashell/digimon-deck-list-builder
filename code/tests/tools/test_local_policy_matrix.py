from collections import Counter

import pytest

from tools.cloud_search.local_policy_matrix import make_cells, reference_differences, summarize_cell
from tools.cloud_search.final_policy_eval import summarize


def test_full_grid_has_9100_games_and_balances_roles_openers_and_variants():
    decks={f'arch{i}':[f'{i}-a',f'{i}-b'] if i<8 else [f'{i}-a'] for i in range(13)}
    cells=make_cells(decks)
    assert len(cells)==91
    assert sum(len(c['rows']) for c in cells)==9100
    for cell in cells:
        rows=cell['rows']
        assert len(rows)==100
        if cell['panel']=='self':
            counts=Counter((r['deck1'],r['seed']%2) for r in rows)
            assert len(counts)==4 and set(counts.values())=={25}
            combinations=Counter(tuple(sorted((r['deck1_id'],r['deck2_id']))) for r in rows)
            assert len(combinations)==len(decks[cell['deck']])*len(decks[cell['opponent']])
            assert max(combinations.values())-min(combinations.values())<=4
            for offset in range(0,100,4):
                block=rows[offset:offset+4]
                assert len({r['block'] for r in block})==1
                assert len({tuple(sorted((r['deck1_id'],r['deck2_id']))) for r in block})==1
        else:
            assert all(r['deck1_id']==r['deck2_id'] for r in rows)
            assert Counter(r['seed']%2 for r in rows)=={0:50,1:50}
            assert len({r['seed'] for r in rows})==100


def test_result_directions_are_complements_and_mirror_uses_first_player():
    cells=make_cells({'a':['a1','a2'],'b':['b1']},games=4)
    for c in cells:
        games=[dict(r,winner=1 if r['seed']%2==0 else 2,terminal_reason='security_attack') for r in c['rows']]
        result=summarize(c['rows'],games,c['panel'])
        if c['panel']=='self':
            assert all(row['wins']==row['losses']==2 for row in result['matrix'])
        else:
            assert result['score']==1
            assert result['perspective']=='first_player'


@pytest.mark.parametrize('decks,games', [({},100),({'a':[]},100),({'a':['a','a']},100),({'a':['a']},2),({'a':['a']},99)])
def test_rejects_unbalanced_or_ambiguous_schedule(decks,games):
    with pytest.raises(ValueError):
        make_cells(decks,games)


def test_cloud_reference_comparison_ignores_table_order_but_preserves_behavior():
    game=dict(winner_id=1,terminal_reason='security_attack',steps=40,
              cards=[{'player':1,'plays':1},{'player':2,'plays':2}],
              routes=[],route_decisions=[],effect_sequences=[])
    actual=dict(game,cards=list(reversed(game['cards'])))
    assert reference_differences(game,actual)==[]
    actual['steps']=41
    actual['cards'][0]=dict(player=2,plays=3)
    assert reference_differences(game,actual)==['steps','cards']


def test_cell_breaks_out_opening_advantage_without_changing_balanced_result():
    cell=make_cells({'a':['a1'],'b':['b1']},games=4)[0]
    games=[dict(r,winner=1 if r['seed']%2==0 else 2,terminal_reason='security_attack') for r in cell['rows']]
    result=summarize_cell(cell,games)
    for row in result['matrix']:
        assert row['wins']==row['losses']==2
        assert row['wins_first']==row['games_first']==2
        assert row['losses_second']==row['games_second']==2
        assert row['wins_second']==0
