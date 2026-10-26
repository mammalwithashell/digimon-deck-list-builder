from collections import Counter

import pytest

from tools.cloud_search.final_policy_eval import schedule, summarize


def test_self_schedule_balances_every_pair_seat_and_opening():
    names = [f'deck{i}' for i in range(13)]
    rows = schedule(names, 'self', 7)
    assert len(rows) == 3120
    cells = Counter((r['deck1'], r['deck2'], r['seed'] % 2) for r in rows)
    assert len(cells) == 13*12*2
    assert set(cells.values()) == {10}
    assert all(r['model1'] == r['model2'] == 4 for r in rows)
    appearances = Counter(d for r in rows for d in [r['deck1'], r['deck2']])
    assert set(appearances.values()) == {480}


def test_adjacent_schedule_balances_model_roles_and_opening():
    rows = schedule([str(i) for i in range(13)], 'adjacent', 8)
    assert len(rows) == 6760
    cells = Counter((r['deck1'], r['deck2'], r['model1'], r['seed'] % 2) for r in rows)
    assert len(cells) == 13*13*4
    assert set(cells.values()) == {10}
    assert all({r['model1'], r['model2']} == {3, 4} for r in rows)


def outcomes(rows, winner):
    return [dict(r, winner=winner(r), terminal_reason='security_attack') for r in rows]


def test_self_summary_attributes_both_players_not_just_player_one():
    rows = schedule(['a', 'b'], 'self', 1, 2)
    result = summarize(rows, outcomes(rows, lambda r: 1 if r['deck1'] == 'a' else 2), 'self')
    assert result['games'] == 8
    assert result['decks'][0]['score'] == 1
    assert result['decks'][1]['score'] == 0
    assert all(d['games'] == 8 for d in result['decks'])


def test_adjacent_summary_normalizes_checkpoint_four_on_both_sides():
    rows = schedule(['a', 'b'], 'adjacent', 2, 2)
    result = summarize(rows, outcomes(rows, lambda r: 1 if r['model1'] == 4 else 2), 'adjacent')
    assert result['score'] == 1
    assert result['ci95'] == [1, 1]
    assert result['wins'] == 32
    assert result['losses'] == 0


def test_first_player_advantage_cancels_in_adjacent_comparison():
    rows = schedule(['a', 'b'], 'adjacent', 2, 2)
    result = summarize(rows, outcomes(rows, lambda r: 1 if r['seed'] % 2 == 0 else 2), 'adjacent')
    assert result['score'] == .5
    assert result['ci95'] == [.5, .5]


def test_summary_rejects_duplicate_missing_changed_and_incomplete_games():
    rows = schedule(['a', 'b'], 'self', 1, 1)
    games = outcomes(rows, lambda r: 1)
    for bad in [games[:-1], games[:-1]+[games[0]],
                [dict(games[0], seed=99)]+games[1:],
                [dict(games[0], terminal_reason='truncated')]+games[1:]]:
        with pytest.raises(ValueError):
            summarize(rows, bad, 'self')
