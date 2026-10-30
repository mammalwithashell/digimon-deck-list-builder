import pytest
from tools.run_belief_comparison import native_record, training_flags, exploiter_parameters


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


def test_cloud_comparison_uses_frozen_warm_start_and_runbook_budgets():
    flags = training_flags({"training": {
        "uniform_initial": False, "full_search": False,
        "reduced_simulations": 64, "cheap_simulations": 64,
        "full_search_fraction": 0.25, "batch_size": 512,
        "replay_generations": 3, "fail_on_truncation": True, "decision_cap": 1200,
    }})
    assert "--uniform" not in flags
    assert "--full-search" not in flags
    assert flags[flags.index("--batch-size") + 1] == 512
    assert flags[flags.index("--full-search-fraction") + 1] == 0.25
    assert "--fail-on-truncation" in flags
    assert flags[flags.index("--decision-cap") + 1] == 1200
    assert "--uniform" in training_flags({})  # Original registered study stays reproducible.


def test_fresh_exploiter_can_match_frozen_target_capacity_without_mutating_it():
    target = {"net_arch": {"pi": [128, 128], "vf": [128, 128]},
              "features_extractor_kwargs": {"embedding_dim": 8}}
    result = exploiter_parameters({"exploiter": {"capacity": "match_target", "n_steps": 512,
        "batch_size": 512, "learning_rate": 1e-4}}, target)
    assert result["policy_kwargs"] == target
    result["policy_kwargs"]["net_arch"]["pi"].append(1)
    assert target["net_arch"]["pi"] == [128, 128]
    assert result["n_steps"] == result["batch_size"] == 512
    assert exploiter_parameters({}, target)["policy_kwargs"]["net_arch"]["pi"] == [32]
    with pytest.raises(ValueError, match="capacity"):
        exploiter_parameters({"exploiter": {"capacity": "typo"}}, target)
