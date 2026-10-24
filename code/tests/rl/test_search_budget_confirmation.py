"""Cover seat/role provenance and telemetry exclusions in the native adapter."""
import copy
import csv
import json

import pytest

from digimon_gym.agents.card_eval import CardEvalCollector
from tools.search_budget_confirmation import collect_game, confirmation_jobs, normalized_event, write_reports


def test_full_grid_is_balanced_unique_and_uses_fresh_seeds():
    plan = {"decks": [{}] * 8, "match_seed": 2026102600, "search_seed": 2026102700}
    jobs = confirmation_jobs(plan)
    assert len(jobs) == len({name for name, _, _ in jobs}) == 64
    assert len({r["game_seed"] for _, _, r in jobs}) == 64
    assert {tuple(r["deck_indices"]) for _, _, r in jobs} == {(i, j) for i in range(8) for j in range(8)}
    assert all(r["card_metrics"] and r["candidate"]["simulations"] == 640 and r["opponent"]["simulations"] == 320 for _, _, r in jobs)


def test_native_event_adapter_preserves_effect_route_raising_and_seat():
    event = {"type": "Play", "player": 0, "card_id": "A", "card_name": "A name", "cost_paid": 0, "effect_initiated": True}
    assert normalized_event(event, True)["player"] == 2
    assert normalized_event(event)["meta"] == {"cost_paid": 0, "effect_initiated": True}
    event = {"type": "EvalDigivolve", "player": 1, "top_card_id": "B", "card_name": "B name", "field_index": 14, "was_dna": False, "memory_paid": 2}
    assert normalized_event(event, True)["source_slot"] == 14
    assert normalized_event(event, True)["player"] == 1
    assert normalized_event({"type": "MemoryChange"}) is None
    with pytest.raises(KeyError):
        normalized_event({"type": "Play", "player": 0, "card_id": "A"})


def fixture():
    ui = {"pendingSelection": False, "player1": {"handIds": ["A"]}, "player2": {"handIds": ["B"]}}
    plan = {"decks": [{"name": "DeckA", "cards": ["A"]}, {"name": "DeckB", "cards": ["B"]}]}
    record = {"deck_indices": [0, 1], "game_seed": 1}
    game = {"candidate_seat": 1, "winner": 1, "truncated": False, "terminal_reason": "security_attack", "decisions": 1,
        "card_metrics": {"schema": "native-card-eval-v1", "event_version": 1, "initial_ui": ui,
            "steps": [{"mover": 2, "phase": "Main", "legal_actions": [0], "action": 0, "after_ui": ui, "turn_count": 2,
                "events": [{"type": "Play", "player": 1, "card_id": "B", "card_name": "B", "cost_paid": 3, "effect_initiated": False}]}]}}
    return plan, record, game


def test_candidate_seat_two_is_remapped_to_budget_role_one():
    plan, record, game = fixture()
    collector = CardEvalCollector(0, 0)
    collect_game(collector, plan, record, game, 640)
    report = collector.report()
    b = next(r for r in report["cards"] if r["card_id"] == "B" and r["player"] == 1)
    assert b["wins"] == b["plays_direct"] == b["wins_when_entered"] == 1
    assert report["routes"][0]["choices"] == report["routes"][0]["opportunities"] == 1
    assert collector.games[0]["context"]["agent_archetype"] == "DeckB"


def test_pending_selections_excluded_and_capped_entries_not_outcome_labels():
    plan, record, game = fixture()
    game["truncated"] = True
    game["card_metrics"]["initial_ui"]["pendingSelection"] = True
    collector = CardEvalCollector(0, 0)
    collect_game(collector, plan, record, game, 640)
    report = collector.report()
    assert report["routes"] == []
    b = next(r for r in report["cards"] if r["card_id"] == "B" and r["player"] == 1)
    assert b["incomplete"] == 1 and b["completed_games_entered"] == b["wins_when_entered"] == 0


def test_completed_report_exports_budget_roles_without_doubling_game_count(tmp_path):
    plan, record, game = fixture()
    parent, out = tmp_path / "parent", tmp_path / "out"
    parent.mkdir(); out.mkdir(); (out / "results").mkdir()
    cards = tmp_path / "cards.json"
    cards.write_text(json.dumps({"A": {"card_name_eng": "Card A"}, "B": {"card_name_eng": "Card B"}}))
    model = tmp_path / "model.onnx"
    model.with_suffix(".onnx.meta.json").write_text('{}')
    plan.update(decks=[plan["decks"][0]], match_seed=1, search_seed=2, cards_json=str(cards),
                model=str(model), parent_study=str(parent), main_revision="test", regime="test", hashes={})
    (out / "plan.json").write_text(json.dumps(plan))
    (parent / "evaluation_extra_summary.json").write_text(json.dumps({"panels": {"s640-w4": {"s320-w4": {
        "mean_score": .65, "games": 32, "score_ci95": [.4, .8]}}}}))
    record["deck_indices"] = [0, 0]
    game["first_player"] = 0
    game["score"] = 1
    game["card_metrics"]["steps"][0].update(legal_actions=[0, 93], selection_seconds=.1)
    other = copy.deepcopy(game)
    other.update(candidate_seat=0, score=0)
    record["games"] = [other, game]
    (out / "results" / "confirm-00-00.json").write_text(json.dumps(record))
    write_reports(plan, out)
    summary = json.loads((out / "summary.json").read_text())
    assert summary["games"] == 2 and summary["seat_pairs"] == 1
    for budget in (320, 640):
        report = json.loads(next((out / "eval_cards" / f"s{budget}-w4").glob('*.json')).read_text())
        assert len(report["games"]) == 2
        assert {g["context"]["budget"] for g in report["games"]} == {f"s{budget}-w4"}
    rows = list(csv.DictReader((out / "cards.csv").open()))
    assert {r["budget"] for r in rows} == {"320", "640"} and {r["player"] for r in rows} == {"1"}
