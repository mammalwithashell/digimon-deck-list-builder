from __future__ import annotations

import json
from types import SimpleNamespace
from unittest.mock import MagicMock, patch

import pytest

from digimon_gym.agents.card_eval import (
    CardEvalCollector, decision_routes, load_local_reports, load_wandb_reports,
    publish_report, save_report,
)


def board():
    return {"player1": {"handIds": ["A", "A", "B"], "battleArea": [{"topCardId": "C"}],
                        "breedingArea": {"topCardId": "D"}},
            "player2": {"handIds": ["X"]}, "pendingSelection": None}


def mask(*actions):
    values = [False] * 2192
    for action in actions:
        values[action] = True
    return values


def test_phase_scoped_opportunities_and_pending_id_reuse():
    ui = board()
    state = {"current_player_id": 1, "phase": "Main"}
    assert decision_routes(state, ui, mask(0, 1, 400, 401, 414, 100, 114)) == {
        0: ("A", "play"), 1: ("A", "play"), 400: ("A", "digivolve_battle"),
        401: ("A", "digivolve_battle"), 414: ("A", "digivolve_raising"),
        100: ("C", "attack"), 114: ("C", "attack")}
    ui["pendingSelection"] = {"kind": "SelectHand"}
    assert decision_routes(state, ui, mask(0, 400)) == {}
    ui["pendingSelection"] = None
    state["phase"] = "Mulligan"
    assert decision_routes(state, ui, mask(0, 1)) == {}
    state["phase"] = "BlockTiming"
    assert decision_routes(state, ui, mask(100)) == {}
    state["phase"] = "Breeding"
    assert decision_routes(state, ui, mask(61)) == {61: ("D", "promote")}


def test_deduplicated_decision_exposures_choices_and_seat():
    c = CardEvalCollector(100, 0)
    c.start_game(board(), (["A", "A", "B"], ["X"]), 42)
    c.before_step({"current_player_id": 1, "phase": "Main"}, board(), mask(0, 1, 400, 401), 1)
    c.before_step({"current_player_id": 2, "phase": "Main"}, board(), mask(0), 0)
    c.finish_game(1, "security_attack")
    routes = c.report()["routes"]
    assert [(r["player"], r["route"], r["opportunities"], r["choices"]) for r in routes] == [
        (r["player"], r["route"], 1, int(r["route"] == "play")) for r in routes]
    assert {r["player"] for r in routes} == {1, 2}


def test_resolved_entries_independent_of_choices_and_bo3_boundaries(tmp_path):
    c = CardEvalCollector(100, 0)
    c.context = {"agent_archetype": "deck", "opponent_archetype": "opp"}
    c.start_game(board(), (["A"], ["X"]), 7)
    events = [
        {"type": "Digivolve", "player": 1, "source_card_id": "A", "source_slot": 14,
         "meta": {"memory_paid": 2}},
        {"type": "BreedingMove", "player": 1, "source_card_id": "A"},
        {"type": "Digivolve", "player": 1, "source_card_id": "B", "source_slot": 0,
         "meta": {"was_dna": True, "memory_paid": 0}},
        {"type": "Play", "player": 2, "source_card_id": "X", "meta": {"cost_paid": 3}},
    ]
    c.after_step({"turn_count": 3, "winner_id": 1, "terminal_reason": "security_attack"}, board(), events, True, False)
    c.start_game(board(), (["A"], ["X"]), 8)
    c.after_step({"turn_count": 1, "winner_id": 2, "terminal_reason": "deck_out"}, board(), [], True, False)
    report = c.report()
    a = next(r for r in report["cards"] if r["card_id"] == "A")
    assert (a["games"], a["wins"], a["losses"], a["games_entered"]) == (2, 1, 1, 1)
    assert (a["digivolves_raising"], a["promotions"], a["memory_paid"]) == (1, 1, 2)
    assert a["wins_when_entered"] == 1
    b = next(r for r in report["cards"] if r["card_id"] == "B")
    assert b["digivolves_battle"] == b["dna_digivolves"] == 1
    save_report(report, tmp_path)
    assert load_local_reports(tmp_path) == [report]
    # Same training step after resume cannot overwrite the prior window.
    other = CardEvalCollector(100, 0).report()
    save_report(other, tmp_path)
    assert len(load_local_reports(tmp_path)) == 2


