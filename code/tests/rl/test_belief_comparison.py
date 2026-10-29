import pytest
from tools.run_belief_comparison import native_record


def test_native_evidence_preserves_every_requested_outcome_and_seat():
    m = {
        "games": 4,
        "opponent_games": [
            {"learner_seat": 0, "first_player": 0, "winner": 0, "truncated": False},
            {"learner_seat": 1, "first_player": 0, "winner": 0, "truncated": False},
            {"learner_seat": 0, "first_player": 0, "winner": None, "truncated": False},
            {"learner_seat": 1, "first_player": 0, "winner": None, "truncated": True},
        ],
    }
    r = native_record(m)
    assert (r["wins"], r["losses"], r["draws"], r["truncated"]) == (1, 1, 1, 1)
    assert r["first_seat_games"] == r["second_seat_games"] == 2
    assert (
        sum(r[k] for k in ("wins", "losses", "draws", "failed", "truncated"))
        == r["games"]
    )


def test_missing_native_game_rows_cannot_be_scored_as_completed():
    with pytest.raises(ValueError):
        native_record({"games": 2, "opponent_games": []})
