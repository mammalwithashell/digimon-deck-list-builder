import copy
from collections import Counter
import pytest
from tools.cloud_search.prepare_frozen_mode_campaign import blocks,validate_blocks,panels,normalize


def decks(count):
    return [dict(name=f'arch{i:02d}',cards=[f'card{i}'],source_deck_id=f'list{i}') for i in range(count)]


@pytest.mark.parametrize('count,games',[(13,520),(12,480)])
def test_fixed_block_roles_seats_openers_and_variant_coverage(count,games):
    ds=decks(count)
    ds.extend(dict(name=ds[i]['name'],cards=[f'variant{i}'],source_deck_id=f'variant{i}') for i in range(8))
    rows=blocks(ds,count,11426103000,11526103000)
    proof=validate_blocks(ds,rows,count)
    assert proof['games']==games and proof['receipts']==games//2 and proof['four_game_blocks']==games//4
    assert len(proof['list_appearances'])==count+8
    assert sum(proof['list_appearances'].values())==games


def test_finite_panel_budget_and_exact_paired_modes():
    ps=panels()
    assert len(ps)==22 and sum(p['games'] for p in ps)==11200
    assert Counter(p['stage'] for p in ps)=={'A':6,'B':6,'C':4,'D':6}
    by_id={p['id']:p for p in ps}
    for a in [p for p in ps if p['stage']=='A']:
        b=by_id[a['paired_control']]
        assert all(a[k]==b[k] for k in ['checkpoint','opponent_checkpoint','partition','preferred_queue','games'])
        assert a['candidate_mode']==a['opponent_mode']=='closed-list'
        assert b['candidate_mode']==b['opponent_mode']=='known-list'
    for c in [p for p in ps if p['stage']=='C']:
        assert c['checkpoint']==c['opponent_checkpoint']
        assert (c['candidate_mode'],c['opponent_mode'])==('known-list','closed-list')


def test_heldout_variants_include_last_sorted_archetype():
    ds=decks(12)
    ds.extend(dict(name=ds[i]['name'],cards=[f'variant{i}'],source_deck_id=f'variant{i}') for i in range(12) if i!=4)
    proof=validate_blocks(ds,blocks(ds,12,100,200),12)
    assert len(proof['list_appearances'])==23
    assert proof['list_appearances']['list4']==40
    assert all(n==20 for k,n in proof['list_appearances'].items() if k!='list4')


@pytest.mark.parametrize('mutation',[lambda r:r.pop(),lambda r:r[0].update(candidate_seats=[0]),
    lambda r:r[1].update(expected_first_player=0),lambda r:r[1].update(search_seed=2),
    lambda r:r[0].update(deck_indices=[0,1])])
def test_rejects_damaged_block_schedule(mutation):
    ds=decks(13);rows=blocks(ds,13,100,200);mutation(rows)
    with pytest.raises(ValueError):validate_blocks(ds,rows,13)


def test_rejects_duplicate_exact_lists_and_wrong_denominators():
    snapshot=dict(decks=[dict(archetype='a',card_ids=['x'],deck_id='one'),dict(archetype='b',card_ids=['x'],deck_id='two')])
    with pytest.raises(ValueError,match='duplicate list content'):normalize(snapshot,2,2)
    with pytest.raises(ValueError,match='denominator'):normalize(snapshot,13,21)


def test_odd_namespace_cannot_break_opener_pairing():
    with pytest.raises(ValueError,match='even seed'):blocks(decks(13),13,101,200)