def test_incomplete_game_is_not_silently_a_draw():
    c = CardEvalCollector(0, 0)
    c.start_game(board(), (["A"], ["X"]), 0)
    c.start_game(board(), (["A"], ["X"]), 1)
    c.finish_game(None, "evaluation_end")
    a = next(r for r in c.report()["cards"] if r["card_id"] == "A")
    assert (a["incomplete"], a["draws"]) == (2, 0)


def test_telemetry_only_events_do_not_create_reward_occurrences():
    from digimon_gym.agents.reward.event_bus import RewardEventBus
    events = [{"type": kind, "player": 1, "source_card_id": "A", "meta": {}}
              for kind in ("EvalDigivolve", "Hatch", "BreedingMove")]
    from digimon_gym.agents.reward.occurrences import StepElapsed
    occurrences = RewardEventBus().derive({}, events, 1)
    assert len(occurrences) == 1 and isinstance(occurrences[0], StepElapsed)
    c = CardEvalCollector(0, 0)
    c.start_game(board(), (["A"], ["X"]), 0)
    c.after_step({"turn_count": 1}, board(), events, False, False)
    c.finish_game(1, "security_attack")
    a = next(r for r in c.report()["cards"] if r["card_id"] == "A")
    assert (a["hatches"], a["promotions"], a["digivolves_battle"]) == (1, 1, 1)


def test_shared_event_stream_and_real_environment_boundaries():
    from digimon_gym.digimon_gym import DigimonEnv, _make_reward_event_bus
    env = DigimonEnv()
    collector = CardEvalCollector(0, 0)
    env.card_eval_collector = collector
    env._reward_event_bus = _make_reward_event_bus()
    env.reset(seed=1)
    # Both consumers must see the same events; neither may drain twice.
    original = collector.after_step
    observed = []
    def observe(state, ui, events, terminated, truncated):
        observed.extend(events)
        original(state, ui, events, terminated, truncated)
    collector.after_step = observe
    for _ in range(8):
        _, _, terminated, truncated, info = env.step(env.greedy_action())
        assert "reward_occurrences" in info
        if terminated or truncated:
            break
    assert collector.current is not None or collector.games
    assert observed
    assert any(e["type"] in ("Play", "Digivolve", "TurnStart") for e in observed)
    env.close()


def test_direct_and_effect_plays_are_separate():
    c = CardEvalCollector(0, 0)
    c.start_game(board(), (["A"], ["X"]), 0)
    c.after_step({"turn_count": 2}, board(), [
        {"type": "Play", "player": 1, "source_card_id": "A", "meta": {"cost_paid": 0, "effect_initiated": False}},
        {"type": "Play", "player": 1, "source_card_id": "A", "meta": {"cost_paid": 0, "effect_initiated": True}},
    ], False, False)
    c.finish_game(1, "security_attack")
    a = next(r for r in c.report()["cards"] if r["card_id"] == "A")
    assert (a["plays"], a["plays_direct"], a["plays_effect"]) == (2, 1, 1)


