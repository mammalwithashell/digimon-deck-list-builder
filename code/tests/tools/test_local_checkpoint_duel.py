from collections import Counter
from copy import deepcopy
from types import SimpleNamespace
from unittest.mock import patch

import pytest

from tools.cloud_search.local_checkpoint_duel import make_schedule, checked_outcome, summarize


def pool():
    return SimpleNamespace(archetypes={
        f'arch-{i:02d}': [SimpleNamespace(deck_id=f'deck-{i}-{j}', archetype_name=f'arch-{i:02d}')
                          for j in range(2 if i < 8 else 1)] for i in range(13)})


def test_frozen_schedule_has_equal_model_deck_seat_and_opener_exposure():
    blocks = make_schedule(pool())
    assert len(blocks) == 650
    exposure = Counter()
    decks = set()
    for b in blocks:
        assert len(b['rows']) == 4
        assert len({(r['deck1_id'], r['deck2_id']) for r in b['rows']}) == 1
        assert len({r['seed'] for r in b['rows']}) == 2
        for row in b['rows']:
            seat = row['cp12_seat']
            first = seat == (1 if row['seed'] % 2 == 0 else 2)
            exposure[row[f'deck{seat}_archetype'], seat, first] += 1
            decks.update((row['deck1_id'], row['deck2_id']))
    assert len(decks) == 21
    assert len(exposure) == 13 * 4
    assert set(exposure.values()) == {50}


def receipt():
    b = make_schedule(pool())[0]
    # P1 wins every game, so model CP12 must win precisely two of four.
    games = [checked_outcome(r, dict(seed=r['seed'], context=r, winner_id=1,
                                    terminal_reason='security_attack')) for r in b['rows']]
    return b, dict(block=b['id'], games=games)


def test_score_is_model_perspective_and_duplicate_or_missing_receipts_fail():
    b, r = receipt()
    with patch('tools.cloud_search.local_checkpoint_duel.interval', return_value=[0, 1]):
        result = summarize([b], [r])
    assert (result['wins'], result['losses'], result['score']) == (2, 2, .5)
    assert result['first']['score'] == result['second']['score'] == .5
    assert result['engine_player1']['score'] == 1
    assert result['engine_player2']['score'] == 0
    with pytest.raises(ValueError, match='Missing or duplicate'):
        summarize([b], [r, r])
    with pytest.raises(ValueError, match='Missing or duplicate'):
        summarize([b], [])


@pytest.mark.parametrize('field,value', [('score', 0), ('seed', -1), ('terminal_reason', 'step_limit')])
def test_changed_or_rejected_results_fail_closed(field, value):
    b, r = receipt()
    altered = deepcopy(r)
    altered['games'][0][field] = value
    with pytest.raises(ValueError):
        summarize([b], [altered])


def test_draw_and_incomplete_are_distinct():
    row = make_schedule(pool())[0]['rows'][0]
    game = dict(seed=row['seed'], context=row, winner_id=None, terminal_reason='draw')
    assert checked_outcome(row, game)['score'] == .5
    for reason in ['evaluation_end', 'crash', 'step_limit', 'truncated', None]:
        with pytest.raises(ValueError):
            checked_outcome(row, dict(game, terminal_reason=reason))
