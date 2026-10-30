import copy
import pytest
from digimon_gym.agents.belief_evaluation import score_cards, promotion, fingerprint


def protocol():
    return {
        "schema": "fair-belief-protocol-v1",
        "seeds": [11, 22, 33],
        "budgets": {"exploiter_steps": 2048},
        "panels": {
            "heldout": {
                "direction": "higher",
                "margin": 0.02,
                "minimum_games_per_seed": 32,
                "evaluation_mode": "policy_only",
            },
            "anchors": {
                "direction": "higher",
                "margin": -0.03,
                "minimum_games_per_seed": 32,
                "evaluation_mode": "policy_only",
            },
            "exploiter": {
                "direction": "lower",
                "margin": 0.03,
                "minimum_games_per_seed": 32,
                "evaluation_mode": "policy_only",
            },
        },
    }


def evidence():
    p = protocol()

    def row(wins):
        return {
            "wins": wins,
            "losses": 100000 - wins,
            "draws": 0,
            "failed": 0,
            "truncated": 0,
            "games": 100000,
            "first_seat_games": 50000,
            "second_seat_games": 50000,
        }

    panels = {}
    for name in p["panels"]:
        panels[name] = [
            {
                "seed": seed,
                "status": "completed",
                "evaluation_mode": "policy_only",
                "candidate": row(70000 if name != "exploiter" else 30000),
                "baseline": row(50000),
            }
            for seed in p["seeds"]
        ]
    return {
        "protocol_hash": fingerprint(p),
        "seeds": p["seeds"],
        "budgets": p["budgets"],
        "evaluation_information_regime": "closed_decklist",
        "panels": panels,
    }


def test_synthetic_success_passes_all_independent_gates():
    assert promotion(protocol(), evidence())["eligible"]


@pytest.mark.parametrize(
    "change",
    [
        "missing",
        "failed",
        "truncated",
        "seats",
        "budget",
        "regime",
        "prior_protocol",
        "mode",
        "uncertain",
        "denominator",
    ],
)
def test_invalid_or_inconclusive_evidence_blocks_promotion(change):
    p = protocol()
    r = evidence()
    target = r["panels"]["heldout"][0]
    if change == "denominator":
        target["candidate"]["losses"] -= 1
    if change == "missing":
        del r["panels"]["exploiter"]
    if change == "failed":
        target["status"] = "failed"
    if change == "truncated":
        target["candidate"]["truncated"] = 1
    if change == "seats":
        target["candidate"]["first_seat_games"] = 1
    if change == "budget":
        r["budgets"] = {"exploiter_steps": 1}
    if change == "regime":
        r["evaluation_information_regime"] = "open_decklist"
    if change == "prior_protocol":
        r["protocol_hash"] = "bad"
    if change == "mode":
        target["evaluation_mode"] = "search_assisted"
    if change == "uncertain":
        target["candidate"] = {
            "wins": 16,
            "games": 32,
            "first_seat_games": 16,
            "second_seat_games": 16,
        }
        target["baseline"] = copy.deepcopy(target["candidate"])
    assert not promotion(p, r)["eligible"]


def test_offline_proper_scores_reward_accurate_probabilities():
    truth = ["A", "A", "A", "B"]
    good = score_cards({"A": 0.75, "B": 0.25}, truth)
    bad = score_cards({"A": 0.25, "B": 0.75}, truth)
    assert good["log_loss"] < bad["log_loss"] and good["brier"] < bad["brier"]
    assert score_cards({"A": 1}, truth)["support_coverage"] == 0.75
    with pytest.raises(ValueError):
        score_cards({"A": 0.1}, truth)