def test_wandb_tables_artifact_roundtrip_and_failure_keeps_local(tmp_path):
    c = CardEvalCollector(123, 2)
    c.start_game(board(), (["A"], ["X"]), 3)
    c.before_step({"current_player_id": 1, "phase": "Main"}, board(), mask(0), 0)
    c.finish_game(1, "security_attack")
    report = c.report()
    path = save_report(report, tmp_path)
    wb = MagicMock()
    run = MagicMock(id="run1")
    with patch.dict("sys.modules", {"wandb": wb}):
        publish_report(report, path, run)
    assert wb.Table.call_count == 2
    run.log_artifact.assert_called_once()
    payload = run.log.call_args.args[0]
    assert payload["card_eval/checkpoint_step"] == 123
    assert "step" not in run.log.call_args.kwargs
    artifact = MagicMock(type="card-evaluation")
    artifact.get_path.return_value.download.return_value = str(path)
    api = MagicMock()
    api.run.return_value.logged_artifacts.return_value = [SimpleNamespace(type="other"), artifact]
    assert load_wandb_reports("entity/project/run", api) == [report]
    run.log_artifact.side_effect = RuntimeError("offline")
    with patch.dict("sys.modules", {"wandb": wb}):
        publish_report(report, path, run)
    assert json.loads(path.read_text(encoding="utf-8")) == report


def test_callback_scopes_collector_and_saves_on_failure(tmp_path):
    from digimon_gym.agents.pilot_training import WinRateCallback
    from digimon_gym.digimon_gym import DigimonEnv
    inner = DigimonEnv()
    cb = WinRateCallback(lambda: inner, card_eval_dir=tmp_path, verbose=0)
    cb.num_timesteps = 42
    def fail():
        inner.card_eval_collector.start_game(board(), (["A"], ["X"]), 2)
        raise RuntimeError("broken evaluation")
    with patch.object(cb, "_run_evaluation_inner", side_effect=fail), patch("digimon_engine.CARD_EVAL_EVENT_VERSION", 1, create=True):
        with pytest.raises(RuntimeError, match="broken evaluation"):
            cb._run_evaluation()
    assert inner.card_eval_collector is None
    reports = load_local_reports(tmp_path)
    assert reports[0]["checkpoint_step"] == 42
    assert reports[0]["games"][0]["terminal_reason"] == "evaluation_end"


def test_periodic_callback_collects_each_game_and_opponent_steps(tmp_path):
    from stable_baselines3.common.logger import configure
    from digimon_gym.agents.pilot_training import WinRateCallback, OpponentWrapper
    from digimon_gym.digimon_gym import DigimonEnv
    inner = DigimonEnv(max_turns=4)
    env = OpponentWrapper(inner)
    cb = WinRateCallback(lambda: env, n_eval_episodes=2, card_eval_dir=tmp_path, verbose=0)
    cb.model = SimpleNamespace(logger=configure(str(tmp_path / "logs"), []),
                               predict=lambda obs, **kwargs: (inner.greedy_action(), None))
    cb.num_timesteps = 250
    # Also exercises the collector against the installed wheel on dev machines
    # that have not rebuilt yet; native event payloads have separate Rust tests.
    with patch("digimon_engine.CARD_EVAL_EVENT_VERSION", 1, create=True):
        cb._run_evaluation()
    report = load_local_reports(tmp_path)[0]
    assert len(report["games"]) == 2
    assert all(g["terminal_reason"] not in ("evaluation_end", "reset_before_terminal") for g in report["games"])
    assert {r["player"] for r in report["routes"]} == {1, 2}
    assert sum(r["opportunities"] for r in report["routes"]) > 0
    assert inner.card_eval_collector is None
    cb.close()


def test_real_wandb_offline_artifact_and_tables(tmp_path):
    wandb = pytest.importorskip("wandb")
    collector = CardEvalCollector(42, 0)
    collector.start_game(board(), (["A"], ["X"]), 1)
    collector.before_step({"current_player_id": 1, "phase": "Main"}, board(), mask(0), 0)
    collector.finish_game(1, "security_attack")
    report = collector.report()
    path = save_report(report, tmp_path)
    with wandb.init(project="card-eval-test", mode="offline", dir=str(tmp_path),
                    settings=wandb.Settings(quiet=True)) as run:
        publish_report(report, path, run)
        assert run.summary["card_eval/checkpoint_step"] == 42
        assert run.summary["card_eval/cards"] is not None
        assert run.summary["card_eval/routes"] is not None
